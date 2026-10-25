//! Which settings to recommend for a game.
//!
//! A staff-blessed report wins. Without one, every setting takes the value
//! most reports chose: Rust-DOS offers a fixed set of values, so an average
//! of 128 and 256 MB would be meaningless, and one odd report shouldn't move
//! the advice. Green reports count fully, yellow half; red reports describe
//! what didn't work, so they don't vote.

use super::entities::{blessing, game, report, report_setting, user};
use super::error::Result;
use crate::model::{Rating, RatingSummary, Recommendation, RecommendationSource, RecommendedSetting, Scope};
use crate::settings_schema;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::{BTreeMap, HashMap};

/// Below this many voting reports for a version, reports for every version count.
pub const MIN_VERSION_REPORTS: usize = 3;

#[derive(Clone, Debug)]
pub struct Vote {
    pub rating: Rating,
    pub created_at: i64,
    /// (section, key, value), normalised.
    pub settings: Vec<(String, String, String)>,
}

fn weight(r: Rating) -> f64 {
    match r {
        Rating::Green => 1.0,
        Rating::Yellow => 0.5,
        Rating::Red => 0.0,
    }
}

/// Per (order, section, key): how many reports set it, and each value's tally.
type Tallies = BTreeMap<(usize, String, String), (u32, HashMap<String, Tally>)>;

struct Tally {
    score: f64,
    count: u32,
    latest_green: Option<i64>,
}

pub fn consensus(votes: &[Vote]) -> Vec<RecommendedSetting> {
    let mut by_key: Tallies = BTreeMap::new();
    for v in votes.iter().filter(|v| weight(v.rating) > 0.0) {
        for (section, key, value) in &v.settings {
            let entry = by_key
                .entry((settings_schema::order(section, key), section.clone(), key.clone()))
                .or_insert_with(|| (0, HashMap::new()));
            entry.0 += 1;
            let t = entry.1.entry(value.clone()).or_insert(Tally { score: 0.0, count: 0, latest_green: None });
            t.score += weight(v.rating);
            t.count += 1;
            if v.rating == Rating::Green {
                t.latest_green = t.latest_green.max(Some(v.created_at));
            }
        }
    }

    let mut out = Vec::new();
    for ((_, section, key), (voters, tallies)) in by_key {
        let def = settings_schema::find(&section, &key);
        let top = tallies.values().map(|t| t.score).fold(f64::MIN, f64::max);
        let mut tied: Vec<(&String, &Tally)> = tallies.iter().filter(|(_, t)| (t.score - top).abs() < 1e-9).collect();
        if tied.len() > 1 {
            // The value of the most recent green report among the tied ones.
            let newest = tied.iter().filter_map(|(_, t)| t.latest_green).max();
            if newest.is_some() {
                tied.retain(|(_, t)| t.latest_green == newest);
            }
        }
        if tied.len() > 1
            && let Some(def) = def
        {
            // Numeric settings: the value nearest the median of everyone's choices.
            let mut all: Vec<f64> = tallies
                .iter()
                .filter_map(|(v, t)| def.numeric_value(v).map(|n| std::iter::repeat_n(n, t.count as usize)))
                .flatten()
                .collect();
            if !all.is_empty() {
                all.sort_by(f64::total_cmp);
                let median = all[all.len() / 2];
                tied.sort_by(|(a, _), (b, _)| {
                    let da = def.numeric_value(a).map_or(f64::MAX, |n| (n - median).abs());
                    let db = def.numeric_value(b).map_or(f64::MAX, |n| (n - median).abs());
                    da.total_cmp(&db).then_with(|| a.cmp(b))
                });
                tied.truncate(1);
            }
        }
        tied.sort_by_key(|(v, _)| *v);
        let (value, tally) = tied[0];
        out.push(RecommendedSetting { section, key, value: value.clone(), agree: Some(tally.count), voters: Some(voters) });
    }
    out
}

async fn votes_for(db: &DatabaseConnection, reports: &[report::Model]) -> Result<Vec<Vote>> {
    let ids: Vec<i32> = reports.iter().map(|r| r.id).collect();
    let mut settings: HashMap<i32, Vec<(String, String, String)>> = HashMap::new();
    if !ids.is_empty() {
        for s in report_setting::Entity::find().filter(report_setting::Column::ReportId.is_in(ids)).all(db).await? {
            settings.entry(s.report_id).or_default().push((s.section, s.key, s.value));
        }
    }
    Ok(reports
        .iter()
        .filter_map(|r| {
            Some(Vote {
                rating: Rating::parse(&r.rating)?,
                created_at: r.created_at,
                settings: settings.remove(&r.id).unwrap_or_default(),
            })
        })
        .collect())
}

fn ini_of(settings: &[RecommendedSetting]) -> String {
    settings_schema::to_ini(settings.iter().map(|s| (s.section.as_str(), s.key.as_str(), s.value.as_str())))
}

/// The recommendation for `game`, for one version or for any.
pub async fn recommend(db: &DatabaseConnection, game: &game::Model, version_id: Option<i32>) -> Result<Recommendation> {
    let reports = report::Entity::find()
        .filter(report::Column::GameId.eq(game.id))
        .filter(report::Column::Hidden.eq(false))
        .all(db)
        .await?;
    let for_version: Vec<report::Model> = match version_id {
        Some(v) => reports.iter().filter(|r| r.version_id == Some(v)).cloned().collect(),
        None => reports.clone(),
    };
    let rating = RatingSummary::from_ratings(for_version.iter().filter_map(|r| Rating::parse(&r.rating)));

    // A blessing for this version, then one for the whole game.
    let blessings = blessing::Entity::find().filter(blessing::Column::GameId.eq(game.id)).all(db).await?;
    let mut keys = vec![0];
    if let Some(v) = version_id {
        keys.insert(0, v);
    }
    for key in keys {
        let Some(b) = blessings.iter().find(|b| b.version_key == key) else { continue };
        // A hidden report's blessing is suspended, not followed.
        let Some(r) = reports.iter().find(|r| r.id == b.report_id) else { continue };
        let vote = votes_for(db, std::slice::from_ref(r)).await?.pop();
        let settings: Vec<RecommendedSetting> = vote
            .map(|v| v.settings)
            .unwrap_or_default()
            .into_iter()
            .map(|(section, key, value)| RecommendedSetting { section, key, value, agree: None, voters: None })
            .collect();
        let blessed_by = match b.user_id {
            Some(uid) => user::Entity::find_by_id(uid).one(db).await?.map(|u| u.username),
            None => None,
        };
        return Ok(Recommendation {
            game: game.slug.clone(),
            version_id: version_id.map(i64::from),
            scope: if key == 0 { Scope::Game } else { Scope::Version },
            source: RecommendationSource::Blessed { report_id: r.id.into(), blessed_by },
            rating,
            ini: ini_of(&settings),
            settings,
        });
    }

    let voting = |rs: &[report::Model]| rs.iter().filter(|r| Rating::parse(&r.rating).is_some_and(|x| weight(x) > 0.0)).count();
    let (pool, scope) = match version_id {
        Some(_) if voting(&for_version) >= MIN_VERSION_REPORTS => (for_version, Scope::Version),
        _ => (reports, Scope::Game),
    };
    let votes = votes_for(db, &pool).await?;
    let settings = consensus(&votes);
    let voters = votes.iter().filter(|v| weight(v.rating) > 0.0 && !v.settings.is_empty()).count() as u32;
    Ok(Recommendation {
        game: game.slug.clone(),
        version_id: version_id.map(i64::from),
        scope,
        source: if settings.is_empty() { RecommendationSource::None } else { RecommendationSource::Consensus { reports: voters } },
        rating,
        ini: ini_of(&settings),
        settings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(rating: Rating, at: i64, settings: &[(&str, &str, &str)]) -> Vote {
        Vote { rating, created_at: at, settings: settings.iter().map(|(s, k, v)| (s.to_string(), k.to_string(), v.to_string())).collect() }
    }

    fn value_of<'a>(r: &'a [RecommendedSetting], key: &str) -> &'a RecommendedSetting {
        r.iter().find(|s| s.key == key).unwrap()
    }

    #[test]
    fn most_common_value_wins_over_outlier() {
        use Rating::*;
        let votes = vec![
            vote(Green, 1, &[("emulator", "memsize", "128")]),
            vote(Green, 2, &[("emulator", "memsize", "128")]),
            vote(Green, 3, &[("emulator", "memsize", "256")]),
        ];
        let r = consensus(&votes);
        let m = value_of(&r, "memsize");
        assert_eq!((m.value.as_str(), m.agree, m.voters), ("128", Some(2), Some(3)));
    }

    #[test]
    fn red_reports_do_not_vote_and_yellow_counts_half() {
        use Rating::*;
        let votes = vec![
            vote(Red, 1, &[("sound", "sbtype", "awe32")]),
            vote(Red, 2, &[("sound", "sbtype", "awe32")]),
            vote(Yellow, 3, &[("sound", "sbtype", "sbpro2")]),
            vote(Yellow, 4, &[("sound", "sbtype", "sbpro2")]),
            vote(Green, 5, &[("sound", "sbtype", "sb16")]),
            vote(Green, 6, &[("sound", "sbtype", "sb16")]),
        ];
        let r = consensus(&votes);
        assert_eq!(value_of(&r, "sbtype").value, "sb16");
        assert_eq!(value_of(&r, "sbtype").voters, Some(4));
    }

    #[test]
    fn tie_goes_to_most_recent_green() {
        use Rating::*;
        let votes = vec![vote(Green, 10, &[("emulator", "cpu", "486")]), vote(Green, 20, &[("emulator", "cpu", "pentium")])];
        assert_eq!(value_of(&consensus(&votes), "cpu").value, "pentium");
    }

    #[test]
    fn yellow_only_tie_uses_median_for_numbers() {
        use Rating::*;
        let votes = vec![
            vote(Yellow, 1, &[("emulator", "memsize", "8")]),
            vote(Yellow, 2, &[("emulator", "memsize", "16")]),
            vote(Yellow, 3, &[("emulator", "memsize", "64")]),
        ];
        assert_eq!(value_of(&consensus(&votes), "memsize").value, "16");
    }

    #[test]
    fn keys_are_independent_and_in_schema_order() {
        use Rating::*;
        let votes = vec![
            vote(Green, 1, &[("sound", "sbtype", "sb16"), ("emulator", "cpu", "486")]),
            vote(Green, 2, &[("emulator", "cpu", "486")]),
        ];
        let r = consensus(&votes);
        assert_eq!(r.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(), ["cpu", "sbtype"]);
        assert_eq!(value_of(&r, "sbtype").voters, Some(1));
    }
}

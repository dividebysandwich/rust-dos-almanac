use super::games::{find_version, find_visible};
use super::{id32, log_action};
use crate::model::*;
use crate::server::auth::{Actor, now};
use crate::server::entities::{blessing, game, game_version, report, report_setting, user};
use crate::server::error::{Error, Result};
use crate::server::recommend;
use crate::server::state::AppState;
use crate::settings_schema;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};
use std::collections::{HashMap, HashSet};

const MAX_DESCRIPTION: usize = 4000;

fn description(d: Option<&str>) -> Result<Option<String>> {
    let Some(d) = d.map(str::trim).filter(|d| !d.is_empty()) else { return Ok(None) };
    if d.chars().count() > MAX_DESCRIPTION {
        return Err(Error::invalid(format!("the description is longer than {MAX_DESCRIPTION} characters")));
    }
    Ok(Some(d.to_string()))
}

fn rust_dos_version(v: Option<&str>) -> Result<Option<String>> {
    let Some(v) = v.map(str::trim).filter(|v| !v.is_empty()) else { return Ok(None) };
    let v = v.strip_prefix('v').unwrap_or(v);
    if v.len() > 32 || !v.chars().all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c)) {
        return Err(Error::invalid("the Rust-DOS version looks like 1.7.5"));
    }
    Ok(Some(v.to_string()))
}

fn settings(map: &SettingsMap) -> Result<Vec<(String, String, String)>> {
    settings_schema::normalize_all(map).map_err(Error::Invalid)
}

async fn write_settings(db: &impl ConnectionTrait, report_id: i32, settings: Vec<(String, String, String)>) -> Result<()> {
    report_setting::Entity::delete_many().filter(report_setting::Column::ReportId.eq(report_id)).exec(db).await?;
    for (section, key, value) in settings {
        report_setting::ActiveModel { report_id: Set(report_id), section: Set(section), key: Set(key), value: Set(value) }.insert(db).await?;
    }
    Ok(())
}

/// Reports as the actor may see them, with settings, labels and authors.
pub async fn infos(db: &DatabaseConnection, actor: &Actor, reports: Vec<report::Model>) -> Result<Vec<ReportInfo>> {
    let ids: Vec<i32> = reports.iter().map(|r| r.id).collect();
    let mut settings: HashMap<i32, Vec<Setting>> = HashMap::new();
    let mut labels: HashMap<i32, String> = HashMap::new();
    let mut authors: HashMap<i32, String> = HashMap::new();
    let mut blessed: HashSet<i32> = HashSet::new();
    if !ids.is_empty() {
        for s in report_setting::Entity::find().filter(report_setting::Column::ReportId.is_in(ids.clone())).all(db).await? {
            settings.entry(s.report_id).or_default().push(Setting { section: s.section, key: s.key, value: s.value });
        }
        let vids: Vec<i32> = reports.iter().filter_map(|r| r.version_id).collect();
        if !vids.is_empty() {
            for v in game_version::Entity::find().filter(game_version::Column::Id.is_in(vids)).all(db).await? {
                labels.insert(v.id, v.label);
            }
        }
        let uids: Vec<i32> = reports.iter().filter_map(|r| r.user_id).collect();
        if !uids.is_empty() {
            for u in user::Entity::find().filter(user::Column::Id.is_in(uids)).all(db).await? {
                authors.insert(u.id, u.username);
            }
        }
        for b in blessing::Entity::find().filter(blessing::Column::ReportId.is_in(ids)).all(db).await? {
            blessed.insert(b.report_id);
        }
    }
    Ok(reports
        .into_iter()
        .filter_map(|r| {
            let mut s = settings.remove(&r.id).unwrap_or_default();
            s.sort_by_key(|x| settings_schema::order(&x.section, &x.key));
            Some(ReportInfo {
                id: r.id.into(),
                game_id: r.game_id.into(),
                version_id: r.version_id.map(i64::from),
                version_label: r.version_id.and_then(|v| labels.get(&v).cloned()),
                rating: Rating::parse(&r.rating)?,
                description: r.description,
                rust_dos_version: r.rust_dos_version,
                settings: s,
                hidden: r.hidden,
                blessed: blessed.contains(&r.id),
                author: r.user_id.and_then(|u| authors.get(&u).cloned()),
                source: actor.is_staff().then_some(r.source_hash),
                created_at: r.created_at,
            })
        })
        .collect())
}

pub async fn for_game(db: &DatabaseConnection, actor: &Actor, slug: &str) -> Result<Vec<ReportInfo>> {
    let g = find_visible(db, actor, slug).await?;
    let mut q = report::Entity::find().filter(report::Column::GameId.eq(g.id));
    if !actor.is_staff() {
        q = q.filter(report::Column::Hidden.eq(false));
    }
    let rows = q.order_by_desc(report::Column::CreatedAt).limit(500).all(db).await?;
    infos(db, actor, rows).await
}

async fn visible_report(db: &DatabaseConnection, actor: &Actor, id: i64) -> Result<report::Model> {
    let r = report::Entity::find_by_id(id32(id)?).one(db).await?.ok_or(Error::NotFound)?;
    if r.hidden && !actor.is_staff() {
        return Err(Error::NotFound);
    }
    let g = game::Entity::find_by_id(r.game_id).one(db).await?.ok_or(Error::NotFound)?;
    if g.hidden && !actor.is_staff() {
        return Err(Error::NotFound);
    }
    Ok(r)
}

pub async fn get(db: &DatabaseConnection, actor: &Actor, id: i64) -> Result<ReportInfo> {
    let r = visible_report(db, actor, id).await?;
    infos(db, actor, vec![r]).await?.pop().ok_or(Error::NotFound)
}

pub async fn create(state: &AppState, actor: &Actor, input: NewReport) -> Result<ReportInfo> {
    let db = &state.db;
    let g = find_visible(db, actor, input.game.trim()).await?;
    let version_id = match input.version_id {
        Some(v) => Some(find_version(db, actor, g.id, v).await?.id),
        None => None,
    };
    let settings = settings(&input.settings)?;
    // Only staff write experience notes; anonymous text would need moderating first.
    let description = if actor.is_staff() { description(input.description.as_deref())? } else { None };
    let rdv = rust_dos_version(input.rust_dos_version.as_deref())?;
    actor.spend_write(state)?;

    let txn = db.begin().await?;
    let t = now();
    let r = report::ActiveModel {
        game_id: Set(g.id),
        version_id: Set(version_id),
        rating: Set(input.rating.as_str().into()),
        description: Set(description),
        rust_dos_version: Set(rdv),
        user_id: Set(actor.user_id()),
        source_hash: Set(actor.source.clone()),
        hidden: Set(false),
        created_at: Set(t),
        updated_at: Set(t),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    write_settings(&txn, r.id, settings).await?;
    let mut gm = g.into_active_model();
    gm.updated_at = Set(t);
    gm.update(&txn).await?;
    txn.commit().await?;
    infos(db, actor, vec![r]).await?.pop().ok_or(Error::NotFound)
}

pub async fn update(state: &AppState, actor: &Actor, id: i64, patch: ReportPatch) -> Result<ReportInfo> {
    let staff = actor.staff()?.clone();
    let db = &state.db;
    let r = report::Entity::find_by_id(id32(id)?).one(db).await?.ok_or(Error::NotFound)?;
    let mut changes = Vec::new();
    let mut m = r.clone().into_active_model();
    if let Some(rating) = patch.rating {
        m.rating = Set(rating.as_str().into());
        changes.push("rating");
    }
    if let Some(v) = patch.version_id {
        let v = match v {
            Some(v) => Some(find_version(db, actor, r.game_id, v).await?.id),
            None => None,
        };
        m.version_id = Set(v);
        changes.push("version");
    }
    if let Some(d) = &patch.description {
        m.description = Set(description(d.as_deref())?);
        changes.push("description");
    }
    if let Some(v) = &patch.rust_dos_version {
        m.rust_dos_version = Set(rust_dos_version(v.as_deref())?);
        changes.push("rust-dos version");
    }
    if let Some(h) = patch.hidden {
        m.hidden = Set(h);
        changes.push(if h { "hidden" } else { "shown" });
    }
    let new_settings = patch.settings.as_ref().map(settings).transpose()?;
    m.updated_at = Set(now());

    let txn = db.begin().await?;
    let r = m.update(&txn).await?;
    if let Some(s) = new_settings {
        write_settings(&txn, r.id, s).await?;
        changes.push("settings");
    }
    // Moving a blessed report to another version would leave the blessing pointing elsewhere.
    if patch.version_id.is_some() {
        blessing::Entity::delete_many().filter(blessing::Column::ReportId.eq(r.id)).exec(&txn).await?;
    }
    txn.commit().await?;
    let action = match patch.hidden {
        Some(true) => "hide",
        Some(false) => "unhide",
        None => "edit",
    };
    log_action(db, id32(staff.id)?, action, "report", r.id, Some(changes.join(", "))).await?;
    infos(db, actor, vec![r]).await?.pop().ok_or(Error::NotFound)
}

/// Make a report's settings the recommended ones for its game and version.
pub async fn bless(state: &AppState, actor: &Actor, id: i64) -> Result<ReportInfo> {
    let staff = actor.staff()?.clone();
    let db = &state.db;
    let r = report::Entity::find_by_id(id32(id)?).one(db).await?.ok_or(Error::NotFound)?;
    if r.hidden {
        return Err(Error::invalid("a hidden report can't be blessed"));
    }
    let version_key = r.version_id.unwrap_or(0);
    let txn = db.begin().await?;
    blessing::Entity::delete_many()
        .filter(blessing::Column::GameId.eq(r.game_id))
        .filter(blessing::Column::VersionKey.eq(version_key))
        .exec(&txn)
        .await?;
    blessing::ActiveModel {
        game_id: Set(r.game_id),
        version_key: Set(version_key),
        report_id: Set(r.id),
        user_id: Set(Some(id32(staff.id)?)),
        blessed_at: Set(now()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    log_action(db, id32(staff.id)?, "bless", "report", r.id, None).await?;
    infos(db, actor, vec![r]).await?.pop().ok_or(Error::NotFound)
}

/// Drop the blessing for a game and version (None: the game-wide one).
pub async fn unbless(state: &AppState, actor: &Actor, slug: &str, version_id: Option<i64>) -> Result<()> {
    let staff = actor.staff()?.clone();
    let db = &state.db;
    let g = find_visible(db, actor, slug).await?;
    let key = match version_id {
        Some(v) => id32(v)?,
        None => 0,
    };
    let removed = blessing::Entity::delete_many()
        .filter(blessing::Column::GameId.eq(g.id))
        .filter(blessing::Column::VersionKey.eq(key))
        .exec(db)
        .await?;
    if removed.rows_affected == 0 {
        return Err(Error::NotFound);
    }
    log_action(db, id32(staff.id)?, "unbless", "game", g.id, version_id.map(|v| format!("version {v}"))).await?;
    Ok(())
}

pub async fn recommendation(db: &DatabaseConnection, actor: &Actor, slug: &str, version_id: Option<i64>) -> Result<Recommendation> {
    let g = find_visible(db, actor, slug).await?;
    let version = match version_id {
        Some(v) => Some(find_version(db, actor, g.id, v).await?.id),
        None => None,
    };
    recommend::recommend(db, &g, version).await
}

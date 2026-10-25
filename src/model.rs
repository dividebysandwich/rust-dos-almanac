//! Data shared by the API, the server functions and the browser.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `{section: {key: value}}`, as Rust-DOS config files group them.
pub type SettingsMap = BTreeMap<String, BTreeMap<String, String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Green,
    Yellow,
    Red,
}

impl Rating {
    pub const ALL: [Rating; 3] = [Rating::Green, Rating::Yellow, Rating::Red];

    pub fn as_str(self) -> &'static str {
        match self {
            Rating::Green => "green",
            Rating::Yellow => "yellow",
            Rating::Red => "red",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "green" => Some(Rating::Green),
            "yellow" => Some(Rating::Yellow),
            "red" => Some(Rating::Red),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rating::Green => "Runs great",
            Rating::Yellow => "Playable, some problems",
            Rating::Red => "Not playable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Moderator,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Moderator => "moderator",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "moderator" | "mod" => Some(Role::Moderator),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserAdminInfo {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RatingSummary {
    pub green: u32,
    pub yellow: u32,
    pub red: u32,
    /// The colour most reports give; on a tie, the worse one. None without reports.
    pub overall: Option<Rating>,
}

impl RatingSummary {
    pub fn from_ratings(ratings: impl IntoIterator<Item = Rating>) -> Self {
        let mut s = RatingSummary::default();
        for r in ratings {
            match r {
                Rating::Green => s.green += 1,
                Rating::Yellow => s.yellow += 1,
                Rating::Red => s.red += 1,
            }
        }
        let counts = [(Rating::Red, s.red), (Rating::Yellow, s.yellow), (Rating::Green, s.green)];
        // Worse colours come first, so max_by_key's "last maximum wins" is wrong here: pick by hand.
        let mut best: Option<(Rating, u32)> = None;
        for (r, n) in counts {
            if n > 0 && best.is_none_or(|(_, b)| n > b) {
                best = Some((r, n));
            }
        }
        s.overall = best.map(|(r, _)| r);
        s
    }

    pub fn total(&self) -> u32 {
        self.green + self.yellow + self.red
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub year: Option<i32>,
    pub publisher: Option<String>,
    pub hidden: bool,
    pub rating: RatingSummary,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameDetail {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub year: Option<i32>,
    pub publisher: Option<String>,
    pub hidden: bool,
    pub versions: Vec<VersionInfo>,
    pub rating: RatingSummary,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: i64,
    pub label: String,
    pub hidden: bool,
    pub files: Vec<FileInfo>,
    /// RetroAchievements-style archive hashes, as Rust-DOS computes them.
    pub ra_md5: Vec<String>,
    pub rating: RatingSummary,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub section: String,
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportInfo {
    pub id: i64,
    pub game_id: i64,
    pub version_id: Option<i64>,
    pub version_label: Option<String>,
    pub rating: Rating,
    pub description: Option<String>,
    pub rust_dos_version: Option<String>,
    pub settings: Vec<Setting>,
    pub hidden: bool,
    pub blessed: bool,
    /// Username of the staff member who wrote it; None for anonymous reports.
    pub author: Option<String>,
    /// The hashed source address. Only sent to staff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Drawn from reports for the requested version.
    Version,
    /// Drawn from reports for any version of the game.
    Game,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RecommendationSource {
    /// A staff member picked one report's settings.
    Blessed { report_id: i64, blessed_by: Option<String> },
    /// The most common value per setting across reports.
    Consensus { reports: u32 },
    /// Nobody has reported settings yet.
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecommendedSetting {
    pub section: String,
    pub key: String,
    pub value: String,
    /// Reports that chose this value (consensus only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agree: Option<u32>,
    /// Reports that set this key at all (consensus only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voters: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub game: String,
    pub version_id: Option<i64>,
    pub scope: Scope,
    pub source: RecommendationSource,
    pub rating: RatingSummary,
    pub settings: Vec<RecommendedSetting>,
    /// The settings as a Rust-DOS config fragment.
    pub ini: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewVersion {
    pub label: String,
    #[serde(default)]
    pub files: Vec<FileInfo>,
    #[serde(default)]
    pub ra_md5: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewGame {
    pub name: String,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub version: Option<NewVersion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewReport {
    /// The game's slug.
    pub game: String,
    #[serde(default)]
    pub version_id: Option<i64>,
    pub rating: Rating,
    #[serde(default)]
    pub settings: SettingsMap,
    #[serde(default)]
    pub rust_dos_version: Option<String>,
    /// Staff only; ignored for anonymous submissions.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GamePatch {
    pub name: Option<String>,
    pub year: Option<Option<i32>>,
    pub publisher: Option<Option<String>>,
    pub aliases: Option<Vec<String>>,
    pub hidden: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionPatch {
    pub label: Option<String>,
    pub hidden: Option<bool>,
    /// Replaces the version's files.
    pub files: Option<Vec<FileInfo>>,
    /// Replaces the version's archive hashes.
    pub ra_md5: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportPatch {
    pub rating: Option<Rating>,
    pub version_id: Option<Option<i64>>,
    pub description: Option<Option<String>>,
    /// Replaces all settings.
    pub settings: Option<SettingsMap>,
    pub rust_dos_version: Option<Option<String>>,
    pub hidden: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LookupRequest {
    #[serde(default)]
    pub ra_md5: Option<String>,
    #[serde(default)]
    pub files: Vec<FileInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LookupMatch {
    pub game: String,
    pub name: String,
    pub version_id: i64,
    pub version_label: String,
    pub score: u32,
    pub ra_match: bool,
    /// Files that matched by name and checksum.
    pub hash_matches: u32,
    /// Files that matched by name only (one side had no checksum).
    pub name_matches: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModLogEntry {
    pub id: i64,
    pub user: String,
    pub action: String,
    pub target_type: String,
    pub target_id: i64,
    pub detail: Option<String>,
    pub at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModReport {
    pub report: ReportInfo,
    pub game_slug: String,
    pub game_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaSetting {
    pub section: String,
    pub key: String,
    pub label: String,
    /// The accepted values, or None where any value in range is accepted (cycles).
    pub options: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub version: u32,
    pub settings: Vec<SchemaSetting>,
}

pub fn schema_info() -> SchemaInfo {
    SchemaInfo {
        version: crate::settings_schema::SCHEMA_VERSION,
        settings: crate::settings_schema::SETTINGS
            .iter()
            .map(|d| SchemaSetting { section: d.section.into(), key: d.key.into(), label: d.label.into(), options: d.options() })
            .collect(),
    }
}

/// `games/<slug>`: lowercase ASCII words joined by dashes.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Text to search on: lowercase letters and digits, single spaces.
pub fn search_key(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Files typed one per line: `NAME [SIZE] [CRC32] [MD5] [SHA1]`. Hashes are
/// told apart by length (8, 32, 40 hex digits); `size=`, `crc32=`, `md5=`
/// and `sha1=` prefixes make a token explicit.
pub fn parse_file_lines(text: &str) -> Result<Vec<FileInfo>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let mut words = line.split_whitespace();
        let Some(name) = words.next() else { continue };
        let mut f = FileInfo { name: name.to_string(), ..Default::default() };
        for w in words {
            let lower = w.to_ascii_lowercase();
            let (tag, v) = lower.split_once('=').map_or((None, lower.as_str()), |(t, v)| (Some(t), v));
            let hex = !v.is_empty() && v.bytes().all(|b| b.is_ascii_hexdigit());
            let decimal = !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit());
            let unclear = || format!("line {}: can't tell what '{w}' is", n + 1);
            match (tag, v.len()) {
                (Some("size"), _) | (None, _) if decimal && (tag.is_some() || v.len() != 8) => {
                    f.size = Some(v.parse().map_err(|_| unclear())?);
                }
                (Some("crc32"), 1..=8) | (None, 8) if hex => f.crc32 = Some(v.to_string()),
                (Some("md5") | None, 32) if hex => f.md5 = Some(v.to_string()),
                (Some("sha1") | None, 40) if hex => f.sha1 = Some(v.to_string()),
                _ => return Err(unclear()),
            }
        }
        out.push(f);
    }
    Ok(out)
}

pub fn file_lines(files: &[FileInfo]) -> String {
    files
        .iter()
        .map(|f| {
            let mut parts = vec![f.name.clone()];
            if let Some(s) = f.size {
                parts.push(format!("size={s}"));
            }
            parts.extend([&f.crc32, &f.md5, &f.sha1].into_iter().flatten().cloned());
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// "2026-10-25" for a Unix timestamp, without a date library.
pub fn format_date(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_majority_and_ties() {
        use Rating::*;
        assert_eq!(RatingSummary::from_ratings([Green, Green, Red]).overall, Some(Green));
        assert_eq!(RatingSummary::from_ratings([Green, Yellow]).overall, Some(Yellow));
        assert_eq!(RatingSummary::from_ratings([Green, Red]).overall, Some(Red));
        assert_eq!(RatingSummary::from_ratings([]).overall, None);
    }

    #[test]
    fn slugs_and_dates() {
        assert_eq!(slugify("Commander Keen 4: Secret of the Oracle"), "commander-keen-4-secret-of-the-oracle");
        assert_eq!(slugify("  DOOM II "), "doom-ii");
        assert_eq!(search_key("Duke Nukem 3D (Atomic)"), "duke nukem 3d atomic");
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(1_792_886_400), "2026-10-25");
    }

    #[test]
    fn file_lines_round_trip() {
        let text = "KEEN4E.EXE 263046 1a2b3c4d\nDATA.DAT size=12345678 d41d8cd98f00b204e9800998ecf8427e\n\n";
        let files = parse_file_lines(text).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].size, Some(263046));
        assert_eq!(files[0].crc32.as_deref(), Some("1a2b3c4d"));
        assert_eq!(files[1].size, Some(12_345_678));
        assert_eq!(files[1].md5.as_deref(), Some("d41d8cd98f00b204e9800998ecf8427e"));
        assert_eq!(parse_file_lines(&file_lines(&files)).unwrap(), files);
        assert!(parse_file_lines("A.EXE xyz").is_err());
    }
}

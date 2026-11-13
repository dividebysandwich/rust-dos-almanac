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

/// The system a version of a game runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// A DOS program (MZ, COM or batch).
    Dos,
    /// A Windows 3.1 program (NE), run on Rust-DOS's built-in Windows.
    Win16,
    /// A Windows 95/98 program (PE), run on Rust-DOS's built-in Windows.
    Win32,
}

impl Platform {
    pub const ALL: [Platform; 3] = [Platform::Dos, Platform::Win16, Platform::Win32];

    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Dos => "dos",
            Platform::Win16 => "win16",
            Platform::Win32 => "win32",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dos" => Some(Platform::Dos),
            "win16" | "win3" | "win31" | "ne" => Some(Platform::Win16),
            "win32" | "win9x" | "win95" | "pe" => Some(Platform::Win32),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Platform::Dos => "DOS",
            Platform::Win16 => "Windows 3.1",
            Platform::Win32 => "Windows 95/98",
        }
    }

    /// Short text for a badge.
    pub fn badge(self) -> &'static str {
        match self {
            Platform::Dos => "DOS",
            Platform::Win16 => "Win 3.1",
            Platform::Win32 => "Win 9x",
        }
    }

    /// Whether it needs Rust-DOS's built-in Windows (`win32=true`).
    pub fn is_windows(self) -> bool {
        matches!(self, Platform::Win16 | Platform::Win32)
    }
}

/// What a game search keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformFilter {
    #[default]
    All,
    Dos,
    /// Windows 3.1 or 95/98.
    Windows,
}

impl PlatformFilter {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "all" => Some(PlatformFilter::All),
            "dos" => Some(PlatformFilter::Dos),
            "windows" | "win" => Some(PlatformFilter::Windows),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PlatformFilter::All => "all",
            PlatformFilter::Dos => "dos",
            PlatformFilter::Windows => "windows",
        }
    }

    /// The stored platform values it keeps; None keeps everything.
    pub fn platforms(self) -> Option<&'static [&'static str]> {
        match self {
            PlatformFilter::All => None,
            PlatformFilter::Dos => Some(&["dos"]),
            PlatformFilter::Windows => Some(&["win16", "win32"]),
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
    /// The platforms of its visible versions that have one.
    #[serde(default)]
    pub platforms: Vec<Platform>,
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
    /// The platforms of its versions that have one.
    #[serde(default)]
    pub platforms: Vec<Platform>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: i64,
    pub label: String,
    pub hidden: bool,
    /// None when nobody said.
    #[serde(default)]
    pub platform: Option<Platform>,
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
    /// The player's own words about how it went.
    #[serde(default)]
    pub notes: Option<String>,
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
    /// When it last changed: its sender's key replaced it, or staff edited it.
    #[serde(default)]
    pub updated_at: i64,
}

/// What `POST /reports` answers: the report, and whether it replaced the
/// one its key sent before rather than adding one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportSaved {
    #[serde(flatten)]
    pub report: ReportInfo,
    pub replaced: bool,
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
    /// Not from reports: the version's platform calls for it (a Windows
    /// program needs `win32=true`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub implied: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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
    pub platform: Option<Platform>,
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
    /// The adder's key (see `NewReport::key`): their reports may correct
    /// the year and publisher until staff edit the game.
    #[serde(default)]
    pub key: Option<String>,
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
    /// The player's own words about how it went, shown with the report.
    #[serde(default)]
    pub notes: Option<String>,
    /// A secret of the sender's, such as a Rust-DOS game profile's UUID
    /// (see `valid_key`): a report with a key replaces the one sent with
    /// the same key on the same game before. Only its SHA-256 is stored.
    #[serde(default)]
    pub key: Option<String>,
    /// The game's year and publisher as the sender knows them: they fill
    /// in what the game lacks, and correct what the sender's key added.
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub publisher: Option<String>,
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
    /// Some(None) clears it.
    pub platform: Option<Option<Platform>>,
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
    pub notes: Option<Option<String>>,
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
    #[serde(default)]
    pub platform: Option<Platform>,
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
    /// Where the report form shows it: machine, windows, sound or input.
    #[serde(default)]
    pub group: String,
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
            .map(|d| SchemaSetting {
                section: d.section.into(),
                key: d.key.into(),
                label: d.label.into(),
                group: d.group().into(),
                options: d.options(),
            })
            .collect(),
    }
}

/// Whether `key` can be a report's key: 16 to 64 letters, digits and
/// dashes, as a UUID is. Shorter ones would be guessed.
pub fn valid_key(key: &str) -> bool {
    (16..=64).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// What a pasted Rust-DOS game profile (`games/<id>.conf`) or
/// configuration file gives a form: `[game]`'s name, year, publisher,
/// notes and UUID, and the settings the almanac knows, normalised.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PastedConfig {
    pub name: Option<String>,
    pub year: Option<i32>,
    pub publisher: Option<String>,
    pub notes: Option<String>,
    /// `[game]`'s `uuid`, the report's key.
    pub key: Option<String>,
    /// (section, key, value), in the schema's order.
    pub settings: Vec<(String, String, String)>,
    /// Settings of the machine's sections that the almanac doesn't take
    /// (host matters such as `scale`), as `section.key`.
    pub ignored: Vec<String>,
    /// Lines that couldn't be taken, and why.
    pub problems: Vec<String>,
    /// Settings whose value came under an old name, as `section.key`, so a
    /// later line under the current name replaces it.
    from_old: Vec<String>,
}

/// Read a Rust-DOS configuration text as `PastedConfig`. Section and key
/// names are case-insensitive, `#` and `;` start comments, and sections
/// the almanac doesn't take (`[drives]`, `[autoexec]`, ...) are skipped.
pub fn parse_config(text: &str) -> PastedConfig {
    let mut out = PastedConfig::default();
    let mut section = String::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
        let opt = || (!value.is_empty()).then(|| value.to_string());
        if section == "game" {
            match key.as_str() {
                "name" => out.name = opt(),
                "publisher" => out.publisher = opt(),
                "notes" => out.notes = opt(),
                "uuid" if valid_key(value) => out.key = Some(value.to_string()),
                "uuid" => out.problems.push(format!("line {}: '{value}' isn't a UUID", n + 1)),
                "year" if value.is_empty() => {}
                "year" => match value.parse() {
                    Ok(y) => out.year = Some(y),
                    Err(_) => out.problems.push(format!("line {}: the year '{value}' isn't a number", n + 1)),
                },
                _ => {}
            }
            continue;
        }
        if !crate::settings_schema::SECTIONS.contains(&section.as_str()) {
            continue;
        }
        let Some(def) = crate::settings_schema::find(&section, &key) else {
            out.ignored.push(format!("{section}.{key}"));
            continue;
        };
        // Under an old name, a value already given under the current one stays.
        let old = crate::settings_schema::is_old_name(&section, &key);
        if old && out.settings.iter().any(|(s, k, _)| s == def.section && k == def.key) && !out.from_old.contains(&def.id()) {
            continue;
        }
        match def.normalize(value) {
            Ok(v) => {
                if old {
                    out.from_old.push(def.id());
                } else {
                    out.from_old.retain(|id| *id != def.id());
                }
                out.settings.retain(|(s, k, _)| !(s == def.section && k == def.key));
                out.settings.push((def.section.to_string(), def.key.to_string(), v));
            }
            Err(e) => out.problems.push(format!("line {}: {e}", n + 1)),
        }
    }
    out.settings.sort_by_key(|(s, k, _)| crate::settings_schema::order(s, k));
    out
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

    #[test]
    fn a_pasted_profile_gives_its_game_and_the_settings_the_almanac_takes() {
        let text = "\
[game]
name=Commander Keen 4
Year = 1991
publisher=Apogee
notes=Smooth with the Gravis
uuid=0f8fad5b-d9cb-469f-a165-70867728950e
overlay=true

[emulator]
; a comment
CPU=586
memsize=16MB
scale=3
cycles=auto 20000

[Sound]
sbbase=0x220
sbtype=sb99

[drives]
c=/home/someone/keen

[autoexec]
KEEN4E
";
        let p = parse_config(text);
        assert_eq!(p.name.as_deref(), Some("Commander Keen 4"));
        assert_eq!(p.year, Some(1991));
        assert_eq!(p.publisher.as_deref(), Some("Apogee"));
        assert_eq!(p.notes.as_deref(), Some("Smooth with the Gravis"));
        assert_eq!(p.key.as_deref(), Some("0f8fad5b-d9cb-469f-a165-70867728950e"));
        let settings: Vec<String> = p.settings.iter().map(|(s, k, v)| format!("{s}.{k}={v}")).collect();
        assert_eq!(settings, ["emulator.cpu=pentium", "emulator.cycles=auto 20000", "emulator.memsize=16", "sound.sbbase=220"]);
        assert_eq!(p.ignored, ["emulator.scale"]);
        assert_eq!(p.problems.len(), 1, "{:?}", p.problems);
        assert!(p.problems[0].contains("sb99"));
    }

    #[test]
    fn a_pasted_voodoo_fps_cap_is_taken_as_fps_cap() {
        let settings = |text: &str| parse_config(text).settings.into_iter().map(|(s, k, v)| format!("{s}.{k}={v}")).collect::<Vec<_>>();
        assert_eq!(settings("[emulator]\nvoodoo_fps_cap=60\n"), ["emulator.fps_cap=60"]);
        assert_eq!(settings("[emulator]\nfps_cap=off\nvoodoo_fps_cap=60\n"), ["emulator.fps_cap=off"]);
        assert_eq!(settings("[emulator]\nvoodoo_fps_cap=60\nfps_cap=off\n"), ["emulator.fps_cap=off"]);
        assert_eq!(settings("[emulator]\nvoodoo_fps_cap=30\nvoodoo_fps_cap=60\n"), ["emulator.fps_cap=60"]);
        let p = parse_config("[emulator]\nfps_cap=300\n");
        assert!(p.settings.is_empty());
        assert_eq!(p.problems.len(), 1);
    }

    #[test]
    fn keys_are_long_and_plain() {
        assert!(valid_key("0f8fad5b-d9cb-469f-a165-70867728950e"));
        assert!(!valid_key("short"));
        assert!(!valid_key("0f8fad5b d9cb 469f a165 70867728950e"));
        let p = parse_config("[game]\nuuid=abc\n");
        assert_eq!(p.key, None);
        assert_eq!(p.problems.len(), 1);
    }
}

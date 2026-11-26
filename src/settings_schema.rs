//! The Rust-DOS settings the almanac knows about, with the values Rust-DOS
//! accepts for each (`rust-dos/src/config/parse.rs` and `types.rs`).
//!
//! Only settings that describe the emulated machine are here. Host-side
//! display options, `[drives]`, `[autoexec]` and file paths are left out on
//! purpose: a public database must not hand commands or paths to emulators.

use std::collections::BTreeMap;

/// Bumped whenever a key or a value domain changes, so clients can notice.
pub const SCHEMA_VERSION: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    /// One of a fixed list. Aliases map to their canonical value.
    Choice(&'static [&'static str], &'static [(&'static str, &'static str)]),
    Bool,
    /// Whole MB, optional "MB" suffix.
    MemSize,
    /// One of the listed integers, optionally followed by one of the listed
    /// unit suffixes ("8mb", "512k").
    IntSet(&'static [i64], &'static [&'static str]),
    /// A hex port: "220", "220h" or "0x220", one of the listed values.
    HexPort(&'static [u16]),
    /// `max`, `auto`, `auto N` or N.
    Cycles,
    /// "6.22", "7.1", "5".
    DosVersion,
    /// A frame rate cap: `auto`, `off` (or `0`), or 10 to 240 frames a second.
    FpsCap,
    /// A locale name such as "ru-RU", in any case, with a hyphen or an underscore.
    Locale(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct SettingDef {
    pub section: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub domain: Domain,
}

/// The groups the report form shows settings in, by id and title. Only the
/// form uses them: config files still go by section.
pub const GROUPS: &[(&str, &str)] = &[("machine", "Machine"), ("windows", "Windows"), ("sound", "Sound"), ("input", "Input")];

impl SettingDef {
    pub fn id(&self) -> String {
        format!("{}.{}", self.section, self.key)
    }

    /// Which of [`GROUPS`] the form shows it in.
    pub fn group(&self) -> &'static str {
        match (self.section, self.key) {
            ("emulator", "win32" | "win32_version" | "win32_depth" | "win32_desktop" | "win32_locale") => "windows",
            ("emulator", _) => "machine",
            ("sound", _) => "sound",
            _ => "input",
        }
    }

    /// Values to offer in a drop-down, or None for free entry.
    pub fn options(&self) -> Option<Vec<String>> {
        match self.domain {
            Domain::Choice(values, _) => Some(values.iter().map(|v| v.to_string()).collect()),
            Domain::Bool => Some(vec!["true".into(), "false".into()]),
            Domain::IntSet(values, _) => Some(values.iter().map(|v| v.to_string()).collect()),
            Domain::HexPort(values) => Some(values.iter().map(|v| format!("{v:x}")).collect()),
            Domain::MemSize => Some(MEMSIZE_PRESETS.iter().map(|v| v.to_string()).collect()),
            Domain::DosVersion => Some(DOS_PRESETS.iter().map(|v| v.to_string()).collect()),
            Domain::Locale(values) => Some(values.iter().map(|v| v.to_string()).collect()),
            Domain::FpsCap => Some(["auto", "off"].into_iter().map(String::from).chain(FPS_CAP_PRESETS.iter().map(|v| v.to_string())).collect()),
            Domain::Cycles => None,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self.domain, Domain::MemSize | Domain::IntSet(..) | Domain::Cycles | Domain::FpsCap)
    }

    /// The canonical form of `raw`, or why it isn't valid.
    pub fn normalize(&self, raw: &str) -> Result<String, String> {
        let value = raw.trim().to_ascii_lowercase();
        let bad = || format!("invalid value '{}' for {}", raw.trim(), self.id());
        match self.domain {
            Domain::Choice(values, aliases) => {
                if let Some(v) = values.iter().find(|v| **v == value) {
                    return Ok(v.to_string());
                }
                aliases.iter().find(|(a, _)| *a == value).map(|(_, v)| v.to_string()).ok_or_else(bad)
            }
            Domain::Bool => match value.as_str() {
                "true" | "on" | "yes" | "1" => Ok("true".into()),
                "false" | "off" | "no" | "0" => Ok("false".into()),
                _ => Err(bad()),
            },
            Domain::MemSize => {
                let n: i64 = value.strip_suffix("mb").unwrap_or(&value).trim_end().parse().map_err(|_| bad())?;
                if (MIN_MEMSIZE..=MAX_MEMSIZE).contains(&n) { Ok(n.to_string()) } else { Err(bad()) }
            }
            Domain::IntSet(values, units) => {
                let digits = units.iter().find_map(|u| value.strip_suffix(u)).unwrap_or(&value).trim_end();
                let n: i64 = digits.parse().map_err(|_| bad())?;
                if values.contains(&n) { Ok(n.to_string()) } else { Err(bad()) }
            }
            Domain::HexPort(values) => {
                let digits = value.strip_prefix("0x").or_else(|| value.strip_suffix('h')).unwrap_or(&value);
                let n = u16::from_str_radix(digits, 16).map_err(|_| bad())?;
                if values.contains(&n) { Ok(format!("{n:x}")) } else { Err(bad()) }
            }
            Domain::Cycles => {
                let words: Vec<&str> = value.split_whitespace().collect();
                let fixed = |s: &str| -> Result<u32, String> {
                    let n: u32 = s.parse().map_err(|_| bad())?;
                    if (MIN_CYCLES..=MAX_CYCLES).contains(&n) { Ok(n) } else { Err(bad()) }
                };
                match words.as_slice() {
                    ["max"] => Ok("max".into()),
                    ["auto"] => Ok("auto".into()),
                    ["auto", n] => Ok(format!("auto {}", fixed(n)?)),
                    [n] => Ok(fixed(n)?.to_string()),
                    _ => Err(bad()),
                }
            }
            Domain::DosVersion => {
                let (major, minor) = value.split_once('.').unwrap_or((&value, "00"));
                let major: u8 = major.parse().ok().filter(|m| (2..=9).contains(m)).ok_or_else(bad)?;
                if !(1..=2).contains(&minor.len()) || !minor.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad());
                }
                let n: u8 = minor.parse().map_err(|_| bad())?;
                let n = if minor.len() == 1 { n * 10 } else { n };
                Ok(format!("{major}.{n:02}"))
            }
            Domain::Locale(values) => {
                let value = value.replace('_', "-");
                values.iter().find(|v| v.eq_ignore_ascii_case(&value)).map(|v| v.to_string()).ok_or_else(bad)
            }
            Domain::FpsCap => match value.as_str() {
                "auto" => Ok("auto".into()),
                "off" | "0" => Ok("off".into()),
                _ => {
                    let n: u32 = value.parse().map_err(|_| bad())?;
                    if (MIN_FPS_CAP..=MAX_FPS_CAP).contains(&n) { Ok(n.to_string()) } else { Err(bad()) }
                }
            },
        }
    }

    /// A number to order values by when picking a median, if there is one.
    pub fn numeric_value(&self, normalized: &str) -> Option<f64> {
        match self.domain {
            Domain::MemSize | Domain::IntSet(..) | Domain::FpsCap => normalized.parse().ok(),
            Domain::Cycles => normalized.trim_start_matches("auto ").parse().ok(),
            Domain::HexPort(_) => u16::from_str_radix(normalized, 16).ok().map(f64::from),
            Domain::DosVersion => normalized.parse().ok(),
            _ => None,
        }
    }
}

pub const MIN_MEMSIZE: i64 = 2;
pub const MAX_MEMSIZE: i64 = 512;
pub const MIN_CYCLES: u32 = 100;
pub const MAX_CYCLES: u32 = 2_000_000;
pub const MIN_FPS_CAP: u32 = 10;
pub const MAX_FPS_CAP: u32 = 240;
const FPS_CAP_PRESETS: &[u32] = &[30, 35, 50, 60, 70, 72, 75, 85, 100, 120, 144];
const MEMSIZE_PRESETS: &[i64] = &[2, 4, 8, 16, 32, 48, 64, 128, 256, 512];
const DOS_PRESETS: &[&str] = &["5.00", "6.22", "7.00", "7.10"];

/// The built-in Windows' system locales (`rust-dos/src/win32/nls/locales.rs`).
const WIN32_LOCALES: &[&str] = &[
    "en-US", "ru-RU", "pl-PL", "cs-CZ", "hu-HU", "el-GR", "tr-TR", "lt-LT", "lv-LV", "et-EE", "be-BY", "bg-BG",
    "sr-Cyrl-YU", "uk-UA", "hr-HR", "ro-RO", "sr-Latn-YU", "sk-SK", "sl-SI", "da-DK", "de-DE", "en-GB", "es-ES",
    "fi-FI", "fr-FR", "it-IT", "nb-NO", "nl-NL", "pt-BR", "pt-PT", "sv-SE",
];

const IRQS_SB: &[i64] = &[2, 3, 5, 7, 9, 10, 11, 12, 15];
const IRQS_GUS: &[i64] = &[2, 3, 5, 7, 11, 12, 15];

pub static SETTINGS: &[SettingDef] = &[
    // [emulator]
    SettingDef {
        section: "emulator",
        key: "machine",
        label: "Graphics card",
        domain: Domain::Choice(
            &[
                "svga", "svga_s3", "svga_s3virge", "svga_s3virgevx", "svga_verite", "svga_et4000", "svga_voodoo3",
                "svga_banshee", "svga_ati_3drage", "svga_ati_3drage2", "svga_toadforce", "vga", "ega", "cga", "tandy", "pcjr", "hercules",
            ],
            &[
                ("svga_et3000", "svga"),
                ("svga_paradise", "svga"),
                ("vesa_nolfb", "svga"),
                ("vesa_oldvbe", "svga"),
                ("s3", "svga_s3"),
                ("s3trio", "svga_s3"),
                ("virge", "svga_s3virge"),
                ("s3virge", "svga_s3virge"),
                ("virgevx", "svga_s3virgevx"),
                ("s3virgevx", "svga_s3virgevx"),
                ("verite", "svga_verite"),
                ("rendition", "svga_verite"),
                ("et4000", "svga_et4000"),
                ("tseng", "svga_et4000"),
                ("voodoo3", "svga_voodoo3"),
                ("banshee", "svga_banshee"),
                ("rage", "svga_ati_3drage"),
                ("3drage", "svga_ati_3drage"),
                ("mach64", "svga_ati_3drage"),
                ("rage2", "svga_ati_3drage2"),
                ("3drage2", "svga_ati_3drage2"),
                ("toadforce", "svga_toadforce"),
                ("vgaonly", "vga"),
                ("herc", "hercules"),
                ("hgc", "hercules"),
            ],
        ),
    },
    SettingDef {
        section: "emulator",
        key: "cpu",
        label: "CPU",
        domain: Domain::Choice(
            &["386", "486", "pentium", "pentium_mmx", "k6_2", "pentium_pro", "pentium_ii", "pentium_iii"],
            &[
                ("586", "pentium"),
                ("pentiummmx", "pentium_mmx"),
                ("mmx", "pentium_mmx"),
                ("k6-2", "k6_2"),
                ("k62", "k6_2"),
                ("686", "pentium_pro"),
                ("pentiumpro", "pentium_pro"),
                ("ppro", "pentium_pro"),
                ("pentiumii", "pentium_ii"),
                ("pentium2", "pentium_ii"),
                ("pentiumiii", "pentium_iii"),
                ("pentium3", "pentium_iii"),
            ],
        ),
    },
    SettingDef { section: "emulator", key: "core", label: "CPU core", domain: Domain::Choice(&["auto", "dynamic", "normal"], &[]) },
    SettingDef { section: "emulator", key: "fpu", label: "FPU", domain: Domain::Choice(&["exact", "fast"], &[]) },
    SettingDef { section: "emulator", key: "cycles", label: "Cycles", domain: Domain::Cycles },
    SettingDef { section: "emulator", key: "memsize", label: "Memory (MB)", domain: Domain::MemSize },
    SettingDef { section: "emulator", key: "ems", label: "EMS", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "umb", label: "UMB", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "dpmi", label: "DPMI", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "dos_high", label: "DOS=HIGH", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "dos_version", label: "DOS version", domain: Domain::DosVersion },
    SettingDef { section: "emulator", key: "win32", label: "Built-in Windows", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "win32_version", label: "Windows version", domain: Domain::Choice(&["95", "98", "98se"], &[]) },
    SettingDef { section: "emulator", key: "win32_depth", label: "Windows colour depth", domain: Domain::Choice(&["auto", "8", "16"], &[]) },
    SettingDef { section: "emulator", key: "win32_desktop", label: "Windows desktop size", domain: Domain::Choice(&["640x480", "800x600", "1024x768"], &[]) },
    SettingDef { section: "emulator", key: "win32_locale", label: "Windows locale", domain: Domain::Locale(WIN32_LOCALES) },
    SettingDef { section: "emulator", key: "fps_cap", label: "Frame rate cap", domain: Domain::FpsCap },
    SettingDef { section: "emulator", key: "voodoo", label: "3dfx Voodoo", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "voodoo_model", label: "Voodoo model", domain: Domain::Choice(&["voodoo1", "voodoo2"], &[("voodoo", "voodoo1"), ("1", "voodoo1"), ("2", "voodoo2")]) },
    SettingDef { section: "emulator", key: "voodoo_memory", label: "Voodoo memory (MB)", domain: Domain::IntSet(&[4, 8, 12], &["mb"]) },
    SettingDef {
        section: "emulator",
        key: "powervr",
        label: "PowerVR",
        domain: Domain::Choice(
            &["off", "pcx1", "pcx2"],
            &[("false", "off"), ("no", "off"), ("none", "off"), ("on", "pcx2"), ("true", "pcx2"), ("yes", "pcx2")],
        ),
    },
    SettingDef {
        section: "emulator",
        key: "composite",
        label: "CGA composite",
        domain: Domain::Choice(
            &["auto", "on", "off"],
            &[("true", "on"), ("yes", "on"), ("1", "on"), ("false", "off"), ("no", "off"), ("0", "off")],
        ),
    },
    SettingDef { section: "emulator", key: "monochrome", label: "Monochrome", domain: Domain::Choice(&["off", "white", "amber", "green"], &[]) },
    SettingDef { section: "emulator", key: "hard_disk_speed", label: "Hard disk speed", domain: Domain::Choice(&["maximum", "fast", "medium", "slow"], &[]) },
    SettingDef { section: "emulator", key: "floppy_disk_speed", label: "Floppy speed", domain: Domain::Choice(&["maximum", "fast", "medium", "slow"], &[]) },
    // [sound]
    SettingDef {
        section: "sound",
        key: "sbtype",
        label: "Sound Blaster",
        domain: Domain::Choice(
            &["none", "gb", "sb2", "sbpro1", "sbpro2", "sb16", "sb16vibra", "awe32"],
            &[("sb20", "sb2"), ("sbpro", "sbpro2"), ("vibra", "sb16vibra"), ("sbawe", "awe32"), ("awe", "awe32")],
        ),
    },
    SettingDef { section: "sound", key: "sbbase", label: "SB port", domain: Domain::HexPort(&[0x210, 0x220, 0x230, 0x240, 0x250, 0x260, 0x270, 0x280]) },
    SettingDef { section: "sound", key: "irq", label: "SB IRQ", domain: Domain::IntSet(IRQS_SB, &[]) },
    SettingDef { section: "sound", key: "dma", label: "SB DMA", domain: Domain::IntSet(&[0, 1, 3], &[]) },
    SettingDef { section: "sound", key: "hdma", label: "SB high DMA", domain: Domain::IntSet(&[5, 6, 7], &[]) },
    SettingDef {
        section: "sound",
        key: "awe32ram",
        label: "AWE32 RAM (KB)",
        domain: Domain::IntSet(&[0, 512, 1024, 2048, 4096, 8192, 12288, 16384, 28672], &["kb", "k"]),
    },
    SettingDef { section: "sound", key: "opl", label: "FM chip", domain: Domain::Choice(&["auto", "opl3", "opl2", "dualopl2"], &[]) },
    SettingDef { section: "sound", key: "gus", label: "Gravis Ultrasound", domain: Domain::Bool },
    SettingDef { section: "sound", key: "gusbase", label: "GUS port", domain: Domain::HexPort(&[0x210, 0x220, 0x240, 0x250, 0x260]) },
    SettingDef { section: "sound", key: "gusirq", label: "GUS IRQ", domain: Domain::IntSet(IRQS_GUS, &[]) },
    SettingDef { section: "sound", key: "gusdma", label: "GUS DMA", domain: Domain::IntSet(&[1, 3, 5, 6, 7], &[]) },
    SettingDef {
        section: "sound",
        key: "midisynth",
        label: "MIDI synth",
        domain: Domain::Choice(
            &["auto", "soundfont", "gus", "mt32", "sc55", "host", "none"],
            &[
                ("mt-32", "mt32"),
                ("munt", "mt32"),
                ("sc-55", "sc55"),
                ("soundcanvas", "sc55"),
                ("hostmidi", "host"),
                ("external", "host"),
            ],
        ),
    },
    SettingDef { section: "sound", key: "mt32model", label: "MT-32 model", domain: Domain::Choice(&["auto", "mt32", "cm32l"], &[("mt-32", "mt32"), ("cm-32l", "cm32l")]) },
    SettingDef { section: "sound", key: "adlibgold", label: "AdLib Gold", domain: Domain::Bool },
    SettingDef { section: "sound", key: "lpt_dac", label: "Parallel DAC", domain: Domain::Choice(&["none", "disney", "covox"], &[("off", "none"), ("false", "none")]) },
    SettingDef {
        section: "sound",
        key: "tandy",
        label: "Tandy sound",
        domain: Domain::Choice(
            &["auto", "on", "off"],
            &[("true", "on"), ("yes", "on"), ("1", "on"), ("false", "off"), ("no", "off"), ("0", "off")],
        ),
    },
    // [joystick]
    SettingDef {
        section: "joystick",
        key: "joysticktype",
        label: "Joystick",
        domain: Domain::Choice(&["auto", "4axis", "2axis", "mouse", "none"], &[]),
    },
];

/// Sections in the order they appear in a config file.
pub const SECTIONS: &[&str] = &["emulator", "sound", "joystick"];

/// Old names Rust-DOS still reads, as (section, old key, current key).
/// [`find`] takes them and gives the current setting; nothing is stored or
/// sent under an old name.
pub const OLD_NAMES: &[(&str, &str, &str)] = &[("emulator", "voodoo_fps_cap", "fps_cap")];

pub fn find(section: &str, key: &str) -> Option<&'static SettingDef> {
    let (section, key) = (section.trim().to_ascii_lowercase(), key.trim().to_ascii_lowercase());
    let key = OLD_NAMES.iter().find(|(s, old, _)| *s == section && *old == key).map_or(key.as_str(), |(_, _, new)| new);
    SETTINGS.iter().find(|d| d.section == section && d.key == key)
}

/// Whether `key` is an old name of a setting.
pub fn is_old_name(section: &str, key: &str) -> bool {
    let (section, key) = (section.trim().to_ascii_lowercase(), key.trim().to_ascii_lowercase());
    OLD_NAMES.iter().any(|(s, old, _)| *s == section && *old == key)
}

/// Position of a setting in [`SETTINGS`], for stable ordering.
pub fn order(section: &str, key: &str) -> usize {
    SETTINGS.iter().position(|d| d.section == section && d.key == key).unwrap_or(usize::MAX)
}

/// `{section: {key: value}}` as submitted, validated and normalised into
/// `(section, key, value)` triples in schema order. Empty values are dropped.
/// A setting under an old name takes its current one; given under both, the
/// current name wins.
pub fn normalize_all(input: &BTreeMap<String, BTreeMap<String, String>>) -> Result<Vec<(String, String, String)>, String> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    for (section, keys) in input {
        // Old names first, so the current name's value replaces theirs.
        let mut keys: Vec<_> = keys.iter().collect();
        keys.sort_by_key(|(key, _)| !is_old_name(section, key));
        for (key, value) in keys {
            if value.trim().is_empty() {
                continue;
            }
            let def = find(section, key).ok_or_else(|| format!("unknown setting {section}.{key}"))?;
            let value = def.normalize(value)?;
            out.retain(|(s, k, _)| !(s == def.section && k == def.key));
            out.push((def.section.to_string(), def.key.to_string(), value));
        }
    }
    out.sort_by_key(|(s, k, _)| order(s, k));
    Ok(out)
}

/// A soft check: settings Rust-DOS will accept but warn about.
pub fn warnings(settings: &[(String, String, String)]) -> Vec<String> {
    let get = |k: &str| settings.iter().find(|(s, key, _)| s == "emulator" && key == k).map(|(_, _, v)| v.as_str());
    let mut out = Vec::new();
    if let (Some(cpu), Some(mem)) = (get("cpu"), get("memsize").and_then(|m| m.parse::<i64>().ok())) {
        let cap = match cpu {
            "386" => 64,
            "486" => 128,
            "pentium" | "pentium_mmx" => 256,
            _ => 512,
        };
        if mem > cap {
            out.push(format!("{mem} MB is more than a {cpu} system takes ({cap} MB)"));
        }
    }
    let windows = get("win32") == Some("true");
    if !windows {
        for key in ["win32_version", "win32_depth", "win32_desktop", "win32_locale"] {
            if get(key).is_some() {
                out.push(format!("{key} has no effect without win32=true"));
            }
        }
    }
    if windows && matches!(get("cpu"), Some("386" | "486")) {
        out.push("many Windows 9x games need a Pentium or better (RDTSC, MMX)".into());
    }
    out
}

/// An INI fragment Rust-DOS's config parser reads as-is.
pub fn to_ini<'a>(settings: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>) -> String {
    type Section<'s> = (&'s str, Vec<(&'s str, &'s str)>);
    let mut by_section: BTreeMap<usize, Section> = BTreeMap::new();
    for (section, key, value) in settings {
        let idx = SECTIONS.iter().position(|s| *s == section).unwrap_or(SECTIONS.len());
        by_section.entry(idx).or_insert((section, Vec::new())).1.push((key, value));
    }
    let mut out = String::new();
    for (_, (section, mut keys)) in by_section {
        keys.sort_by_key(|(k, _)| order(section, k));
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("[{section}]\n"));
        for (k, v) in keys {
            out.push_str(&format!("{k}={v}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(section: &str, key: &str, v: &str) -> Result<String, String> {
        find(section, key).unwrap().normalize(v)
    }

    #[test]
    fn normalises_rust_dos_spellings() {
        assert_eq!(norm("emulator", "cpu", "586").unwrap(), "pentium");
        assert_eq!(norm("emulator", "machine", "VESA_NOLFB").unwrap(), "svga");
        assert_eq!(norm("emulator", "memsize", "128MB").unwrap(), "128");
        assert_eq!(norm("emulator", "ems", "on").unwrap(), "true");
        assert_eq!(norm("emulator", "cycles", "auto  30000").unwrap(), "auto 30000");
        assert_eq!(norm("emulator", "cycles", "MAX").unwrap(), "max");
        assert_eq!(norm("emulator", "dos_version", "7.1").unwrap(), "7.10");
        assert_eq!(norm("sound", "sbbase", "0x220").unwrap(), "220");
        assert_eq!(norm("sound", "sbbase", "220h").unwrap(), "220");
        assert_eq!(norm("emulator", "composite", "yes").unwrap(), "on");
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(norm("emulator", "memsize", "1").is_err());
        assert!(norm("emulator", "memsize", "1024").is_err());
        assert!(norm("emulator", "cycles", "50").is_err());
        assert!(norm("sound", "irq", "4").is_err());
        assert!(norm("sound", "sbbase", "300").is_err());
        assert!(norm("emulator", "machine", "voodoo5").is_err());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let mut input = BTreeMap::new();
        input.insert("autoexec".to_string(), BTreeMap::from([("cmd".to_string(), "del *.*".to_string())]));
        assert!(normalize_all(&input).is_err());
    }

    #[test]
    fn ini_is_in_section_and_schema_order() {
        let ini = to_ini([("sound", "sbtype", "sb16"), ("emulator", "memsize", "16"), ("emulator", "cpu", "486")]);
        assert_eq!(ini, "[emulator]\ncpu=486\nmemsize=16\n\n[sound]\nsbtype=sb16\n");
    }

    #[test]
    fn memsize_over_cpu_cap_warns() {
        let s = vec![("emulator".into(), "cpu".into(), "386".into()), ("emulator".into(), "memsize".into(), "128".into())];
        assert_eq!(warnings(&s).len(), 1);
    }

    #[test]
    fn win32_depth_takes_auto_8_or_16() {
        assert_eq!(norm("emulator", "win32_depth", "AUTO").unwrap(), "auto");
        assert_eq!(norm("emulator", "win32_depth", "8").unwrap(), "8");
        assert!(norm("emulator", "win32_depth", "24").is_err());
        assert!(norm("emulator", "win32_depth", "32").is_err());
    }

    #[test]
    fn fps_cap_takes_off_or_10_to_240() {
        assert_eq!(norm("emulator", "fps_cap", "AUTO").unwrap(), "auto");
        assert_eq!(norm("emulator", "fps_cap", "OFF").unwrap(), "off");
        assert_eq!(norm("emulator", "fps_cap", "0").unwrap(), "off");
        assert_eq!(norm("emulator", "fps_cap", " 60 ").unwrap(), "60");
        assert_eq!(norm("emulator", "fps_cap", "10").unwrap(), "10");
        assert_eq!(norm("emulator", "fps_cap", "240").unwrap(), "240");
        for bad in ["9", "241", "-1", "60fps", "on", "", "1e2"] {
            assert!(norm("emulator", "fps_cap", bad).is_err(), "{bad}");
        }
        let def = find("emulator", "fps_cap").unwrap();
        assert_eq!(def.numeric_value("60"), Some(60.0));
        assert_eq!(def.numeric_value("off"), None);
        assert_eq!(def.numeric_value("auto"), None);
        assert_eq!(def.options().unwrap()[..2], ["auto", "off"]);
        assert_eq!(def.group(), "machine");
    }

    #[test]
    fn voodoo_fps_cap_is_an_old_name_of_fps_cap() {
        assert_eq!(find("emulator", "VOODOO_FPS_CAP").unwrap().key, "fps_cap");
        assert!(find("sound", "voodoo_fps_cap").is_none());
        assert!(SETTINGS.iter().all(|d| d.key != "voodoo_fps_cap"));

        let input = |pairs: &[(&str, &str)]| {
            BTreeMap::from([("emulator".to_string(), pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())])
        };
        let one = |k: &str, v: &str| vec![("emulator".to_string(), k.to_string(), v.to_string())];
        assert_eq!(normalize_all(&input(&[("voodoo_fps_cap", "60")])).unwrap(), one("fps_cap", "60"));
        // Both names: the current one wins, whatever the order.
        assert_eq!(normalize_all(&input(&[("voodoo_fps_cap", "30"), ("fps_cap", "off")])).unwrap(), one("fps_cap", "off"));
        assert_eq!(normalize_all(&input(&[("fps_cap", "60"), ("voodoo_fps_cap", "30")])).unwrap(), one("fps_cap", "60"));
        assert!(normalize_all(&input(&[("voodoo_fps_cap", "300")])).is_err());
    }

    #[test]
    fn windows_desktop_and_locale() {
        assert_eq!(norm("emulator", "win32_desktop", "1024X768").unwrap(), "1024x768");
        assert!(norm("emulator", "win32_desktop", "1280x1024").is_err());
        assert_eq!(norm("emulator", "win32_locale", "ru_ru").unwrap(), "ru-RU");
        assert_eq!(norm("emulator", "win32_locale", "SR-LATN-YU").unwrap(), "sr-Latn-YU");
        assert!(norm("emulator", "win32_locale", "ja-JP").is_err());
        assert_eq!(find("emulator", "win32_desktop").unwrap().group(), "windows");
        assert_eq!(find("emulator", "win32_locale").unwrap().group(), "windows");
    }

    #[test]
    fn takes_rust_dos_aliases() {
        assert_eq!(norm("emulator", "machine", "toadforce").unwrap(), "svga_toadforce");
        assert_eq!(norm("emulator", "machine", "svga_toadforce").unwrap(), "svga_toadforce");
        assert_eq!(norm("emulator", "machine", "herc").unwrap(), "hercules");
        assert_eq!(norm("emulator", "cpu", "pentium2").unwrap(), "pentium_ii");
        assert_eq!(norm("emulator", "cpu", "k6-2").unwrap(), "k6_2");
        assert_eq!(norm("sound", "sbtype", "sbpro").unwrap(), "sbpro2");
        assert_eq!(norm("sound", "midisynth", "munt").unwrap(), "mt32");
        assert_eq!(norm("sound", "mt32model", "cm-32l").unwrap(), "cm32l");
        assert_eq!(norm("sound", "lpt_dac", "off").unwrap(), "none");
        assert_eq!(norm("sound", "tandy", "yes").unwrap(), "on");
        assert_eq!(norm("sound", "adlibgold", "on").unwrap(), "true");
    }

    /// What Rust-DOS sends with a report: every one of its almanac keys
    /// (`rust-dos/src/almanac.rs`), as its configuration writes them.
    #[test]
    fn a_current_rust_dos_report_is_accepted() {
        let sent: &[(&str, &[(&str, &str)])] = &[
            (
                "emulator",
                &[
                    ("machine", "svga_toadforce"), ("cpu", "pentium_ii"), ("core", "auto"), ("fpu", "exact"),
                    ("cycles", "auto"), ("memsize", "64"), ("ems", "true"), ("umb", "true"), ("dpmi", "true"),
                    ("dos_high", "true"), ("dos_version", "7.10"), ("win32", "true"), ("win32_version", "98se"),
                    ("win32_depth", "16"), ("win32_desktop", "800x600"), ("win32_locale", "en-US"), ("voodoo", "false"),
                    ("voodoo_model", "voodoo1"), ("voodoo_memory", "4"), ("powervr", "off"), ("composite", "auto"),
                    ("monochrome", "off"), ("hard_disk_speed", "fast"), ("floppy_disk_speed", "fast"), ("fps_cap", "auto"),
                ],
            ),
            (
                "sound",
                &[
                    ("sbtype", "sb16"), ("sbbase", "220"), ("irq", "7"), ("dma", "1"), ("hdma", "5"), ("awe32ram", "512"),
                    ("opl", "auto"), ("gus", "false"), ("gusbase", "240"), ("gusirq", "5"), ("gusdma", "3"),
                    ("midisynth", "auto"), ("mt32model", "auto"), ("adlibgold", "false"), ("lpt_dac", "none"), ("tandy", "auto"),
                ],
            ),
            ("joystick", &[("joysticktype", "auto")]),
        ];
        let input: BTreeMap<String, BTreeMap<String, String>> = sent
            .iter()
            .map(|(section, keys)| (section.to_string(), keys.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()))
            .collect();
        let out = normalize_all(&input).unwrap();
        assert_eq!(out.len(), 42, "Rust-DOS's KEYS has 42");
        for (section, keys) in sent {
            for (key, value) in *keys {
                assert!(out.iter().any(|(s, k, v)| s == section && k == key && v == value), "{section}.{key}={value}");
            }
        }
    }

    /// What Rust-DOS's configuration parser takes for each of its almanac
    /// keys (`KEYS` in `rust-dos/src/almanac.rs`), from `src/config/parse.rs`,
    /// `src/config/types.rs` and the types they call. A setting with a list of
    /// values has every one of them here, aliases too; a range has its ends
    /// and a few values between. Change this together with [`SETTINGS`]
    /// whenever Rust-DOS's parser or `KEYS` change.
    const RUST_DOS_TAKES: &[(&str, &str, &[&str])] = &[
        (
            "emulator",
            "machine",
            &[
                "svga", "svga_et3000", "svga_paradise", "vesa_nolfb", "vesa_oldvbe", "svga_s3", "s3", "s3trio",
                "svga_s3virge", "virge", "s3virge", "svga_s3virgevx", "virgevx", "s3virgevx", "svga_verite", "verite",
                "rendition", "svga_et4000", "et4000", "tseng", "svga_voodoo3", "voodoo3", "svga_banshee", "banshee",
                "svga_ati_3drage", "rage", "3drage", "mach64", "svga_ati_3drage2", "rage2", "3drage2", "svga_toadforce",
                "toadforce", "vga", "vgaonly", "ega", "cga", "hercules", "herc", "hgc", "tandy", "pcjr",
            ],
        ),
        (
            "emulator",
            "cpu",
            &[
                "386", "486", "pentium", "586", "pentium_mmx", "pentiummmx", "mmx", "k6_2", "k6-2", "k62", "pentium_pro",
                "pentiumpro", "ppro", "686", "pentium_ii", "pentiumii", "pentium2", "pentium_iii", "pentiumiii", "pentium3",
            ],
        ),
        ("emulator", "core", &["auto", "dynamic", "normal"]),
        ("emulator", "fpu", &["exact", "fast"]),
        ("emulator", "cycles", &["auto", "max", "100", "30000", "2000000", "auto 100", "auto 2000000"]),
        ("emulator", "memsize", &["2", "16", "16MB", "16 mb", "512"]),
        ("emulator", "ems", BOOL),
        ("emulator", "umb", BOOL),
        ("emulator", "dpmi", BOOL),
        ("emulator", "dos_high", BOOL),
        ("emulator", "dos_version", &["2", "5", "5.0", "6.22", "7.1", "7.10", "9.99"]),
        ("emulator", "win32", BOOL),
        ("emulator", "win32_version", &["95", "98", "98se"]),
        ("emulator", "win32_depth", &["auto", "8", "16"]),
        ("emulator", "win32_desktop", &["640x480", "800x600", "1024x768"]),
        (
            "emulator",
            "win32_locale",
            &[
                "en-US", "ru-RU", "pl-PL", "cs-CZ", "hu-HU", "el-GR", "tr-TR", "lt-LT", "lv-LV", "et-EE", "be-BY", "bg-BG",
                "sr-Cyrl-YU", "uk-UA", "hr-HR", "ro-RO", "sr-Latn-YU", "sk-SK", "sl-SI", "da-DK", "de-DE", "en-GB", "es-ES",
                "fi-FI", "fr-FR", "it-IT", "nb-NO", "nl-NL", "pt-BR", "pt-PT", "sv-SE",
            ],
        ),
        ("emulator", "voodoo", BOOL),
        ("emulator", "voodoo_model", &["voodoo1", "voodoo", "1", "voodoo2", "2"]),
        ("emulator", "voodoo_memory", &["4", "4mb", "8", "8mb", "12", "12mb"]),
        ("emulator", "powervr", &["off", "false", "no", "none", "pcx1", "pcx2", "on", "true", "yes"]),
        ("emulator", "composite", &["auto", "on", "true", "yes", "1", "off", "false", "no", "0"]),
        ("emulator", "monochrome", &["off", "white", "amber", "green"]),
        ("emulator", "hard_disk_speed", &["maximum", "fast", "medium", "slow"]),
        ("emulator", "floppy_disk_speed", &["maximum", "fast", "medium", "slow"]),
        ("emulator", "fps_cap", &["auto", "off", "0", "10", "60", "240"]),
        (
            "sound",
            "sbtype",
            &["none", "gb", "sb2", "sb20", "sbpro1", "sbpro2", "sbpro", "sb16", "sb16vibra", "vibra", "awe32", "sbawe", "awe"],
        ),
        ("sound", "sbbase", &["210", "220", "230", "240", "250", "260", "270", "280", "0x220"]),
        ("sound", "irq", &["2", "3", "5", "7", "9", "10", "11", "12", "15"]),
        ("sound", "dma", &["0", "1", "3"]),
        ("sound", "hdma", &["5", "6", "7"]),
        ("sound", "awe32ram", &["0", "512", "512k", "512kb", "1024", "2048", "4096", "8192", "12288", "16384", "28672"]),
        ("sound", "opl", &["auto", "opl3", "opl2", "dualopl2"]),
        ("sound", "gus", BOOL),
        ("sound", "gusbase", &["210", "220", "240", "250", "260"]),
        ("sound", "gusirq", &["2", "3", "5", "7", "11", "12", "15"]),
        ("sound", "gusdma", &["1", "3", "5", "6", "7"]),
        (
            "sound",
            "midisynth",
            &["auto", "soundfont", "gus", "mt32", "mt-32", "munt", "sc55", "sc-55", "soundcanvas", "host", "hostmidi", "external", "none"],
        ),
        ("sound", "mt32model", &["auto", "mt32", "mt-32", "cm32l", "cm-32l"]),
        ("sound", "adlibgold", BOOL),
        ("sound", "lpt_dac", &["none", "off", "false", "disney", "covox"]),
        ("sound", "tandy", &["auto", "on", "true", "yes", "1", "off", "false", "no", "0"]),
        ("joystick", "joysticktype", &["auto", "4axis", "2axis", "mouse", "none"]),
    ];
    const BOOL: &[&str] = &["true", "on", "yes", "1", "false", "off", "no", "0"];

    /// The keys are Rust-DOS's `KEYS`, and every value its parser takes is
    /// taken here, so a report from any setup Rust-DOS runs is accepted.
    #[test]
    fn takes_every_value_rust_dos_takes() {
        assert_eq!(RUST_DOS_TAKES.len(), 42, "Rust-DOS's KEYS has 42");
        assert_eq!(RUST_DOS_TAKES.len(), SETTINGS.len());
        for def in SETTINGS {
            assert!(RUST_DOS_TAKES.iter().any(|(s, k, _)| *s == def.section && *k == def.key), "{} isn't in RUST_DOS_TAKES", def.id());
        }
        for (section, key, values) in RUST_DOS_TAKES {
            let def = find(section, key).unwrap_or_else(|| panic!("{section}.{key} isn't a setting"));
            for value in *values {
                assert!(def.normalize(value).is_ok(), "{section}.{key}={value}, which Rust-DOS takes");
            }
        }
    }

    /// The other way: every value the almanac offers or stores for a setting
    /// with a list of values is one Rust-DOS's parser takes, so a
    /// recommendation never carries one it rejects.
    #[test]
    fn offers_only_values_rust_dos_takes() {
        for (section, key, values) in RUST_DOS_TAKES {
            let def = find(section, key).unwrap();
            if !matches!(def.domain, Domain::Choice(..) | Domain::Bool | Domain::IntSet(..) | Domain::HexPort(_) | Domain::Locale(_)) {
                continue;
            }
            for option in def.options().unwrap() {
                assert!(values.iter().any(|v| v.eq_ignore_ascii_case(&option)), "{section}.{key}={option}, which Rust-DOS doesn't take");
            }
        }
    }

    #[test]
    fn opl_and_sbtype_take_rust_dos_defaults_and_cards() {
        assert_eq!(norm("sound", "opl", "AUTO").unwrap(), "auto");
        assert_eq!(norm("sound", "opl", "DualOPL2").unwrap(), "dualopl2");
        assert_eq!(norm("sound", "sbtype", "SBPro1").unwrap(), "sbpro1");
        assert_eq!(norm("sound", "sbtype", "gb").unwrap(), "gb");
        assert_eq!(norm("sound", "awe32ram", "512K").unwrap(), "512");
        assert_eq!(norm("emulator", "voodoo_memory", "8 MB").unwrap(), "8");
        assert!(norm("sound", "irq", "5k").is_err());
        assert_eq!(find("sound", "opl").unwrap().options().unwrap()[0], "auto");
    }

    #[test]
    fn windows_warnings() {
        let s = |list: &[(&str, &str)]| list.iter().map(|(k, v)| ("emulator".to_string(), k.to_string(), v.to_string())).collect::<Vec<_>>();
        assert_eq!(warnings(&s(&[("win32_depth", "8")])).len(), 1);
        assert_eq!(warnings(&s(&[("win32", "false"), ("win32_version", "98se"), ("win32_depth", "16")])).len(), 2);
        assert_eq!(warnings(&s(&[("win32_desktop", "800x600"), ("win32_locale", "de-DE")])).len(), 2);
        assert_eq!(warnings(&s(&[("win32", "true"), ("cpu", "486")])).len(), 1);
        assert!(warnings(&s(&[("win32", "true"), ("cpu", "pentium_ii"), ("win32_depth", "8")])).is_empty());
    }

    #[test]
    fn every_setting_has_a_group() {
        for d in SETTINGS {
            assert!(GROUPS.iter().any(|(id, _)| *id == d.group()), "{}", d.id());
        }
        assert_eq!(find("emulator", "win32_depth").unwrap().group(), "windows");
        assert_eq!(find("emulator", "cpu").unwrap().group(), "machine");
        assert_eq!(find("joystick", "joysticktype").unwrap().group(), "input");
    }
}

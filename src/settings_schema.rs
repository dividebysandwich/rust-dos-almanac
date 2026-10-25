//! The Rust-DOS settings the almanac knows about, with the values Rust-DOS
//! accepts for each (`rust-dos/src/config/parse.rs` and `types.rs`).
//!
//! Only settings that describe the emulated machine are here. Host-side
//! display options, `[drives]`, `[autoexec]` and file paths are left out on
//! purpose: a public database must not hand commands or paths to emulators.

use std::collections::BTreeMap;

/// Bumped whenever a key or a value domain changes, so clients can notice.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    /// One of a fixed list. Aliases map to their canonical value.
    Choice(&'static [&'static str], &'static [(&'static str, &'static str)]),
    Bool,
    /// Whole MB, optional "MB" suffix.
    MemSize,
    /// One of the listed integers.
    IntSet(&'static [i64]),
    /// A hex port: "220", "220h" or "0x220", one of the listed values.
    HexPort(&'static [u16]),
    /// `max`, `auto`, `auto N` or N.
    Cycles,
    /// "6.22", "7.1", "5".
    DosVersion,
}

#[derive(Clone, Copy, Debug)]
pub struct SettingDef {
    pub section: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub domain: Domain,
}

impl SettingDef {
    pub fn id(&self) -> String {
        format!("{}.{}", self.section, self.key)
    }

    /// Values to offer in a drop-down, or None for free entry.
    pub fn options(&self) -> Option<Vec<String>> {
        match self.domain {
            Domain::Choice(values, _) => Some(values.iter().map(|v| v.to_string()).collect()),
            Domain::Bool => Some(vec!["true".into(), "false".into()]),
            Domain::IntSet(values) => Some(values.iter().map(|v| v.to_string()).collect()),
            Domain::HexPort(values) => Some(values.iter().map(|v| format!("{v:x}")).collect()),
            Domain::MemSize => Some(MEMSIZE_PRESETS.iter().map(|v| v.to_string()).collect()),
            Domain::DosVersion => Some(DOS_PRESETS.iter().map(|v| v.to_string()).collect()),
            Domain::Cycles => None,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self.domain, Domain::MemSize | Domain::IntSet(_) | Domain::Cycles)
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
            Domain::IntSet(values) => {
                let n: i64 = value.parse().map_err(|_| bad())?;
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
        }
    }

    /// A number to order values by when picking a median, if there is one.
    pub fn numeric_value(&self, normalized: &str) -> Option<f64> {
        match self.domain {
            Domain::MemSize | Domain::IntSet(_) => normalized.parse().ok(),
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
const MEMSIZE_PRESETS: &[i64] = &[2, 4, 8, 16, 32, 48, 64, 128, 256, 512];
const DOS_PRESETS: &[&str] = &["5.00", "6.22", "7.00", "7.10"];

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
                "svga_banshee", "svga_ati_3drage", "svga_ati_3drage2", "vga", "ega", "cga", "tandy", "pcjr", "hercules",
            ],
            &[("svga_et3000", "svga"), ("svga_paradise", "svga"), ("vesa_nolfb", "svga"), ("vesa_oldvbe", "svga")],
        ),
    },
    SettingDef {
        section: "emulator",
        key: "cpu",
        label: "CPU",
        domain: Domain::Choice(
            &["386", "486", "pentium", "pentium_mmx", "k6_2", "pentium_pro", "pentium_ii", "pentium_iii"],
            &[("586", "pentium"), ("686", "pentium_pro")],
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
    SettingDef { section: "emulator", key: "win32", label: "Windows 9x", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "win32_version", label: "Windows version", domain: Domain::Choice(&["95", "98", "98se"], &[]) },
    SettingDef { section: "emulator", key: "voodoo", label: "3dfx Voodoo", domain: Domain::Bool },
    SettingDef { section: "emulator", key: "voodoo_model", label: "Voodoo model", domain: Domain::Choice(&["voodoo1", "voodoo2"], &[]) },
    SettingDef { section: "emulator", key: "voodoo_memory", label: "Voodoo memory (MB)", domain: Domain::IntSet(&[4, 8, 12]) },
    SettingDef { section: "emulator", key: "powervr", label: "PowerVR", domain: Domain::Choice(&["off", "pcx1", "pcx2"], &[]) },
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
        domain: Domain::Choice(&["none", "sb2", "sbpro2", "sb16", "sb16vibra", "awe32"], &[]),
    },
    SettingDef { section: "sound", key: "sbbase", label: "SB port", domain: Domain::HexPort(&[0x210, 0x220, 0x230, 0x240, 0x250, 0x260, 0x270, 0x280]) },
    SettingDef { section: "sound", key: "irq", label: "SB IRQ", domain: Domain::IntSet(IRQS_SB) },
    SettingDef { section: "sound", key: "dma", label: "SB DMA", domain: Domain::IntSet(&[0, 1, 3]) },
    SettingDef { section: "sound", key: "hdma", label: "SB high DMA", domain: Domain::IntSet(&[5, 6, 7]) },
    SettingDef {
        section: "sound",
        key: "awe32ram",
        label: "AWE32 RAM (KB)",
        domain: Domain::IntSet(&[0, 512, 1024, 2048, 4096, 8192, 12288, 16384, 28672]),
    },
    SettingDef { section: "sound", key: "opl", label: "FM chip", domain: Domain::Choice(&["opl3", "opl2"], &[]) },
    SettingDef { section: "sound", key: "gus", label: "Gravis Ultrasound", domain: Domain::Bool },
    SettingDef { section: "sound", key: "gusbase", label: "GUS port", domain: Domain::HexPort(&[0x210, 0x220, 0x240, 0x250, 0x260]) },
    SettingDef { section: "sound", key: "gusirq", label: "GUS IRQ", domain: Domain::IntSet(IRQS_GUS) },
    SettingDef { section: "sound", key: "gusdma", label: "GUS DMA", domain: Domain::IntSet(&[1, 3, 5, 6, 7]) },
    SettingDef {
        section: "sound",
        key: "midisynth",
        label: "MIDI synth",
        domain: Domain::Choice(&["auto", "soundfont", "gus", "mt32", "sc55", "host", "none"], &[]),
    },
    SettingDef { section: "sound", key: "mt32model", label: "MT-32 model", domain: Domain::Choice(&["auto", "mt32", "cm32l"], &[]) },
    SettingDef { section: "sound", key: "lpt_dac", label: "Parallel DAC", domain: Domain::Choice(&["none", "disney", "covox"], &[]) },
    SettingDef { section: "sound", key: "tandy", label: "Tandy sound", domain: Domain::Choice(&["auto", "on", "off"], &[]) },
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

pub fn find(section: &str, key: &str) -> Option<&'static SettingDef> {
    let (section, key) = (section.trim().to_ascii_lowercase(), key.trim().to_ascii_lowercase());
    SETTINGS.iter().find(|d| d.section == section && d.key == key)
}

/// Position of a setting in [`SETTINGS`], for stable ordering.
pub fn order(section: &str, key: &str) -> usize {
    SETTINGS.iter().position(|d| d.section == section && d.key == key).unwrap_or(usize::MAX)
}

/// `{section: {key: value}}` as submitted, validated and normalised into
/// `(section, key, value)` triples in schema order. Empty values are dropped.
pub fn normalize_all(input: &BTreeMap<String, BTreeMap<String, String>>) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    for (section, keys) in input {
        for (key, value) in keys {
            if value.trim().is_empty() {
                continue;
            }
            let def = find(section, key).ok_or_else(|| format!("unknown setting {section}.{key}"))?;
            out.push((def.section.to_string(), def.key.to_string(), def.normalize(value)?));
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
}

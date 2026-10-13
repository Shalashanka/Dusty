use crate::model::Kind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Off,
    Ask,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Where new stuff lands (Downloads). Stale/installer/archive rules apply here.
    pub inbox_dirs: Vec<String>,
    pub desktop_dir: String,
    /// Folders containing code projects.
    pub dev_roots: Vec<String>,
    /// Personal libraries searched for duplicates and big dormant files.
    pub library_dirs: Vec<String>,
    pub archive_root: String,
    pub rules: BTreeMap<Kind, Mode>,

    pub stale_days: u32,
    pub installer_days: u32,
    pub backup_days: u32,
    pub dev_idle_days: u32,
    pub big_mb: u32,
    pub big_days: u32,
    pub dup_min_kb: u32,
    pub desktop_max_items: u32,
    pub junk_min_mb: u32,
    pub recycle_bin_gb: u32,
    pub snooze_days: u32,

    pub notify: bool,
    pub quiet_start: u32,
    pub quiet_end: u32,
    pub nudge_gap_hours: u32,

    pub scan_floor_min: u32,
    pub scan_cap_min: u32,
    pub idle_min: u32,

    pub start_with_windows: bool,
    pub onboarded: bool,
}

fn s(p: Option<PathBuf>) -> Option<String> {
    p.map(|p| p.to_string_lossy().into_owned())
}

impl Default for Settings {
    fn default() -> Self {
        let downloads = s(dirs::download_dir());
        let desktop = s(dirs::desktop_dir()).unwrap_or_default();
        let documents = s(dirs::document_dir());
        let mut library_dirs: Vec<String> = [
            downloads.clone(),
            documents.clone(),
            s(dirs::picture_dir()),
            s(dirs::video_dir()),
            s(dirs::audio_dir()),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !desktop.is_empty() {
            library_dirs.push(desktop.clone());
        }
        let dev_roots = ["C:\\dev", "C:\\Projects", "C:\\src"]
            .iter()
            .filter(|p| std::path::Path::new(p).is_dir())
            .map(|p| p.to_string())
            .collect();
        let archive_root = documents
            .map(|d| format!("{d}\\Dustling Archive"))
            .unwrap_or_else(|| "C:\\Dustling Archive".into());

        let rules = Kind::ALL.iter().map(|k| (*k, Mode::Ask)).collect();

        Settings {
            inbox_dirs: downloads.into_iter().collect(),
            desktop_dir: desktop,
            dev_roots,
            library_dirs,
            archive_root,
            rules,
            stale_days: 30,
            installer_days: 3,
            backup_days: 14,
            dev_idle_days: 45,
            big_mb: 500,
            big_days: 120,
            dup_min_kb: 256,
            desktop_max_items: 15,
            junk_min_mb: 64,
            recycle_bin_gb: 2,
            snooze_days: 14,
            notify: true,
            quiet_start: 22,
            quiet_end: 9,
            nudge_gap_hours: 6,
            scan_floor_min: 30,
            scan_cap_min: 720,
            idle_min: 3,
            start_with_windows: true,
            onboarded: false,
        }
    }
}

impl Settings {
    pub fn mode(&self, k: Kind) -> Mode {
        self.rules.get(&k).copied().unwrap_or(Mode::Ask)
    }

    pub fn on(&self, k: Kind) -> bool {
        self.mode(k) != Mode::Off
    }

    pub fn is_quiet_hour(&self, hour: u32) -> bool {
        let (a, b) = (self.quiet_start, self.quiet_end);
        if a == b {
            false
        } else if a < b {
            hour >= a && hour < b
        } else {
            hour >= a || hour < b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_midnight() {
        let s = Settings { quiet_start: 22, quiet_end: 9, ..Settings::default() };
        assert!(s.is_quiet_hour(23));
        assert!(s.is_quiet_hour(3));
        assert!(!s.is_quiet_hour(9));
        assert!(!s.is_quiet_hour(15));
    }

    #[test]
    fn settings_roundtrip_with_missing_fields() {
        let s: Settings = serde_json::from_str(r#"{"staleDays": 10, "rules": {"installer": "auto"}}"#).unwrap();
        assert_eq!(s.stale_days, 10);
        assert_eq!(s.mode(Kind::Installer), Mode::Auto);
        assert_eq!(s.mode(Kind::Backup), Mode::Ask);
    }
}

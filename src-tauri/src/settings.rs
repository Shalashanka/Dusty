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

/// How strictly a folder should be kept.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    /// Flag stale things individually (the default).
    Watch,
    /// Everything must leave: Dusty proposes where each item should go.
    Zero,
    /// Keep the folder sorted into category sub-folders.
    Sorted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FolderPolicy {
    pub path: String,
    pub mode: PolicyMode,
    /// Items younger than this are left alone (you're probably still using them).
    pub grace_hours: u32,
    /// How often Dusty may propose a plan for this folder.
    pub ask_every_days: u32,
}

impl Default for FolderPolicy {
    fn default() -> Self {
        FolderPolicy {
            path: String::new(),
            mode: PolicyMode::Watch,
            grace_hours: 48,
            ask_every_days: 3,
        }
    }
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
    /// Extra folders the user asked Dusty to keep an eye on.
    pub watched_dirs: Vec<String>,
    /// Drive roots Dusty indexes and looks after ("C:\\", "D:\\"). Empty = all fixed drives.
    pub drives: Vec<String>,
    pub archive_root: String,
    /// Where hibernated projects are zipped to.
    pub cold_storage: String,
    pub rules: BTreeMap<Kind, Mode>,
    pub policies: Vec<FolderPolicy>,
    /// Category -> destination folder for organizing plans ("recycle" / "leave" are special).
    pub sort_dest: BTreeMap<String, String>,

    pub stale_days: u32,
    pub installer_days: u32,
    pub backup_days: u32,
    pub dev_idle_days: u32,
    pub stale_project_days: u32,
    pub big_mb: u32,
    pub big_days: u32,
    pub dup_min_kb: u32,
    pub desktop_max_items: u32,
    pub junk_min_mb: u32,
    pub recycle_bin_gb: u32,
    pub snooze_days: u32,
    pub unsure_min_gb: u32,
    pub unsure_days: u32,

    pub notify: bool,
    pub quick_actions: bool,
    pub weekly_digest: bool,
    pub quiet_start: u32,
    pub quiet_end: u32,
    pub nudge_gap_hours: u32,

    pub scan_floor_min: u32,
    pub scan_cap_min: u32,
    pub idle_min: u32,

    pub index_enabled: bool,
    pub search_hotkey: String,
    pub watch_startup: bool,
    pub watch_hogs: bool,
    pub learn: bool,

    pub start_with_windows: bool,
    pub onboarded: bool,
    pub dev_mode: bool,
}

fn s(p: Option<PathBuf>) -> Option<String> {
    p.map(|p| p.to_string_lossy().into_owned())
}

pub fn default_sort_dest() -> BTreeMap<String, String> {
    let docs = s(dirs::document_dir()).unwrap_or_else(|| "C:\\Users\\Public\\Documents".into());
    let pics = s(dirs::picture_dir()).unwrap_or_else(|| docs.clone());
    let vids = s(dirs::video_dir()).unwrap_or_else(|| docs.clone());
    let music = s(dirs::audio_dir()).unwrap_or_else(|| docs.clone());
    let dev = if std::path::Path::new("C:\\dev").is_dir() {
        "C:\\dev".to_string()
    } else {
        format!("{docs}\\Code")
    };
    [
        ("installer", format!("{docs}\\Installers")),
        ("archive", format!("{docs}\\Archives")),
        ("document", format!("{docs}\\Documents Inbox")),
        ("spreadsheet", format!("{docs}\\Documents Inbox")),
        ("presentation", format!("{docs}\\Documents Inbox")),
        ("ebook", format!("{docs}\\Books")),
        ("image", format!("{pics}\\Inbox")),
        ("screenshot", format!("{pics}\\Screenshots")),
        ("video", format!("{vids}\\Inbox")),
        ("audio", format!("{music}\\Inbox")),
        ("script", format!("{docs}\\Scripts")),
        ("code", dev),
        ("font", format!("{docs}\\Fonts")),
        ("disk_image", format!("{docs}\\Disk Images")),
        ("design", format!("{docs}\\Design")),
        ("data", format!("{docs}\\Data")),
        ("torrent", "recycle".into()),
        ("partial", "recycle".into()),
        ("shortcut", "leave".into()),
        ("folder", format!("{docs}\\Unsorted Folders")),
        ("other", format!("{docs}\\Unsorted")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

pub fn fixed_drives() -> Vec<String> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut v: Vec<String> = disks
        .list()
        .iter()
        .filter(|d| !d.is_removable() && d.total_space() > 0)
        .map(|d| d.mount_point().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v.dedup();
    v
}

impl Default for Settings {
    fn default() -> Self {
        let downloads = s(dirs::download_dir());
        let desktop = s(dirs::desktop_dir()).unwrap_or_default();
        let documents = s(dirs::document_dir());
        let mut library_dirs: Vec<String> = [
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
            .as_ref()
            .map(|d| format!("{d}\\Dusty Archive"))
            .unwrap_or_else(|| "C:\\Dusty Archive".into());
        let cold_storage = format!("{archive_root}\\Cold Storage");

        let mut rules: BTreeMap<Kind, Mode> = Kind::ALL.iter().map(|k| (*k, Mode::Ask)).collect();
        rules.insert(Kind::SimilarPhoto, Mode::Ask);

        let mut policies = Vec::new();
        if let Some(d) = &downloads {
            policies.push(FolderPolicy {
                path: d.clone(),
                ..Default::default()
            });
        }
        if !desktop.is_empty() {
            policies.push(FolderPolicy {
                path: desktop.clone(),
                ..Default::default()
            });
        }

        Settings {
            inbox_dirs: downloads.into_iter().collect(),
            desktop_dir: desktop,
            dev_roots,
            library_dirs,
            watched_dirs: Vec::new(),
            drives: Vec::new(),
            archive_root,
            cold_storage,
            rules,
            policies,
            sort_dest: default_sort_dest(),
            stale_days: 30,
            installer_days: 3,
            backup_days: 14,
            dev_idle_days: 45,
            stale_project_days: 180,
            big_mb: 500,
            big_days: 120,
            dup_min_kb: 256,
            desktop_max_items: 15,
            junk_min_mb: 64,
            recycle_bin_gb: 2,
            snooze_days: 14,
            unsure_min_gb: 1,
            unsure_days: 365,
            notify: true,
            quick_actions: true,
            weekly_digest: true,
            quiet_start: 22,
            quiet_end: 9,
            nudge_gap_hours: 6,
            scan_floor_min: 30,
            scan_cap_min: 720,
            idle_min: 3,
            index_enabled: true,
            search_hotkey: "Ctrl+Alt+Space".into(),
            watch_startup: true,
            watch_hogs: true,
            learn: true,
            start_with_windows: true,
            onboarded: false,
            dev_mode: false,
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

    /// Drives to index: the configured ones, or every fixed drive.
    pub fn drive_roots(&self) -> Vec<String> {
        if self.drives.is_empty() {
            fixed_drives()
        } else {
            self.drives.clone()
        }
    }

    pub fn dest_for(&self, category: &str) -> String {
        self.sort_dest
            .get(category)
            .cloned()
            .or_else(|| default_sort_dest().get(category).cloned())
            .unwrap_or_else(|| format!("{}\\Unsorted", self.archive_root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_midnight() {
        let s = Settings {
            quiet_start: 22,
            quiet_end: 9,
            ..Settings::default()
        };
        assert!(s.is_quiet_hour(23));
        assert!(s.is_quiet_hour(3));
        assert!(!s.is_quiet_hour(9));
        assert!(!s.is_quiet_hour(15));
    }

    #[test]
    fn settings_roundtrip_with_missing_fields() {
        let s: Settings =
            serde_json::from_str(r#"{"staleDays": 10, "rules": {"installer": "auto"}}"#).unwrap();
        assert_eq!(s.stale_days, 10);
        assert_eq!(s.mode(Kind::Installer), Mode::Auto);
        assert_eq!(s.mode(Kind::Backup), Mode::Ask);
        assert_eq!(s.dest_for("script").ends_with("Scripts"), true);
    }
}

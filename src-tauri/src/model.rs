use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

/// What a detector found. Ordered by priority: when two detectors flag the
/// same path, the one declared first wins.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Duplicate,
    SimilarPhoto,
    Installer,
    ExtractedArchive,
    Backup,
    StaleProject,
    DevJunk,
    BigDormant,
    StaleDownload,
    Unsure,
    DesktopClutter,
    FolderPlan,
    EmptyFolder,
    SystemJunk,
    RecycleBin,
    NewStartup,
    ResourceHog,
}

impl Kind {
    pub const ALL: [Kind; 17] = [
        Kind::StaleDownload,
        Kind::Installer,
        Kind::ExtractedArchive,
        Kind::Duplicate,
        Kind::SimilarPhoto,
        Kind::DevJunk,
        Kind::StaleProject,
        Kind::Backup,
        Kind::BigDormant,
        Kind::Unsure,
        Kind::DesktopClutter,
        Kind::FolderPlan,
        Kind::SystemJunk,
        Kind::RecycleBin,
        Kind::EmptyFolder,
        Kind::NewStartup,
        Kind::ResourceHog,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Duplicate => "duplicate",
            Kind::SimilarPhoto => "similar_photo",
            Kind::Installer => "installer",
            Kind::ExtractedArchive => "extracted_archive",
            Kind::Backup => "backup",
            Kind::StaleProject => "stale_project",
            Kind::DevJunk => "dev_junk",
            Kind::BigDormant => "big_dormant",
            Kind::StaleDownload => "stale_download",
            Kind::Unsure => "unsure",
            Kind::DesktopClutter => "desktop_clutter",
            Kind::FolderPlan => "folder_plan",
            Kind::EmptyFolder => "empty_folder",
            Kind::SystemJunk => "system_junk",
            Kind::RecycleBin => "recycle_bin",
            Kind::NewStartup => "new_startup",
            Kind::ResourceHog => "resource_hog",
        }
    }

    pub fn noun(self) -> &'static str {
        match self {
            Kind::Duplicate => "duplicate",
            Kind::SimilarPhoto => "near-identical photo",
            Kind::Installer => "old installer",
            Kind::ExtractedArchive => "extracted archive",
            Kind::Backup => "forgotten backup",
            Kind::StaleProject => "sleepy project",
            Kind::DevJunk => "idle project",
            Kind::BigDormant => "big dormant file",
            Kind::StaleDownload => "stale download",
            Kind::Unsure => "mystery folder",
            Kind::DesktopClutter => "desktop pile",
            Kind::FolderPlan => "tidy plan",
            Kind::EmptyFolder => "empty folder",
            Kind::SystemJunk => "junk location",
            Kind::RecycleBin => "full recycle bin",
            Kind::NewStartup => "new startup app",
            Kind::ResourceHog => "resource hog",
        }
    }

    /// Findings that are about files on disk (and can be protected/previewed).
    pub fn is_file_based(self) -> bool {
        !matches!(
            self,
            Kind::NewStartup | Kind::ResourceHog | Kind::RecycleBin
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Send to the Windows Recycle Bin (undoable).
    Recycle,
    /// Permanent delete, only offered for regenerable things.
    Delete,
    /// Move into the archive folder (undoable).
    Archive,
    /// Move loose desktop items into a dated folder (undoable).
    Sweep,
    /// Empty the Recycle Bin.
    Empty,
    /// Zip a project to cold storage and leave a note (undoable).
    Hibernate,
    /// Open the organizing plan (handled by the UI).
    Organize,
    /// Disable a startup entry (undoable).
    Disable,
    /// End a process.
    Quit,
    /// Never ask about this again.
    Keep,
    /// Ask again later.
    Snooze,
}

impl Action {
    pub fn key(self) -> &'static str {
        match self {
            Action::Recycle => "recycle",
            Action::Delete => "delete",
            Action::Archive => "archive",
            Action::Sweep => "sweep",
            Action::Empty => "empty",
            Action::Hibernate => "hibernate",
            Action::Organize => "organize",
            Action::Disable => "disable",
            Action::Quit => "quit",
            Action::Keep => "keep",
            Action::Snooze => "snooze",
        }
    }

    /// Removes something from disk or the system.
    pub fn is_removal(self) -> bool {
        matches!(
            self,
            Action::Recycle | Action::Delete | Action::Empty | Action::Quit
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub path: String,
    /// Kind-specific companions: the original of a duplicate, the junk dirs
    /// of a project, the loose items of the desktop.
    pub related: Vec<String>,
    pub bytes: u64,
    pub items: u64,
    pub last_touched: i64,
    pub is_dir: bool,
    pub reason: String,
    pub origin: Option<String>,
    pub confidence: f32,
    pub suggested: Action,
    pub actions: Vec<Action>,
    pub ext: String,
    /// For system junk: only files older than this many days are purged.
    pub older_than_days: Option<u32>,
    /// The program this is about (knowledge base / startup / process).
    pub app: Option<String>,
}

impl Finding {
    pub fn new(
        kind: Kind,
        path: &str,
        title: String,
        suggested: Action,
        actions: Vec<Action>,
    ) -> Self {
        Finding {
            id: finding_id(kind, path),
            kind,
            title,
            path: path.to_string(),
            related: Vec::new(),
            bytes: 0,
            items: 1,
            last_touched: 0,
            is_dir: false,
            reason: String::new(),
            origin: None,
            confidence: 0.5,
            suggested,
            actions,
            ext: String::new(),
            older_than_days: None,
            app: None,
        }
    }

    /// Ranking used for triage order: confident and big first.
    pub fn score(&self) -> f64 {
        let mb = self.bytes as f64 / 1_048_576.0;
        self.confidence as f64 * (1.0 + (1.0 + mb).ln())
    }
}

pub fn finding_id(kind: Kind, path: &str) -> String {
    let key = format!("{}|{}", kind.key(), path.to_lowercase());
    format!("{:016x}", xxh3_64(key.as_bytes()))
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub at: i64,
    pub duration_ms: u64,
    pub count: usize,
    pub bytes: u64,
    pub new_count: usize,
    pub auto_count: usize,
    pub auto_freed: u64,
    pub background: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRow {
    pub id: i64,
    pub at: i64,
    pub action: String,
    pub kind: String,
    pub finding_id: String,
    pub title: String,
    pub path: String,
    pub dest: Option<String>,
    pub bytes: u64,
    pub batch: String,
    pub undoable: bool,
    pub undone: bool,
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub freed_total: u64,
    pub freed_week: u64,
    pub tidied_total: u64,
    pub actions_total: u64,
    pub kept_total: u64,
    pub streak_days: u32,
}

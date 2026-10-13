use crate::health::procs::Monitor;
use crate::index::IndexState;
use crate::model::{Finding, ScanSummary};
use crate::settings::Settings;
use crate::store::Store;
use parking_lot::{Condvar, Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64};

#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    pub next_due: i64,
    pub interval_min: f64,
    pub force: bool,
}

pub struct AppState {
    pub store: Store,
    pub data_dir: PathBuf,
    pub settings: RwLock<Settings>,
    pub findings: RwLock<Vec<Finding>>,
    /// Findings removed by an action, keyed by history id, so undo can bring them back.
    pub acted: Mutex<HashMap<i64, Finding>>,
    pub scanning: AtomicBool,
    pub phase: RwLock<String>,
    pub last_scan: RwLock<Option<ScanSummary>>,
    pub schedule: Mutex<Schedule>,
    pub wake: Condvar,
    pub paused_until: AtomicI64,
    pub launched_hidden: bool,
    pub shown_once: AtomicBool,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub index: Arc<IndexState>,
    pub procs: Mutex<Monitor>,
    /// True while the Speed view is open (sample processes more often).
    pub live_procs: AtomicBool,
    /// When the current scan started (0 when idle).
    pub scan_started: AtomicI64,
}

impl AppState {
    pub fn new(store: Store, data_dir: PathBuf, launched_hidden: bool) -> Self {
        let settings = store.settings();
        let paused = store.get_i64("paused_until");
        let floor = settings.scan_floor_min as f64;
        AppState {
            store,
            data_dir,
            settings: RwLock::new(settings),
            findings: RwLock::new(Vec::new()),
            acted: Mutex::new(HashMap::new()),
            scanning: AtomicBool::new(false),
            phase: RwLock::new(String::new()),
            last_scan: RwLock::new(None),
            schedule: Mutex::new(Schedule {
                // First look shortly after startup.
                next_due: crate::timeutil::now() + 45,
                interval_min: floor,
                force: false,
            }),
            wake: Condvar::new(),
            paused_until: AtomicI64::new(paused),
            launched_hidden,
            shown_once: AtomicBool::new(false),
            watcher: Mutex::new(None),
            index: Arc::new(IndexState::default()),
            procs: Mutex::new(Monitor::new()),
            live_procs: AtomicBool::new(false),
            scan_started: AtomicI64::new(0),
        }
    }

    #[cfg(test)]
    pub fn for_tests(store: Store) -> Self {
        Self::new(store, std::env::temp_dir(), true)
    }

    pub fn request_scan(&self) {
        let mut s = self.schedule.lock();
        s.force = true;
        self.wake.notify_all();
    }

    pub fn pending(&self) -> (usize, u64) {
        let f = self.findings.read();
        (f.len(), f.iter().map(|x| x.bytes).sum())
    }

    pub fn index_path(&self) -> PathBuf {
        self.data_dir.join("index.bin")
    }

    /// Extension overrides learned from edited plans ("ext:pdf" -> folder).
    pub fn sort_overrides(&self) -> HashMap<String, String> {
        self.store
            .get("sort_overrides")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

use crate::model::{Finding, ScanSummary};
use crate::settings::Settings;
use crate::store::Store;
use parking_lot::{Condvar, Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64};

#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    pub next_due: i64,
    pub interval_min: f64,
    pub force: bool,
}

pub struct AppState {
    pub store: Store,
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
}

impl AppState {
    pub fn new(store: Store, launched_hidden: bool) -> Self {
        let settings = store.settings();
        let paused = store.get_i64("paused_until");
        let floor = settings.scan_floor_min as f64;
        AppState {
            store,
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
        }
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
}

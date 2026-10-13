use crate::actions::{self, Outcome};
use crate::model::{Action, Finding, HistoryRow, ScanSummary, Stats};
use crate::settings::Settings;
use crate::state::AppState;
use crate::timeutil::now;
use crate::walk;
use rayon::prelude::*;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

type R<T> = Result<T, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    findings: Vec<Finding>,
    scanning: bool,
    phase: String,
    last_scan: Option<ScanSummary>,
    last_scan_at: i64,
    next_scan_at: i64,
    paused_until: i64,
    stats: Stats,
    idle_secs: u64,
}

#[tauri::command]
pub fn snapshot(state: State<AppState>) -> Snapshot {
    Snapshot {
        findings: state.findings.read().clone(),
        scanning: state.scanning.load(Ordering::SeqCst),
        phase: state.phase.read().clone(),
        last_scan: state.last_scan.read().clone(),
        last_scan_at: state.store.get_i64("last_scan"),
        next_scan_at: state.schedule.lock().next_due,
        paused_until: state.paused_until.load(Ordering::Relaxed),
        stats: state.store.stats(),
        idle_secs: crate::winsys::idle_secs(),
    }
}

/// Called by the UI once it has painted; shows the window unless launched
/// silently at login.
#[tauri::command]
pub fn ready(app: AppHandle, state: State<AppState>) {
    if !state.launched_hidden && !state.shown_once.swap(true, Ordering::SeqCst) {
        crate::show_main(&app);
    }
}

#[tauri::command]
pub fn scan_now(state: State<AppState>) {
    state.request_scan();
}

#[tauri::command]
pub async fn act(app: AppHandle, ids: Vec<String>, action: Action) -> R<Outcome> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let out = actions::apply(&state, &ids, action, false);
        crate::tray::refresh(&app);
        crate::scheduler::emit_changed(&app);
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn undo(app: AppHandle, history_id: i64) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let r = actions::undo(&state, history_id);
        crate::tray::refresh(&app);
        crate::scheduler::emit_changed(&app);
        r
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn history(state: State<AppState>, limit: Option<u32>) -> Vec<HistoryRow> {
    state.store.history(limit.unwrap_or(300))
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.read().clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> R<()> {
    let autostart = app.autolaunch();
    let enabled = autostart.is_enabled().unwrap_or(false);
    if settings.start_with_windows != enabled {
        let r = if settings.start_with_windows { autostart.enable() } else { autostart.disable() };
        r.map_err(|e| e.to_string())?;
    }
    state.store.save_settings(&settings);
    *state.settings.write() = settings;
    crate::watcher::start(&app);
    Ok(())
}

#[tauri::command]
pub fn pause(app: AppHandle, state: State<AppState>, minutes: i64) {
    let until = if minutes > 0 { now() + minutes * 60 } else { 0 };
    state.paused_until.store(until, Ordering::Relaxed);
    state.store.set("paused_until", &until.to_string());
    crate::tray::refresh(&app);
    crate::scheduler::emit_changed(&app);
}

#[tauri::command]
pub fn reveal(path: String) {
    if path.starts_with("shell:") {
        let _ = std::process::Command::new("explorer.exe").arg(&path).spawn();
    } else {
        crate::winsys::reveal(&path);
    }
}

#[tauri::command]
pub fn open_path(path: String) -> R<()> {
    let _ = std::process::Command::new("explorer.exe").arg(&path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    name: String,
    mount: String,
    total: u64,
    free: u64,
    kind: String,
    fs: String,
}

#[tauri::command]
pub fn drives() -> Vec<Drive> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut v: Vec<Drive> = disks
        .list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| Drive {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            total: d.total_space(),
            free: d.available_space(),
            kind: match d.kind() {
                sysinfo::DiskKind::SSD => "ssd".into(),
                sysinfo::DiskKind::HDD => "hdd".into(),
                _ => "unknown".into(),
            },
            fs: d.file_system().to_string_lossy().into_owned(),
        })
        .collect();
    v.sort_by(|a, b| a.mount.cmp(&b.mount));
    v.dedup_by(|a, b| a.mount == b.mount);
    v
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceNode {
    name: String,
    path: String,
    bytes: u64,
    files: u64,
    is_dir: bool,
    ext: String,
    newest: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceResult {
    path: String,
    bytes: u64,
    files: u64,
    children: Vec<SpaceNode>,
    ms: u64,
}

/// Sizes of every child of `path` (recursive), biggest first.
#[tauri::command]
pub async fn space(path: String) -> R<SpaceResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let t = std::time::Instant::now();
        let root = Path::new(&path);
        if !root.is_dir() {
            return Err("that folder doesn't exist".to_string());
        }
        let mut children: Vec<SpaceNode> = walk::children(root)
            .into_par_iter()
            .filter(|(_, md)| !walk::is_link(md))
            .map(|(p, md)| {
                let (bytes, files, newest) = if md.is_dir() {
                    let st = walk::dir_stats(&p);
                    (st.bytes, st.files, st.newest)
                } else {
                    (if walk::is_cloud(&md) { 0 } else { md.len() }, 1, walk::touched(&md))
                };
                SpaceNode {
                    name: walk::name_of(&p),
                    ext: if md.is_dir() { String::new() } else { walk::ext_of(&p) },
                    path: p.to_string_lossy().into_owned(),
                    bytes,
                    files,
                    is_dir: md.is_dir(),
                    newest,
                }
            })
            .collect();
        children.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        Ok(SpaceResult {
            path: path.clone(),
            bytes: children.iter().map(|c| c.bytes).sum(),
            files: children.iter().map(|c| c.files).sum(),
            children,
            ms: t.elapsed().as_millis() as u64,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sends something from the Space view straight to the Recycle Bin.
#[tauri::command]
pub async fn recycle_path(app: AppHandle, path: String, bytes: u64) -> R<i64> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        trash::delete(&path).map_err(|e| e.to_string())?;
        let title = walk::name_of(Path::new(&path));
        let id = state.store.add_history(crate::store::NewHistory {
            action: "recycle",
            kind: "manual",
            finding_id: "",
            title: &title,
            path: &path,
            dest: None,
            bytes,
            batch: &actions::new_batch(),
            undoable: true,
            auto: false,
        });
        crate::scheduler::emit_changed(&app);
        Ok(id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn home_dir() -> String {
    dirs::home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "C:\\".into())
}

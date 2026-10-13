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
    // Debug builds never register themselves to run at login.
    if settings.start_with_windows != enabled && !cfg!(debug_assertions) {
        let r = if settings.start_with_windows {
            autostart.enable()
        } else {
            autostart.disable()
        };
        r.map_err(|e| e.to_string())?;
    }
    let old = state.settings.read().clone();
    state.store.save_settings(&settings);
    let drives_changed =
        old.drives != settings.drives || old.index_enabled != settings.index_enabled;
    let hotkey_changed = old.search_hotkey != settings.search_hotkey;
    *state.settings.write() = settings;
    crate::watcher::start(&app);
    if drives_changed {
        state.index.rebuild.store(true, Ordering::SeqCst);
    }
    if hotkey_changed {
        crate::register_hotkey(&app)?;
    }
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
        let _ = std::process::Command::new("explorer.exe")
            .arg(&path)
            .spawn();
    } else {
        crate::winsys::reveal(&path);
    }
}

#[tauri::command]
pub fn open_path(path: String) -> R<()> {
    let _ = std::process::Command::new("explorer.exe")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
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
                    (
                        if walk::is_cloud(&md) { 0 } else { md.len() },
                        1,
                        walk::touched(&md),
                    )
                };
                SpaceNode {
                    name: walk::name_of(&p),
                    ext: if md.is_dir() {
                        String::new()
                    } else {
                        walk::ext_of(&p)
                    },
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
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "C:\\".into())
}

// ---- index & search ---------------------------------------------------------

#[tauri::command]
pub fn index_status(state: State<AppState>) -> crate::index::IndexStatus {
    state.index.status()
}

#[tauri::command]
pub fn index_rebuild(state: State<AppState>) {
    state.index.rebuild.store(true, Ordering::SeqCst);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    query: crate::nlq::Query,
    hits: Vec<crate::index::Hit>,
    ms: u64,
    indexed: bool,
}

#[tauri::command]
pub async fn search(app: AppHandle, query: String, limit: Option<usize>) -> R<SearchResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let t = std::time::Instant::now();
        let state = app.state::<AppState>();
        let q = crate::nlq::parse(&query);
        let hits = if q.is_empty() {
            vec![]
        } else {
            state.index.search(&q, limit.unwrap_or(60))
        };
        SearchResult {
            indexed: state.index.get().is_some(),
            query: q,
            hits,
            ms: t.elapsed().as_millis() as u64,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Nested sizes for the Space view: instant from the index, live walk otherwise.
#[tauri::command]
pub async fn space_tree(
    app: AppHandle,
    path: String,
    depth: Option<usize>,
) -> R<crate::index::TreeNode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        if let Some(idx) = state.index.get() {
            if let Some(t) = crate::index::tree(&idx, &path, depth.unwrap_or(3)) {
                return Ok(t);
            }
        }
        let root = Path::new(&path);
        if !root.is_dir() {
            return Err("that folder does not exist".to_string());
        }
        let mut children: Vec<crate::index::TreeNode> = walk::children(root)
            .into_par_iter()
            .filter(|(_, md)| !walk::is_link(md))
            .map(|(p, md)| {
                let (bytes, files, newest) = if md.is_dir() {
                    let st = walk::dir_stats(&p);
                    (st.bytes, st.files, st.newest)
                } else {
                    (md.len(), 1, walk::touched(&md))
                };
                crate::index::TreeNode {
                    name: walk::name_of(&p),
                    path: p.to_string_lossy().into_owned(),
                    bytes,
                    files,
                    is_dir: md.is_dir(),
                    newest,
                    children: vec![],
                }
            })
            .collect();
        children.sort_by_key(|c| std::cmp::Reverse(c.bytes));
        Ok(crate::index::TreeNode {
            name: walk::name_of(root),
            bytes: children.iter().map(|c| c.bytes).sum(),
            files: children.iter().map(|c| c.files).sum(),
            path,
            is_dir: true,
            newest: 0,
            children,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn kb_lookup(paths: Vec<String>) -> Vec<Option<crate::kb::KbHit>> {
    let resolved = crate::kb::resolve();
    paths
        .iter()
        .map(|p| crate::kb::lookup(p, &resolved))
        .collect()
}

#[tauri::command]
pub fn kb_all() -> Vec<crate::kb::KbEntry> {
    crate::kb::all()
}

// ---- previews -----------------------------------------------------------------

#[tauri::command]
pub async fn thumbnail(path: String, size: Option<i32>) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::thumbs::data_url(&path, size.unwrap_or(256))
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
pub fn read_text(path: String, max: Option<usize>) -> R<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; max.unwrap_or(16 * 1024)];
    let n = f.read(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(n);
    if buf.iter().take(2048).filter(|b| **b == 0).count() > 2 {
        return Err("binary file".into());
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[tauri::command]
pub fn open_file(path: String) -> R<()> {
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_path(app: AppHandle, path: String, bytes: u64) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let p = Path::new(&path);
        walk::remove_tree(p).map_err(|e| e.to_string())?;
        let title = walk::name_of(p);
        state.store.add_history(crate::store::NewHistory {
            action: "delete",
            kind: "manual",
            finding_id: "",
            title: &title,
            path: &path,
            dest: None,
            bytes,
            batch: &actions::new_batch(),
            undoable: false,
            auto: false,
        });
        crate::scheduler::emit_changed(&app);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn recycle_bin() -> (u64, u64) {
    crate::winsys::recycle_bin_info()
}

#[tauri::command]
pub async fn empty_bin(app: AppHandle) -> R<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        let (bytes, _) = crate::winsys::recycle_bin_info();
        crate::winsys::empty_recycle_bin()?;
        let state = app.state::<AppState>();
        state.store.add_history(crate::store::NewHistory {
            action: "empty",
            kind: "recycle_bin",
            finding_id: "",
            title: "Recycle Bin",
            path: "shell:RecycleBinFolder",
            dest: None,
            bytes,
            batch: &actions::new_batch(),
            undoable: false,
            auto: false,
        });
        state
            .findings
            .write()
            .retain(|f| f.kind != crate::model::Kind::RecycleBin);
        crate::scheduler::emit_changed(&app);
        Ok(bytes)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---- protection, plans, hibernation -----------------------------------------

#[tauri::command]
pub fn protect(app: AppHandle, state: State<AppState>, path: String, days: u32) {
    actions::protect(&state, &path, days);
    crate::tray::refresh(&app);
    crate::scheduler::emit_changed(&app);
}

#[tauri::command]
pub fn unprotect(app: AppHandle, state: State<AppState>, path: String) {
    state.store.unprotect(&path);
    crate::scheduler::emit_changed(&app);
}

#[tauri::command]
pub fn protected_list(state: State<AppState>) -> Vec<crate::store::Protected> {
    state.store.protected()
}

#[tauri::command]
pub async fn make_plan(app: AppHandle, folder: String) -> R<crate::organize::Plan> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let s = state.settings.read().clone();
        Ok(crate::organize::make_plan(
            &s,
            &folder,
            &crate::winsys::installed_programs(),
            &state.sort_overrides(),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_plan(
    app: AppHandle,
    folder: String,
    items: Vec<crate::organize::PlanItem>,
    original: Vec<crate::organize::PlanItem>,
) -> R<Outcome> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let out = actions::apply_plan(&state, &folder, &items, &original);
        crate::tray::refresh(&app);
        crate::scheduler::emit_changed(&app);
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hibernated(state: State<AppState>) -> Vec<crate::store::Hibernated> {
    state.store.hibernated()
}

#[tauri::command]
pub async fn wake_project(app: AppHandle, project: String, archive: String) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::hibernate::wake(Path::new(&project), Path::new(&archive))?;
        app.state::<AppState>().store.mark_restored(&archive);
        crate::scheduler::emit_changed(&app);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---- speed --------------------------------------------------------------------

#[tauri::command]
pub fn startup_list() -> Vec<crate::health::startup::StartupItem> {
    crate::health::startup::list()
}

#[tauri::command]
pub async fn startup_set(id: String, enabled: bool) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || crate::health::startup::set_enabled(&id, enabled))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcSnapshot {
    groups: Vec<crate::health::procs::ProcGroup>,
    total_memory: u64,
    used_memory: u64,
}

#[tauri::command]
pub async fn processes(app: AppHandle) -> R<ProcSnapshot> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let groups = state.procs.lock().groups();
        let (total_memory, used_memory) = crate::health::procs::Monitor::total_memory();
        ProcSnapshot {
            groups,
            total_memory,
            used_memory,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_live_procs(state: State<AppState>, live: bool) {
    state.live_procs.store(live, Ordering::Relaxed);
}

#[tauri::command]
pub fn kill_processes(state: State<AppState>, pids: Vec<u32>) -> usize {
    state.procs.lock().kill(&pids)
}

#[tauri::command]
pub async fn system_files() -> R<crate::health::tune::SystemFiles> {
    tauri::async_runtime::spawn_blocking(crate::health::tune::system_files)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn power_plans() -> R<Vec<crate::health::tune::PowerPlan>> {
    tauri::async_runtime::spawn_blocking(crate::health::tune::power_plans)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_power_plan(guid: String) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || crate::health::tune::set_power_plan(&guid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn optimize_info() -> R<crate::health::tune::OptimizeInfo> {
    tauri::async_runtime::spawn_blocking(crate::health::tune::optimize_schedule)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn optimize_drive(letter: String) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || crate::health::tune::optimize_drive(&letter))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn analyze_drive(letter: String) -> R<u32> {
    tauri::async_runtime::spawn_blocking(move || crate::health::tune::analyze_drive(&letter))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tune(action: String, arg: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || match action.as_str() {
        "hibernation_off" => crate::health::tune::set_hibernation(false),
        "hibernation_on" => crate::health::tune::set_hibernation(true),
        "delivery_optimization" => crate::health::tune::clear_delivery_optimization(),
        "component_cleanup" => crate::health::tune::component_cleanup(),
        "remove_app" => crate::health::tune::remove_app(&arg.unwrap_or_default()),
        "storage_sense" => open_file("ms-settings:storagesense".into()),
        "system_protection" => std::process::Command::new("SystemPropertiesProtection.exe")
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string()),
        _ => Err("unknown tune-up".into()),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn preinstalled_apps() -> R<Vec<crate::health::tune::AppxApp>> {
    tauri::async_runtime::spawn_blocking(crate::health::tune::preinstalled_apps)
        .await
        .map_err(|e| e.to_string())
}

// ---- search window --------------------------------------------------------------

#[tauri::command]
pub fn hide_search(app: AppHandle) {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn open_in_dusty(app: AppHandle, view: String) {
    hide_search(app.clone());
    crate::show_main(&app);
    let _ = tauri::Emitter::emit(&app, "navigate", view);
}

// ---- developer mode -------------------------------------------------------------

#[tauri::command]
pub fn dev_action(app: AppHandle, state: State<AppState>, action: String) -> R<String> {
    match action.as_str() {
        "toast" => {
            crate::toast::show(
                &app,
                "Test notification",
                "If you can read this, toasts work.",
                &[("Open Dusty", "review"), ("Dismiss", "later")],
            );
            Ok("sent".into())
        }
        "nudge" => {
            let f: Vec<crate::model::Finding> =
                state.findings.read().iter().take(5).cloned().collect();
            if f.is_empty() {
                return Err("no findings to nudge about".into());
            }
            crate::scheduler::maybe_nudge(&app, &f, 0, 0, true);
            Ok("nudged".into())
        }
        "autopilot_toast" => {
            crate::scheduler::maybe_nudge(&app, &[], 3, 1_234_567_890, true);
            Ok("sent".into())
        }
        "digest" => {
            crate::scheduler::send_digest(&app);
            Ok("sent".into())
        }
        "scan_bg" => {
            let a = app.clone();
            std::thread::spawn(move || crate::scheduler::run_scan(&a, true));
            Ok("started".into())
        }
        "reset_seen" => {
            state.store.reset_all();
            Ok("history, decisions and caches cleared".into())
        }
        "reset_onboarding" => {
            let mut s = state.settings.write();
            s.onboarded = false;
            state.store.save_settings(&s);
            Ok("onboarding will show again".into())
        }
        "rebuild_index" => {
            state.index.rebuild.store(true, Ordering::SeqCst);
            Ok("rebuilding".into())
        }
        "open_data" => {
            open_file(state.data_dir.to_string_lossy().into_owned())?;
            Ok("opened".into())
        }
        _ => Err("unknown action".into()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    debug: bool,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        debug: cfg!(debug_assertions),
    }
}

#[tauri::command]
pub fn toggle_search(app: AppHandle) {
    crate::toggle_search(&app);
}

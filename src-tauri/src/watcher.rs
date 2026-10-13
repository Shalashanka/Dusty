//! Watches inbox folders (Downloads, Desktop) so Dustling knows when things
//! *arrived*, which file timestamps often hide.

use crate::state::AppState;
use crate::timeutil::now;
use notify::event::{CreateKind, ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const TEMP_EXT: &[&str] = &[
    "crdownload",
    "part",
    "tmp",
    "partial",
    "download",
    "opdownload",
];

pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let dirs: Vec<PathBuf> = {
        let s = state.settings.read();
        let mut d: Vec<PathBuf> = s.inbox_dirs.iter().map(PathBuf::from).collect();
        if !s.desktop_dir.is_empty() {
            d.push(PathBuf::from(&s.desktop_dir));
        }
        d
    };
    let handle = app.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let Ok(ev) = res else { return };
        let arrived = matches!(
            ev.kind,
            EventKind::Create(CreateKind::Any | CreateKind::File | CreateKind::Folder)
                | EventKind::Modify(ModifyKind::Name(
                    RenameMode::To | RenameMode::Both | RenameMode::Any
                ))
        );
        if !arrived {
            return;
        }
        let state = handle.state::<AppState>();
        let t = now();
        for p in ev.paths.iter().filter(|p| !is_temp(p)) {
            state.store.mark_first_seen(&p.to_string_lossy(), t);
        }
        crate::scheduler::nudge_dirty(&state);
    });
    let Ok(mut watcher) = watcher else { return };
    for d in dirs.iter().filter(|d| d.is_dir()) {
        let _ = watcher.watch(d, RecursiveMode::NonRecursive);
    }
    *state.watcher.lock() = Some(watcher);
}

fn is_temp(p: &Path) -> bool {
    let ext = crate::walk::ext_of(p);
    TEMP_EXT.contains(&ext.as_str())
}

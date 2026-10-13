//! Carrying out decisions, and undoing them.

use crate::detect::remove_empty_tree;
use crate::model::{Action, Finding, Kind};
use crate::settings::Settings;
use crate::state::AppState;
use crate::store::NewHistory;
use crate::timeutil::{DAY, day_stamp, month_stamp, now};
use crate::walk::{self, move_path, name_of, unique_dest};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub done: usize,
    pub freed: u64,
    pub moved: usize,
    pub errors: Vec<String>,
    pub batch: String,
}

pub fn new_batch() -> String {
    format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

/// Applies `action` to the findings with `ids`. Successful ones leave the
/// pending list and can be undone through history.
pub fn apply(state: &AppState, ids: &[String], action: Action, auto: bool) -> Outcome {
    let settings = state.settings.read().clone();
    let targets: Vec<Finding> = {
        let all = state.findings.read();
        all.iter()
            .filter(|f| ids.contains(&f.id))
            .cloned()
            .collect()
    };
    let mut out = Outcome {
        batch: new_batch(),
        ..Default::default()
    };
    for f in targets {
        if !allowed(&f, action) {
            out.errors
                .push(format!("{}: can't {} this", f.title, action.key()));
            continue;
        }
        match apply_one(state, &settings, &f, action, &out.batch, auto) {
            Ok((hid, freed, moved)) => {
                if settings.learn && !auto {
                    crate::learn::record(&state.store, &f, action);
                }
                // Caches refill; don't nag about the same spot right away.
                if f.kind == Kind::SystemJunk && action == Action::Delete {
                    let days = f.older_than_days.unwrap_or(7).max(7) as i64;
                    state
                        .store
                        .set_decision(&f.id, &f.path, "snooze", now() + days * DAY);
                }
                out.done += 1;
                out.freed += freed;
                out.moved += moved;
                state.findings.write().retain(|x| x.id != f.id);
                state.acted.lock().insert(hid, f);
            }
            Err(e) => out.errors.push(format!("{}: {e}", f.title)),
        }
    }
    out
}

/// Whether `action` may be applied: the finding's own actions, plus Recycle
/// and Delete as alternatives for each other on plain files and folders.
pub fn allowed(f: &Finding, action: Action) -> bool {
    if f.actions.contains(&action) || matches!(action, Action::Keep | Action::Snooze) {
        return true;
    }
    let swappable = f.kind.is_file_based()
        && !matches!(
            f.kind,
            Kind::SystemJunk | Kind::FolderPlan | Kind::DesktopClutter
        );
    match action {
        Action::Delete => swappable && f.actions.contains(&Action::Recycle),
        Action::Recycle => {
            swappable && f.actions.contains(&Action::Delete) && f.kind != Kind::EmptyFolder
        }
        _ => false,
    }
}

/// Returns (history id, bytes freed, items moved).
fn apply_one(
    state: &AppState,
    s: &Settings,
    f: &Finding,
    action: Action,
    batch: &str,
    auto: bool,
) -> Result<(i64, u64, usize), String> {
    let store = &state.store;
    let path = Path::new(&f.path);
    let mut hist = NewHistory {
        action: action.key(),
        kind: f.kind.key(),
        finding_id: &f.id,
        title: &f.title,
        path: &f.path,
        dest: None,
        bytes: f.bytes,
        batch,
        undoable: true,
        auto,
    };
    let err = |e: std::io::Error| friendly(e);

    match action {
        Action::Keep => {
            store.set_decision(&f.id, &f.path, "keep", 0);
            if f.kind == Kind::NewStartup {
                remember_startup(state, &f.path);
            }
            Ok((store.add_history(hist), 0, 0))
        }
        Action::Snooze => {
            let days = if f.kind == Kind::FolderPlan {
                s.policies
                    .iter()
                    .find(|p| p.path.eq_ignore_ascii_case(&f.path))
                    .map(|p| p.ask_every_days)
                    .unwrap_or(3)
            } else {
                s.snooze_days
            };
            store.set_decision(&f.id, &f.path, "snooze", now() + days as i64 * DAY);
            Ok((store.add_history(hist), 0, 0))
        }
        Action::Hibernate => {
            let out = crate::hibernate::hibernate(path, Path::new(&s.cold_storage))?;
            let a = out.archive.to_string_lossy().into_owned();
            store.add_hibernated(&f.path, &a, out.original_bytes, out.files);
            hist.dest = Some(&a);
            hist.bytes = out.original_bytes;
            let same_drive = a
                .get(..2)
                .map(|d| f.path.to_lowercase().starts_with(&d.to_lowercase()))
                .unwrap_or(false);
            let freed = if same_drive {
                out.original_bytes.saturating_sub(out.archive_bytes)
            } else {
                out.original_bytes
            };
            Ok((store.add_history(hist), freed, 0))
        }
        Action::Organize => Err("open the plan to organize this folder".into()),
        Action::Disable => {
            let id = f.path.strip_prefix("startup:").ok_or("not a startup app")?;
            crate::health::startup::set_enabled(id, false)?;
            remember_startup(state, &f.path);
            Ok((store.add_history(hist), 0, 0))
        }
        Action::Quit => {
            hist.undoable = false;
            let pids: Vec<u32> = f.related.iter().filter_map(|p| p.parse().ok()).collect();
            let n = state.procs.lock().kill(&pids);
            if n == 0 {
                return Err("couldn't end it (already gone, or protected)".into());
            }
            Ok((store.add_history(hist), 0, 0))
        }
        Action::Recycle => {
            if !path.exists() {
                return Err("it's already gone".into());
            }
            trash::delete(path).map_err(|e| e.to_string())?;
            Ok((store.add_history(hist), f.bytes, 0))
        }
        Action::Delete => {
            hist.undoable = false;
            let freed = match f.kind {
                Kind::SystemJunk => walk::purge_older(path, f.older_than_days.unwrap_or(7)),
                Kind::DevJunk => {
                    let mut freed = 0;
                    for j in &f.related {
                        let jp = Path::new(j);
                        // Only ever delete inside the project.
                        if !jp.starts_with(path) {
                            continue;
                        }
                        let st = walk::dir_stats(jp);
                        if walk::remove_tree(jp).is_ok() || !jp.exists() {
                            freed += st.bytes;
                        }
                    }
                    freed
                }
                Kind::EmptyFolder => {
                    remove_empty_tree(path).map_err(err)?;
                    0
                }
                _ => {
                    walk::remove_tree(path).map_err(err)?;
                    f.bytes
                }
            };
            hist.bytes = freed;
            Ok((store.add_history(hist), freed, 0))
        }
        Action::Archive => {
            let dir = PathBuf::from(&s.archive_root).join(month_stamp());
            let dest = unique_dest(&dir, &name_of(path));
            move_path(path, &dest).map_err(err)?;
            let d = dest.to_string_lossy().into_owned();
            hist.dest = Some(&d);
            Ok((store.add_history(hist), 0, 1))
        }
        Action::Sweep => {
            let dir = Path::new(&s.desktop_dir)
                .join(crate::detect::SWEPT_DIR)
                .join(day_stamp());
            let mut moved = 0;
            let mut last = 0;
            for item in &f.related {
                let src = Path::new(item);
                if !src.exists() {
                    continue;
                }
                let dest = unique_dest(&dir, &name_of(src));
                if move_path(src, &dest).is_ok() {
                    moved += 1;
                    let d = dest.to_string_lossy().into_owned();
                    let t = name_of(src);
                    last = store.add_history(NewHistory {
                        action: "sweep",
                        title: &t,
                        path: item,
                        dest: Some(&d),
                        bytes: 0,
                        ..hist
                    });
                }
            }
            if moved == 0 {
                return Err("nothing could be moved".into());
            }
            Ok((last, 0, moved))
        }
        Action::Empty => {
            crate::winsys::empty_recycle_bin()?;
            hist.undoable = false;
            Ok((store.add_history(hist), f.bytes, 0))
        }
    }
}

/// Undoes a history entry (and for sweeps, the whole batch).
pub fn undo(state: &AppState, history_id: i64) -> Result<(), String> {
    let store = &state.store;
    let row = store.history_get(history_id).ok_or("unknown action")?;
    if !row.undoable || row.undone {
        return Err("this can't be undone".into());
    }
    let rows = if row.action == "sweep" {
        store.history_by_batch(&row.batch)
    } else {
        vec![row]
    };
    for r in &rows {
        if r.undone {
            continue;
        }
        match r.action.as_str() {
            "keep" | "snooze" => store.clear_decision(&r.finding_id),
            "hibernate" => {
                let archive = r.dest.as_deref().ok_or("missing archive")?;
                crate::hibernate::wake(Path::new(&r.path), Path::new(archive))?;
                store.mark_restored(archive);
            }
            "disable" => {
                let id = r.path.strip_prefix("startup:").ok_or("not a startup app")?;
                crate::health::startup::set_enabled(id, true)?;
            }
            "recycle" => restore_from_bin(&r.path)?,
            "archive" | "sweep" => {
                let dest = r.dest.as_deref().ok_or("missing destination")?;
                if Path::new(&r.path).exists() {
                    return Err(format!("something new already lives at {}", r.path));
                }
                move_path(Path::new(dest), Path::new(&r.path)).map_err(friendly)?;
            }
            _ => return Err("this can't be undone".into()),
        }
        store.mark_undone(r.id);
        if let Some(f) = state.acted.lock().remove(&r.id) {
            let mut all = state.findings.write();
            if !all.iter().any(|x| x.id == f.id) {
                all.insert(0, f);
            }
        }
    }
    Ok(())
}

fn restore_from_bin(original: &str) -> Result<(), String> {
    use trash::os_limited::{list, restore_all};
    let items = list().map_err(|e| e.to_string())?;
    // The bin records long, canonical paths; ours may be 8.3 or differently cased.
    let canon = |p: &Path| -> String {
        std::fs::canonicalize(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    let target = Path::new(original);
    let parent = canon(target.parent().unwrap_or(target));
    let name = name_of(target).to_lowercase();
    let item = items
        .into_iter()
        .filter(|i| {
            i.name.to_string_lossy().to_lowercase() == name && canon(&i.original_parent) == parent
        })
        .max_by_key(|i| i.time_deleted)
        .ok_or("couldn't find it in the Recycle Bin anymore")?;
    restore_all([item]).map_err(|e| e.to_string())
}

fn friendly(e: std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "it's in use or protected".into(),
        std::io::ErrorKind::NotFound => "it's already gone".into(),
        _ => e.to_string(),
    }
}

fn remember_startup(state: &AppState, path: &str) {
    let id = path.trim_start_matches("startup:").to_string();
    let mut known: Vec<String> = state
        .store
        .get("startup_known")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if !known.contains(&id) {
        known.push(id);
        state.store.set(
            "startup_known",
            &serde_json::to_string(&known).unwrap_or_default(),
        );
    }
}

/// Carries out an (edited) organizing plan as one undoable batch.
pub fn apply_plan(
    state: &AppState,
    folder: &str,
    items: &[crate::organize::PlanItem],
    original: &[crate::organize::PlanItem],
) -> Outcome {
    let store = &state.store;
    let mut out = Outcome {
        batch: new_batch(),
        ..Default::default()
    };
    let mut overrides = state.sort_overrides();
    for it in items {
        let src = Path::new(&it.path);
        match it.action.as_str() {
            "leave" => {
                let id = crate::model::finding_id(Kind::FolderPlan, &it.path);
                store.set_decision(&id, &it.path, "keep", 0);
            }
            "recycle" => match trash::delete(src) {
                Ok(()) => {
                    store.add_history(NewHistory {
                        action: "recycle",
                        kind: "folder_plan",
                        finding_id: "",
                        title: &it.name,
                        path: &it.path,
                        dest: None,
                        bytes: it.bytes,
                        batch: &out.batch,
                        undoable: true,
                        auto: false,
                    });
                    out.done += 1;
                    out.freed += it.bytes;
                }
                Err(e) => out.errors.push(format!("{}: {e}", it.name)),
            },
            "move" => {
                if it.dest.is_empty() {
                    continue;
                }
                let dest = unique_dest(Path::new(&it.dest), &it.name);
                match move_path(src, &dest) {
                    Ok(()) => {
                        let d = dest.to_string_lossy().into_owned();
                        store.add_history(NewHistory {
                            action: "archive",
                            kind: "folder_plan",
                            finding_id: "",
                            title: &it.name,
                            path: &it.path,
                            dest: Some(&d),
                            bytes: it.bytes,
                            batch: &out.batch,
                            undoable: true,
                            auto: false,
                        });
                        out.done += 1;
                        out.moved += 1;
                        // Learn: the user picked a different home for this kind of file.
                        let was = original.iter().find(|o| o.path == it.path);
                        if let Some(was) = was {
                            if was.dest != it.dest && !it.is_dir {
                                let ext = crate::walk::ext_of(src);
                                if !ext.is_empty() {
                                    overrides.insert(format!("ext:{ext}"), it.dest.clone());
                                }
                            }
                        }
                    }
                    Err(e) => out.errors.push(format!("{}: {}", it.name, friendly(e))),
                }
            }
            _ => {}
        }
    }
    state.store.set(
        "sort_overrides",
        &serde_json::to_string(&overrides).unwrap_or_default(),
    );
    let fid = crate::model::finding_id(Kind::FolderPlan, folder);
    state.findings.write().retain(|f| f.id != fid);
    out
}

/// "Don't touch this": hides findings under `path` for `days` (0 = forever).
pub fn protect(state: &AppState, path: &str, days: u32) {
    let until = if days == 0 {
        0
    } else {
        now() + days as i64 * DAY
    };
    state.store.protect(path, until);
    let low = path.trim_end_matches('\\').to_lowercase();
    state.findings.write().retain(|f| {
        let p = f.path.to_lowercase();
        !(p == low || p.starts_with(&format!("{low}\\")))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn setup(name: &str) -> (AppState, PathBuf, Finding) {
        let dir = std::env::temp_dir().join(format!("dusty-act-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("{name}.txt"));
        std::fs::write(&file, b"dust").unwrap();
        let state = AppState::for_tests(Store::memory());
        state.settings.write().archive_root = dir.join("archive").to_string_lossy().into_owned();
        let mut f = Finding::new(
            Kind::StaleDownload,
            &file.to_string_lossy(),
            "x".into(),
            Action::Recycle,
            vec![Action::Recycle, Action::Archive, Action::Keep],
        );
        f.bytes = 4;
        state.findings.write().push(f.clone());
        (state, file, f)
    }

    #[test]
    fn recycle_then_undo_restores_file() {
        let (state, file, f) = setup("recycle");
        let out = apply(&state, &[f.id.clone()], Action::Recycle, false);
        assert_eq!(out.done, 1, "{:?}", out.errors);
        assert!(!file.exists());
        assert!(state.findings.read().is_empty());
        let hid = state.store.history(1)[0].id;
        undo(&state, hid).unwrap();
        assert!(file.exists());
        assert_eq!(state.findings.read().len(), 1);
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn archive_then_undo_moves_back() {
        let (state, file, f) = setup("archive");
        let out = apply(&state, &[f.id.clone()], Action::Archive, false);
        assert_eq!(out.moved, 1, "{:?}", out.errors);
        assert!(!file.exists());
        let row = state.store.history(1).remove(0);
        assert!(Path::new(row.dest.as_deref().unwrap()).exists());
        undo(&state, row.id).unwrap();
        assert!(file.exists());
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn keep_suppresses_and_rejects_foreign_actions() {
        let (state, file, f) = setup("keep");
        let out = apply(&state, &[f.id.clone()], Action::Sweep, false);
        assert_eq!(out.done, 0);
        assert!(file.exists(), "Sweep is not offered for this finding");
        apply(&state, &[f.id.clone()], Action::Keep, false);
        assert!(state.store.suppressed().contains(&f.id));
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }
}

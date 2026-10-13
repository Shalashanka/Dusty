//! Carrying out decisions, and undoing them.

use crate::detect::remove_empty_tree;
use crate::model::{Action, Finding, Kind};
use crate::settings::Settings;
use crate::state::AppState;
use crate::store::NewHistory;
use crate::timeutil::{day_stamp, month_stamp, now, DAY};
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
    format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
}

/// Applies `action` to the findings with `ids`. Successful ones leave the
/// pending list and can be undone through history.
pub fn apply(state: &AppState, ids: &[String], action: Action, auto: bool) -> Outcome {
    let settings = state.settings.read().clone();
    let targets: Vec<Finding> = {
        let all = state.findings.read();
        all.iter().filter(|f| ids.contains(&f.id)).cloned().collect()
    };
    let mut out = Outcome { batch: new_batch(), ..Default::default() };
    for f in targets {
        if !f.actions.contains(&action) {
            out.errors.push(format!("{}: can't {} this", f.title, action.key()));
            continue;
        }
        match apply_one(state, &settings, &f, action, &out.batch, auto) {
            Ok((hid, freed, moved)) => {
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
            Ok((store.add_history(hist), 0, 0))
        }
        Action::Snooze => {
            store.set_decision(&f.id, &f.path, "snooze", now() + s.snooze_days as i64 * DAY);
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
            let dir = Path::new(&s.desktop_dir).join(crate::detect::SWEPT_DIR).join(day_stamp());
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
    let rows = if row.action == "sweep" { store.history_by_batch(&row.batch) } else { vec![row] };
    for r in &rows {
        if r.undone {
            continue;
        }
        match r.action.as_str() {
            "keep" | "snooze" => store.clear_decision(&r.finding_id),
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
    let target = Path::new(original);
    let item = items
        .into_iter()
        .filter(|i| i.original_path() == target)
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

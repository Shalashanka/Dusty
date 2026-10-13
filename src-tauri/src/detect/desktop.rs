//! Desktop piles and empty folders.

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::{human_age, unix_res};
use crate::walk::{self, children, ext_of, is_hidden, is_link, name_of};
use std::fs;
use std::path::{Path, PathBuf};

pub const SWEPT_DIR: &str = "Swept by Dustling";

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();
    let desktop = Path::new(&s.desktop_dir);

    if s.on(Kind::DesktopClutter) && desktop.is_dir() {
        let loose: Vec<(PathBuf, fs::Metadata)> = children(desktop)
            .into_iter()
            .filter(|(p, md)| {
                let ext = ext_of(p);
                !is_hidden(md) && !matches!(ext.as_str(), "lnk" | "url") && name_of(p) != SWEPT_DIR
            })
            .collect();
        if loose.len() as u32 > s.desktop_max_items {
            let mut f = Finding::new(
                Kind::DesktopClutter,
                &s.desktop_dir,
                format!("{} loose things on your desktop", loose.len()),
                Action::Sweep,
                vec![Action::Sweep, Action::Snooze],
            );
            f.items = loose.len() as u64;
            f.bytes = loose
                .iter()
                .filter(|(_, m)| m.is_file())
                .map(|(_, m)| m.len())
                .sum();
            f.is_dir = true;
            f.related = loose
                .iter()
                .map(|(p, _)| p.to_string_lossy().into_owned())
                .collect();
            f.last_touched = loose
                .iter()
                .map(|(_, m)| walk::touched(m))
                .max()
                .unwrap_or(0);
            f.confidence = 0.7;
            f.reason = format!(
                "A clear desktop is a clear head. I'll move everything except shortcuts into “{SWEPT_DIR}\\{}”. Nothing gets deleted.",
                crate::timeutil::day_stamp()
            );
            out.push(f);
        }
    }

    if s.on(Kind::EmptyFolder) {
        let mut roots: Vec<&String> = s.inbox_dirs.iter().collect();
        if !s.desktop_dir.is_empty() {
            roots.push(&s.desktop_dir);
        }
        for root in roots {
            for (p, md) in children(Path::new(root)) {
                if !md.is_dir() || is_link(&md) || is_hidden(&md) {
                    continue;
                }
                let created = unix_res(md.created());
                let age = ctx.age_days(created);
                if age >= 7 && is_empty_tree(&p) {
                    let mut f = Finding::new(
                        Kind::EmptyFolder,
                        &p.to_string_lossy(),
                        name_of(&p),
                        Action::Delete,
                        vec![Action::Delete, Action::Keep, Action::Snooze],
                    );
                    f.is_dir = true;
                    f.items = 0;
                    f.last_touched = created;
                    f.confidence = 0.9;
                    f.reason = format!(
                        "An empty folder, created {} ago and never filled.",
                        human_age(age)
                    );
                    out.push(f);
                }
            }
        }
    }
    out
}

/// True when the tree contains no files at all (only empty sub-folders).
pub fn is_empty_tree(p: &Path) -> bool {
    let Ok(rd) = fs::read_dir(p) else {
        return false;
    };
    for e in rd.flatten() {
        match e.metadata() {
            Ok(m) if m.is_dir() && !is_link(&m) => {
                if !is_empty_tree(&e.path()) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Removes an empty tree bottom-up; refuses if any file appeared meanwhile.
pub fn remove_empty_tree(p: &Path) -> std::io::Result<()> {
    if !is_empty_tree(p) {
        return Err(std::io::Error::other("the folder is not empty anymore"));
    }
    for e in fs::read_dir(p)?.flatten() {
        remove_empty_tree(&e.path())?;
    }
    fs::remove_dir(p)
}

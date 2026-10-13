//! "What's this?": big folders in unusual places that nothing has touched
//! for a long time. Dusty doesn't know, so it asks.

use super::Ctx;
use crate::index::NONE;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::{human_age, human_bytes};

/// Folder names that are never mysterious.
const KNOWN: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "users",
    "appdata",
    "$recycle.bin",
    "system volume information",
    "recovery",
    "perflogs",
    "msocache",
    "intel",
    "amd",
    "nvidia",
    "drivers",
    "documents",
    "downloads",
    "desktop",
    "pictures",
    "videos",
    "music",
    "onedrive",
    "saved games",
    "contacts",
    "favorites",
    "links",
    "searches",
    "3d objects",
    "steamlibrary",
    "steam",
    "epic games",
    "xboxgames",
    "windowsapps",
    "games",
    "public",
    "default",
    "all users",
    ".vscode",
    ".cargo",
    ".rustup",
    ".gradle",
    ".android",
    ".nuget",
    ".m2",
    ".cache",
    ".conda",
    "miniconda3",
    "anaconda3",
    ".ollama",
    ".lmstudio",
    ".docker",
    "wsl",
    "source",
    "repos",
    "dev",
    "go",
    "node_modules",
];

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let Some(idx) = ctx.index.as_ref() else {
        return vec![];
    };
    let min = s.unsure_min_gb as u64 * 1_073_741_824;
    let home = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut bases: Vec<u32> = idx.roots.iter().map(|r| r.0).collect();
    if let Some(h) = idx.lookup(&home) {
        bases.push(h);
    }
    let excluded: Vec<String> = s
        .dev_roots
        .iter()
        .chain(&s.inbox_dirs)
        .map(|d| d.to_lowercase())
        .chain(ctx.kb.iter().map(|k| k.path.to_lowercase()))
        .collect();

    // Folders one or two levels below a drive root or the home folder.
    let mut cands = Vec::new();
    let level1 = idx.children_of(&bases.iter().copied().collect());
    let mut parents = std::collections::HashSet::new();
    for kids in level1.values() {
        for &k in kids {
            let e = &idx.entries[k as usize];
            if e.is_dir() && !KNOWN.contains(&idx.name(k).to_lowercase().as_str()) {
                cands.push(k);
                if e.size >= min {
                    parents.insert(k);
                }
            }
        }
    }
    for kids in idx.children_of(&parents).values() {
        cands.extend(
            kids.iter()
                .copied()
                .filter(|&k| idx.entries[k as usize].is_dir()),
        );
    }

    let mut out: Vec<Finding> = cands
        .into_iter()
        .filter_map(|i| {
            let e = &idx.entries[i as usize];
            let name = idx.name(i).to_lowercase();
            if e.size < min || e.parent == NONE || name.starts_with('.') || KNOWN.contains(&name.as_str()) {
                return None;
            }
            let age = ctx.age_days(e.mtime());
            if e.mtime() == 0 || age < s.unsure_days as i64 {
                return None;
            }
            let path = idx.path(i);
            let low = path.to_lowercase();
            if excluded.iter().any(|x| low == *x || low.starts_with(&format!("{x}\\")) || x.starts_with(&format!("{low}\\")))
                || ctx.is_protected(&path)
            {
                return None;
            }
            let mut f = Finding::new(
                Kind::Unsure,
                &path,
                idx.name(i).to_string(),
                Action::Archive,
                vec![Action::Archive, Action::Recycle, Action::Keep, Action::Snooze],
            );
            f.bytes = e.size;
            f.is_dir = true;
            f.last_touched = e.mtime();
            f.confidence = 0.3;
            f.reason = format!(
                "I don't recognise this folder. It holds {} and nothing inside changed in {}. What is it? Do you still need it?",
                human_bytes(e.size),
                human_age(age)
            );
            Some(f)
        })
        .collect();
    out.sort_by_key(|f| std::cmp::Reverse(f.bytes));
    out.truncate(25);
    out
}

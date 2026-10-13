//! Exact duplicates (size -> 64 KiB head hash -> full hash) and big files
//! nobody touched in months.

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::{human_age, human_bytes};
use crate::walk::{FileEntry, collect_files, ext_of, name_of};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

const HEAD: usize = 64 * 1024;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "__pycache__",
    ".venv",
    "venv",
    "$recycle.bin",
    "appdata",
    "my music",
    "my pictures",
    "my videos",
    ".cache",
];

/// Folder names that hold legitimately repeated files (games, apps, system).
const SYSTEMISH: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "$recycle.bin",
    "system volume information",
    "steamapps",
    "steamlibrary",
    "epic games",
    "xboxgames",
    "windowsapps",
    "site-packages",
    "miniconda3",
    "anaconda3",
];

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let dev_roots: Vec<String> = s.dev_roots.iter().map(|d| d.to_lowercase()).collect();
    let skip = |name: &str| {
        SKIP_DIRS.contains(&name) || SYSTEMISH.contains(&name) || name.starts_with('.')
    };

    // Roots may nest (Desktop inside OneDrive...), so dedupe by path.
    let mut roots: Vec<String> = s
        .library_dirs
        .iter()
        .chain(&s.inbox_dirs)
        .chain(&s.watched_dirs)
        .cloned()
        .collect();
    let min = (s.dup_min_kb as u64 * 1024).min(s.big_mb as u64 * 1_048_576);
    let mut seen = HashSet::new();
    let mut files: Vec<FileEntry> = Vec::new();

    if let Some(idx) = ctx.index.as_ref() {
        // With the index, look at whole data drives too (not the system drive's guts).
        let sys = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".into())
            .to_lowercase();
        for (_, r) in &idx.roots {
            if !r.to_lowercase().starts_with(&sys) {
                roots.push(r.clone());
            }
        }
        let scope = idx.scope(&roots, &|n: &str| skip(n));
        for (i, e) in idx.entries.iter().enumerate() {
            if !scope[i] || e.is_dir() || e.size < min {
                continue;
            }
            let path = idx.path(i as u32);
            let low = path.to_lowercase();
            if dev_roots.iter().any(|d| low.starts_with(d)) || !seen.insert(low) {
                continue;
            }
            files.push(FileEntry {
                path: path.into(),
                size: e.size,
                created: e.mtime(),
                touched: e.mtime(),
            });
        }
    } else {
        for root in &roots {
            if dev_roots.iter().any(|d| root.to_lowercase().starts_with(d)) {
                continue;
            }
            for f in collect_files(Path::new(root), min, &skip) {
                if seen.insert(f.path.to_string_lossy().to_lowercase()) {
                    files.push(f);
                }
            }
        }
    }
    files.retain(|f| !ctx.is_protected(&f.path.to_string_lossy()));

    let mut out = Vec::new();
    if s.on(Kind::BigDormant) {
        out.extend(big_dormant(ctx, &files));
    }
    if s.on(Kind::Duplicate) {
        let min_dup = s.dup_min_kb as u64 * 1024;
        files.retain(|f| f.size >= min_dup);
        out.extend(duplicates(ctx, files));
    }
    out
}

fn big_dormant(ctx: &Ctx, files: &[FileEntry]) -> Vec<Finding> {
    let s = ctx.s;
    let min = s.big_mb as u64 * 1_048_576;
    files
        .iter()
        .filter(|f| f.size >= min && ctx.age_days(f.touched) >= s.big_days as i64)
        .map(|e| {
            let p = e.path.to_string_lossy();
            let mut f = Finding::new(
                Kind::BigDormant,
                &p,
                name_of(&e.path),
                Action::Archive,
                vec![
                    Action::Archive,
                    Action::Recycle,
                    Action::Keep,
                    Action::Snooze,
                ],
            );
            f.bytes = e.size;
            f.last_touched = e.touched;
            f.ext = ext_of(&e.path);
            f.confidence = 0.5;
            f.reason = format!(
                "{} that hasn't been opened in {}. Archive it somewhere roomier, or let it go.",
                human_bytes(e.size),
                human_age(ctx.age_days(e.touched))
            );
            f
        })
        .collect()
}

fn hash_file(path: &Path, limit: Option<usize>) -> Option<u64> {
    let mut f = File::open(path).ok()?;
    let mut h = Xxh3::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut left = limit.unwrap_or(usize::MAX);
    while left > 0 {
        let want = buf.len().min(left);
        let n = f.read(&mut buf[..want]).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        left -= n;
    }
    Some(h.digest())
}

fn duplicates(ctx: &Ctx, files: Vec<FileEntry>) -> Vec<Finding> {
    // 1. Group by size.
    let mut by_size: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for f in files {
        by_size.entry(f.size).or_default().push(f);
    }
    let candidates: Vec<FileEntry> = by_size
        .into_values()
        .filter(|g| g.len() > 1)
        .flatten()
        .collect();
    if candidates.is_empty() {
        return vec![];
    }
    (ctx.phase)(&format!("Comparing {} look-alike files", candidates.len()));

    // 2. Head hash, 3. full hash (only for files bigger than the head).
    let headed: Vec<(u64, u64, FileEntry)> = candidates
        .into_par_iter()
        .filter_map(|f| hash_file(&f.path, Some(HEAD)).map(|h| (f.size, h, f)))
        .collect();
    let mut by_head: HashMap<(u64, u64), Vec<FileEntry>> = HashMap::new();
    for (size, h, f) in headed {
        by_head.entry((size, h)).or_default().push(f);
    }
    let cache = ctx.store.hashes("xxh3");
    let mut budget: u64 = 40 << 30; // bytes read per scan
    let mut todo = Vec::new();
    let mut full: Vec<(u64, u64, FileEntry)> = Vec::new();
    for f in by_head.into_values().filter(|g| g.len() > 1).flatten() {
        let key = f.path.to_string_lossy().into_owned();
        match cache.get(&key) {
            Some((size, mtime, h)) if *size == f.size && *mtime == f.touched => {
                full.push((f.size, *h, f))
            }
            _ if budget >= f.size => {
                budget -= f.size;
                todo.push(f);
            }
            _ => {}
        }
    }
    let hashed: Vec<(u64, u64, FileEntry)> = todo
        .into_par_iter()
        .filter_map(|f| {
            let limit = if f.size as usize <= HEAD {
                Some(HEAD)
            } else {
                None
            };
            hash_file(&f.path, limit).map(|h| (f.size, h, f))
        })
        .collect();
    ctx.store.put_hashes(
        "xxh3",
        &hashed
            .iter()
            .map(|(sz, h, f)| (f.path.to_string_lossy().into_owned(), *sz, f.touched, *h))
            .collect::<Vec<_>>(),
    );
    full.extend(hashed);
    let mut groups: HashMap<(u64, u64), Vec<FileEntry>> = HashMap::new();
    for (size, h, f) in full {
        groups.entry((size, h)).or_default().push(f);
    }

    let inboxes: Vec<String> = ctx
        .s
        .inbox_dirs
        .iter()
        .chain(std::iter::once(&ctx.s.desktop_dir))
        .map(|d| d.to_lowercase())
        .collect();
    let in_inbox = |p: &Path| {
        let l = p.to_string_lossy().to_lowercase();
        inboxes.iter().any(|d| !d.is_empty() && l.starts_with(d))
    };

    let mut out = Vec::new();
    for (_, mut g) in groups.into_iter().filter(|(_, g)| g.len() > 1) {
        // Keep the copy that lives in a "real" place, then the oldest, then the shortest path.
        g.sort_by_key(|f| (in_inbox(&f.path), f.created, f.path.as_os_str().len()));
        let keeper = g.remove(0);
        for dup in g {
            let p = dup.path.to_string_lossy();
            let mut f = Finding::new(
                Kind::Duplicate,
                &p,
                name_of(&dup.path),
                Action::Recycle,
                vec![Action::Recycle, Action::Keep, Action::Snooze],
            );
            f.bytes = dup.size;
            f.last_touched = dup.touched;
            f.ext = ext_of(&dup.path);
            f.confidence = if in_inbox(&dup.path) { 0.92 } else { 0.78 };
            let where_ = keeper
                .path
                .parent()
                .map(|p| short_dir(&p.to_string_lossy()))
                .unwrap_or_default();
            f.reason = format!("Exact copy of “{}” in {}.", name_of(&keeper.path), where_);
            f.related = vec![keeper.path.to_string_lossy().into_owned()];
            out.push(f);
        }
    }
    out
}

/// `C:\Users\me\Documents\Taxes` -> `Documents\Taxes`
fn short_dir(p: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let h = home.to_string_lossy();
        if let Some(rest) = p.strip_prefix(h.as_ref()) {
            return rest.trim_start_matches('\\').to_string();
        }
    }
    p.to_string()
}

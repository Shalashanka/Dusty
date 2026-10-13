//! Downloads: stale items, used installers, already-extracted archives and
//! forgotten backups.

use super::{normalize, Ctx};
use crate::model::{Action, Finding, Kind};
use crate::timeutil::human_age;
use crate::walk::{self, children, ext_of, is_hidden, name_of, touched};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const INSTALLER_EXT: &[&str] = &["exe", "msi", "msix", "msixbundle", "appx", "appxbundle"];
const ARCHIVE_EXT: &[&str] = &["zip", "7z", "rar", "tar", "gz", "tgz", "xz", "bz2", "zst"];
const PARTIAL_EXT: &[&str] = &["crdownload", "part", "partial", "download", "opdownload", "tmp"];
const IGNORE_NAMES: &[&str] = &["desktop.ini", "thumbs.db", ".ds_store"];

struct Item {
    path: PathBuf,
    name: String,
    ext: String,
    is_dir: bool,
    bytes: u64,
    files: u64,
    touched: i64,
}

fn load_items(ctx: &Ctx, dir: &Path) -> Vec<Item> {
    children(dir)
        .into_par_iter()
        .filter_map(|(path, md)| {
            let name = name_of(&path);
            if is_hidden(&md) || walk::is_link(&md) || IGNORE_NAMES.contains(&name.to_lowercase().as_str()) {
                return None;
            }
            let mut t = touched(&md).max(ctx.first_seen(&path));
            let (bytes, files) = if md.is_dir() {
                let st = walk::dir_stats(&path);
                t = t.max(st.newest);
                (st.bytes, st.files)
            } else {
                (md.len(), 1)
            };
            Some(Item { ext: if md.is_dir() { String::new() } else { ext_of(&path) }, name, is_dir: md.is_dir(), bytes, files, touched: t, path })
        })
        .collect()
}

fn base(ctx: &Ctx, it: &Item, kind: Kind, suggested: Action) -> Finding {
    let mut f = Finding::new(
        kind,
        &it.path.to_string_lossy(),
        it.name.clone(),
        suggested,
        vec![Action::Recycle, Action::Archive, Action::Keep, Action::Snooze],
    );
    f.bytes = it.bytes;
    f.items = it.files;
    f.last_touched = it.touched;
    f.is_dir = it.is_dir;
    f.ext = it.ext.clone();
    if !it.is_dir && ctx.s.inbox_dirs.iter().any(|d| it.path.starts_with(d)) {
        f.origin = crate::winsys::download_origin(&it.path);
    }
    f
}

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();
    for dir in &s.inbox_dirs {
        let dir = Path::new(dir);
        let items = load_items(ctx, dir);
        let dir_names: HashSet<String> =
            items.iter().filter(|i| i.is_dir).map(|i| i.name.to_lowercase()).collect();
        out.extend(items.par_iter().filter_map(|it| classify(ctx, it, &dir_names)).collect::<Vec<_>>());
    }
    out
}

fn classify(ctx: &Ctx, it: &Item, dir_names: &HashSet<String>) -> Option<Finding> {
    let s = ctx.s;
    let age = ctx.age_days(it.touched);
    let ext = it.ext.as_str();

    if s.on(Kind::Installer) && !it.is_dir && INSTALLER_EXT.contains(&ext) && age >= s.installer_days as i64 {
        let mut f = base(ctx, it, Kind::Installer, Action::Recycle);
        let stem = it.path.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let looks_setup = is_setup_name(&stem) || ext != "exe";
        if let Some(app) = match_installed(&stem, &ctx.installed) {
            f.confidence = 0.92;
            f.reason = format!("{app} is already installed, so the installer can go.");
        } else if looks_setup {
            f.confidence = (0.6 + age as f32 / 365.0).min(0.85);
            f.reason = format!(
                "Installer downloaded {} ago. Once a program is installed, you rarely need its setup file again.",
                human_age(age)
            );
        } else {
            f.confidence = 0.45;
            f.reason = format!(
                "A program file that's been sitting here for {}. If it's a portable app you still use, keep it.",
                human_age(age)
            );
        }
        return Some(f);
    }

    if s.on(Kind::ExtractedArchive) && !it.is_dir && ARCHIVE_EXT.contains(&ext) && age >= 1 {
        if let Some(folder) = extracted_twin(&it.name, dir_names) {
            let mut f = base(ctx, it, Kind::ExtractedArchive, Action::Recycle);
            f.confidence = 0.88;
            f.reason = format!("Already extracted to the “{folder}” folder right next to it. The archive is a leftover.");
            f.related = vec![it.path.with_file_name(&folder).to_string_lossy().into_owned()];
            return Some(f);
        }
    }

    if s.on(Kind::Backup) && age >= s.backup_days as i64 && is_backup_name(&it.name) {
        return Some(backup_finding(ctx, it, age));
    }

    if s.on(Kind::StaleDownload) {
        let partial = PARTIAL_EXT.contains(&ext);
        if partial && age >= 2 {
            let mut f = base(ctx, it, Kind::StaleDownload, Action::Recycle);
            f.confidence = 0.95;
            f.reason = "An unfinished download that never completed.".into();
            return Some(f);
        }
        if age >= s.stale_days as i64 {
            let mut f = base(ctx, it, Kind::StaleDownload, Action::Recycle);
            f.confidence = (0.4 + (age - s.stale_days as i64) as f32 / 300.0).min(0.85);
            f.reason = if it.is_dir {
                format!("Nothing inside this folder changed in {}.", human_age(age))
            } else {
                format!("Untouched for {}. Downloads you still need usually get opened or moved.", human_age(age))
            };
            return Some(f);
        }
    }
    None
}

fn backup_finding(ctx: &Ctx, it: &Item, age: i64) -> Finding {
    let mut f = base(ctx, it, Kind::Backup, Action::Recycle);
    match original_of(&it.path) {
        Some(orig) => {
            f.confidence = 0.75;
            f.reason = format!(
                "A backup made {} ago. The original “{}” is still right next to it.",
                human_age(age),
                name_of(&orig)
            );
            f.related = vec![orig.to_string_lossy().into_owned()];
        }
        None => {
            f.confidence = 0.5;
            f.reason = format!("Looks like a backup copy that nobody touched in {}.", human_age(age));
        }
    }
    f
}

/// Backups on the desktop, in project roots and at the top of Documents.
pub fn scan_backups_elsewhere(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut dirs: Vec<&String> = s.dev_roots.iter().collect();
    if !s.desktop_dir.is_empty() {
        dirs.push(&s.desktop_dir);
    }
    if let Some(docs) = s.library_dirs.iter().find(|d| d.to_lowercase().ends_with("documents")) {
        dirs.push(docs);
    }
    let mut out = Vec::new();
    for dir in dirs {
        if s.inbox_dirs.contains(dir) {
            continue;
        }
        let candidates: Vec<(PathBuf, std::fs::Metadata)> = children(Path::new(dir))
            .into_iter()
            .filter(|(p, md)| !is_hidden(md) && is_backup_name(&name_of(p)))
            .collect();
        out.extend(
            candidates
                .into_par_iter()
                .filter_map(|(path, md)| {
                    let mut t = touched(&md);
                    let (bytes, files) = if md.is_dir() {
                        let st = walk::dir_stats(&path);
                        t = t.max(st.newest);
                        (st.bytes, st.files)
                    } else {
                        (md.len(), 1)
                    };
                    let age = ctx.age_days(t);
                    if age < s.backup_days as i64 {
                        return None;
                    }
                    let it = Item {
                        name: name_of(&path),
                        ext: if md.is_dir() { String::new() } else { ext_of(&path) },
                        is_dir: md.is_dir(),
                        bytes,
                        files,
                        touched: t,
                        path,
                    };
                    Some(backup_finding(ctx, &it, age))
                })
                .collect::<Vec<_>>(),
        );
    }
    out
}

// ---- name heuristics -------------------------------------------------------

const BACKUP_MARKERS: &[&str] = &[
    "_backup", "-backup", " backup", ".backup", "backup_", "backup-", "_bckp", "-bckp", "_bkp", "-bkp",
    "_old", "-old", " old", " - copy", " - copia", " - kopie", " - copie", "(copy", "copy of ",
];

pub fn is_backup_name(name: &str) -> bool {
    let n = name.to_lowercase();
    let ext = Path::new(&n).extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    if matches!(ext.as_str(), "bak" | "old" | "orig" | "bkp") {
        return true;
    }
    let stem = Path::new(&n).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(n.clone());
    BACKUP_MARKERS.iter().any(|m| stem.contains(m)) || stem.ends_with("backup") || stem == "old"
}

/// Finds the original next to a backup: `Foo_backup` -> `Foo`,
/// `Foo-backup-2026-09-29b.bundle` -> `Foo`, `x.json.bak` -> `x.json`.
fn original_of(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let name = name_of(path);
    let lower = name.to_lowercase();
    let mut candidates = Vec::new();
    for ext in [".bak", ".old", ".orig", ".bkp"] {
        if let Some(stripped) = lower.strip_suffix(ext) {
            candidates.push(name[..stripped.len()].to_string());
        }
    }
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for m in BACKUP_MARKERS.iter().chain([&"backup"]) {
        if let Some(pos) = lower.find(m) {
            if pos > 0 {
                let head = name[..pos].trim_end_matches([' ', '-', '_', '.']).to_string();
                candidates.push(head.clone());
                if !ext.is_empty() {
                    candidates.push(format!("{head}{ext}"));
                }
            }
        }
    }
    candidates
        .into_iter()
        .filter(|c| !c.is_empty() && !c.eq_ignore_ascii_case(&name))
        .map(|c| parent.join(c))
        .find(|p| p.exists())
}

/// `foo.zip` + folder `foo` (also `foo.tar.gz`, `foo (1).zip`).
fn extracted_twin(name: &str, dir_names: &HashSet<String>) -> Option<String> {
    let lower = name.to_lowercase();
    let mut stem = lower.as_str();
    for _ in 0..2 {
        if let Some(i) = stem.rfind('.') {
            let ext = &stem[i + 1..];
            if ARCHIVE_EXT.contains(&ext) {
                stem = &stem[..i];
            }
        }
    }
    let cleaned = strip_copy_suffix(stem);
    [stem, cleaned]
        .into_iter()
        .find(|c| !c.is_empty() && dir_names.contains(*c))
        .map(|c| c.to_string())
}

fn strip_copy_suffix(s: &str) -> &str {
    // "foo (1)" -> "foo"
    if s.ends_with(')') {
        if let Some(i) = s.rfind(" (") {
            if s[i + 2..s.len() - 1].chars().all(|c| c.is_ascii_digit()) {
                return &s[..i];
            }
        }
    }
    s
}

const SETUP_WORDS: &[&str] = &["setup", "install", "installer", "update", "upgrade", "x64", "x86", "win", "amd64", "arm64"];

fn is_setup_name(stem: &str) -> bool {
    let l = stem.to_lowercase();
    SETUP_WORDS[..5].iter().any(|w| l.contains(w)) || l.chars().filter(|c| c.is_ascii_digit()).count() >= 3
}

const NAME_NOISE: &[&str] = &[
    "windows", "win", "win32", "win64", "x64", "x86", "amd64", "arm64", "arm", "bit", "bits", "user",
    "setup", "installer", "install", "release", "portable", "latest", "stable", "version", "full",
    "export", "web", "online", "offline", "the", "for", "and", "msi", "exe",
];

/// Up to two distinctive words of a file name: `NVIDIA_Broadcast_v2.0` ->
/// ["nvidia", "broadcast"], `DiscordSetup` -> ["discord"].
fn name_tokens(stem: &str) -> Vec<String> {
    stem.to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .map(|t| {
            let mut t = t;
            for suffix in ["installer", "install", "setup"] {
                t = t.strip_suffix(suffix).unwrap_or(t);
            }
            t.to_string()
        })
        .filter(|t| t.len() >= 3 && !NAME_NOISE.contains(&t.as_str()))
        .take(2)
        .collect()
}

/// Matches `DiscordSetup.exe` to "Discord", `node-v20.11.0-x64` to "Node.js".
/// Every distinctive word must appear, so `NVIDIA_Broadcast` never matches
/// "NVIDIA Nsight".
fn match_installed(stem: &str, installed: &[String]) -> Option<String> {
    let toks = name_tokens(stem);
    let first = toks.first()?;
    installed
        .iter()
        .map(|name| (name, normalize(name)))
        .filter(|(_, n)| {
            let first_ok = n.starts_with(first.as_str()) || (first.len() >= 5 && n.contains(first.as_str()));
            first_ok && toks[1..].iter().all(|t| n.contains(t.as_str()))
        })
        .min_by_key(|(_, n)| n.len())
        .map(|(name, _)| name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backups() {
        assert!(is_backup_name("Fije-backup-2026-09-29b.bundle"));
        assert!(is_backup_name("OpusStudio_backup"));
        assert!(is_backup_name("settings.local_bckp.json"));
        assert!(is_backup_name("report - Copia.docx"));
        assert!(is_backup_name("thing.bak"));
        assert!(!is_backup_name("backpack.png"));
        assert!(!is_backup_name("Golden.txt"));
    }

    #[test]
    fn twins() {
        let dirs: HashSet<String> = ["ffmpeg-7.1".into(), "repo-main".into()].into_iter().collect();
        assert_eq!(extracted_twin("ffmpeg-7.1.zip", &dirs).as_deref(), Some("ffmpeg-7.1"));
        assert_eq!(extracted_twin("repo-main (1).zip", &dirs).as_deref(), Some("repo-main"));
        assert_eq!(extracted_twin("other.zip", &dirs), None);
    }

    #[test]
    fn installers() {
        let installed = vec!["Discord".to_string(), "Node.js".to_string(), "Google Chrome".to_string()];
        assert_eq!(match_installed("DiscordSetup", &installed).as_deref(), Some("Discord"));
        assert_eq!(match_installed("node-v20.11.0-x64", &installed).as_deref(), Some("Node.js"));
        assert_eq!(match_installed("Blender-4.2", &installed), None);

        let installed = vec![
            "Microsoft Visual C++ 2013 x64 Additional Runtime".to_string(),
            "NVIDIA Nsight Compute 2025.2.0".to_string(),
            "Ubisoft Connect".to_string(),
            "Shutter Encoder (64bit)".to_string(),
        ];
        assert_eq!(match_installed("microsoft-jdk-17.0.18-windows-x64", &installed), None);
        assert_eq!(match_installed("NVIDIA_Broadcast_v2.0.2.31240911", &installed), None);
        assert_eq!(match_installed("UbisoftConnectInstaller", &installed).as_deref(), Some("Ubisoft Connect"));
        assert_eq!(
            match_installed("Shutter Encoder 20.2 Windows 64bits", &installed).as_deref(),
            Some("Shutter Encoder (64bit)")
        );
    }

    #[test]
    fn originals() {
        let d = std::env::temp_dir().join(format!("dustling-orig-{}", std::process::id()));
        std::fs::create_dir_all(d.join("OpusStudio")).unwrap();
        std::fs::write(d.join("settings.local.json"), b"{}").unwrap();
        assert_eq!(original_of(&d.join("OpusStudio_backup")), Some(d.join("OpusStudio")));
        assert_eq!(original_of(&d.join("settings.local_bckp.json")), Some(d.join("settings.local.json")));
        std::fs::remove_dir_all(&d).unwrap();
    }
}

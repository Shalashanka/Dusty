//! Parallel filesystem traversal.
//!
//! On Windows `std::fs::DirEntry::metadata()` is served from the
//! `FindNextFileW` buffer, so walking costs one syscall per directory, not per
//! file. Sub-directories are fanned out over the current rayon pool.

use crate::timeutil::{now, unix_res};
use rayon::prelude::*;
use std::fs::{self, Metadata};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
const ATTR_REPARSE: u32 = 0x400;
const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x40000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x400000;

/// Junctions, symlinks and cloud placeholders: never follow, never read.
pub fn is_link(md: &Metadata) -> bool {
    md.file_attributes() & ATTR_REPARSE != 0
}

/// OneDrive / cloud placeholders whose bytes are not on this disk. Reading
/// them would trigger a download, so they are skipped for hashing.
pub fn is_cloud(md: &Metadata) -> bool {
    md.file_attributes() & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0
}

pub fn is_hidden(md: &Metadata) -> bool {
    md.file_attributes() & (ATTR_HIDDEN | ATTR_SYSTEM) != 0
}

/// Latest of modified / accessed / created.
pub fn touched(md: &Metadata) -> i64 {
    unix_res(md.modified())
        .max(unix_res(md.accessed()))
        .max(unix_res(md.created()))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DirStats {
    pub bytes: u64,
    pub files: u64,
    /// Most recent modification of any file inside.
    pub newest: i64,
}

impl DirStats {
    fn merge(self, o: DirStats) -> DirStats {
        DirStats {
            bytes: self.bytes + o.bytes,
            files: self.files + o.files,
            newest: self.newest.max(o.newest),
        }
    }
}

/// Recursive size, file count and newest mtime.
pub fn dir_stats(path: &Path) -> DirStats {
    stats_rec(path, None)
}

/// Like [`dir_stats`] but only counts files untouched for `older_than_days`.
pub fn dir_stats_older(path: &Path, older_than_days: u32) -> DirStats {
    stats_rec(
        path,
        Some(now() - older_than_days as i64 * crate::timeutil::DAY),
    )
}

fn stats_rec(path: &Path, cutoff: Option<i64>) -> DirStats {
    let Ok(rd) = fs::read_dir(path) else {
        return DirStats::default();
    };
    let mut s = DirStats::default();
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if is_link(&md) {
            continue;
        }
        if md.is_dir() {
            subdirs.push(e.path());
            continue;
        }
        let m = unix_res(md.modified());
        s.newest = s.newest.max(m);
        if let Some(c) = cutoff
            && m.max(unix_res(md.created())) >= c
        {
            continue;
        }
        if !is_cloud(&md) {
            s.bytes += md.len();
        }
        s.files += 1;
    }
    let sub = subdirs
        .par_iter()
        .map(|d| stats_rec(d, cutoff))
        .reduce(DirStats::default, DirStats::merge);
    s.merge(sub)
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub created: i64,
    pub touched: i64,
}

/// Collects all regular, local (non-cloud), non-hidden files under `root`
/// with `size >= min_size`. `skip_dir` prunes directories by lowercase name.
pub fn collect_files(
    root: &Path,
    min_size: u64,
    skip_dir: &(dyn Fn(&str) -> bool + Sync),
) -> Vec<FileEntry> {
    let Ok(rd) = fs::read_dir(root) else {
        return vec![];
    };
    let mut out = Vec::new();
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if is_link(&md) || is_hidden(&md) {
            continue;
        }
        if md.is_dir() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if !skip_dir(&name) {
                subdirs.push(e.path());
            }
            continue;
        }
        if md.len() < min_size || is_cloud(&md) {
            continue;
        }
        out.push(FileEntry {
            path: e.path(),
            size: md.len(),
            created: unix_res(md.created()),
            touched: touched(&md),
        });
    }
    let nested: Vec<FileEntry> = subdirs
        .par_iter()
        .flat_map_iter(|d| collect_files(d, min_size, skip_dir))
        .collect();
    out.extend(nested);
    out
}

/// Top-level entries of a directory with their metadata.
pub fn children(path: &Path) -> Vec<(PathBuf, Metadata)> {
    let Ok(rd) = fs::read_dir(path) else {
        return vec![];
    };
    rd.flatten()
        .filter_map(|e| e.metadata().ok().map(|m| (e.path(), m)))
        .collect()
}

pub fn name_of(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

pub fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Deletes a directory tree fast: files of each level are removed in
/// parallel, read-only flags are cleared on the way (git objects are RO).
pub fn remove_tree(path: &Path) -> std::io::Result<()> {
    let md = fs::symlink_metadata(path)?;
    if is_link(&md) {
        // A junction: remove the link, never its target.
        return fs::remove_dir(path).or_else(|_| fs::remove_file(path));
    }
    if !md.is_dir() {
        return remove_file_force(path);
    }
    let entries: Vec<_> = fs::read_dir(path)?.flatten().collect();
    entries.par_iter().for_each(|e| {
        let p = e.path();
        match e.metadata() {
            Ok(m) if m.is_dir() && !is_link(&m) => {
                let _ = remove_tree(&p);
            }
            Ok(m) if m.is_dir() => {
                let _ = fs::remove_dir(&p);
            }
            _ => {
                let _ = remove_file_force(&p);
            }
        }
    });
    fs::remove_dir(path)
}

pub fn remove_file_force(p: &Path) -> std::io::Result<()> {
    match fs::remove_file(p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let md = fs::metadata(p)?;
            let mut perms = md.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(p, perms)?;
            fs::remove_file(p)
        }
        Err(e) => Err(e),
    }
}

/// Removes files older than `older_than_days` inside `root` (keeping `root`),
/// then prunes directories left empty. Locked files are skipped silently.
/// Returns bytes freed.
pub fn purge_older(root: &Path, older_than_days: u32) -> u64 {
    let cutoff = now() - older_than_days as i64 * crate::timeutil::DAY;
    purge_rec(root, cutoff, true)
}

fn purge_rec(dir: &Path, cutoff: i64, is_root: bool) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else { return 0 };
    let entries: Vec<_> = rd.flatten().collect();
    let freed: u64 = entries
        .par_iter()
        .map(|e| {
            let Ok(md) = e.metadata() else { return 0 };
            if is_link(&md) {
                return 0;
            }
            if md.is_dir() {
                return purge_rec(&e.path(), cutoff, false);
            }
            let m = unix_res(md.modified()).max(unix_res(md.created()));
            if m < cutoff && remove_file_force(&e.path()).is_ok() {
                md.len()
            } else {
                0
            }
        })
        .sum();
    if !is_root {
        // Fails (harmlessly) when not empty.
        let _ = fs::remove_dir(dir);
    }
    freed
}

/// Move that also works across volumes (copy + delete fallback).
pub fn move_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        // ERROR_NOT_SAME_DEVICE
        Err(e) if e.raw_os_error() == Some(17) => {
            copy_tree(src, dst)?;
            remove_tree(src)
        }
        Err(e) => Err(e),
    }
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    let md = fs::metadata(src)?;
    if md.is_dir() {
        fs::create_dir_all(dst)?;
        for e in fs::read_dir(src)? {
            let e = e?;
            copy_tree(&e.path(), &dst.join(e.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

/// `dir/name`, or `dir/name (2)`, `dir/name (3)`… if taken.
pub fn unique_dest(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|c| !c.exists())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dusty-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn stats_and_remove() {
        let d = tmp("stats");
        fs::create_dir_all(d.join("a/b/c")).unwrap();
        fs::write(d.join("a/x.txt"), b"hello").unwrap();
        fs::write(d.join("a/b/c/y.txt"), b"world!!").unwrap();
        let s = dir_stats(&d);
        assert_eq!(s.files, 2);
        assert_eq!(s.bytes, 12);
        // Fresh files are not older than 1 day.
        assert_eq!(dir_stats_older(&d, 1).files, 0);
        remove_tree(&d).unwrap();
        assert!(!d.exists());
    }

    #[test]
    fn unique_names() {
        let d = tmp("unique");
        fs::write(d.join("a.txt"), b"1").unwrap();
        fs::write(d.join("a (2).txt"), b"1").unwrap();
        assert_eq!(unique_dest(&d, "a.txt"), d.join("a (3).txt"));
        assert_eq!(unique_dest(&d, "b.txt"), d.join("b.txt"));
        remove_tree(&d).unwrap();
    }
}

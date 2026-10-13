//! Near-identical photos (resized, re-saved, burst shots) via perceptual
//! hashes of the Windows thumbnail. Hashes are cached, so each photo is only
//! looked at once.

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::walk::{FileEntry, collect_files, ext_of, name_of};
use std::collections::HashMap;
use std::path::Path;

const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "heic", "bmp", "tif", "tiff"];
/// New photos hashed per scan (keeps background scans short).
const BUDGET: usize = 1500;
const MAX_DISTANCE: u32 = 4;

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let skip = |n: &str| n.starts_with('.') || n == "node_modules" || n == "appdata";
    let mut roots: Vec<&String> = s.library_dirs.iter().collect();
    roots.extend(s.inbox_dirs.iter());
    let mut photos: Vec<FileEntry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in roots {
        for f in collect_files(Path::new(r), 20 * 1024, &skip) {
            if EXTS.contains(&ext_of(&f.path).as_str())
                && seen.insert(f.path.to_string_lossy().to_lowercase())
            {
                photos.push(f);
            }
        }
    }
    if photos.len() < 2 {
        return vec![];
    }
    let cache = ctx.store.hashes("dhash");
    let mut fresh = Vec::new();
    let mut hashes: Vec<(usize, u64)> = Vec::new();
    let mut budget = BUDGET;
    for (i, p) in photos.iter().enumerate() {
        let key = p.path.to_string_lossy().into_owned();
        match cache.get(&key) {
            Some((size, mtime, h)) if *size == p.size && *mtime == p.touched => {
                hashes.push((i, *h))
            }
            _ if budget > 0 => {
                budget -= 1;
                if let Some(h) = crate::thumbs::dhash(&key) {
                    fresh.push((key, p.size, p.touched, h));
                    hashes.push((i, h));
                }
            }
            _ => {}
        }
    }
    if !fresh.is_empty() {
        (ctx.phase)(&format!("Looked at {} new photos", fresh.len()));
        ctx.store.put_hashes("dhash", &fresh);
    }

    // Group by Hamming distance (bucketed by the top 16 bits of each 16-bit band).
    let mut parent: Vec<usize> = (0..hashes.len()).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        p[x] = r;
        r
    }
    for band in 0..4 {
        let mut buckets: HashMap<u16, Vec<usize>> = HashMap::new();
        for (k, (_, h)) in hashes.iter().enumerate() {
            buckets
                .entry((h >> (band * 16)) as u16)
                .or_default()
                .push(k);
        }
        for b in buckets.values().filter(|b| b.len() > 1 && b.len() < 200) {
            for x in 0..b.len() {
                for y in x + 1..b.len() {
                    let (a, c) = (b[x], b[y]);
                    if (hashes[a].1 ^ hashes[c].1).count_ones() <= MAX_DISTANCE {
                        let (ra, rc) = (find(&mut parent, a), find(&mut parent, c));
                        if ra != rc {
                            parent[ra] = rc;
                        }
                    }
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for k in 0..hashes.len() {
        let r = find(&mut parent, k);
        groups.entry(r).or_default().push(k);
    }

    let mut out = Vec::new();
    for g in groups.values().filter(|g| g.len() > 1 && g.len() <= 12) {
        let mut members: Vec<&FileEntry> = g.iter().map(|&k| &photos[hashes[k].0]).collect();
        // Exact duplicates are the duplicate detector's job.
        let sizes: std::collections::HashSet<u64> = members.iter().map(|m| m.size).collect();
        if sizes.len() == 1 {
            continue;
        }
        // Keep the biggest (usually the best quality) one.
        members.sort_by_key(|m| std::cmp::Reverse(m.size));
        let keeper = members[0];
        for m in &members[1..] {
            if m.size == keeper.size {
                continue;
            }
            let p = m.path.to_string_lossy();
            let mut f = Finding::new(
                Kind::SimilarPhoto,
                &p,
                name_of(&m.path),
                Action::Recycle,
                vec![Action::Recycle, Action::Keep, Action::Snooze],
            );
            f.bytes = m.size;
            f.last_touched = m.touched;
            f.ext = ext_of(&m.path);
            f.confidence = 0.55;
            f.related = vec![keeper.path.to_string_lossy().into_owned()];
            f.reason = format!(
                "Looks almost identical to “{}”, which is bigger (probably the better copy). A resized or re-saved version, or a burst shot?",
                name_of(&keeper.path)
            );
            out.push(f);
        }
    }
    out
}

//! Whole-drive file index (Everything-style search, instant Space views).
//!
//! Every file and folder of the looked-after drives lives in one flat arena:
//! 24 bytes per entry plus its UTF-8 name. Parents always come before their
//! children, so subtree passes are a single forward sweep. Folder entries
//! carry the total size and newest modification of their whole subtree.
//!
//! The index is rebuilt in the background at low priority, saved to disk and
//! loaded at startup. Between rebuilds a recursive watcher on each drive root
//! records additions/removals in a small delta that search merges in.

use crate::nlq::Query;
use crate::timeutil::now;
use crate::walk::{is_cloud, is_link};
use parking_lot::{Mutex, RwLock};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const NONE: u32 = u32::MAX;
const F_DIR: u8 = 1;
const F_HIDDEN: u8 = 2;
/// Seconds between the Unix epoch and 2000-01-01; mtimes are stored relative to it.
const EPOCH: i64 = 946_684_800;

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub parent: u32,
    pub name_off: u32,
    pub size: u64,
    mtime: u32,
    pub name_len: u16,
    pub flags: u8,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.flags & F_DIR != 0
    }
    pub fn mtime(&self) -> i64 {
        if self.mtime == 0 {
            0
        } else {
            self.mtime as i64 + EPOCH
        }
    }
}

fn pack_time(t: i64) -> u32 {
    (t - EPOCH).clamp(0, u32::MAX as i64) as u32
}

#[derive(Default)]
pub struct Index {
    pub entries: Vec<Entry>,
    pub names: String,
    /// (entry index, "C:\\")
    pub roots: Vec<(u32, String)>,
    pub built_at: i64,
    pub build_ms: u64,
    /// Lowercased, accent-folded copy of every name (built on first search),
    /// with each entry's start offset, for SIMD substring scans.
    folded: std::sync::OnceLock<(Vec<u8>, Vec<u32>)>,
}

impl Index {
    fn folded(&self) -> &(Vec<u8>, Vec<u32>) {
        self.folded.get_or_init(|| {
            let mut text = Vec::with_capacity(self.names.len());
            let mut offs = Vec::with_capacity(self.entries.len() + 1);
            for i in 0..self.entries.len() as u32 {
                offs.push(text.len() as u32);
                let n = self.name(i);
                if n.is_ascii() {
                    text.extend(n.bytes().map(|b| b.to_ascii_lowercase()));
                } else {
                    text.extend(crate::nlq::fold(n).bytes());
                }
                // Separator so a match can never span two names.
                text.push(0);
            }
            offs.push(text.len() as u32);
            (text, offs)
        })
    }

    /// For each entry, which of `alts` groups occur in its name (bitmask).
    /// Scans the folded arena with SIMD substring search, in parallel chunks.
    pub fn warm(&self) {
        let _ = self.folded();
    }

    /// `primary[k]` is the typed term; short synonyms must match whole words
    /// ("trip" shouldn't hit "stripes").
    pub fn name_masks(&self, alts: &[Vec<String>], primary: &[String]) -> Vec<u32> {
        let (text, offs) = self.folded();
        let n = self.entries.len();
        let mut masks = vec![0u32; n];
        if alts.is_empty() || n == 0 {
            return masks;
        }
        let finders: Vec<(u32, bool, memchr::memmem::Finder)> = alts
            .iter()
            .enumerate()
            .take(32)
            .flat_map(|(k, group)| {
                let p = primary.get(k).cloned().unwrap_or_default();
                group.iter().filter(|a| !a.is_empty()).map(move |a| {
                    let whole_word = a.len() <= 4 && *a != p;
                    (1u32 << k, whole_word, memchr::memmem::Finder::new(a.as_bytes()).into_owned())
                })
            })
            .collect();
        let chunk = n.div_ceil(rayon::current_num_threads().max(1) * 4).max(1);
        masks.par_chunks_mut(chunk).enumerate().for_each(|(c, out)| {
            let first = c * chunk;
            let lo = offs[first] as usize;
            let hi = offs[first + out.len()] as usize;
            let slice = &text[lo..hi];
            for (bit, whole_word, f) in &finders {
                let len = f.needle().len();
                for pos in f.find_iter(slice) {
                    if *whole_word {
                        let before = if pos == 0 { 0 } else { slice[pos - 1] };
                        let after = slice.get(pos + len).copied().unwrap_or(0);
                        if before.is_ascii_alphanumeric() || after.is_ascii_alphanumeric() {
                            continue;
                        }
                    }
                    let abs = (lo + pos) as u32;
                    // Entry containing this byte: last start <= abs.
                    let j = offs[first..first + out.len() + 1].partition_point(|&o| o <= abs) - 1;
                    out[j] |= bit;
                }
            }
        });
        masks
    }

    pub fn name(&self, i: u32) -> &str {
        let e = &self.entries[i as usize];
        &self.names[e.name_off as usize..e.name_off as usize + e.name_len as usize]
    }

    pub fn path(&self, i: u32) -> String {
        let mut parts = Vec::with_capacity(12);
        let mut cur = i;
        while cur != NONE {
            parts.push(self.name(cur));
            cur = self.entries[cur as usize].parent;
        }
        parts.reverse();
        let mut s = String::with_capacity(parts.iter().map(|p| p.len() + 1).sum());
        for (n, p) in parts.iter().enumerate() {
            if n > 0 {
                s.push('\\');
            }
            s.push_str(p);
        }
        if parts.len() == 1 {
            s.push('\\');
        }
        s
    }

    /// Finds the entry for an absolute path (case-insensitive).
    pub fn lookup(&self, path: &str) -> Option<u32> {
        let p = path.trim_end_matches('\\');
        let low = p.to_lowercase();
        let (root, label) = self
            .roots
            .iter()
            .filter(|(_, r)| {
                let rl = r.trim_end_matches('\\').to_lowercase();
                low == rl || low.starts_with(&format!("{rl}\\"))
            })
            .max_by_key(|(_, r)| r.len())?;
        let mut cur = *root;
        let rest = &p[label.trim_end_matches('\\').len()..];
        for c in rest.split('\\').filter(|c| !c.is_empty()) {
            let parent = cur;
            cur = (0..self.entries.len() as u32)
                .into_par_iter()
                .find_any(|&j| {
                    self.entries[j as usize].parent == parent
                        && self.name(j).eq_ignore_ascii_case(c)
                })?;
        }
        Some(cur)
    }

    /// Children of each listed parent (one parallel sweep).
    pub fn children_of(&self, parents: &HashSet<u32>) -> HashMap<u32, Vec<u32>> {
        let pairs: Vec<(u32, u32)> = self
            .entries
            .par_iter()
            .enumerate()
            .filter(|(_, e)| parents.contains(&e.parent))
            .map(|(i, e)| (e.parent, i as u32))
            .collect();
        let mut out: HashMap<u32, Vec<u32>> = HashMap::new();
        for (p, c) in pairs {
            out.entry(p).or_default().push(c);
        }
        out
    }

    /// Marks every entry inside any of `roots` (prefix paths), minus subtrees
    /// whose folder name matches `prune`.
    pub fn scope(&self, roots: &[String], prune: &dyn Fn(&str) -> bool) -> Vec<bool> {
        let starts: HashSet<u32> = roots.iter().filter_map(|r| self.lookup(r)).collect();
        let mut v = vec![false; self.entries.len()];
        for (i, e) in self.entries.iter().enumerate() {
            let inherited = e.parent != NONE && v[e.parent as usize];
            let start = starts.contains(&(i as u32));
            v[i] = (inherited || start)
                && !(e.is_dir() && !start && prune(&self.name(i as u32).to_ascii_lowercase()));
        }
        v
    }
}

// ---------------------------------------------------------------------------
// building

struct Flat {
    entries: Vec<Entry>,
    names: String,
}

const REL_SELF: u32 = u32::MAX - 1;

fn walk_dir(
    path: &Path,
    name: &str,
    md_mtime: i64,
    hidden: bool,
    counter: &AtomicU64,
    cancel: &AtomicBool,
) -> Flat {
    let mut flat = Flat {
        entries: Vec::new(),
        names: String::new(),
    };
    flat.names.push_str(name);
    flat.entries.push(Entry {
        parent: NONE,
        name_off: 0,
        size: 0,
        mtime: pack_time(md_mtime),
        name_len: name.len().min(u16::MAX as usize) as u16,
        flags: F_DIR | if hidden { F_HIDDEN } else { 0 },
    });
    if cancel.load(Ordering::Relaxed) {
        return flat;
    }
    let Ok(rd) = fs::read_dir(path) else {
        return flat;
    };
    let mut subdirs = Vec::new();
    let mut total = 0u64;
    let mut newest = md_mtime;
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if is_link(&md) {
            continue;
        }
        let n = e.file_name().to_string_lossy().into_owned();
        let h = md.file_attributes() & 0x6 != 0;
        let m = crate::timeutil::unix_res(md.modified());
        if md.is_dir() {
            subdirs.push((e.path(), n, m, h));
            continue;
        }
        let size = if is_cloud(&md) { 0 } else { md.len() };
        total += size;
        newest = newest.max(m);
        flat.entries.push(Entry {
            parent: 0,
            name_off: flat.names.len() as u32,
            size,
            mtime: pack_time(m),
            name_len: n.len().min(u16::MAX as usize) as u16,
            flags: if h { F_HIDDEN } else { 0 },
        });
        flat.names.push_str(&n);
    }
    counter.fetch_add(flat.entries.len() as u64, Ordering::Relaxed);
    let subs: Vec<Flat> = subdirs
        .par_iter()
        .map(|(p, n, m, h)| walk_dir(p, n, *m, *h, counter, cancel))
        .collect();
    for sf in subs {
        let off = flat.entries.len() as u32;
        let name_off = flat.names.len() as u32;
        total += sf.entries[0].size;
        newest = newest.max(sf.entries[0].mtime());
        for mut e in sf.entries {
            e.parent = if e.parent == NONE { 0 } else { e.parent + off };
            e.name_off += name_off;
            flat.entries.push(e);
        }
        flat.names.push_str(&sf.names);
    }
    flat.entries[0].size = total;
    flat.entries[0].mtime = pack_time(newest);
    let _ = REL_SELF;
    flat
}

pub fn build(roots: &[String], counter: &AtomicU64, cancel: &AtomicBool) -> Index {
    let t = std::time::Instant::now();
    let mut idx = Index::default();
    for root in roots {
        let p = PathBuf::from(root);
        if !p.is_dir() {
            continue;
        }
        let label = root.trim_end_matches('\\').to_string();
        let flat = walk_dir(&p, &label, 0, false, counter, cancel);
        let off = idx.entries.len() as u32;
        let name_off = idx.names.len() as u32;
        idx.roots.push((off, format!("{label}\\")));
        for mut e in flat.entries {
            e.parent = if e.parent == NONE {
                NONE
            } else {
                e.parent + off
            };
            e.name_off += name_off;
            idx.entries.push(e);
        }
        idx.names.push_str(&flat.names);
    }
    idx.built_at = now();
    idx.build_ms = t.elapsed().as_millis() as u64;
    idx
}

// ---------------------------------------------------------------------------
// persistence

const MAGIC: &[u8; 8] = b"DUSTYIX1";

pub fn save(idx: &Index, path: &Path) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut w = BufWriter::with_capacity(1 << 20, fs::File::create(&tmp)?);
        w.write_all(MAGIC)?;
        w.write_all(&idx.built_at.to_le_bytes())?;
        w.write_all(&idx.build_ms.to_le_bytes())?;
        w.write_all(&(idx.roots.len() as u32).to_le_bytes())?;
        for (i, r) in &idx.roots {
            w.write_all(&i.to_le_bytes())?;
            w.write_all(&(r.len() as u32).to_le_bytes())?;
            w.write_all(r.as_bytes())?;
        }
        w.write_all(&(idx.entries.len() as u64).to_le_bytes())?;
        for e in &idx.entries {
            w.write_all(&e.parent.to_le_bytes())?;
            w.write_all(&e.name_off.to_le_bytes())?;
            w.write_all(&e.size.to_le_bytes())?;
            w.write_all(&e.mtime.to_le_bytes())?;
            w.write_all(&e.name_len.to_le_bytes())?;
            w.write_all(&[e.flags, 0])?;
        }
        w.write_all(&(idx.names.len() as u64).to_le_bytes())?;
        w.write_all(idx.names.as_bytes())?;
        w.flush()?;
    }
    fs::rename(tmp, path)
}

pub fn load(path: &Path) -> std::io::Result<Index> {
    let mut r = BufReader::with_capacity(1 << 20, fs::File::open(path)?);
    let mut magic = [0u8; 8];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::other("bad index file"));
    }
    let mut b8 = [0u8; 8];
    let mut b4 = [0u8; 4];
    let mut b2 = [0u8; 2];
    let mut idx = Index::default();
    r.read_exact(&mut b8)?;
    idx.built_at = i64::from_le_bytes(b8);
    r.read_exact(&mut b8)?;
    idx.build_ms = u64::from_le_bytes(b8);
    r.read_exact(&mut b4)?;
    for _ in 0..u32::from_le_bytes(b4) {
        r.read_exact(&mut b4)?;
        let i = u32::from_le_bytes(b4);
        r.read_exact(&mut b4)?;
        let mut s = vec![0u8; u32::from_le_bytes(b4) as usize];
        r.read_exact(&mut s)?;
        idx.roots
            .push((i, String::from_utf8_lossy(&s).into_owned()));
    }
    r.read_exact(&mut b8)?;
    let n = u64::from_le_bytes(b8) as usize;
    idx.entries.reserve_exact(n);
    for _ in 0..n {
        r.read_exact(&mut b4)?;
        let parent = u32::from_le_bytes(b4);
        r.read_exact(&mut b4)?;
        let name_off = u32::from_le_bytes(b4);
        r.read_exact(&mut b8)?;
        let size = u64::from_le_bytes(b8);
        r.read_exact(&mut b4)?;
        let mtime = u32::from_le_bytes(b4);
        r.read_exact(&mut b2)?;
        let name_len = u16::from_le_bytes(b2);
        r.read_exact(&mut b2)?;
        idx.entries.push(Entry {
            parent,
            name_off,
            size,
            mtime,
            name_len,
            flags: b2[0],
        });
    }
    r.read_exact(&mut b8)?;
    let mut names = vec![0u8; u64::from_le_bytes(b8) as usize];
    r.read_exact(&mut names)?;
    idx.names = String::from_utf8(names).map_err(std::io::Error::other)?;
    Ok(idx)
}

// ---------------------------------------------------------------------------
// live state

#[derive(Debug, Clone)]
struct DeltaEntry {
    path: String,
    name: String,
    is_dir: bool,
    size: u64,
    mtime: i64,
}

#[derive(Default)]
pub struct IndexState {
    pub idx: RwLock<Option<Arc<Index>>>,
    added: Mutex<Vec<DeltaEntry>>,
    removed: Mutex<HashSet<String>>,
    pub building: AtomicBool,
    pub cancel: AtomicBool,
    /// Set to ask the index thread for a rebuild now.
    pub rebuild: AtomicBool,
    pub progress: AtomicU64,
    watchers: Mutex<Vec<notify::RecommendedWatcher>>,
    /// Live corrections between rebuilds (deleted entries, shrunken folders).
    pub adjust: RwLock<Adjust>,
    pending_removed: Mutex<Vec<String>>,
}

#[derive(Default)]
pub struct Adjust {
    pub dead: HashSet<u32>,
    pub shrink: HashMap<u32, u64>,
}

impl Adjust {
    pub fn size(&self, idx: &Index, i: u32) -> u64 {
        idx.entries[i as usize].size.saturating_sub(self.shrink.get(&i).copied().unwrap_or(0))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub ready: bool,
    pub building: bool,
    pub entries: u64,
    pub progress: u64,
    pub built_at: i64,
    pub build_ms: u64,
    pub memory_mb: u64,
    pub drives: Vec<String>,
}

impl IndexState {
    pub fn get(&self) -> Option<Arc<Index>> {
        self.idx.read().clone()
    }

    pub fn status(&self) -> IndexStatus {
        let idx = self.get();
        IndexStatus {
            ready: idx.is_some(),
            building: self.building.load(Ordering::Relaxed),
            entries: idx.as_ref().map(|i| i.entries.len() as u64).unwrap_or(0),
            progress: self.progress.load(Ordering::Relaxed),
            built_at: idx.as_ref().map(|i| i.built_at).unwrap_or(0),
            build_ms: idx.as_ref().map(|i| i.build_ms).unwrap_or(0),
            memory_mb: idx
                .as_ref()
                .map(|i| {
                    (i.entries.len() * std::mem::size_of::<Entry>() + i.names.len()) as u64
                        / 1_048_576
                })
                .unwrap_or(0),
            drives: idx
                .as_ref()
                .map(|i| i.roots.iter().map(|r| r.1.clone()).collect())
                .unwrap_or_default(),
        }
    }

    pub fn install(&self, idx: Index) {
        let idx = Arc::new(idx);
        // Prepare the search arena in the background so the first query is instant.
        let warm = idx.clone();
        std::thread::spawn(move || warm.warm());
        *self.idx.write() = Some(idx);
        self.added.lock().clear();
        self.removed.lock().clear();
        *self.adjust.write() = Adjust::default();
        self.pending_removed.lock().clear();
    }

    /// Applies queued deletions to the live size corrections (every few seconds).
    /// Returns true when there were so many that a rebuild is the better option.
    pub fn apply_removals(&self) -> bool {
        let mut batch = std::mem::take(&mut *self.pending_removed.lock());
        if batch.is_empty() {
            return false;
        }
        let Some(idx) = self.get() else { return false };
        // Only the topmost removed paths matter (a deleted folder covers its files).
        batch.sort_by_key(|p| p.len());
        let mut tops: Vec<String> = Vec::new();
        for p in batch {
            let low = p.to_lowercase();
            if !tops.iter().any(|t| low == *t || low.starts_with(&format!("{t}\\"))) {
                tops.push(low);
            }
        }
        if tops.len() > 300 {
            return true;
        }
        for p in tops {
            // Came back (rename round-trip, recreated file)?
            if Path::new(&p).exists() {
                continue;
            }
            let Some(i) = idx.lookup(&p) else { continue };
            let mut adj = self.adjust.write();
            if adj.dead.contains(&i) {
                continue;
            }
            let lost = adj.size(&idx, i);
            adj.dead.insert(i);
            let mut cur = idx.entries[i as usize].parent;
            while cur != NONE {
                *adj.shrink.entry(cur).or_default() += lost;
                cur = idx.entries[cur as usize].parent;
            }
        }
        false
    }

    pub fn clear(&self) {
        *self.idx.write() = None;
        self.watchers.lock().clear();
    }

    /// Watches drive roots recursively; changes land in the delta.
    pub fn watch(self: &Arc<Self>, roots: &[String]) {
        use notify::{EventKind, RecursiveMode, Watcher};
        let me = self.clone();
        let mut ws = Vec::new();
        for r in roots {
            let me2 = me.clone();
            let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let Ok(ev) = res else { return };
                match ev.kind {
                    EventKind::Create(_)
                    | EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::To,
                    )) => {
                        for p in &ev.paths {
                            me2.note_added(p);
                        }
                    }
                    EventKind::Remove(_)
                    | EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::From,
                    )) => {
                        for p in &ev.paths {
                            let s = p.to_string_lossy().into_owned();
                            me2.removed.lock().insert(s.to_lowercase());
                            let mut pend = me2.pending_removed.lock();
                            if pend.len() < 100_000 {
                                pend.push(s);
                            }
                        }
                    }
                    _ => {}
                }
            });
            if let Ok(mut w) = w
                && w.watch(Path::new(r), RecursiveMode::Recursive).is_ok()
            {
                ws.push(w);
            }
        }
        *self.watchers.lock() = ws;
    }

    fn note_added(&self, p: &Path) {
        let s = p.to_string_lossy().into_owned();
        let low = s.to_lowercase();
        if low.contains("\\appdata\\local\\temp\\") || low.contains("\\$recycle.bin") {
            return;
        }
        let md = fs::metadata(p).ok();
        let mut added = self.added.lock();
        if added.len() > 200_000 {
            return;
        }
        self.removed.lock().remove(&low);
        added.push(DeltaEntry {
            name: crate::walk::name_of(p),
            is_dir: md.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            size: md.as_ref().map(|m| m.len()).unwrap_or(0),
            mtime: md
                .and_then(|m| m.modified().ok())
                .map(crate::timeutil::unix)
                .unwrap_or(now()),
            path: s,
        });
    }

    pub fn search(&self, q: &Query, limit: usize) -> Vec<Hit> {
        let removed = self.removed.lock().clone();
        let mut hits = match self.get() {
            Some(idx) => search_index(&idx, q, limit * 4, &removed),
            None => Vec::new(),
        };
        for d in self.added.lock().iter() {
            if let Some(score) = q.score(&d.name, &d.path, d.is_dir, d.size, d.mtime) {
                hits.push(Hit {
                    path: d.path.clone(),
                    name: d.name.clone(),
                    is_dir: d.is_dir,
                    size: d.size,
                    mtime: d.mtime,
                    score: score + 5.0,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut seen = HashSet::new();
        hits.retain(|h| seen.insert(h.path.to_lowercase()));
        hits.truncate(limit);
        hits
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
    pub score: f32,
}

fn search_index(idx: &Index, q: &Query, limit: usize, removed: &HashSet<String>) -> Vec<Hit> {
    let hits = search_pass(idx, q, limit, removed);
    if !hits.is_empty() || q.alts.is_empty() {
        return hits;
    }
    // Nothing at all: forgive typos ("invocie", "screnshot") and try again.
    let mut fuzzy = q.clone();
    for (k, t) in q.terms.iter().enumerate() {
        if let Some(alts) = fuzzy.alts.get_mut(k) {
            alts.extend(crate::nlq::typo_variants(&crate::nlq::fold(t)));
        }
    }
    search_pass(idx, &fuzzy, limit, removed)
}

/// One ranking pass:
/// 1. folders whose name matches a term get a bitmask of matched terms;
/// 2. a forward sweep hands every entry the union of its ancestors' masks
///    (parents precede children in the arena, so this is linear);
/// 3. entries are ranked by how many terms they cover (name or folders),
///    then by name quality, recency and location.
fn search_pass(idx: &Index, q: &Query, limit: usize, removed: &HashSet<String>) -> Vec<Hit> {
    let n_terms = q.alts.len().min(32);
    let full: u32 = if n_terms == 32 { u32::MAX } else { (1u32 << n_terms) - 1 };

    // Per-entry name masks, computed once in parallel.
    let name_masks: Vec<u32> = if n_terms == 0 { Vec::new() } else { idx.name_masks(&q.alts, &q.primary) };
    // Inherited masks from matching folders above each entry.
    let mut inherited: Vec<u32> = Vec::new();
    if n_terms > 0 {
        inherited = vec![0u32; idx.entries.len()];
        for (i, e) in idx.entries.iter().enumerate() {
            if e.parent != NONE {
                let p = e.parent as usize;
                let from_parent = inherited[p] | if idx.entries[p].is_dir() { name_masks[p] } else { 0 };
                inherited[i] = from_parent;
            }
        }
    }

    let mut cands: Vec<(u32, u32, f32)> = idx
        .entries
        .par_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if e.parent == NONE || !q.passes(idx.name(i as u32), e.is_dir(), e.size, e.mtime()) {
                return None;
            }
            let (nm, cover) = if n_terms > 0 {
                let nm = name_masks[i];
                (nm, (nm | inherited[i]) & full)
            } else {
                (0, 0)
            };
            if n_terms > 0 && cover == 0 {
                return None;
            }
            let name = idx.name(i as u32);
            let mut score = cover.count_ones() as f32 * 40.0 + q.name_score(name, nm) + q.recency(e.mtime());
            // A term found only in a folder name counts, but less than in the file name.
            score -= ((cover & !nm).count_ones() as f32) * 6.0;
            score -= (name.len() as f32 / 40.0).min(4.0);
            if e.is_dir() {
                score += 1.0;
            }
            Some((i as u32, cover.count_ones(), score))
        })
        .collect();
    if cands.is_empty() {
        return Vec::new();
    }
    // Keep the best-covered results; with many terms allow one miss.
    let best = cands.iter().map(|c| c.1).max().unwrap_or(0);
    let floor = if n_terms >= 3 { best.saturating_sub(1).max(1) } else { best };
    cands.retain(|c| c.1 >= floor);
    cands.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    cands.truncate(20_000);
    let mut hits: Vec<Hit> = cands
        .par_iter()
        .filter_map(|&(i, _, base)| {
            let path = idx.path(i);
            let e = &idx.entries[i as usize];
            let score = q.path_score(&path, base)?;
            if removed.contains(&path.to_lowercase()) {
                return None;
            }
            Some(Hit { name: idx.name(i).to_string(), is_dir: e.is_dir(), size: e.size, mtime: e.mtime(), score, path })
        })
        .collect();
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(limit);
    hits
}

// ---------------------------------------------------------------------------
// Space tree

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    pub is_dir: bool,
    pub newest: i64,
    pub children: Vec<TreeNode>,
}

/// Nested sizes under `path`, `depth` levels deep, small items folded.
/// `adj` hides what was deleted since the index was built.
pub fn tree(idx: &Index, path: &str, depth: usize, adj: &Adjust) -> Option<TreeNode> {
    let root = idx.lookup(path)?;
    if adj.dead.contains(&root) {
        return None;
    }
    let mut levels: Vec<HashMap<u32, Vec<u32>>> = Vec::new();
    let mut frontier: HashSet<u32> = [root].into_iter().collect();
    for _ in 0..depth {
        let kids = idx.children_of(&frontier);
        frontier = kids
            .values()
            .flatten()
            .copied()
            .filter(|&k| idx.entries[k as usize].is_dir() && !adj.dead.contains(&k) && adj.size(idx, k) > 0)
            .collect();
        levels.push(kids);
        if frontier.is_empty() {
            break;
        }
    }
    let limits = [60usize, 16, 8, 5];
    fn node(idx: &Index, i: u32, level: usize, levels: &[HashMap<u32, Vec<u32>>], limits: &[usize], adj: &Adjust) -> TreeNode {
        let e = idx.entries[i as usize];
        let size = adj.size(idx, i);
        let mut children = Vec::new();
        if level < levels.len()
            && let Some(kids) = levels[level].get(&i)
        {
            let mut kids: Vec<u32> = kids.iter().copied().filter(|k| !adj.dead.contains(k)).collect();
            kids.sort_by_key(|k| std::cmp::Reverse(adj.size(idx, *k)));
            let lim = limits.get(level).copied().unwrap_or(6);
            let total = size.max(1);
            let mut shown = 0;
            let mut rest_bytes = 0u64;
            let mut rest_n = 0u64;
            for k in kids {
                let ks = adj.size(idx, k);
                if shown < lim && (shown < 3 || ks * 250 >= total) {
                    children.push(node(idx, k, level + 1, levels, limits, adj));
                    shown += 1;
                } else {
                    rest_bytes += ks;
                    rest_n += 1;
                }
            }
            if rest_bytes > 0 {
                children.push(TreeNode {
                    name: format!("{rest_n} smaller items"),
                    path: String::new(),
                    bytes: rest_bytes,
                    files: rest_n,
                    is_dir: false,
                    newest: 0,
                    children: vec![],
                });
            }
        }
        TreeNode {
            name: idx.name(i).to_string(),
            path: idx.path(i),
            bytes: size,
            files: 0,
            is_dir: e.is_dir(),
            newest: e.mtime(),
            children,
        }
    }
    Some(node(idx, root, 0, &levels, &limits, adj))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_save_load_lookup_tree() {
        let d = std::env::temp_dir().join(format!("dusty-index-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("a/b")).unwrap();
        fs::write(d.join("a/b/hello invoice.pdf"), vec![0u8; 3000]).unwrap();
        fs::write(d.join("a/x.txt"), b"12345").unwrap();
        let root = d.to_string_lossy().into_owned();
        let c = AtomicU64::new(0);
        let idx = build(&[root.clone()], &c, &AtomicBool::new(false));
        assert_eq!(idx.entries[0].size, 3005);
        let i = idx
            .lookup(&format!("{root}\\a\\b\\HELLO invoice.pdf"))
            .unwrap();
        assert_eq!(
            idx.path(i).to_lowercase(),
            format!("{root}\\a\\b\\hello invoice.pdf").to_lowercase()
        );

        let f = d.join("ix.bin");
        save(&idx, &f).unwrap();
        let back = load(&f).unwrap();
        assert_eq!(back.entries.len(), idx.entries.len());
        assert_eq!(back.name(i), "hello invoice.pdf");

        let t = tree(&back, &root, 2, &Adjust::default()).unwrap();
        assert_eq!(t.children[0].name, "a");
        assert_eq!(t.children[0].bytes, 3005);

        let st = IndexState::default();
        st.install(back);
        let hits = st.search(&crate::nlq::parse("invoice pdf"), 10);
        assert_eq!(hits.len(), 1);

        // Deleting a folder updates Space without a rebuild.
        fs::remove_dir_all(d.join("a").join("b")).unwrap();
        st.pending_removed.lock().push(format!(r"{root}\a\b"));
        assert!(!st.apply_removals());
        let t = tree(&st.get().unwrap(), &root, 2, &st.adjust.read()).unwrap();
        assert_eq!(t.bytes, 5, "only x.txt is left");
        assert!(t.children[0].children.iter().all(|c| c.name != "b"));
        fs::remove_dir_all(&d).unwrap();
    }

    /// Read-only: `cargo test --release real_index -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_index() {
        let roots = crate::settings::fixed_drives();
        let c = AtomicU64::new(0);
        let t = std::time::Instant::now();
        let idx = build(&roots, &c, &AtomicBool::new(false));
        let mb = (idx.entries.len() * std::mem::size_of::<Entry>() + idx.names.len()) / 1_048_576;
        eprintln!(
            "indexed {:?}: {} entries in {} ms, ~{mb} MB",
            roots,
            idx.entries.len(),
            t.elapsed().as_millis()
        );
        let f = std::env::temp_dir().join("dusty-real-index.bin");
        let t = std::time::Instant::now();
        save(&idx, &f).unwrap();
        eprintln!("saved in {} ms", t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let back = load(&f).unwrap();
        eprintln!("loaded in {} ms", t.elapsed().as_millis());
        let st = IndexState::default();
        st.install(back);
        for q in [
            "invoice",
            "pdf from about 5 months ago",
            "big videos",
            "setup exe in downloads",
            "fije folder",
        ] {
            let t = std::time::Instant::now();
            let hits = st.search(&crate::nlq::parse(q), 5);
            eprintln!(
                "search {q:?}: {} hits in {} ms -> {:?}",
                hits.len(),
                t.elapsed().as_millis(),
                hits.iter().map(|h| h.name.clone()).collect::<Vec<_>>()
            );
        }
        let home = dirs::home_dir().unwrap().to_string_lossy().into_owned();
        let t = std::time::Instant::now();
        let tr = tree(&st.get().unwrap(), &home, 3, &Adjust::default()).unwrap();
        eprintln!(
            "tree(home) in {} ms: {} top children",
            t.elapsed().as_millis(),
            tr.children.len()
        );
        let _ = fs::remove_file(f);
    }

    /// Read-only, uses the saved index: `cargo test --release real_search -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_search() {
        let f = dirs::data_local_dir().unwrap().join("com.griz.dusty").join("index.bin");
        let st = IndexState::default();
        st.install(load(&f).expect("run Dusty once to build the index"));
        for q in [
            "invoice",
            "invocie pdf",
            "find the invoice pdf I downloaded about five months ago",
            "show me big videos on the D drive",
            "dusty mascot svg",
            "fije results hdr tif",
            "foto vacanze",
            "old zip files in downloads",
            "screenshots last week",
            "cargo toml dusty",
        ] {
            let t = std::time::Instant::now();
            let parsed = crate::nlq::parse(q);
            let hits = st.search(&parsed, 4);
            eprintln!(
                "{q:?} {:?} -> {} ms: {:?}",
                parsed.explain,
                t.elapsed().as_millis(),
                hits.iter().map(|h| h.path.clone()).collect::<Vec<_>>()
            );
        }
    }
}

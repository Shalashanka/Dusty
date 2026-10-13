//! Detectors: each one inspects part of the filesystem and emits findings.

mod desktop;
mod dev;
mod dupes;
mod inbox;
mod system;

pub use desktop::{SWEPT_DIR, remove_empty_tree};

use crate::model::{Finding, Kind};
use crate::settings::Settings;
use std::collections::HashMap;
use std::path::Path;

pub struct Ctx<'a> {
    pub s: &'a Settings,
    pub now: i64,
    /// Lowercased path -> first time the watcher saw it arrive.
    pub first_seen: HashMap<String, i64>,
    /// Normalized names of installed programs.
    pub installed: Vec<String>,
    pub phase: &'a (dyn Fn(&str) + Sync),
}

impl Ctx<'_> {
    pub fn age_days(&self, ts: i64) -> i64 {
        ((self.now - ts) / crate::timeutil::DAY).max(0)
    }

    pub fn first_seen(&self, p: &Path) -> i64 {
        self.first_seen
            .get(&p.to_string_lossy().to_lowercase())
            .copied()
            .unwrap_or(0)
    }
}

pub fn run(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();

    (ctx.phase)("Looking through your downloads");
    out.extend(inbox::scan(ctx));
    if s.on(Kind::Backup) {
        out.extend(inbox::scan_backups_elsewhere(ctx));
    }

    if s.on(Kind::DesktopClutter) || s.on(Kind::EmptyFolder) {
        (ctx.phase)("Peeking at the desktop");
        out.extend(desktop::scan(ctx));
    }

    if s.on(Kind::DevJunk) && !s.dev_roots.is_empty() {
        (ctx.phase)("Checking on your old projects");
        out.extend(dev::scan(ctx));
    }

    if s.on(Kind::Duplicate) || s.on(Kind::BigDormant) {
        (ctx.phase)("Hunting for duplicates");
        out.extend(dupes::scan(ctx));
    }

    if s.on(Kind::SystemJunk) || s.on(Kind::RecycleBin) {
        (ctx.phase)("Sweeping temp folders and caches");
        out.extend(system::scan(ctx));
    }

    dedupe(out)
}

/// One finding per path (highest-priority kind wins), ordered by score.
fn dedupe(mut v: Vec<Finding>) -> Vec<Finding> {
    v.sort_by_key(|a| a.kind);
    let mut seen = std::collections::HashSet::new();
    v.retain(|f| seen.insert(f.path.to_lowercase()));
    v.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

/// Lowercase alphanumeric only, for fuzzy name matching.
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read-only scan of this machine: `cargo test real_scan -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_scan() {
        let s = Settings::default();
        let t = std::time::Instant::now();
        let phase = |p: &str| eprintln!("[{:>6} ms] {p}", t.elapsed().as_millis());
        let ctx = Ctx {
            s: &s,
            now: crate::timeutil::now(),
            first_seen: HashMap::new(),
            installed: crate::winsys::installed_programs(),
            phase: &phase,
        };
        let found = run(&ctx);
        eprintln!("{} findings in {} ms", found.len(), t.elapsed().as_millis());
        let mut by_kind: std::collections::BTreeMap<Kind, (usize, u64)> = Default::default();
        for f in &found {
            let e = by_kind.entry(f.kind).or_default();
            e.0 += 1;
            e.1 += f.bytes;
        }
        for (k, (n, b)) in by_kind {
            eprintln!(
                "  {:<18} {n:>4}  {}",
                k.key(),
                crate::timeutil::human_bytes(b)
            );
        }
        for f in found.iter().take(25) {
            eprintln!(
                "  [{:.2}] {:<17} {:>9}  {}  -- {}",
                f.confidence,
                f.kind.key(),
                crate::timeutil::human_bytes(f.bytes),
                f.title,
                f.reason
            );
        }
    }
}

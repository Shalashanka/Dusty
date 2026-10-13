//! Temp folders, caches, logs (from the knowledge base) and the Recycle Bin.

use super::Ctx;
use crate::kb;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::{human_age, human_bytes};
use crate::walk;
use rayon::prelude::*;
use std::path::Path;

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();

    if s.on(Kind::SystemJunk) {
        let min = s.junk_min_mb as u64 * 1_048_576;
        let mut spots: Vec<kb::KbHit> = kb::resolve()
            .into_iter()
            .filter(|h| h.safety == "safe" || (h.safety == "caution" && h.days > 0))
            .collect();
        // Outer locations win over nested ones (no double counting).
        spots.sort_by_key(|h| h.path.len());
        let mut kept: Vec<kb::KbHit> = Vec::new();
        for h in spots {
            let low = h.path.to_lowercase();
            let nested = kept.iter().any(|k| {
                let kl = k.path.to_lowercase();
                low == kl || low.starts_with(&format!("{kl}\\"))
            });
            if !nested {
                kept.push(h);
            }
        }
        out.extend(
            kept.into_par_iter()
                .filter_map(|spot| {
                    if ctx.is_protected(&spot.path) {
                        return None;
                    }
                    let st = walk::dir_stats_older(Path::new(&spot.path), spot.days);
                    let caution = spot.safety == "caution";
                    if st.bytes < if caution { min.max(1 << 30) } else { min } {
                        return None;
                    }
                    let label = match spot.cat.as_str() {
                        "temp" => "temp files",
                        "crash" => "crash reports",
                        "logs" => "logs",
                        "update" => "update leftovers",
                        "build" => "download cache",
                        "models" => "AI models",
                        _ => "cache",
                    };
                    let mut f = Finding::new(
                        Kind::SystemJunk,
                        &spot.path,
                        format!("{} {label}", spot.app),
                        Action::Delete,
                        vec![Action::Delete, Action::Keep, Action::Snooze],
                    );
                    f.bytes = st.bytes;
                    f.items = st.files;
                    f.is_dir = true;
                    f.last_touched = st.newest;
                    f.confidence = if caution { 0.35 } else { 0.9 };
                    f.older_than_days = Some(spot.days);
                    f.app = Some(spot.app.clone());
                    f.reason = if caution {
                        format!(
                            "Careful: {} I'd only remove files older than {}. {} could go.",
                            spot.note,
                            human_age(spot.days as i64),
                            human_bytes(st.bytes)
                        )
                    } else {
                        format!(
                            "{} Usually safe to delete. I'll only remove files older than {} and skip anything in use.",
                            spot.note,
                            human_age(spot.days as i64)
                        )
                    };
                    Some(f)
                })
                .collect::<Vec<_>>(),
        );
    }

    if s.on(Kind::RecycleBin) {
        let (bytes, items) = crate::winsys::recycle_bin_info();
        if bytes >= s.recycle_bin_gb as u64 * 1_073_741_824 {
            let mut f = Finding::new(
                Kind::RecycleBin,
                "shell:RecycleBinFolder",
                "Recycle Bin".into(),
                Action::Empty,
                vec![Action::Empty, Action::Snooze],
            );
            f.bytes = bytes;
            f.items = items;
            f.confidence = 0.6;
            f.reason = format!(
                "{items} deleted items still take up space: recycled files only free the disk once the bin is emptied. Emptying is permanent."
            );
            out.push(f);
        }
    }
    out
}

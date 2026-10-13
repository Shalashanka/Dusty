//! Temp folders, crash dumps, caches and the Recycle Bin.

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::human_age;
use crate::walk;
use rayon::prelude::*;
use std::path::PathBuf;

struct Spot {
    label: String,
    path: PathBuf,
    days: u32,
    why: &'static str,
}

fn spots() -> Vec<Spot> {
    let local = dirs::data_local_dir().unwrap_or_default();
    let roaming = dirs::data_dir().unwrap_or_default();
    let home = dirs::home_dir().unwrap_or_default();
    let windir = PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()));
    let progdata = PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()));
    let temp = std::env::temp_dir();

    let mut v = vec![
        Spot { label: "Temporary files".into(), path: temp, days: 2, why: "Leftovers apps created and forgot to clean up." },
        Spot { label: "Windows temp".into(), path: windir.join("Temp"), days: 3, why: "System leftovers from installers and updates." },
        Spot { label: "Crash dumps".into(), path: local.join("CrashDumps"), days: 7, why: "Memory dumps from apps that crashed. Only useful for debugging." },
        Spot { label: "Error reports".into(), path: local.join("Microsoft\\Windows\\WER"), days: 7, why: "Old Windows Error Reporting files." },
        Spot { label: "System error reports".into(), path: progdata.join("Microsoft\\Windows\\WER"), days: 7, why: "Old Windows Error Reporting files." },
        Spot { label: "Windows Update downloads".into(), path: windir.join("SoftwareDistribution\\Download"), days: 10, why: "Update packages that were already installed." },
        Spot { label: "DirectX shader cache".into(), path: local.join("D3DSCache"), days: 14, why: "Rebuilt automatically by games and apps." },
        Spot { label: "NVIDIA shader cache".into(), path: local.join("NVIDIA\\DXCache"), days: 14, why: "Rebuilt automatically by the driver." },
        Spot { label: "NVIDIA GL cache".into(), path: local.join("NVIDIA\\GLCache"), days: 14, why: "Rebuilt automatically by the driver." },
        Spot { label: "AMD shader cache".into(), path: local.join("AMD\\DxCache"), days: 14, why: "Rebuilt automatically by the driver." },
        Spot { label: "Internet cache".into(), path: local.join("Microsoft\\Windows\\INetCache"), days: 7, why: "Old cached web content." },
        Spot { label: "npm cache".into(), path: local.join("npm-cache"), days: 30, why: "Packages npm re-downloads on demand." },
        Spot { label: "pip cache".into(), path: local.join("pip\\cache"), days: 30, why: "Packages pip re-downloads on demand." },
        Spot { label: "Yarn cache".into(), path: local.join("Yarn\\Cache"), days: 30, why: "Packages Yarn re-downloads on demand." },
        Spot { label: "Cargo download cache".into(), path: home.join(".cargo\\registry\\cache"), days: 60, why: "Crate archives Cargo re-downloads on demand." },
        Spot { label: "VS Code caches".into(), path: roaming.join("Code\\Cache"), days: 14, why: "Rebuilt automatically by VS Code." },
        Spot { label: "VS Code old versions".into(), path: roaming.join("Code\\CachedData"), days: 30, why: "Compiled data from older VS Code versions." },
        Spot { label: "Discord cache".into(), path: roaming.join("discord\\Cache"), days: 7, why: "Cached images and media, re-fetched on demand." },
    ];

    for (browser, base) in [
        ("Chrome", local.join("Google\\Chrome\\User Data")),
        ("Edge", local.join("Microsoft\\Edge\\User Data")),
        ("Brave", local.join("BraveSoftware\\Brave-Browser\\User Data")),
    ] {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == "Default" || name.starts_with("Profile ") {
                let who = if name == "Default" { browser.to_string() } else { format!("{browser} ({name})") };
                v.push(Spot { label: format!("{who} cache"), path: e.path().join("Cache"), days: 7, why: "Cached web pages and images, re-fetched on demand." });
                v.push(Spot { label: format!("{who} script cache"), path: e.path().join("Code Cache"), days: 7, why: "Compiled scripts, rebuilt on demand." });
            }
        }
    }
    v.retain(|s| s.path.is_dir());
    v
}

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();

    if s.on(Kind::SystemJunk) {
        let min = s.junk_min_mb as u64 * 1_048_576;
        out.extend(
            spots()
                .into_par_iter()
                .filter_map(|spot| {
                    let st = walk::dir_stats_older(&spot.path, spot.days);
                    if st.bytes < min {
                        return None;
                    }
                    let mut f = Finding::new(
                        Kind::SystemJunk,
                        &spot.path.to_string_lossy(),
                        spot.label.clone(),
                        Action::Delete,
                        vec![Action::Delete, Action::Keep, Action::Snooze],
                    );
                    f.bytes = st.bytes;
                    f.items = st.files;
                    f.is_dir = true;
                    f.last_touched = st.newest;
                    f.confidence = 0.9;
                    f.older_than_days = Some(spot.days);
                    f.reason = format!(
                        "{} I'll only remove files older than {} and skip anything in use.",
                        spot.why,
                        human_age(spot.days as i64)
                    );
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
            f.reason = format!("{items} deleted items are still taking up space. Emptying the bin is permanent.");
            out.push(f);
        }
    }
    out
}

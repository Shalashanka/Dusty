//! Built-in knowledge of where popular programs keep caches, temp files and
//! logs, and how safe they are to clear.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KbEntry {
    pub app: String,
    pub path: String,
    pub cat: String,
    /// "safe" | "caution" | "never"
    pub safety: String,
    /// Only files older than this are cleared.
    pub days: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbHit {
    pub app: String,
    pub cat: String,
    pub safety: String,
    pub days: u32,
    pub note: String,
    pub path: String,
}

fn entries() -> &'static [KbEntry] {
    static KB: OnceLock<Vec<KbEntry>> = OnceLock::new();
    KB.get_or_init(|| serde_json::from_str(include_str!("kb.json")).expect("kb.json"))
}

fn expand_vars(p: &str) -> String {
    let mut s = p.to_string();
    let vars: [(&str, Option<PathBuf>); 6] = [
        ("%LOCALAPPDATA%", dirs::data_local_dir()),
        ("%APPDATA%", dirs::data_dir()),
        ("%USERPROFILE%", dirs::home_dir()),
        ("%TEMP%", Some(std::env::temp_dir())),
        (
            "%PROGRAMDATA%",
            Some(PathBuf::from(
                std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()),
            )),
        ),
        (
            "%WINDIR%",
            Some(PathBuf::from(
                std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()),
            )),
        ),
    ];
    for (k, v) in vars {
        if let Some(v) = v {
            s = s.replace(k, v.to_string_lossy().trim_end_matches('\\'));
        }
    }
    s
}

/// Expands `*` wildcards (one path component each, may be partial like `blender_*`).
fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    let mut acc = vec![PathBuf::new()];
    for (i, comp) in pattern.split('\\').enumerate() {
        let mut next = Vec::new();
        for base in &acc {
            if comp.contains('*') {
                let (pre, post) = comp.split_once('*').unwrap();
                let (pre, post) = (pre.to_lowercase(), post.to_lowercase());
                if let Ok(rd) = std::fs::read_dir(base) {
                    for e in rd.flatten() {
                        let n = e.file_name().to_string_lossy().to_lowercase();
                        if n.starts_with(&pre) && n.ends_with(&post) && e.path().is_dir() {
                            next.push(e.path());
                        }
                    }
                }
            } else if i == 0 {
                next.push(PathBuf::from(format!("{comp}\\")));
            } else {
                next.push(base.join(comp));
            }
        }
        acc = next;
        if acc.is_empty() {
            break;
        }
    }
    acc.into_iter().filter(|p| p.is_dir()).collect()
}

/// Every known location that exists on this PC.
pub fn resolve() -> Vec<KbHit> {
    let mut out = Vec::new();
    for e in entries() {
        for p in expand_glob(&expand_vars(&e.path)) {
            out.push(KbHit {
                app: e.app.clone(),
                cat: e.cat.clone(),
                safety: e.safety.clone(),
                days: e.days,
                note: e.note.clone(),
                path: p.to_string_lossy().into_owned(),
            });
        }
    }
    out
}

/// What do we know about this folder (or the nearest known folder above it)?
pub fn lookup(path: &str, resolved: &[KbHit]) -> Option<KbHit> {
    let low = path.trim_end_matches('\\').to_lowercase();
    resolved
        .iter()
        .filter(|h| {
            let hp = h.path.to_lowercase();
            low == hp || low.starts_with(&format!("{hp}\\"))
        })
        .max_by_key(|h| h.path.len())
        .cloned()
}

pub fn all() -> Vec<KbEntry> {
    entries().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kb_parses_and_resolves() {
        assert!(entries().len() > 50);
        let r = resolve();
        assert!(r.iter().any(|h| h.app == "Windows" && h.cat == "temp"));
        let temp = std::env::temp_dir()
            .join("x.tmp")
            .to_string_lossy()
            .into_owned();
        assert_eq!(lookup(&temp, &r).map(|h| h.cat), Some("temp".into()));
    }
}

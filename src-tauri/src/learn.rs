//! Learning from decisions: if you always keep PDFs from your bank, Dusty
//! stops pestering you about them; if you always recycle setup files, it
//! gets more confident about those.

use crate::model::{Action, Finding};
use crate::store::{Store, Tally};
use std::collections::HashMap;

fn host(url: &Option<String>) -> String {
    url.as_deref()
        .and_then(|u| u.split("//").nth(1))
        .and_then(|r| r.split('/').next())
        .map(|h| h.trim_start_matches("www.").to_lowercase())
        .unwrap_or_default()
}

fn folder(f: &Finding) -> String {
    std::path::Path::new(&f.path)
        .parent()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Records what the user decided about a finding.
pub fn record(store: &Store, f: &Finding, action: Action) {
    let verdict = match action {
        Action::Keep => "keep",
        a if a.is_removal() || matches!(a, Action::Archive | Action::Hibernate) => "remove",
        _ => return,
    };
    store.add_verdict(f.kind.key(), &f.ext, &host(&f.origin), &folder(f), verdict);
}

/// Adjusts confidence and explains why, based on past decisions.
pub fn apply(findings: &mut [Finding], tallies: &HashMap<String, Tally>) {
    for f in findings.iter_mut() {
        let keys = [
            (
                format!("ext:{}:{}", f.kind.key(), f.ext),
                format!(
                    "{} files like this",
                    if f.ext.is_empty() {
                        "these".to_string()
                    } else {
                        format!(".{}", f.ext)
                    }
                ),
            ),
            (
                format!("host:{}", host(&f.origin)),
                format!("downloads from {}", host(&f.origin)),
            ),
            (
                format!("folder:{}", folder(f)),
                "things in this folder".to_string(),
            ),
        ];
        for (key, what) in keys {
            if key.ends_with(':') || key.ends_with("::") {
                continue;
            }
            let Some(t) = tallies.get(&key) else { continue };
            let n = t.keep + t.remove;
            if n >= 3 && t.keep * 10 >= n * 8 {
                f.confidence *= 0.5;
                f.reason
                    .push_str(&format!(" (You usually keep {what}, so I'm not pushing.)"));
                break;
            }
            if n >= 4 && t.remove * 10 >= n * 9 {
                f.confidence = (f.confidence + 0.1).min(0.97);
                f.reason.push_str(&format!(" You usually let {what} go."));
                break;
            }
        }
    }
}

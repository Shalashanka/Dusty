//! Folder policies: "Downloads must stay empty", "keep the desktop sorted".
//! When a folder breaks its rule, Dusty proposes an organizing plan.

use super::Ctx;
use crate::model::{Action, Finding, Kind, finding_id};
use crate::settings::PolicyMode;
use crate::timeutil::now;
use crate::walk::{children, is_hidden, name_of};
use std::path::Path;

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();
    for policy in s.policies.iter().filter(|p| p.mode != PolicyMode::Watch) {
        let folder = Path::new(&policy.path);
        if !folder.is_dir() || ctx.is_protected(&policy.path) {
            continue;
        }
        let grace = policy.grace_hours as i64 * 3600;
        let cat_dirs: Vec<String> = s
            .sort_dest
            .keys()
            .map(|k| crate::organize::category_folder(k).to_lowercase())
            .collect();
        let items: Vec<_> = children(folder)
            .into_iter()
            .filter(|(p, md)| {
                let name = name_of(p);
                let low = name.to_lowercase();
                !is_hidden(md)
                    && low != "desktop.ini"
                    && name != crate::detect::SWEPT_DIR
                    && !(policy.mode == PolicyMode::Sorted && md.is_dir() && cat_dirs.contains(&low))
                    && !matches!(crate::walk::ext_of(p).as_str(), "lnk" | "url")
                    && now() - crate::walk::touched(md) >= grace
                    // Items the user chose to leave in a previous plan.
                    && !ctx.suppressed.contains(&finding_id(Kind::FolderPlan, &p.to_string_lossy()))
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        let label = name_of(folder);
        let mut f = Finding::new(
            Kind::FolderPlan,
            &policy.path,
            format!(
                "{label}: {} to tidy",
                if items.len() == 1 {
                    "1 thing".to_string()
                } else {
                    format!("{} things", items.len())
                }
            ),
            Action::Organize,
            vec![Action::Organize, Action::Snooze],
        );
        f.is_dir = true;
        f.items = items.len() as u64;
        f.bytes = items
            .iter()
            .filter(|(_, m)| m.is_file())
            .map(|(_, m)| m.len())
            .sum();
        f.related = items
            .iter()
            .take(12)
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();
        f.confidence = 0.8;
        f.reason = match policy.mode {
            PolicyMode::Zero => format!(
                "You asked me to keep {label} empty. I drew up a plan for where each item should live. Check it, tweak anything, and I'll do the moving."
            ),
            _ => format!(
                "You asked me to keep {label} sorted. I made a plan that files each item into its category folder."
            ),
        };
        out.push(f);
    }
    out
}

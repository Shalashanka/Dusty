//! Findings about the system itself: new startup apps, resource hogs.

use super::Ctx;
use crate::model::{Action, Finding, Kind};
use crate::timeutil::human_bytes;

pub fn scan(ctx: &Ctx) -> Vec<Finding> {
    let s = ctx.s;
    let mut out = Vec::new();

    if s.on(Kind::NewStartup) && s.watch_startup {
        for item in ctx
            .startup
            .iter()
            .filter(|i| i.enabled && !ctx.startup_known.contains(&i.id))
        {
            let mut f = Finding::new(
                Kind::NewStartup,
                &format!("startup:{}", item.id),
                item.name.clone(),
                Action::Disable,
                vec![Action::Disable, Action::Keep],
            );
            f.app = Some(item.name.clone());
            f.related = vec![item.command.clone()];
            f.confidence = 0.5;
            f.reason = format!(
                "“{}” now starts automatically with Windows. Every startup app makes boot a little slower. Keep it, or let it start only when you open it?",
                item.name
            );
            out.push(f);
        }
    }

    if s.on(Kind::ResourceHog) && s.watch_hogs {
        for g in ctx.hogs.iter() {
            let leftover = g.flags.iter().any(|f| f == "leftover");
            let busy = g.flags.iter().any(|f| f == "busy_idle");
            let mem = g.flags.iter().any(|f| f == "memory_hog");
            if !(leftover || busy || mem) || g.flags.iter().any(|f| f == "system") {
                continue;
            }
            let mut f = Finding::new(
                Kind::ResourceHog,
                &format!("process:{}", g.name.to_lowercase()),
                g.name.clone(),
                Action::Quit,
                vec![Action::Quit, Action::Keep, Action::Snooze],
            );
            f.app = Some(g.name.clone());
            f.bytes = g.memory;
            f.items = if leftover {
                g.leftover_pids.len() as u64
            } else {
                g.count as u64
            };
            f.related = if leftover {
                g.leftover_pids.iter().map(|p| p.to_string()).collect()
            } else {
                g.pids.iter().map(|p| p.to_string()).collect()
            };
            f.confidence = if leftover { 0.7 } else { 0.4 };
            f.reason = if leftover {
                format!(
                    "{} copies of {} kept running after the app that started them closed. They're probably leftovers from a dev session, using {}.",
                    g.leftover_pids.len(),
                    g.name,
                    human_bytes(g.memory)
                )
            } else if busy {
                format!(
                    "{} kept using {:.0}% of your CPU while you were away. Something stuck in the background?",
                    g.name, g.idle_cpu
                )
            } else {
                format!(
                    "{} holds {} of memory in the background without any window.",
                    g.name,
                    human_bytes(g.avg_memory)
                )
            };
            out.push(f);
        }
    }
    out
}

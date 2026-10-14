//! Dusty's desktop presence: now and then, while you work, a tiny Dusty
//! strolls along the bottom of the screen with a broom and a thought.
//! The strip is transparent and click-through; it never steals focus.

use crate::state::AppState;
use crate::timeutil::{human_bytes, local_now, now};
use crate::winsys::{self, UserState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[derive(Serialize, Clone)]
pub struct Stroll {
    pub message: String,
    pub mood: String,
}

/// Shows the stroll strip at the bottom of the primary screen.
pub fn stroll(app: &AppHandle, message: &str, mood: &str) {
    let Some(w) = app.get_webview_window("buddy") else { return };
    if let Ok(Some(m)) = w.primary_monitor() {
        let wa = m.work_area();
        let h = (190.0 * m.scale_factor()) as u32;
        let _ = w.set_size(PhysicalSize::new(wa.size.width, h));
        let _ = w.set_position(PhysicalPosition::new(wa.position.x, wa.position.y + wa.size.height as i32 - h as i32));
    }
    let _ = w.set_ignore_cursor_events(true);
    let _ = w.set_focusable(false);
    // The page shows the window itself once Dusty is parked off-screen.
    let _ = app.emit_to("buddy", "buddy-stroll", Stroll { message: message.into(), mood: mood.into() });
    app.state::<AppState>().store.set("last_stroll", &now().to_string());
}

pub fn done(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("buddy") {
        let _ = w.hide();
    }
}

/// Called every scheduler tick: a stroll every couple of hours, only while
/// the user is around (so they notice), never in fullscreen or quiet hours.
pub fn maybe_stroll(app: &AppHandle) {
    let state = app.state::<AppState>();
    let s = state.settings.read().clone();
    if !s.wander {
        return;
    }
    let (_, _, _, hour) = local_now();
    if s.is_quiet_hour(hour as u32) || winsys::user_state() != UserState::Available || winsys::idle_secs() > 90 {
        return;
    }
    // ~every 2-3 hours, a bit random so it feels alive rather than scheduled.
    // Never right after install or start-up: the first pass waits a full gap.
    let last = state.store.get_i64("last_stroll");
    if last == 0 {
        state.store.set("last_stroll", &now().to_string());
        return;
    }
    let gap = 2 * 3600 + (now() % 3600);
    if now() - last < gap {
        return;
    }
    let (n, bytes) = state.pending();
    let freed = state.store.stats().freed_week;
    let scanning = state.scanning.load(Ordering::Relaxed);
    let pick = |opts: &[&str]| opts[(now() as usize / 7) % opts.len()].to_string();
    let (msg, mood) = if scanning {
        (pick(&["Just dusting a few corners…", "Don't mind me, sweeping up.", "Checking the cupboards…"]), "sweeping")
    } else if n > 0 {
        (
            format!(
                "{} {} waiting ({}). No rush!",
                n,
                if n == 1 { "thing" } else { "things" },
                human_bytes(bytes)
            ),
            "curious",
        )
    } else if freed > 0 {
        (format!("We freed {} this week ✨", human_bytes(freed)), "happy")
    } else {
        (pick(&["All tidy around here.", "Everything's sparkling!", "Just passing by 👋", "Nothing to see, all clean."]), "happy")
    };
    stroll(app, &msg, mood);
}

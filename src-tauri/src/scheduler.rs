//! The heartbeat: decides *when* to look around, runs scans at background
//! priority, applies autopilot rules and decides whether a nudge is worth it.

use crate::detect::{self, Ctx};
use crate::model::{Finding, Kind, ScanSummary};
use crate::settings::Mode;
use crate::state::AppState;
use crate::timeutil::{human_age, human_bytes, local_now, now};
use crate::winsys::{self, UserState};
use crate::{toast, tray};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const TICK: Duration = Duration::from_secs(30);
/// If the user never goes idle, scan anyway this long after it was due.
const MAX_DEFER: i64 = 3 * 3600;

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("dustling-scheduler".into())
        .spawn(move || run(app))
        .expect("spawn scheduler");
}

fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    loop {
        let forced = {
            let mut sched = state.schedule.lock();
            if !sched.force {
                state.wake.wait_for(&mut sched, TICK);
            }
            std::mem::take(&mut sched.force)
        };

        if forced {
            run_scan(&app, false);
            continue;
        }
        let t = now();
        if state.paused_until.load(Ordering::Relaxed) > t {
            continue;
        }
        let due = state.schedule.lock().next_due;
        if t < due {
            continue;
        }
        let idle_needed = state.settings.read().idle_min as u64 * 60;
        let idle = winsys::idle_secs() >= idle_needed;
        let overdue = t >= due + MAX_DEFER;
        let busy = winsys::user_state() == UserState::Busy;
        if (idle || overdue) && !busy && !winsys::low_battery() {
            run_scan(&app, true);
        }
    }
}

/// The watcher saw files arrive: look again sooner.
pub fn nudge_dirty(state: &AppState) {
    let floor = state.settings.read().scan_floor_min as f64;
    let mut s = state.schedule.lock();
    s.interval_min = (s.interval_min / 2.0).max(floor);
    s.next_due = s.next_due.min(now() + (floor * 60.0) as i64);
}

pub fn emit_changed(app: &AppHandle) {
    let _ = app.emit("changed", ());
}

fn set_phase(app: &AppHandle, phase: &str) {
    *app.state::<AppState>().phase.write() = phase.to_string();
    let _ = app.emit("phase", phase);
}

pub fn run_scan(app: &AppHandle, background: bool) {
    let state = app.state::<AppState>();
    if state.scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    let started = Instant::now();
    set_phase(app, "Waking up");
    emit_changed(app);

    let settings = state.settings.read().clone();

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(if background { (threads / 2).max(2) } else { threads })
        .thread_name(|i| format!("dustling-scan-{i}"))
        .start_handler(move |_| {
            if background {
                winsys::set_background_mode(true)
            }
        })
        .build()
        .expect("scan pool");

    let phase_app = app.clone();
    let phase = move |p: &str| set_phase(&phase_app, p);
    let ctx = Ctx {
        s: &settings,
        now: now(),
        first_seen: state.store.first_seen_map(),
        installed: winsys::installed_programs(),
        phase: &phase,
    };
    if background {
        winsys::set_background_mode(true);
    }
    let mut found = pool.install(|| detect::run(&ctx));
    if background {
        winsys::set_background_mode(false);
    }

    let suppressed = state.store.suppressed();
    found.retain(|f| !suppressed.contains(&f.id));
    let ids: Vec<String> = found.iter().map(|f| f.id.clone()).collect();
    let fresh = state.store.mark_seen(&ids);

    // Autopilot: act on rules the user trusts.
    *state.findings.write() = found;
    let auto_ids: Vec<(String, crate::model::Action)> = state
        .findings
        .read()
        .iter()
        .filter(|f| settings.mode(f.kind) == Mode::Auto && f.suggested != crate::model::Action::Keep)
        .map(|f| (f.id.clone(), f.suggested))
        .collect();
    let mut auto_count = 0;
    let mut auto_freed = 0;
    for (id, action) in auto_ids {
        set_phase(app, "Tidying up on autopilot");
        let o = crate::actions::apply(&state, &[id], action, true);
        auto_count += o.done;
        auto_freed += o.freed;
    }

    let (count, bytes) = state.pending();
    let new_findings: Vec<Finding> = state
        .findings
        .read()
        .iter()
        .filter(|f| fresh.contains(&f.id))
        .cloned()
        .collect();
    let summary = ScanSummary {
        at: now(),
        duration_ms: started.elapsed().as_millis() as u64,
        count,
        bytes,
        new_count: new_findings.len(),
        auto_count,
        auto_freed,
        background,
    };
    *state.last_scan.write() = Some(summary.clone());
    state.store.set("last_scan", &now().to_string());

    // Time dilation: quiet results stretch the interval, news shrinks it.
    {
        let mut s = state.schedule.lock();
        let floor = settings.scan_floor_min as f64;
        let cap = settings.scan_cap_min as f64;
        s.interval_min = if new_findings.is_empty() { (s.interval_min * 1.6).min(cap) } else { floor };
        s.next_due = now() + (s.interval_min * 60.0) as i64;
    }

    set_phase(app, "");
    state.scanning.store(false, Ordering::SeqCst);
    tray::refresh(app);
    emit_changed(app);
    let _ = app.emit("scan-finished", &summary);

    maybe_nudge(app, &new_findings, auto_count, auto_freed);
}

fn maybe_nudge(app: &AppHandle, fresh: &[Finding], auto_count: usize, auto_freed: u64) {
    let state = app.state::<AppState>();
    let s = state.settings.read().clone();
    if !s.notify || (fresh.is_empty() && auto_count == 0) {
        return;
    }
    // Don't ping about things the user is looking at right now.
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
            return;
        }
    }
    let (_, _, _, hour) = local_now();
    if s.is_quiet_hour(hour as u32) || winsys::user_state() != UserState::Available {
        return;
    }
    let last = state.store.get_i64("last_nudge");
    if now() - last < s.nudge_gap_hours as i64 * 3600 {
        return;
    }

    if !fresh.is_empty() {
        let bytes: u64 = fresh.iter().map(|f| f.bytes).sum();
        let (title, body) = nudge_text(fresh, bytes);
        toast::show(app, &title, &body, true);
    } else {
        toast::show(
            app,
            &format!("I tidied up {} while you were away", human_bytes(auto_freed)),
            &format!("{auto_count} things on autopilot. Everything is in your history if you want it back."),
            false,
        );
    }
    state.store.set("last_nudge", &now().to_string());
}

fn nudge_text(fresh: &[Finding], bytes: u64) -> (String, String) {
    let n = fresh.len();
    let star = fresh
        .iter()
        .filter(|f| f.kind != Kind::SystemJunk)
        .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap())
        .unwrap_or(&fresh[0]);
    let title = if n == 1 {
        format!("Found a {} ({})", star.kind.noun(), human_bytes(bytes))
    } else {
        format!("{n} things are gathering dust · {}", human_bytes(bytes))
    };
    let age = (now() - star.last_touched) / crate::timeutil::DAY;
    let body = match star.kind {
        Kind::DevJunk => format!("“{}” has been idle for {}. Want me to clear its build leftovers?", star.title, human_age(age)),
        Kind::DesktopClutter => "Your desktop is getting crowded. Want me to sweep it?".into(),
        Kind::Duplicate => format!("“{}” exists twice. Shall I recycle the extra copy?", star.title),
        _ if star.last_touched > 0 => format!("Like “{}”, untouched for {}. Keep it?", star.title, human_age(age)),
        _ => format!("Like “{}”. Want to take a look?", star.title),
    };
    (title, body)
}

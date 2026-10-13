//! The heartbeat: decides *when* to look around, runs scans at background
//! priority, keeps the file index fresh, samples processes, applies
//! autopilot rules and decides whether a nudge is worth it.

use crate::detect::{self, Ctx};
use crate::model::{Action, Finding, Kind, ScanSummary};
use crate::settings::Mode;
use crate::state::AppState;
use crate::timeutil::{DAY, human_age, human_bytes, local_now, now};
use crate::winsys::{self, UserState};
use crate::{toast, tray};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const TICK: Duration = Duration::from_secs(30);
/// If the user never goes idle, scan anyway this long after it was due.
const MAX_DEFER: i64 = 3 * 3600;
/// Rebuild the index (at idle) when older than this.
const INDEX_MAX_AGE: i64 = 12 * 3600;

pub fn start(app: AppHandle) {
    let a = app.clone();
    std::thread::Builder::new()
        .name("dusty-scheduler".into())
        .spawn(move || run(a))
        .expect("spawn scheduler");
    let a = app.clone();
    std::thread::Builder::new()
        .name("dusty-index".into())
        .spawn(move || index_loop(a))
        .expect("spawn index");
    std::thread::Builder::new()
        .name("dusty-procs".into())
        .spawn(move || procs_loop(app))
        .expect("spawn procs");
}

fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.store.get_i64("last_digest") == 0 {
        state.store.set("last_digest", &now().to_string());
    }
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
        maybe_digest(&app);
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

// ---- file index -------------------------------------------------------------

fn index_loop(app: AppHandle) {
    let state = app.state::<AppState>();
    let path = state.index_path();
    if state.settings.read().index_enabled
        && let Ok(idx) = crate::index::load(&path)
    {
        let roots: Vec<String> = idx.roots.iter().map(|r| r.1.clone()).collect();
        state.index.install(idx);
        state.index.watch(&roots);
        let _ = app.emit("index", state.index.status());
    }
    loop {
        let s = state.settings.read().clone();
        let requested = state.index.rebuild.swap(false, Ordering::SeqCst);
        if s.index_enabled {
            let status = state.index.status();
            let stale = !status.ready || now() - status.built_at > INDEX_MAX_AGE;
            let idle = winsys::idle_secs() >= s.idle_min as u64 * 60;
            let busy = winsys::user_state() == UserState::Busy;
            if requested || (stale && (!status.ready || idle) && !busy && !winsys::low_battery()) {
                rebuild_index(&app, !requested);
            }
        } else if state.index.get().is_some() {
            state.index.clear();
            let _ = std::fs::remove_file(&path);
            let _ = app.emit("index", state.index.status());
        }
        for _ in 0..60 {
            std::thread::sleep(Duration::from_secs(5));
            if state.index.rebuild.load(Ordering::Relaxed) {
                break;
            }
        }
    }
}

pub fn rebuild_index(app: &AppHandle, background: bool) {
    let state = app.state::<AppState>();
    if state.index.building.swap(true, Ordering::SeqCst) {
        return;
    }
    let roots = state.settings.read().drive_roots();
    state.index.progress.store(0, Ordering::Relaxed);
    state.index.cancel.store(false, Ordering::Relaxed);
    let _ = app.emit("index", state.index.status());
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(if background {
            (threads / 2).max(2)
        } else {
            threads
        })
        .start_handler(move |_| {
            if background {
                winsys::set_background_mode(true)
            }
        })
        .build()
        .expect("index pool");
    // Progress ticks for the UI while it runs.
    let ticker_app = app.clone();
    let ticker = std::thread::spawn(move || {
        let st = ticker_app.state::<AppState>();
        while st.index.building.load(Ordering::Relaxed) {
            let _ = ticker_app.emit("index", st.index.status());
            std::thread::sleep(Duration::from_millis(700));
        }
    });
    let idx =
        pool.install(|| crate::index::build(&roots, &state.index.progress, &state.index.cancel));
    if !state.index.cancel.load(Ordering::Relaxed) {
        let _ = crate::index::save(&idx, &state.index_path());
        state.index.install(idx);
        state.index.watch(&roots);
    }
    state.index.building.store(false, Ordering::SeqCst);
    let _ = ticker.join();
    let _ = app.emit("index", state.index.status());
}

// ---- processes --------------------------------------------------------------

fn procs_loop(app: AppHandle) {
    let state = app.state::<AppState>();
    let mut last = Instant::now() - Duration::from_secs(3600);
    loop {
        let live = state.live_procs.load(Ordering::Relaxed);
        let every = if live {
            Duration::from_secs(3)
        } else {
            Duration::from_secs(60)
        };
        if last.elapsed() >= every {
            let idle = winsys::idle_secs() >= 120;
            winsys::set_background_mode(true);
            state.procs.lock().sample(idle);
            winsys::set_background_mode(false);
            last = Instant::now();
            if live {
                let _ = app.emit("procs", ());
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

// ---- scanning ---------------------------------------------------------------

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
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(if background {
            (threads / 2).max(2)
        } else {
            threads
        })
        .thread_name(|i| format!("dusty-scan-{i}"))
        .start_handler(move |_| {
            if background {
                winsys::set_background_mode(true)
            }
        })
        .build()
        .expect("scan pool");

    let phase_app = app.clone();
    let phase = move |p: &str| set_phase(&phase_app, p);
    let startup = crate::health::startup::list();
    // First run: everything that's there now is "known"; only newcomers get flagged.
    let startup_known: std::collections::HashSet<String> = match state.store.get("startup_known") {
        Some(js) => serde_json::from_str::<Vec<String>>(&js)
            .unwrap_or_default()
            .into_iter()
            .collect(),
        None => {
            let ids: Vec<String> = startup.iter().map(|i| i.id.clone()).collect();
            state.store.set(
                "startup_known",
                &serde_json::to_string(&ids).unwrap_or_default(),
            );
            ids.into_iter().collect()
        }
    };
    let ctx = Ctx {
        s: &settings,
        now: now(),
        first_seen: state.store.first_seen_map(),
        installed: winsys::installed_programs(),
        phase: &phase,
        protected: state
            .store
            .protected()
            .into_iter()
            .map(|p| p.path)
            .collect(),
        index: state.index.get(),
        kb: crate::kb::resolve(),
        store: &state.store,
        suppressed: state.store.suppressed(),
        startup,
        startup_known,
        hogs: state.procs.lock().groups(),
    };
    if background {
        winsys::set_background_mode(true);
    }
    let mut found = pool.install(|| detect::run(&ctx));
    if background {
        winsys::set_background_mode(false);
    }

    let suppressed = &ctx.suppressed;
    found.retain(|f| !suppressed.contains(&f.id));
    if settings.learn {
        crate::learn::apply(&mut found, &state.store.tallies());
        found.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let ids: Vec<String> = found.iter().map(|f| f.id.clone()).collect();
    let fresh = state.store.mark_seen(&ids);

    // Autopilot: act on rules the user trusts.
    *state.findings.write() = found;
    let auto_ids: Vec<(String, Action)> = state
        .findings
        .read()
        .iter()
        .filter(|f| {
            settings.mode(f.kind) == Mode::Auto
                && !matches!(
                    f.suggested,
                    Action::Keep | Action::Organize | Action::Quit | Action::Disable
                )
                && f.confidence >= 0.5
        })
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
        s.interval_min = if new_findings.is_empty() {
            (s.interval_min * 1.6).min(cap)
        } else {
            floor
        };
        s.next_due = now() + (s.interval_min * 60.0) as i64;
    }

    set_phase(app, "");
    state.scanning.store(false, Ordering::SeqCst);
    tray::refresh(app);
    emit_changed(app);
    let _ = app.emit("scan-finished", &summary);

    maybe_nudge(app, &new_findings, auto_count, auto_freed, false);
}

/// Can we ping the user right now?
fn can_ping(app: &AppHandle, force: bool) -> bool {
    let state = app.state::<AppState>();
    let s = state.settings.read().clone();
    if force {
        return true;
    }
    if !s.notify {
        return false;
    }
    if let Some(w) = app.get_webview_window("main")
        && w.is_visible().unwrap_or(false)
        && w.is_focused().unwrap_or(false)
    {
        return false;
    }
    let (_, _, _, hour) = local_now();
    !s.is_quiet_hour(hour as u32) && winsys::user_state() == UserState::Available
}

pub fn maybe_nudge(
    app: &AppHandle,
    fresh: &[Finding],
    auto_count: usize,
    auto_freed: u64,
    force: bool,
) {
    let state = app.state::<AppState>();
    let s = state.settings.read().clone();
    if (fresh.is_empty() && auto_count == 0) || !can_ping(app, force) {
        return;
    }
    let last = state.store.get_i64("last_nudge");
    if !force && now() - last < s.nudge_gap_hours as i64 * 3600 {
        return;
    }

    if !fresh.is_empty() {
        let bytes: u64 = fresh.iter().map(|f| f.bytes).sum();
        let (title, body) = nudge_text(fresh, bytes);
        // Things Dusty is very sure about can be handled straight from the toast.
        let sure: Vec<&Finding> = fresh
            .iter()
            .filter(|f| {
                f.confidence >= 0.85
                    && matches!(f.suggested, Action::Recycle | Action::Delete)
                    && f.kind.is_file_based()
            })
            .collect();
        if s.quick_actions && !sure.is_empty() {
            let ids: Vec<String> = sure.iter().map(|f| f.id.clone()).collect();
            state.store.set(
                "quick_ids",
                &serde_json::to_string(&ids).unwrap_or_default(),
            );
            let sure_bytes: u64 = sure.iter().map(|f| f.bytes).sum();
            let label = format!(
                "Clean the sure ones ({}, {})",
                sure.len(),
                human_bytes(sure_bytes)
            );
            toast::show(
                app,
                &title,
                &body,
                &[
                    (&label, "quick"),
                    ("Let's look", "review"),
                    ("Not now", "later"),
                ],
            );
        } else {
            toast::show(
                app,
                &title,
                &body,
                &[("Let's tidy", "review"), ("Not now", "later")],
            );
        }
    } else {
        toast::show(
            app,
            &format!(
                "I tidied up {} while you were away",
                human_bytes(auto_freed)
            ),
            &format!(
                "{auto_count} things on autopilot. Everything is in your history if you want it back."
            ),
            &[],
        );
    }
    state.store.set("last_nudge", &now().to_string());
}

/// Runs the toast's "Clean the sure ones" button.
pub fn quick_clean(app: &AppHandle) {
    let state = app.state::<AppState>();
    let ids: Vec<String> = state
        .store
        .get("quick_ids")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut freed = 0;
    let mut done = 0;
    let suggestions: Vec<(String, Action)> = state
        .findings
        .read()
        .iter()
        .filter(|f| ids.contains(&f.id))
        .map(|f| (f.id.clone(), f.suggested))
        .collect();
    for (id, action) in suggestions {
        let o = crate::actions::apply(&state, &[id], action, false);
        freed += o.freed;
        done += o.done;
    }
    tray::refresh(app);
    emit_changed(app);
    toast::show(
        app,
        &format!("Done! {} freed", human_bytes(freed)),
        &format!(
            "{done} things handled. Changed your mind? Everything recycled is one click away in History."
        ),
        &[("Open history", "history")],
    );
}

fn maybe_digest(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.settings.read().weekly_digest || now() - state.store.get_i64("last_digest") < 7 * DAY
    {
        return;
    }
    if !can_ping(app, false) {
        return;
    }
    send_digest(app);
}

pub fn send_digest(app: &AppHandle) {
    let state = app.state::<AppState>();
    let st = state.store.stats();
    let (n, bytes) = state.pending();
    let title = if st.freed_week > 0 {
        format!("This week you freed {}", human_bytes(st.freed_week))
    } else {
        "Your weekly tidy report".to_string()
    };
    let body = if n > 0 {
        format!(
            "{n} things ({}) are waiting for a quick decision. Two minutes is all it takes.",
            human_bytes(bytes)
        )
    } else {
        "Everything is tidy. Nice work, see you next week!".to_string()
    };
    toast::show(
        app,
        &title,
        &body,
        if n > 0 {
            &[("Let's tidy", "review"), ("Later", "later")]
        } else {
            &[]
        },
    );
    state.store.set("last_digest", &now().to_string());
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
    let age = (now() - star.last_touched) / DAY;
    let body = match star.kind {
        Kind::DevJunk => format!(
            "“{}” has been idle for {}. Want me to clear its build leftovers?",
            star.title,
            human_age(age)
        ),
        Kind::StaleProject => format!(
            "“{}” has been asleep for {}. Still working on it?",
            star.title,
            human_age(age)
        ),
        Kind::DesktopClutter => "Your desktop is getting crowded. Want me to sweep it?".into(),
        Kind::FolderPlan => format!("{}. I made a plan, want to see it?", star.title),
        Kind::Duplicate => format!(
            "“{}” exists twice. Shall I recycle the extra copy?",
            star.title
        ),
        Kind::NewStartup => format!(
            "“{}” added itself to your startup apps. Keep it?",
            star.title
        ),
        Kind::ResourceHog => format!("{} is eating resources in the background.", star.title),
        Kind::Unsure => format!(
            "What's “{}”? It's big and nobody touched it in {}.",
            star.title,
            human_age(age)
        ),
        _ if star.last_touched > 0 => format!(
            "Like “{}”, untouched for {}. Keep it?",
            star.title,
            human_age(age)
        ),
        _ => format!("Like “{}”. Want to take a look?", star.title),
    };
    (title, body)
}

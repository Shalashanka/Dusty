use crate::state::AppState;
use crate::timeutil::{human_bytes, now};
use std::sync::atomic::Ordering;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Dustling", true, None::<&str>)?;
    let tidy = MenuItem::with_id(app, "tidy", "Look around now", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause for 3 hours", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &tidy, &pause, &resume, &sep, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Dustling")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| {
            let state = app.state::<AppState>();
            match e.id.as_ref() {
                "open" => crate::show_main(app),
                "tidy" => state.request_scan(),
                "pause" => {
                    let until = now() + 3 * 3600;
                    state.paused_until.store(until, Ordering::Relaxed);
                    state.store.set("paused_until", &until.to_string());
                    crate::scheduler::emit_changed(app);
                    refresh(app);
                }
                "resume" => {
                    state.paused_until.store(0, Ordering::Relaxed);
                    state.store.set("paused_until", "0");
                    crate::scheduler::emit_changed(app);
                    refresh(app);
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                let app = tray.app_handle();
                crate::show_main(app);
                let _ = app.emit("navigate", "home");
            }
        })
        .build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let state = app.state::<AppState>();
    let (n, bytes) = state.pending();
    let paused = state.paused_until.load(Ordering::Relaxed) > now();
    let tip = match (paused, n) {
        (true, _) => "Dustling · napping".to_string(),
        (false, 0) => "Dustling · all tidy".to_string(),
        (false, n) => format!("Dustling · {n} things to review ({})", human_bytes(bytes)),
    };
    let _ = tray.set_tooltip(Some(tip));
}

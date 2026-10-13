//! Tray icon: click to open, right-click for Dusty's own menu (a styled
//! popup window), and a little sweeping animation while Dusty works.

use crate::state::AppState;
use crate::timeutil::{human_bytes, now};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

const FRAMES: [&[u8]; 3] = [
    include_bytes!("../icons/tray/sweep0.png"),
    include_bytes!("../icons/tray/sweep1.png"),
    include_bytes!("../icons/tray/sweep2.png"),
];

static ANIMATING: AtomicBool = AtomicBool::new(false);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Dusty")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, position, .. } = e {
                let app = tray.app_handle();
                match button {
                    MouseButton::Left => {
                        crate::show_main(app);
                        let _ = app.emit("navigate", "home");
                    }
                    MouseButton::Right => show_menu(app, position),
                    _ => {}
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Opens the styled tray menu just above the cursor, inside the screen.
fn show_menu(app: &AppHandle, at: PhysicalPosition<f64>) {
    let Some(w) = app.get_webview_window("traymenu") else { return };
    let size = w.outer_size().unwrap_or(tauri::PhysicalSize::new(290, 420));
    let (mut x, mut y) = (at.x as i32 - size.width as i32 + 24, at.y as i32 - size.height as i32 - 8);
    if let Ok(Some(m)) = w.current_monitor().or_else(|_| w.primary_monitor()) {
        let wa = m.work_area();
        x = x.clamp(wa.position.x + 8, wa.position.x + wa.size.width as i32 - size.width as i32 - 8);
        y = y.clamp(wa.position.y + 8, wa.position.y + wa.size.height as i32 - size.height as i32 - 8);
    }
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to("traymenu", "traymenu-open", ());
}

/// Sweeping broom in the tray while scanning/indexing.
pub fn set_busy(app: &AppHandle, busy: bool) {
    if !busy {
        ANIMATING.store(false, Ordering::SeqCst);
        return;
    }
    if ANIMATING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let frames: Vec<Image> = FRAMES.iter().filter_map(|b| Image::from_bytes(b).ok()).collect();
        let order = [0usize, 1, 2, 1];
        let mut i = 0;
        while ANIMATING.load(Ordering::SeqCst) {
            if let (Some(tray), Some(img)) = (app.tray_by_id("main"), frames.get(order[i % order.len()])) {
                let _ = tray.set_icon(Some(img.clone()));
            }
            i += 1;
            std::thread::sleep(std::time::Duration::from_millis(260));
        }
        if let (Some(tray), Some(icon)) = (app.tray_by_id("main"), app.default_window_icon().cloned()) {
            let _ = tray.set_icon(Some(icon));
        }
    });
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let state = app.state::<AppState>();
    let (n, bytes) = state.pending();
    let paused = state.paused_until.load(Ordering::Relaxed) > now();
    let tip = match (paused, n) {
        (true, _) => "Dusty · napping".to_string(),
        (false, 0) => "Dusty · all tidy".to_string(),
        (false, n) => format!("Dusty · {n} things to review ({})", human_bytes(bytes)),
    };
    let _ = tray.set_tooltip(Some(tip));
}

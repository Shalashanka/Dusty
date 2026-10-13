//! Windows toast notifications with action buttons.

use crate::winsys;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};
use tauri_winrt_notification::{Duration, Sound, Toast};

pub const APP_ID: &str = "com.griz.dusty";

static ICON: OnceLock<PathBuf> = OnceLock::new();
static REGISTERED: OnceLock<bool> = OnceLock::new();

pub fn init(data_dir: &std::path::Path) {
    let icon = data_dir.join("toast-icon.png");
    let _ = std::fs::write(&icon, include_bytes!("../icons/128x128@2x.png"));
    let ok = winsys::register_app_id(APP_ID, "Dusty", &icon);
    let _ = ICON.set(icon);
    let _ = REGISTERED.set(ok);
}

/// Shows a toast. `buttons` are (label, action) pairs; actions are handled in
/// `on_action` while Dusty is running.
pub fn show(app: &AppHandle, title: &str, body: &str, buttons: &[(&str, &str)]) {
    let id = if *REGISTERED.get().unwrap_or(&false) {
        APP_ID
    } else {
        Toast::POWERSHELL_APP_ID
    };
    let mut t = Toast::new(id)
        .title(title)
        .text1(body)
        .sound(Some(Sound::Default))
        .duration(Duration::Long);
    if let Some(icon) = ICON.get() {
        t = t.icon(icon, tauri_winrt_notification::IconCrop::Circular, "Dusty");
    }
    for (label, action) in buttons {
        t = t.add_button(label, action);
    }
    let handle = app.clone();
    t = t.on_activated(move |action| {
        on_action(&handle, action.as_deref());
        Ok(())
    });
    if t.show().is_err() && id != Toast::POWERSHELL_APP_ID {
        let _ = Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .show();
    }
}

fn on_action(app: &AppHandle, action: Option<&str>) {
    match action {
        Some("later") => {}
        Some("quick") => {
            let a = app.clone();
            std::thread::spawn(move || crate::scheduler::quick_clean(&a));
        }
        Some("history") => {
            crate::show_main(app);
            let _ = app.emit("navigate", "history");
        }
        _ => {
            crate::show_main(app);
            let _ = app.emit("navigate", "triage");
        }
    }
}

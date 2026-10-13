//! Windows toast notifications with action buttons.

use crate::winsys;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};
use tauri_winrt_notification::{Duration, Sound, Toast};

pub const APP_ID: &str = "com.griz.dustling";

static ICON: OnceLock<PathBuf> = OnceLock::new();
static REGISTERED: OnceLock<bool> = OnceLock::new();

pub fn init(data_dir: &std::path::Path) {
    let icon = data_dir.join("toast-icon.png");
    let _ = std::fs::write(&icon, include_bytes!("../icons/128x128@2x.png"));
    let ok = winsys::register_app_id(APP_ID, "Dustling", &icon);
    let _ = ICON.set(icon);
    let _ = REGISTERED.set(ok);
}

pub fn show(app: &AppHandle, title: &str, body: &str, review_buttons: bool) {
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
        t = t.icon(
            icon,
            tauri_winrt_notification::IconCrop::Circular,
            "Dustling",
        );
    }
    if review_buttons {
        t = t
            .add_button("Let's tidy", "review")
            .add_button("Not now", "later");
    }
    let handle = app.clone();
    t = t.on_activated(move |action| {
        match action.as_deref() {
            Some("later") => {}
            _ => {
                crate::show_main(&handle);
                let _ = handle.emit("navigate", "triage");
            }
        }
        Ok(())
    });
    if t.show().is_err() && id != Toast::POWERSHELL_APP_ID {
        let _ = Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .show();
    }
}

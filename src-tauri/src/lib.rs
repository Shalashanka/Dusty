mod actions;
mod commands;
mod detect;
mod model;
mod scheduler;
mod settings;
mod state;
mod store;
mod timeutil;
mod toast;
mod tray;
mod walk;
mod watcher;
mod winsys;

use state::AppState;
use tauri::{AppHandle, Manager, WindowEvent};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hidden = std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .setup(move |app| {
            let data_dir = dirs::data_local_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("Dustling");
            std::fs::create_dir_all(&data_dir)?;
            let store = store::Store::open(&data_dir.join("dustling.db"))?;
            let first_run = store.get("settings").is_none();
            app.manage(AppState::new(store, hidden));

            // Debug builds never register themselves to run at login.
            if first_run && !cfg!(debug_assertions) {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
                let state = app.state::<AppState>();
                state.store.save_settings(&state.settings.read());
            }

            toast::init(&data_dir);
            tray::create(app.handle())?;
            watcher::start(app.handle());
            scheduler::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window just tucks Dustling back into the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::ready,
            commands::scan_now,
            commands::act,
            commands::undo,
            commands::history,
            commands::get_settings,
            commands::save_settings,
            commands::pause,
            commands::reveal,
            commands::open_path,
            commands::pick_folder,
            commands::drives,
            commands::space,
            commands::recycle_path,
            commands::home_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dustling");
}

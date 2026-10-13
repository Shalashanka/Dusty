mod actions;
mod commands;
mod detect;
mod health;
mod hibernate;
mod index;
mod kb;
mod learn;
mod migrate;
mod model;
mod nlq;
mod organize;
mod scheduler;
mod settings;
mod state;
mod store;
mod thumbs;
mod timeutil;
mod toast;
mod tray;
mod walk;
mod watcher;
mod winsys;

use state::AppState;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Spotlight-style search: toggles the floating search window.
pub fn toggle_search(app: &AppHandle) {
    let Some(w) = app.get_webview_window("search") else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit_to("search", "search-open", ());
    }
}

/// (Re)binds the global search hotkey from settings.
pub fn register_hotkey(app: &AppHandle) -> Result<(), String> {
    let combo = app
        .state::<AppState>()
        .settings
        .read()
        .search_hotkey
        .clone();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if combo.trim().is_empty() {
        return Ok(());
    }
    gs.register(combo.as_str())
        .map_err(|e| format!("couldn't use {combo} as a shortcut: {e}"))
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_search(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let data_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            migrate::run(&data_dir);
            let store = store::Store::open(&data_dir.join("dusty.db"))?;
            let first_run = store.get("settings").is_none();
            app.manage(AppState::new(store, data_dir.clone(), hidden));

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
            let _ = register_hotkey(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the window just tucks Dusty back into the tray.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            // The search popup disappears when you click elsewhere.
            WindowEvent::Focused(false) if window.label() == "search" => {
                let _ = window.hide();
            }
            _ => {}
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
            commands::index_status,
            commands::index_rebuild,
            commands::search,
            commands::space_tree,
            commands::kb_lookup,
            commands::kb_all,
            commands::thumbnail,
            commands::read_text,
            commands::open_file,
            commands::delete_path,
            commands::recycle_bin,
            commands::empty_bin,
            commands::protect,
            commands::unprotect,
            commands::protected_list,
            commands::make_plan,
            commands::apply_plan,
            commands::hibernated,
            commands::wake_project,
            commands::startup_list,
            commands::startup_set,
            commands::processes,
            commands::set_live_procs,
            commands::kill_processes,
            commands::system_files,
            commands::power_plans,
            commands::set_power_plan,
            commands::optimize_info,
            commands::optimize_drive,
            commands::analyze_drive,
            commands::tune,
            commands::preinstalled_apps,
            commands::hide_search,
            commands::open_in_dusty,
            commands::dev_action,
            commands::app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dusty");
}

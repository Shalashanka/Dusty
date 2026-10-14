mod actions;
pub mod bar;
mod buddy;
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
mod speech;
mod state;
mod store;
mod thumbs;
mod timeutil;
mod toast;
mod tray;
mod updater;
mod voice;
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

/// Opens the floating Dusty bar in a mode: "search", "commands" or "voice".
/// Pressing the same shortcut again while it's open closes it.
pub fn toggle_search(app: &AppHandle) {
    open_bar(app, "search");
}

pub fn open_bar(app: &AppHandle, mode: &str) {
    let Some(w) = app.get_webview_window("search") else {
        return;
    };
    let current = BAR_MODE.lock().clone();
    if w.is_visible().unwrap_or(false) && current == mode && mode != "voice" {
        let _ = w.hide();
        return;
    }
    let was_visible = w.is_visible().unwrap_or(false);
    *BAR_MODE.lock() = mode.to_string();
    // Already open: the page morphs in place (bar::fit). Otherwise start where the mode lives.
    if !was_visible {
        bar::snap(app, mode, if mode == "commands" { 470.0 } else { 112.0 });
    }
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to("search", "search-open", mode);
    if mode == "voice" {
        let state = app.state::<AppState>();
        let (enabled, model) = {
            let s = state.settings.read();
            (s.voice_enabled, s.voice_model.clone())
        };
        if enabled {
            voice::listen(app.clone(), model);
        }
    }
}

static BAR_MODE: parking_lot::Mutex<String> = parking_lot::Mutex::new(String::new());

pub fn set_bar_mode(mode: &str) {
    *BAR_MODE.lock() = mode.to_string();
}
/// Shortcut id -> bar mode.
static HOTKEYS: parking_lot::Mutex<Vec<(u32, &'static str)>> = parking_lot::Mutex::new(Vec::new());

/// (Re)binds the global shortcuts from settings.
pub fn register_hotkey(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::Shortcut;
    let (search, palette, voice) = {
        let s = app.state::<AppState>().settings.read().clone();
        (s.search_hotkey, s.palette_hotkey, s.voice_hotkey)
    };
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut map = Vec::new();
    let mut errors = Vec::new();
    for (combo, mode) in [(search, "search"), (palette, "commands"), (voice, "voice")] {
        if combo.trim().is_empty() {
            continue;
        }
        match combo.parse::<Shortcut>() {
            Ok(sc) => match gs.register(sc) {
                Ok(()) => map.push((sc.id(), mode)),
                Err(e) => errors.push(format!("{combo}: {e}")),
            },
            Err(e) => errors.push(format!("{combo}: {e}")),
        }
    }
    *HOTKEYS.lock() = map;
    if errors.is_empty() { Ok(()) } else { Err(format!("couldn't use {}", errors.join(", "))) }
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
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let mode = HOTKEYS.lock().iter().find(|(id, _)| *id == shortcut.id()).map(|(_, m)| *m);
                        open_bar(app, mode.unwrap_or("search"));
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
            voice::warm_up(app.handle().clone());
            // Safety net: if the UI hasn't asked to be shown after a few
            // seconds (slow first WebView start), show the window anyway.
            if !hidden {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(4));
                    let state = handle.state::<AppState>();
                    if !state
                        .shown_once
                        .swap(true, std::sync::atomic::Ordering::SeqCst)
                    {
                        show_main(&handle);
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the window just tucks Dusty back into the tray.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            // Popups disappear when you click elsewhere.
            WindowEvent::Focused(false) if window.label() == "search" || window.label() == "traymenu" => {
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
            commands::toggle_search,
            commands::add_mirror,
            commands::check_update,
            commands::install_update,
            commands::voice_status,
            commands::voice_download,
            commands::voice_listen,
            commands::voice_stop,
            commands::open_bar,
            commands::bar_fit,
            commands::tts_models,
            commands::tts_download,
            commands::say,
            commands::say_stop,
            commands::bar_command,
            commands::buddy_done,
            commands::buddy_test,
            commands::hide_traymenu,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dusty");
}

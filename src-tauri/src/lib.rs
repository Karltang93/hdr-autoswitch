mod background;
mod commands;
mod config;
mod config_storage;
mod database;
mod display;
mod hdr_controller;
mod legacy_upgrade;
mod library;
mod monitor_hook;
mod process;
mod scanner;
mod tray;

use background::BackgroundWork;
use config::{ConfigManager, ConfigMode, ConfigSnapshot};
use monitor_hook::MonitorService;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

struct AppState {
    config_mgr: Arc<ConfigManager>,
    monitor_service: Arc<MonitorService>,
    config_actions: Mutex<()>,
    background: BackgroundWork,
    stopping: AtomicBool,
}

impl AppState {
    fn ensure_admission(&self) -> Result<(), String> {
        if self.stopping.load(Ordering::Acquire) {
            Err("The application is shutting down.".into())
        } else {
            Ok(())
        }
    }

    fn shutdown(&self) {
        if self.stopping.swap(true, Ordering::AcqRel) {
            return;
        }
        self.background.stop();
        self.monitor_service.shutdown();
    }
}

fn emit_config(app: &AppHandle, snapshot: &ConfigSnapshot) {
    if let Err(error) = app.emit("config-changed", snapshot) {
        eprintln!("Cannot notify the UI of canonical configuration: {error}");
    }
}

fn reconcile_controller(manager: &ConfigManager) -> Result<ConfigSnapshot, String> {
    let issue = match legacy_upgrade::check_predecessor() {
        Err(error) => Some(error),
        Ok(()) => {
            let snapshot = manager.snapshot()?;
            if snapshot.mode == ConfigMode::Ready {
                legacy_upgrade::configure_autostart(snapshot.settings.autostart).err()
            } else {
                None
            }
        }
    };
    if let Some(error) = &issue {
        eprintln!("HDR controller is paused: {error}");
    }
    manager.set_controller_issue(issue)
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        for result in [window.unminimize(), window.show(), window.set_focus()] {
            if let Err(error) = result {
                eprintln!("Cannot show application window: {error}");
            }
        }
    }
}

fn report_startup_failure(error: impl std::fmt::Display) {
    let message = format!("HDR Auto-Switch could not start safely.\n\n{error}");
    eprintln!("{message}");
    rfd::MessageDialog::new()
        .set_title("HDR Auto-Switch")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Some(code) = legacy_upgrade::installer_command() {
        std::process::exit(code);
    }
    let built = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let local_dir = app.path().app_local_data_dir()?;
            let legacy_path = app
                .path()
                .data_dir()?
                .join("HDRAutoSwitch")
                .join("config.json");
            let config_mgr = Arc::new(
                ConfigManager::load(local_dir, legacy_path).map_err(std::io::Error::other)?,
            );
            let snapshot =
                reconcile_controller(&config_mgr).map_err(std::io::Error::other)?;
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(tauri::include_image!("icons/128x128.png"))?;
                let minimized = std::env::args().any(|arg| arg == "--minimized")
                    || snapshot.settings.start_minimized;
                if minimized
                    && snapshot.mode == ConfigMode::Ready
                    && snapshot.controller_issue.is_none()
                    && matches!(
                        snapshot.settings.target_monitor,
                        config::TargetMonitor::All | config::TargetMonitor::Monitor { .. }
                    )
                    && snapshot.settings.switch_method == config::SwitchMethod::Native
                {
                    window.hide()?;
                }
            }
            tray::setup_tray(app.handle())?;
            let monitor_service = MonitorService::new(config_mgr.clone(), app.handle().clone());
            let background = BackgroundWork::start(
                &config_mgr,
                monitor_service.clone(),
                app.handle().clone(),
            );
            app.manage(AppState {
                config_mgr,
                monitor_service: monitor_service.clone(),
                config_actions: Mutex::new(()),
                background,
                stopping: AtomicBool::new(false),
            });
            monitor_service.start_hook();
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("Cannot hide the application window: {error}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_monitors,
            commands::set_hdr,
            commands::get_current_status,
            commands::get_config,
            commands::patch_settings,
            commands::initialize_config,
            commands::import_legacy_config,
            commands::restore_config,
            commands::reset_config,
            commands::recheck_controller,
            commands::get_catalog,
            commands::sync_database,
            commands::scan_installed_games,
            commands::import_detected_games,
            commands::add_custom_app,
            commands::remove_app,
            commands::toggle_app,
            commands::get_running_processes,
            commands::pick_game_exe,
            commands::set_ui_language,
            commands::inspect_exe_path,
            commands::verify_game_paths,
        ])
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|handle, event| {
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
                if let Some(state) = handle.try_state::<AppState>() {
                    state.shutdown();
                }
            }
        }),
        Err(error) => report_startup_failure(error),
    }
}

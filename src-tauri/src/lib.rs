mod automatic_authority;
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
mod xbox_config;

use background::BackgroundWork;
use config::{ConfigManager, ConfigMode, ConfigSnapshot};
use monitor_hook::MonitorService;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

const SAFE_TEST_ISSUE: &str =
    "Safe test mode is active. HDR control, autostart changes, and background work are disabled.";

struct AppState {
    config_mgr: Arc<ConfigManager>,
    monitor_service: Arc<MonitorService>,
    config_actions: Mutex<()>,
    background: BackgroundWork,
    stopping: AtomicBool,
    safe_test_mode: bool,
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

#[cfg(debug_assertions)]
fn safe_test_paths() -> Result<Option<(PathBuf, PathBuf)>, String> {
    let Some(root) = std::env::var_os("HDR_AUTOSWITCH_SAFE_TEST_DIR") else {
        return Ok(None);
    };
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err("HDR_AUTOSWITCH_SAFE_TEST_DIR must be an absolute path.".into());
    }
    let local_dir = root.join("local");
    std::fs::create_dir_all(&local_dir)
        .map_err(|error| format!("Create safe test settings directory: {error}"))?;
    Ok(Some((local_dir, root.join("legacy-config.json"))))
}

#[cfg(not(debug_assertions))]
fn safe_test_paths() -> Result<Option<(PathBuf, PathBuf)>, String> {
    Ok(None)
}

fn emit_config(app: &AppHandle, snapshot: &ConfigSnapshot) {
    if let Err(error) = app.emit("config-changed", snapshot) {
        eprintln!("Cannot notify the UI of canonical configuration: {error}");
    }
}

fn reconcile_controller(
    manager: &ConfigManager,
    safe_test_mode: bool,
) -> Result<ConfigSnapshot, String> {
    if safe_test_mode {
        return manager.set_controller_issue(Some(SAFE_TEST_ISSUE.into()));
    }
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

fn should_start_hidden(snapshot: &ConfigSnapshot, minimized_requested: bool) -> bool {
    minimized_requested
        && snapshot.mode == ConfigMode::Ready
        && snapshot.controller_issue.is_none()
        && matches!(
            snapshot.settings.target_monitor,
            config::TargetMonitor::All | config::TargetMonitor::Monitor { .. }
        )
        && snapshot.settings.switch_method == config::SwitchMethod::Native
}

fn second_instance_requests_window(arguments: &[String]) -> bool {
    !arguments
        .iter()
        .any(|argument| argument.eq_ignore_ascii_case("--minimized"))
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
        .plugin(tauri_plugin_single_instance::init(|app, arguments, _| {
            if second_instance_requests_window(&arguments) {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let safe_test_paths = safe_test_paths().map_err(std::io::Error::other)?;
            let safe_test_mode = safe_test_paths.is_some();
            let (local_dir, legacy_path) = match safe_test_paths {
                Some(paths) => paths,
                None => (
                    app.path().app_local_data_dir()?,
                    app.path()
                        .data_dir()?
                        .join("HDRAutoSwitch")
                        .join("config.json"),
                ),
            };
            let config_mgr = Arc::new(
                ConfigManager::load(local_dir, legacy_path).map_err(std::io::Error::other)?,
            );
            let snapshot =
                reconcile_controller(&config_mgr, safe_test_mode).map_err(std::io::Error::other)?;
            let minimized_requested = std::env::args().any(|arg| arg == "--minimized")
                || snapshot.settings.start_minimized;
            let start_hidden = should_start_hidden(&snapshot, minimized_requested);
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(tauri::include_image!("icons/128x128.png"))?;
            }
            tray::setup_tray(app.handle())?;
            let monitor_service = MonitorService::new(config_mgr.clone(), app.handle().clone());
            let background = if safe_test_mode {
                BackgroundWork::disabled()
            } else {
                BackgroundWork::start(
                    &config_mgr,
                    monitor_service.clone(),
                    app.handle().clone(),
                )
            };
            app.manage(AppState {
                config_mgr,
                monitor_service: monitor_service.clone(),
                config_actions: Mutex::new(()),
                background,
                stopping: AtomicBool::new(false),
                safe_test_mode,
            });
            if !safe_test_mode {
                monitor_service.start_hook();
            }
            if !start_hidden {
                show_main_window(app.handle());
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use config::{AppConfig, SwitchMethod, TargetMonitor};

    fn ready_snapshot() -> ConfigSnapshot {
        ConfigSnapshot {
            settings: AppConfig::default(),
            mode: ConfigMode::Ready,
            store_id: Some("store".into()),
            revision: "1".into(),
            context_token: "context".into(),
            library_generation: "1".into(),
            control_epoch: "1".into(),
            issue: None,
            controller_issue: None,
            candidates: Vec::new(),
            config_path: "test".into(),
        }
    }

    #[test]
    fn valid_minimized_start_stays_hidden() {
        assert!(should_start_hidden(&ready_snapshot(), true));
        assert!(!should_start_hidden(&ready_snapshot(), false));
    }

    #[test]
    fn recovery_or_incomplete_control_state_is_shown() {
        let mut snapshot = ready_snapshot();
        snapshot.mode = ConfigMode::RecoveryRequired;
        assert!(!should_start_hidden(&snapshot, true));

        snapshot = ready_snapshot();
        snapshot.controller_issue = Some("controller conflict".into());
        assert!(!should_start_hidden(&snapshot, true));

        snapshot = ready_snapshot();
        snapshot.settings.target_monitor = TargetMonitor::NeedsConfirmation {
            legacy_runtime_id: "runtime-id".into(),
        };
        assert!(!should_start_hidden(&snapshot, true));

        snapshot = ready_snapshot();
        snapshot.settings.switch_method = SwitchMethod::Shortcut;
        assert!(!should_start_hidden(&snapshot, true));
    }

    #[test]
    fn duplicate_minimized_start_does_not_open_the_hidden_singleton() {
        assert!(!second_instance_requests_window(&[
            r"E:\HDR Auto-Switch\tauri-app.exe".into(),
            "--minimized".into(),
        ]));
        assert!(!second_instance_requests_window(&[
            "tauri-app.exe".into(),
            "--MINIMIZED".into(),
        ]));
    }

    #[test]
    fn normal_second_instance_requests_the_existing_window() {
        assert!(second_instance_requests_window(&[
            r"E:\HDR Auto-Switch\tauri-app.exe".into(),
        ]));
    }
}

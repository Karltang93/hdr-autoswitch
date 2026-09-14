mod config;
mod database;
mod display;
mod monitor_hook;
mod process;
mod tray;

use config::{AppConfig, ConfigManager, HdrApp};
use display::MonitorInfo;
use monitor_hook::{HdrStatePayload, MonitorService};
use process::RunningProcessInfo;
use std::sync::Arc;
use tauri::{Manager, State, WindowEvent};

struct AppState {
    config_mgr: Arc<ConfigManager>,
    monitor_service: Arc<MonitorService>,
}

#[tauri::command]
fn get_monitors() -> Vec<MonitorInfo> {
    display::get_monitors()
}

#[tauri::command]
fn set_monitor_hdr(
    state: State<'_, AppState>,
    adapter_low: u32,
    adapter_high: i32,
    target_id: u32,
    enable: bool,
) -> bool {
    let success = display::set_monitor_hdr(adapter_low, adapter_high, target_id, enable);
    state.monitor_service.manual_toggle(enable);
    success
}

#[tauri::command]
fn toggle_all_hdr(state: State<'_, AppState>, enable: bool) -> bool {
    let success = display::set_all_hdr(enable);
    state.monitor_service.manual_toggle(enable);
    success
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config_mgr.get_config()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    state.config_mgr.update_config(config)
}

#[tauri::command]
fn get_running_processes() -> Vec<RunningProcessInfo> {
    process::get_running_processes()
}

#[tauri::command]
fn add_custom_app(state: State<'_, AppState>, app: HdrApp) -> Result<(), String> {
    let mut conf = state.config_mgr.get_config();
    let exe_lower = app.exe_name.to_lowercase();

    if let Some(existing) = conf.apps.iter_mut().find(|a| a.exe_name.to_lowercase() == exe_lower) {
        *existing = app;
    } else {
        conf.apps.push(app);
    }

    state.config_mgr.update_config(conf)
}

#[tauri::command]
fn remove_app(state: State<'_, AppState>, exe_name: String) -> Result<(), String> {
    let mut conf = state.config_mgr.get_config();
    let exe_lower = exe_name.to_lowercase();
    conf.apps.retain(|a| a.exe_name.to_lowercase() != exe_lower);
    state.config_mgr.update_config(conf)
}

#[tauri::command]
fn toggle_app(state: State<'_, AppState>, exe_name: String, enabled: bool) -> Result<(), String> {
    let mut conf = state.config_mgr.get_config();
    let exe_lower = exe_name.to_lowercase();
    if let Some(app) = conf.apps.iter_mut().find(|a| a.exe_name.to_lowercase() == exe_lower) {
        app.enabled = enabled;
    }
    state.config_mgr.update_config(conf)
}

#[tauri::command]
async fn sync_database(state: State<'_, AppState>) -> Result<usize, String> {
    let online_apps = database::fetch_online_database().await.unwrap_or_else(|_| {
        database::get_default_catalog()
    });

    let mut conf = state.config_mgr.get_config();
    let mut added_count = 0;

    for item in online_apps {
        let exe = item.exe_name.to_lowercase();
        if !conf.apps.iter().any(|a| a.exe_name.to_lowercase() == exe) {
            conf.apps.push(item);
            added_count += 1;
        }
    }

    state.config_mgr.update_config(conf)?;
    Ok(added_count)
}

#[tauri::command]
fn get_current_status() -> HdrStatePayload {
    let is_hdr = display::is_any_hdr_active();
    HdrStatePayload {
        is_hdr_active: is_hdr,
        current_app_name: None,
        current_exe: None,
        switched_by_app: false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let config_mgr = Arc::new(ConfigManager::new());

            // First time initialization: populate with default catalog if apps list is empty
            {
                let mut conf = config_mgr.get_config();
                if conf.apps.is_empty() {
                    conf.apps = database::get_default_catalog();
                    let _ = config_mgr.update_config(conf);
                }
            }

            let monitor_service = MonitorService::new(config_mgr.clone(), app.handle().clone());
            monitor_service.start_hook();

            // Setup System Tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Tray setup error: {}", e);
            }

            app.manage(AppState {
                config_mgr,
                monitor_service,
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app from quitting on 'X', hide to tray instead
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_monitors,
            set_monitor_hdr,
            toggle_all_hdr,
            get_config,
            save_config,
            get_running_processes,
            add_custom_app,
            remove_app,
            toggle_app,
            sync_database,
            get_current_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

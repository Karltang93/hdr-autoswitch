mod config;
mod config_storage;
mod config_v2;
mod database;
mod display;
mod display_v2;
mod hdr_controller;
mod legacy_upgrade;
mod monitor_hook;
mod process;
mod scanner;
mod tray;

use config::{AppConfig, ConfigManager, HdrApp};
use database::CatalogEntry;
use display::MonitorInfo;
use monitor_hook::{HdrStatePayload, MonitorService};
use process::RunningProcessInfo;
use std::sync::Arc;
use tauri::{Emitter, Manager, State, WindowEvent};

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
fn get_catalog() -> Vec<CatalogEntry> {
    database::get_full_catalog()
}

#[tauri::command]
fn scan_installed_games() -> Vec<HdrApp> {
    scanner::scan_installed_games()
}

#[tauri::command]
fn pick_game_exe() -> Result<Option<scanner::PickedGameInfo>, String> {
    scanner::pick_game_exe_dialog()
}

#[tauri::command]
fn inspect_exe_path(path: String) -> Result<scanner::PickedGameInfo, String> {
    scanner::inspect_exe_path(&path)
}

#[tauri::command]
fn import_detected_games(state: State<'_, AppState>, detected: Vec<HdrApp>) -> Result<usize, String> {
    let mut conf = state.config_mgr.get_config();
    let mut count = 0;

    for item in detected {
        let name_lower = item.name.to_lowercase();
        let exe_lower = item.exe_name.to_lowercase();

        if let Some(existing) = conf.apps.iter_mut().find(|a| {
            a.name.to_lowercase() == name_lower
                || a.exe_name.to_lowercase() == exe_lower
                || a.alternate_exes.iter().any(|alt| alt.to_lowercase() == exe_lower)
        }) {
            let mut updated = false;

            for alt in item.alternate_exes {
                let alt_lower = alt.to_lowercase();
                if !existing.alternate_exes.iter().any(|x| x.to_lowercase() == alt_lower)
                    && existing.exe_name.to_lowercase() != alt_lower
                {
                    existing.alternate_exes.push(alt);
                    updated = true;
                }
            }

            // Always update path if item has a valid path and it differs from existing
            if let Some(new_p) = item.path {
                let path_changed = match &existing.path {
                    Some(old_p) => old_p.to_lowercase() != new_p.to_lowercase(),
                    None => true,
                };
                if path_changed {
                    existing.path = Some(new_p);
                    updated = true;
                }
            }

            if item.launcher.is_some() && existing.launcher != item.launcher {
                existing.launcher = item.launcher;
                updated = true;
            }

            if item.steam_id.is_some() && existing.steam_id != item.steam_id {
                existing.steam_id = item.steam_id;
                updated = true;
            }

            if !existing.enabled {
                existing.enabled = true;
                updated = true;
            }

            if updated {
                count += 1;
            }
        } else {
            conf.apps.push(item);
            count += 1;
        }
    }

    state.config_mgr.update_config(conf)?;
    Ok(count)
}

#[tauri::command]
fn verify_game_paths(paths: Vec<String>) -> std::collections::HashMap<String, bool> {
    let mut map = std::collections::HashMap::new();
    for p in paths {
        let exists = std::path::Path::new(&p).exists();
        map.insert(p, exists);
    }
    map
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
async fn sync_database(_state: State<'_, AppState>) -> Result<usize, String> {
    let online_entries = database::fetch_online_database().await.unwrap_or_else(|_| {
        database::get_full_catalog()
    });

    Ok(online_entries.len())
}

#[tauri::command]
fn get_current_status() -> HdrStatePayload {
    let is_hdr = display::is_any_hdr_active();
    HdrStatePayload {
        is_hdr_active: is_hdr,
        current_app_name: None,
        current_exe: None,
        switched_by_app: false,
        steam_id: None,
        launcher: None,
        hdr_type: None,
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

            // Automatic cleanup & deduplication of existing apps in config:
            {
                let mut conf = config_mgr.get_config();
                let mut deduplicated: Vec<HdrApp> = Vec::new();
                for app in conf.apps {
                    let name_lower = app.name.to_lowercase();
                    let exe_lower = app.exe_name.to_lowercase();

                    if let Some(existing) = deduplicated.iter_mut().find(|a| {
                        a.name.to_lowercase() == name_lower
                            || a.exe_name.to_lowercase() == exe_lower
                            || a.alternate_exes.contains(&exe_lower)
                    }) {
                        if !existing.alternate_exes.contains(&exe_lower) && existing.exe_name.to_lowercase() != exe_lower {
                            existing.alternate_exes.push(exe_lower);
                        }
                        for alt in app.alternate_exes {
                            if !existing.alternate_exes.contains(&alt) && existing.exe_name.to_lowercase() != alt {
                                existing.alternate_exes.push(alt);
                            }
                        }
                    } else {
                        deduplicated.push(app);
                    }
                }
                conf.apps = deduplicated;
                let _ = config_mgr.update_config(conf);
            }

            let monitor_service = MonitorService::new(config_mgr.clone(), app.handle().clone());
            monitor_service.start_hook();

            // Setup System Tray
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("Tray setup error: {}", e);
            }

            // Ensure custom diamond logo icon for main window and start minimized if requested
            if let Some(window) = app.get_webview_window("main") {
                let icon = tauri::include_image!("icons/128x128.png");
                let _ = window.set_icon(icon);

                let is_minimized_arg = std::env::args().any(|a| a == "--minimized");
                if is_minimized_arg || config_mgr.get_config().start_minimized {
                    let _ = window.hide();
                }
            }

            // Background periodic sync & auto-scan (Set & Forget background automation)
            let config_mgr_bg = config_mgr.clone();
            let app_handle_bg = app.handle().clone();
            std::thread::spawn(move || {
                // Sleep 15 seconds after launch to avoid competing with boot or game startup
                std::thread::sleep(std::time::Duration::from_secs(15));

                let conf = config_mgr_bg.get_config();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let should_sync = conf.auto_sync_database && match conf.last_sync_timestamp {
                    Some(last) => now.saturating_sub(last) > 7 * 86400, // 7 days
                    None => true,
                };

                if should_sync {
                    tauri::async_runtime::spawn(async move {
                        let _ = database::fetch_online_database().await;
                    });
                    let mut updated_conf = config_mgr_bg.get_config();
                    updated_conf.last_sync_timestamp = Some(now);
                    let _ = config_mgr_bg.update_config(updated_conf);
                }

                // Also run a quiet background scan for installed games to enrich steam IDs & alternate exes
                let detected = scanner::scan_installed_games();
                if !detected.is_empty() {
                    let mut conf = config_mgr_bg.get_config();
                    let mut modified = false;
                    for item in detected {
                        let name_lower = item.name.to_lowercase();
                        let exe_lower = item.exe_name.to_lowercase();
                        if let Some(existing) = conf.apps.iter_mut().find(|a| {
                            a.name.to_lowercase() == name_lower
                                || a.exe_name.to_lowercase() == exe_lower
                                || a.alternate_exes.contains(&exe_lower)
                        }) {
                            for alt in item.alternate_exes {
                                if !existing.alternate_exes.contains(&alt) && existing.exe_name.to_lowercase() != alt {
                                    existing.alternate_exes.push(alt);
                                    modified = true;
                                }
                            }
                            if existing.steam_id.is_none() && item.steam_id.is_some() {
                                existing.steam_id = item.steam_id;
                                modified = true;
                            }
                            if existing.launcher.is_none() && item.launcher.is_some() {
                                existing.launcher = item.launcher;
                                modified = true;
                            }
                        }
                    }
                    if modified {
                        let _ = config_mgr_bg.update_config(conf);
                        let _ = app_handle_bg.emit("apps-updated", ());
                    }
                }
            });

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
            get_catalog,
            scan_installed_games,
            import_detected_games,
            get_running_processes,
            add_custom_app,
            remove_app,
            toggle_app,
            sync_database,
            get_current_status,
            pick_game_exe,
            inspect_exe_path,
            verify_game_paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

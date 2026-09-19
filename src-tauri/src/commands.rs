use crate::config::{ConfigMode, ConfigSnapshot, HdrApp, SettingsPatch, TargetMonitor};
use crate::database::CatalogEntry;
use crate::display::MonitorInfo;
use crate::monitor_hook::{HdrStatePayload, ManualSetResult};
use crate::process::RunningProcessInfo;
use crate::{database, display, emit_config, legacy_upgrade, library, scanner, AppState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct MonitorView {
    #[serde(flatten)]
    monitor: MonitorInfo,
    is_selected: bool,
}

#[derive(Serialize)]
pub struct ScanResult {
    context_token: String,
    library_generation: String,
    games: Vec<HdrApp>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    Cs,
    En,
}

fn publish(
    app: &AppHandle,
    state: &AppState,
    result: Result<ConfigSnapshot, String>,
) -> Result<ConfigSnapshot, String> {
    let snapshot = match &result {
        Ok(snapshot) => Some(snapshot.clone()),
        Err(error) => {
            eprintln!("Configuration command failed: {error}");
            match state.config_mgr.snapshot() {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    eprintln!("Cannot publish configuration failure state: {error}");
                    None
                }
            }
        }
    };
    if let Some(snapshot) = snapshot {
        emit_config(app, &snapshot);
    }
    state.monitor_service.config_committed();
    result
}

fn require_origin(state: &AppState, expected: &str) -> Result<ConfigSnapshot, String> {
    state.ensure_admission()?;
    let snapshot = state.config_mgr.snapshot()?;
    if snapshot.context_token != expected {
        return Err("Settings history changed. Start this action again.".into());
    }
    Ok(snapshot)
}

fn require_context(state: &AppState, expected: &str) -> Result<ConfigSnapshot, String> {
    let snapshot = require_origin(state, expected)?;
    if snapshot.mode != ConfigMode::Ready {
        return Err("Settings are read-only. Resolve configuration recovery first.".into());
    }
    Ok(snapshot)
}

fn check_predecessor(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if let Err(error) = legacy_upgrade::check_predecessor() {
        let snapshot = state.config_mgr.set_controller_issue(Some(error.clone()))?;
        emit_config(app, &snapshot);
        state.monitor_service.config_committed();
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn get_monitors(state: State<'_, AppState>) -> Result<Vec<MonitorView>, String> {
    let monitors = display::get_monitors()?;
    let target = state.config_mgr.snapshot()?.settings.target_monitor;
    Ok(monitors
        .into_iter()
        .map(|monitor| {
            let is_selected = display::monitor_is_selected(&monitor, &target);
            MonitorView {
                monitor,
                is_selected,
            }
        })
        .collect())
}

#[tauri::command]
pub fn set_hdr(
    state: State<'_, AppState>,
    scope: TargetMonitor,
    enable: bool,
) -> Result<ManualSetResult, String> {
    state.ensure_admission()?;
    state.monitor_service.manual_set(scope, enable)
}

#[tauri::command]
pub fn get_current_status(state: State<'_, AppState>) -> Result<HdrStatePayload, String> {
    state.monitor_service.refresh()?;
    state.monitor_service.status()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<ConfigSnapshot, String> {
    state.config_mgr.snapshot()
}

#[tauri::command]
pub fn patch_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    patch: SettingsPatch,
) -> Result<ConfigSnapshot, String> {
    let _action = state
        .config_actions
        .lock()
        .map_err(|_| "Settings action lock is poisoned")?;
    require_context(&state, &expected_context)?;
    let result = match patch.autostart {
        Some(enabled) => {
            check_predecessor(&app, &state)?;
            legacy_upgrade::configure_autostart_with_commit(enabled, || {
                state.config_mgr.patch(&expected_context, patch)
            })
        }
        None => state.config_mgr.patch(&expected_context, patch),
    };
    publish(&app, &state, result)
}

fn change_history(
    app: &AppHandle,
    state: &AppState,
    expected_context: &str,
    change: impl FnOnce() -> Result<ConfigSnapshot, String>,
) -> Result<ConfigSnapshot, String> {
    let _action = state
        .config_actions
        .lock()
        .map_err(|_| "Settings action lock is poisoned")?;
    require_origin(state, expected_context)?;
    check_predecessor(app, state)?;
    let result = change();
    let reconciliation = crate::reconcile_controller(&state.config_mgr);
    let result = match result {
        Ok(_) => reconciliation,
        Err(error) => {
            if let Err(reconcile_error) = reconciliation {
                eprintln!(
                    "Controller reconciliation after failed history change: {reconcile_error}"
                );
            }
            Err(error)
        }
    };
    publish(app, state, result)
}

#[tauri::command]
pub fn initialize_config(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
) -> Result<ConfigSnapshot, String> {
    change_history(&app, &state, &expected_context, || {
        state.config_mgr.initialize(&expected_context)
    })
}

#[tauri::command]
pub fn import_legacy_config(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
) -> Result<ConfigSnapshot, String> {
    change_history(&app, &state, &expected_context, || {
        state.config_mgr.import_legacy(&expected_context)
    })
}

#[tauri::command]
pub fn restore_config(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    candidate_id: String,
) -> Result<ConfigSnapshot, String> {
    change_history(&app, &state, &expected_context, || {
        state.config_mgr.restore(&expected_context, &candidate_id)
    })
}

#[tauri::command]
pub fn reset_config(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
) -> Result<ConfigSnapshot, String> {
    change_history(&app, &state, &expected_context, || {
        state.config_mgr.reset(&expected_context)
    })
}

#[tauri::command]
pub fn recheck_controller(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
) -> Result<ConfigSnapshot, String> {
    let _action = state
        .config_actions
        .lock()
        .map_err(|_| "Settings action lock is poisoned")?;
    require_context(&state, &expected_context)?;
    publish(
        &app,
        &state,
        crate::reconcile_controller(&state.config_mgr),
    )
}

#[tauri::command]
pub fn get_catalog() -> Vec<CatalogEntry> {
    database::get_full_catalog()
}

#[tauri::command]
pub async fn sync_database() -> Result<usize, String> {
    Ok(database::fetch_online_database().await?.len())
}

#[tauri::command]
pub async fn scan_installed_games(
    state: State<'_, AppState>,
    expected_context: String,
    expected_library_generation: String,
) -> Result<ScanResult, String> {
    let origin = require_context(&state, &expected_context)?;
    if origin.library_generation != expected_library_generation {
        return Err("The library changed before scanning. Start the scan again.".into());
    }
    let games = tauri::async_runtime::spawn_blocking(scanner::scan_installed_games)
        .await
        .map_err(|error| format!("Game scan failed: {error}"))?;
    Ok(ScanResult {
        context_token: origin.context_token,
        library_generation: origin.library_generation,
        games,
    })
}

#[tauri::command]
pub fn import_detected_games(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    expected_library_generation: String,
    detected: Vec<HdrApp>,
) -> Result<ConfigSnapshot, String> {
    state.ensure_admission()?;
    let result = state.config_mgr.mutate(
        &expected_context,
        Some(&expected_library_generation),
        true,
        |settings| library::import_games(settings, detected),
    );
    publish(&app, &state, result)
}

#[tauri::command]
pub fn add_custom_app(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    app: HdrApp,
) -> Result<ConfigSnapshot, String> {
    state.ensure_admission()?;
    let result = state
        .config_mgr
        .mutate(&expected_context, None, true, |settings| {
            library::add_app(settings, app)
        });
    publish(&app_handle, &state, result)
}

#[tauri::command]
pub fn remove_app(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    exe_name: String,
) -> Result<ConfigSnapshot, String> {
    state.ensure_admission()?;
    let result = state
        .config_mgr
        .mutate(&expected_context, None, true, |settings| {
            let before = settings.apps.len();
            settings
                .apps
                .retain(|entry| !entry.exe_name.eq_ignore_ascii_case(&exe_name));
            if settings.apps.len() == before {
                return Err(
                    "This game is no longer in the library. Refresh before editing it.".into(),
                );
            }
            Ok(())
        });
    publish(&app, &state, result)
}

#[tauri::command]
pub fn toggle_app(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_context: String,
    exe_name: String,
    enabled: bool,
) -> Result<ConfigSnapshot, String> {
    state.ensure_admission()?;
    let result = state
        .config_mgr
        .mutate(&expected_context, None, true, |settings| {
            let entry = settings
                .apps
                .iter_mut()
                .find(|entry| entry.exe_name.eq_ignore_ascii_case(&exe_name))
                .ok_or("This game is no longer in the library. Refresh before editing it.")?;
            entry.enabled = enabled;
            Ok(())
        });
    publish(&app, &state, result)
}

#[tauri::command]
pub fn get_running_processes() -> Vec<RunningProcessInfo> {
    crate::process::get_running_processes()
}

#[tauri::command]
pub fn pick_game_exe(language: UiLanguage) -> Result<Option<scanner::PickedGameInfo>, String> {
    scanner::pick_game_exe_dialog(matches!(language, UiLanguage::Cs))
}

#[tauri::command]
pub fn set_ui_language(app: AppHandle, language: UiLanguage) -> Result<(), String> {
    crate::tray::set_language(&app, matches!(language, UiLanguage::Cs))
}

#[tauri::command]
pub fn inspect_exe_path(path: String) -> Result<scanner::PickedGameInfo, String> {
    scanner::inspect_exe_path(&path)
}

#[tauri::command]
pub fn verify_game_paths(paths: Vec<String>) -> std::collections::HashMap<String, bool> {
    paths
        .into_iter()
        .map(|path| {
            let exists = std::path::Path::new(&path).exists();
            (path, exists)
        })
        .collect()
}

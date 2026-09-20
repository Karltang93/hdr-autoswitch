use crate::{
    config::TargetMonitor,
    display::{ScopeHdrState, TargetStatus},
    monitor_hook::{HdrStatePayload, ManualControl, ManualSetResult},
    show_main_window, AppState,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

struct TrayLabels {
    items: [MenuItem<tauri::Wry>; 5],
    status_item: MenuItem<tauri::Wry>,
    czech: AtomicBool,
    attention_badge: AtomicBool,
    latest_status: Mutex<Option<HdrStatePayload>>,
}

fn labels(czech: bool) -> [&'static str; 5] {
    if czech {
        [
            "Ručně zapnout HDR na VŠECH monitorech",
            "Ručně vypnout HDR na VŠECH monitorech",
            "Otevřít okno",
            "Ukončit",
            "Nastavení a obnova",
        ]
    } else {
        [
            "Manually turn HDR ON for ALL displays",
            "Manually turn HDR OFF for ALL displays",
            "Open window",
            "Quit",
            "Settings and recovery",
        ]
    }
}

fn status_label(czech: bool, status: Option<&HdrStatePayload>) -> &'static str {
    let Some(status) = status else {
        return if czech {
            "Ovládání HDR se spouští"
        } else {
            "HDR controller starting"
        };
    };
    let unavailable_next_target = status.target_deferred
        && status.active_target.is_some()
        && status.uncertain_targets.is_empty()
        && matches!(
            status.target_status,
            TargetStatus::Disconnected
                | TargetStatus::NotHdrCapable
                | TargetStatus::NeedsConfirmation
                | TargetStatus::IdentityUnavailable
                | TargetStatus::Ambiguous
                | TargetStatus::StateUnavailable
        );
    if status.inventory_stale
        || (status.target_status != TargetStatus::Ready && !unavailable_next_target)
        || status.scope_hdr_state == ScopeHdrState::Unknown
    {
        if manual_enabled(Some(status)) {
            return if czech {
                "Automatické HDR pozastaveno / stav neznámý - ruční ovládání dostupné"
            } else {
                "Automatic HDR paused / state unknown - manual controls available"
            };
        }
        return if czech {
            "HDR pozastaveno / neznámý stav - otevřete nastavení"
        } else {
            "HDR paused / state unknown - open settings"
        };
    }
    if unavailable_next_target {
        return match status.scope_hdr_state {
            ScopeHdrState::Hdr => {
                if czech {
                    "Aktuální relace: HDR; příští cíl není dostupný"
                } else {
                    "Current session: HDR; next target unavailable"
                }
            }
            ScopeHdrState::Sdr => {
                if czech {
                    "Aktuální relace: SDR; příští cíl není dostupný"
                } else {
                    "Current session: SDR; next target unavailable"
                }
            }
            ScopeHdrState::Mixed => {
                if czech {
                    "Aktuální relace: HDR / SDR; příští cíl není dostupný"
                } else {
                    "Current session: mixed HDR / SDR; next target unavailable"
                }
            }
            ScopeHdrState::Unknown => unreachable!(),
        };
    }
    if status.warning.is_some() && !status.target_deferred {
        return if czech {
            "Upozornění HDR - otevřete nastavení"
        } else {
            "HDR warning - open settings"
        };
    }
    match status.scope_hdr_state {
        ScopeHdrState::Hdr => {
            if czech {
                "Vybrané displeje: HDR"
            } else {
                "Selected displays: HDR"
            }
        }
        ScopeHdrState::Sdr => {
            if czech {
                "Vybrané displeje: SDR"
            } else {
                "Selected displays: SDR"
            }
        }
        ScopeHdrState::Mixed => {
            if czech {
                "Smíšený stav HDR / SDR"
            } else {
                "Mixed HDR / SDR"
            }
        }
        ScopeHdrState::Unknown => unreachable!(),
    }
}

fn status_icon(attention: bool) -> tauri::image::Image<'static> {
    let icon = tauri::include_image!("icons/128x128.png");
    if !attention {
        return icon;
    }
    let (width, height) = (icon.width(), icon.height());
    let mut rgba = icon.rgba().to_vec();
    let radius = i64::from(width.min(height) / 5);
    let center_x = i64::from(width) - radius - 3;
    let center_y = i64::from(height) - radius - 3;
    for y in 0..height {
        for x in 0..width {
            let distance = (i64::from(x) - center_x).pow(2) + (i64::from(y) - center_y).pow(2);
            if distance <= radius.pow(2) {
                let color = if distance > (radius - 3).pow(2) {
                    [15, 11, 11, 255]
                } else {
                    [255, 191, 62, 255]
                };
                let offset = (y as usize * width as usize + x as usize) * 4;
                rgba[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
    tauri::image::Image::new_owned(rgba, width, height)
}

fn render_status(app: &AppHandle, labels: &TrayLabels) -> Result<(), String> {
    let status = labels
        .latest_status
        .lock()
        .map_err(|_| "Tray status lock is poisoned.")?;
    let text = status_label(labels.czech.load(Ordering::Acquire), status.as_ref());
    for item in &labels.items[..2] {
        item.set_enabled(manual_enabled(status.as_ref()))
            .map_err(|error| format!("Cannot update manual HDR admission: {error}"))?;
    }
    labels
        .status_item
        .set_text(text)
        .map_err(|error| error.to_string())?;
    let tray = app.tray_by_id("main").ok_or("Tray icon is unavailable.")?;
    let attention = status.as_ref().is_none_or(|state| {
        state.inventory_stale
            || state.target_status != TargetStatus::Ready
            || state.warning.is_some()
            || state.scope_hdr_state == ScopeHdrState::Unknown
    });
    if attention != labels.attention_badge.load(Ordering::Acquire) {
        tray.set_icon(Some(status_icon(attention)))
            .map_err(|error| error.to_string())?;
        labels.attention_badge.store(attention, Ordering::Release);
    }
    tray.set_tooltip(Some(format!("HDR Auto-Switch: {text}")))
        .map_err(|error| error.to_string())
}

pub fn update_status(app: &AppHandle, status: &HdrStatePayload) {
    let handle = app.clone();
    let status = status.clone();
    // Native menu updates belong on the UI thread, never on the actor.
    if let Err(error) = app.run_on_main_thread(move || {
        if let Some(labels) = handle.try_state::<TrayLabels>() {
            match labels.latest_status.lock() {
                Ok(mut latest) => *latest = Some(status),
                Err(error) => {
                    eprintln!("Cannot retain tray controller status: {error}");
                    return;
                }
            }
            if let Err(error) = render_status(&handle, &labels) {
                eprintln!("Cannot update tray controller status: {error}");
            }
        }
    }) {
        eprintln!("Cannot schedule tray controller status: {error}");
    }
}

pub fn set_language(app: &AppHandle, czech: bool) -> Result<(), String> {
    let state = app
        .try_state::<TrayLabels>()
        .ok_or("Tray menu is not initialized.")?;
    state.czech.store(czech, Ordering::Release);
    for (item, label) in state.items.iter().zip(labels(czech)) {
        item.set_text(label)
            .map_err(|error| format!("Cannot translate tray menu: {error}"))?;
    }
    render_status(app, &state)
}

fn report_control_error(app: &AppHandle, error: String) {
    eprintln!("Tray HDR request failed: {error}");
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        show_main_window(&handle);
        if let Err(error) = handle.emit("controller-error", Some(error)) {
            eprintln!("Cannot display tray HDR error: {error}");
        }
    }) {
        eprintln!("Cannot schedule tray HDR error: {error}");
    }
}

fn manual_enabled(status: Option<&HdrStatePayload>) -> bool {
    status.is_some_and(|status| {
        status.manual_control == ManualControl::Available && !status.inventory_stale
    })
}

fn report_manual_result(app: &AppHandle, result: Result<ManualSetResult, String>) {
    match result {
        Ok(result) => {
            let errors: Vec<String> = result.outcomes.iter()
                .filter(|outcome| !outcome.is_verified())
                .map(|outcome| outcome.message.clone()
                    .unwrap_or_else(|| format!("{:?}", outcome.outcome)))
                .collect();
            if result.outcomes.is_empty() {
                report_control_error(app, "No display result was returned.".into());
            } else if !errors.is_empty() {
                report_control_error(app, errors.join("; "));
            } else if let Err(error) = app.emit("controller-error", Option::<String>::None) {
                eprintln!("Cannot notify the UI of verified manual HDR: {error}");
            }
        }
        Err(error) => report_control_error(app, error),
    }
}

fn manual_all(app: &AppHandle, enable: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        report_control_error(app, "HDR control has not initialized.".into());
        return;
    };
    let request = state.ensure_admission()
        .and_then(|()| state.monitor_service.manual_set(TargetMonitor::All, enable));
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            report_control_error(app, error);
            return;
        }
    };
    // Enqueue on the callback thread, then wait elsewhere: click order is actor order.
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        report_manual_result(&handle, request.resolve().await);
    });
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let czech = matches!(unsafe { GetUserDefaultUILanguage() } & 0x3ff, 0x05 | 0x1b);
    let text = labels(czech);
    let on_item = MenuItemBuilder::with_id("hdr_on_all", text[0]).enabled(false).build(app)?;
    let off_item = MenuItemBuilder::with_id("hdr_off_all", text[1]).enabled(false).build(app)?;
    let show_item = MenuItemBuilder::with_id("show", text[2]).build(app)?;
    let exit_item = MenuItemBuilder::with_id("exit", text[3]).build(app)?;
    let settings_item = MenuItemBuilder::with_id("show_settings", text[4]).build(app)?;
    let status_item = MenuItemBuilder::with_id("controller_status", status_label(czech, None))
        .enabled(false)
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &status_item,
            &on_item,
            &off_item,
            &show_item,
            &settings_item,
            &exit_item,
        ])
        .build()?;
    TrayIconBuilder::with_id("main")
        .icon(tauri::include_image!("icons/128x128.png"))
        .tooltip("HDR Auto-Switch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "hdr_on_all" => manual_all(app, true),
            "hdr_off_all" => manual_all(app, false),
            "show" => show_main_window(app),
            "show_settings" => {
                show_main_window(app);
                if let Err(error) = app.emit("navigate-settings", ()) {
                    eprintln!("Cannot open configuration recovery: {error}");
                }
            }
            "exit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.shutdown();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    match window.is_visible() {
                        Ok(true) => {
                            if let Err(error) = window.hide() {
                                eprintln!("Cannot hide application window: {error}");
                            }
                        }
                        Ok(false) => show_main_window(app),
                        Err(error) => {
                            eprintln!("Cannot inspect application window visibility: {error}")
                        }
                    }
                }
            }
        })
        .build(app)?;
    app.manage(TrayLabels {
        items: [on_item, off_item, show_item, exit_item, settings_item],
        status_item,
        czech: AtomicBool::new(czech),
        attention_badge: AtomicBool::new(false),
        latest_status: Mutex::new(None),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_language_controls_every_tray_action_label() {
        let english = labels(false);
        assert!(english.iter().all(|label| label.is_ascii()));
        assert_eq!(english[2], "Open window");
        assert_ne!(labels(true), english);
    }

    fn status(scope: ScopeHdrState) -> HdrStatePayload {
        HdrStatePayload {
            is_hdr_active: scope == ScopeHdrState::Hdr,
            scope_hdr_state: scope,
            manual_control: ManualControl::Available,
            current_app_name: None,
            current_exe: None,
            switched_by_app: false,
            steam_id: None,
            launcher: None,
            hdr_type: None,
            warning: None,
            target_status: TargetStatus::Ready,
            active_target: None,
            target_deferred: false,
            any_hdr_active: scope != ScopeHdrState::Sdr,
            inventory_stale: false,
            uncertain_targets: Vec::new(),
            operation_outcomes: Vec::new(),
        }
    }

    #[test]
    fn mixed_and_paused_states_never_render_as_sdr() {
        let mut state = status(ScopeHdrState::Mixed);
        assert_eq!(status_label(false, Some(&state)), "Mixed HDR / SDR");
        state.target_status = TargetStatus::Disconnected;
        assert!(status_label(false, Some(&state)).contains("paused"));
        assert!(status_label(true, Some(&state)).contains("pozastaveno"));
        state.target_status = TargetStatus::OutcomeUnknown;
        assert!(status_label(false, Some(&state)).contains("unknown"));
    }

    #[test]
    fn manual_tray_actions_follow_shared_admission_not_automatic_readiness() {
        assert!(!manual_enabled(None));
        let mut state = status(ScopeHdrState::Unknown);
        for target in [
            TargetStatus::AutomationPaused, TargetStatus::NeedsConfirmation,
            TargetStatus::Disconnected,
        ] {
            state.target_status = target;
            assert!(manual_enabled(Some(&state)));
            assert!(status_label(false, Some(&state)).contains("manual controls available"));
            assert!(status_label(true, Some(&state)).contains("ruční ovládání dostupné"));
        }
        state.inventory_stale = true;
        assert!(!manual_enabled(Some(&state)));
        state.inventory_stale = false;
        for reason in ["controller conflict", "shutting down", "authority unavailable"] {
            state.manual_control = ManualControl::Blocked { reason: reason.into() };
            assert!(!manual_enabled(Some(&state)));
        }
        let payload = serde_json::to_value(&state).unwrap();
        assert_eq!(payload["manual_control"]["status"], "blocked");
    }

    #[test]
    fn automatic_failure_remains_visible_even_with_known_physical_state() {
        let mut state = status(ScopeHdrState::Sdr);
        state.warning = Some("A native request failed.".into());
        assert_eq!(
            status_label(false, Some(&state)),
            "HDR warning - open settings"
        );
    }

    #[test]
    fn attention_badge_changes_visible_tray_pixels_without_replacing_the_logo() {
        let normal = status_icon(false);
        let warning = status_icon(true);
        assert_eq!(
            (normal.width(), normal.height()),
            (warning.width(), warning.height())
        );
        assert_eq!(&normal.rgba()[..4], &warning.rgba()[..4]);
        let x = warning.width() - warning.width() / 5 - 3;
        let y = warning.height() - warning.width() / 5 - 3;
        let offset = (y as usize * warning.width() as usize + x as usize) * 4;
        assert_eq!(&warning.rgba()[offset..offset + 4], &[255, 191, 62, 255]);
        assert_ne!(normal.rgba(), warning.rgba());
    }

    #[test]
    fn tray_distinguishes_active_hdr_from_an_unavailable_next_target() {
        let mut state = status(ScopeHdrState::Hdr);
        state.target_status = TargetStatus::Disconnected;
        state.target_deferred = true;
        state.active_target = Some(TargetMonitor::All);
        assert_eq!(
            status_label(false, Some(&state)),
            "Current session: HDR; next target unavailable"
        );
        state.target_status = TargetStatus::ControllerConflict;
        assert!(status_label(false, Some(&state)).contains("paused"));
    }
}

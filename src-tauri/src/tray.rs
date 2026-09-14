use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_item = MenuItemBuilder::with_id("toggle_hdr", "Přepnout HDR").build(app)?;
    let show_item = MenuItemBuilder::with_id("show", "Otevřít okno").build(app)?;
    let exit_item = MenuItemBuilder::with_id("exit", "Ukončit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&toggle_item, &show_item, &exit_item])
        .build()?;

    let icon = tauri::include_image!("icons/128x128.png");

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("HDR Auto-Switch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle_hdr" => {
                let is_on = crate::display::is_any_hdr_active();
                let _ = crate::display::set_all_hdr(!is_on);
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "exit" => {
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
                    if let Ok(is_visible) = window.is_visible() {
                        if is_visible {
                            let _ = window.hide();
                        } else {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

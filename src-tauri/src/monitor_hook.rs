use crate::config::{ConfigManager, SwitchMethod};
use crate::display;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use windows::core::PWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    TranslateMessage, EVENT_SYSTEM_FOREGROUND, MSG,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

#[derive(Clone, serde::Serialize)]
pub struct HdrStatePayload {
    pub is_hdr_active: bool,
    pub current_app_name: Option<String>,
    pub current_exe: Option<String>,
    pub switched_by_app: bool,
}

pub struct MonitorService {
    config_mgr: Arc<ConfigManager>,
    app_handle: AppHandle,
    hdr_switched_by_us: Arc<AtomicBool>,
    current_active_exe: Arc<Mutex<Option<String>>>,
    debounce_sender: Arc<Mutex<Option<Sender<()>>>>,
}

static GLOBAL_SERVICE: std::sync::OnceLock<Arc<MonitorService>> = std::sync::OnceLock::new();

impl MonitorService {
    pub fn new(config_mgr: Arc<ConfigManager>, app_handle: AppHandle) -> Arc<Self> {
        let service = Arc::new(Self {
            config_mgr,
            app_handle,
            hdr_switched_by_us: Arc::new(AtomicBool::new(false)),
            current_active_exe: Arc::new(Mutex::new(None)),
            debounce_sender: Arc::new(Mutex::new(None)),
        });

        let _ = GLOBAL_SERVICE.set(service.clone());

        service
    }

    pub fn start_hook(self: &Arc<Self>) {
        let service = self.clone();
        thread::spawn(move || {
            unsafe {
                let hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                );

                if hook.is_invalid() {
                    eprintln!("Failed to install WinEventHook!");
                    return;
                }

                // Initial check of current foreground window
                service.handle_foreground_change(GetForegroundWindow());

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let _ = UnhookWinEvent(hook);
            }
        });
    }

    pub fn handle_foreground_change(&self, hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }

        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }

        if pid == 0 {
            return;
        }

        let exe_name = unsafe {
            if let Ok(h_process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut path_buf = [0u16; 1024];
                let mut size = path_buf.len() as u32;

                if QueryFullProcessImageNameW(
                    h_process,
                    PROCESS_NAME_FORMAT(0),
                    PWSTR(path_buf.as_mut_ptr()),
                    &mut size,
                )
                .is_ok()
                {
                    let full_path = String::from_utf16_lossy(&path_buf[..size as usize]);
                    Path::new(&full_path)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        };

        let exe_lower = match exe_name {
            Some(name) => name.to_lowercase(),
            None => return,
        };

        {
            let mut active = self.current_active_exe.lock().unwrap();
            *active = Some(exe_lower.clone());
        }

        let is_hdr = self.config_mgr.is_hdr_app(&exe_lower);
        let conf = self.config_mgr.get_config();

        if is_hdr {
            // Cancel any pending turn-off debounce timer
            if let Ok(mut sender_opt) = self.debounce_sender.lock() {
                if let Some(sender) = sender_opt.take() {
                    let _ = sender.send(()); // Signal cancel
                }
            }

            // If HDR is not currently active, turn it on!
            let already_active = display::is_any_hdr_active();
            if !already_active {
                self.turn_on_hdr(&conf, &exe_lower);
            } else {
                // If it was already active manually or earlier, emit current state
                let app_info = self.config_mgr.find_app(&exe_lower);
                let _ = self.app_handle.emit(
                    "hdr-status-changed",
                    HdrStatePayload {
                        is_hdr_active: true,
                        current_app_name: app_info.map(|a| a.name),
                        current_exe: Some(exe_lower.clone()),
                        switched_by_app: self.hdr_switched_by_us.load(Ordering::SeqCst),
                    },
                );
            }
        } else {
            // Non-HDR app (or desktop, browser, etc.)
            // If we previously switched HDR on, schedule turn-off with debounce
            if self.hdr_switched_by_us.load(Ordering::SeqCst) && display::is_any_hdr_active() {
                self.schedule_turn_off(&conf);
            } else {
                let _ = self.app_handle.emit(
                    "hdr-status-changed",
                    HdrStatePayload {
                        is_hdr_active: display::is_any_hdr_active(),
                        current_app_name: None,
                        current_exe: Some(exe_lower.clone()),
                        switched_by_app: false,
                    },
                );
            }
        }
    }

    fn turn_on_hdr(&self, conf: &crate::config::AppConfig, exe: &str) {
        match conf.switch_method {
            SwitchMethod::Native => {
                if conf.target_monitor == "all" {
                    display::set_all_hdr(true);
                } else {
                    let monitors = display::get_monitors();
                    if let Some(m) = monitors.iter().find(|m| m.id == conf.target_monitor) {
                        display::set_monitor_hdr(m.adapter_id_low, m.adapter_id_high, m.target_id, true);
                    } else {
                        display::set_all_hdr(true);
                    }
                }
            }
            SwitchMethod::Shortcut => {
                display::simulate_win_alt_b();
            }
        }

        self.hdr_switched_by_us.store(true, Ordering::SeqCst);

        let app_info = self.config_mgr.find_app(exe);
        let app_display_name = app_info.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| exe.to_string());

        if conf.notifications_enabled {
            let _ = self
                .app_handle
                .notification()
                .builder()
                .title("HDR Auto-Switch")
                .body(format!("HDR zapnuto pro {}", app_display_name))
                .show();
        }

        let _ = self.app_handle.emit(
            "hdr-status-changed",
            HdrStatePayload {
                is_hdr_active: true,
                current_app_name: Some(app_display_name),
                current_exe: Some(exe.to_string()),
                switched_by_app: true,
            },
        );
    }

    fn schedule_turn_off(&self, conf: &crate::config::AppConfig) {
        let (tx, rx) = channel();

        if let Ok(mut sender_opt) = self.debounce_sender.lock() {
            if let Some(old) = sender_opt.take() {
                let _ = old.send(()); // cancel existing timer
            }
            *sender_opt = Some(tx);
        }

        let delay = Duration::from_secs(conf.alt_tab_delay_seconds);
        let config_mgr = self.config_mgr.clone();
        let app_handle = self.app_handle.clone();
        let switched_flag = self.hdr_switched_by_us.clone();
        let current_exe_arc = self.current_active_exe.clone();
        let switch_method = conf.switch_method.clone();
        let target_monitor = conf.target_monitor.clone();
        let notif_enabled = conf.notifications_enabled;

        thread::spawn(move || {
            // Wait for delay or cancellation
            match rx.recv_timeout(delay) {
                Ok(_) => {
                    // Cancelled! User returned to an HDR app before timer fired
                }
                Err(_) => {
                    // Timer expired! Verify current active app is still not HDR
                    let still_non_hdr = {
                        let active = current_exe_arc.lock().unwrap();
                        match &*active {
                            Some(exe) => !config_mgr.is_hdr_app(exe),
                            None => true,
                        }
                    };

                    if still_non_hdr && switched_flag.load(Ordering::SeqCst) {
                        match switch_method {
                            SwitchMethod::Native => {
                                if target_monitor == "all" {
                                    display::set_all_hdr(false);
                                } else {
                                    let monitors = display::get_monitors();
                                    if let Some(m) = monitors.iter().find(|m| m.id == target_monitor) {
                                        display::set_monitor_hdr(m.adapter_id_low, m.adapter_id_high, m.target_id, false);
                                    } else {
                                        display::set_all_hdr(false);
                                    }
                                }
                            }
                            SwitchMethod::Shortcut => {
                                display::simulate_win_alt_b();
                            }
                        }

                        switched_flag.store(false, Ordering::SeqCst);

                        if notif_enabled {
                            let _ = app_handle
                                .notification()
                                .builder()
                                .title("HDR Auto-Switch")
                                .body("HDR vypnuto (návrat do SDR obsahu)")
                                .show();
                        }

                        let _ = app_handle.emit(
                            "hdr-status-changed",
                            HdrStatePayload {
                                is_hdr_active: false,
                                current_app_name: None,
                                current_exe: None,
                                switched_by_app: false,
                            },
                        );
                    }
                }
            }
        });
    }

    pub fn manual_toggle(&self, enable: bool) {
        let conf = self.config_mgr.get_config();
        match conf.switch_method {
            SwitchMethod::Native => {
                if conf.target_monitor == "all" {
                    display::set_all_hdr(enable);
                } else {
                    let monitors = display::get_monitors();
                    if let Some(m) = monitors.iter().find(|m| m.id == conf.target_monitor) {
                        display::set_monitor_hdr(m.adapter_id_low, m.adapter_id_high, m.target_id, enable);
                    } else {
                        display::set_all_hdr(enable);
                    }
                }
            }
            SwitchMethod::Shortcut => {
                display::simulate_win_alt_b();
            }
        }

        self.hdr_switched_by_us.store(enable, Ordering::SeqCst);

        let _ = self.app_handle.emit(
            "hdr-status-changed",
            HdrStatePayload {
                is_hdr_active: enable,
                current_app_name: None,
                current_exe: None,
                switched_by_app: false,
            },
        );
    }
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if let Some(service) = GLOBAL_SERVICE.get() {
        service.handle_foreground_change(hwnd);
    }
}

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
    GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
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
    pub steam_id: Option<String>,
    pub launcher: Option<String>,
    pub hdr_type: Option<String>,
}

pub struct MonitorService {
    config_mgr: Arc<ConfigManager>,
    app_handle: AppHandle,
    hdr_switched_by_us: Arc<AtomicBool>,
    current_active_exe: Arc<Mutex<Option<String>>>,
    debounce_sender: Arc<Mutex<Option<Sender<()>>>>,
    active_game_pid: Arc<Mutex<Option<u32>>>,
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
            active_game_pid: Arc::new(Mutex::new(None)),
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

        let mut is_hdr = self.config_mgr.is_hdr_app(&exe_lower);
        let conf = self.config_mgr.get_config();

        // Automatic enrollment of newly launched HDR games from catalog:
        if !is_hdr && conf.auto_detect_new_games && !conf.blacklist.iter().any(|b| b.to_lowercase() == exe_lower) {
            if let Some(entry) = crate::database::find_in_catalog(&exe_lower) {
                let new_app = crate::config::HdrApp {
                    name: entry.name.clone(),
                    exe_name: exe_lower.clone(),
                    enabled: true,
                    hdr_type: entry.hdr_type.clone(),
                    path: None,
                    alternate_exes: Vec::new(),
                    steam_id: None,
                    launcher: None,
                };
                let _ = self.config_mgr.add_app(new_app);
                let _ = self.app_handle.emit("apps-updated", ());
                is_hdr = true;
            }
        }

        if is_hdr {
            // Cancel any pending turn-off debounce timer
            if let Ok(mut sender_opt) = self.debounce_sender.lock() {
                if let Some(sender) = sender_opt.take() {
                    let _ = sender.send(()); // Signal cancel
                }
            }

            let already_active = display::is_any_hdr_active();
            if !already_active {
                self.turn_on_hdr(&conf, &exe_lower, pid);
            } else {
                // If it was already active (e.g. user Alt+Tabbed back into the game)
                self.hdr_switched_by_us.store(true, Ordering::SeqCst);
                if pid != 0 {
                    *self.active_game_pid.lock().unwrap() = Some(pid);
                }

                let app_info = self.config_mgr.find_app(&exe_lower);
                let app_display_name = app_info.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| exe_lower.clone());
                let steam_id = app_info.as_ref().and_then(|a| a.steam_id.clone());
                let launcher = app_info.as_ref().and_then(|a| a.launcher.clone());
                let hdr_type = app_info.as_ref().map(|a| a.hdr_type.as_str().to_string());

                let _ = self.app_handle.emit(
                    "hdr-status-changed",
                    HdrStatePayload {
                        is_hdr_active: true,
                        current_app_name: Some(app_display_name),
                        current_exe: Some(exe_lower.clone()),
                        switched_by_app: true,
                        steam_id,
                        launcher,
                        hdr_type,
                    },
                );
            }
        } else {
            // Non-HDR app (or desktop, browser, etc.)
            if self.hdr_switched_by_us.load(Ordering::SeqCst) && display::is_any_hdr_active() {
                // Check if the tracked game process is still alive:
                let is_game_alive = {
                    let pid_opt = *self.active_game_pid.lock().unwrap();
                    if let Some(game_pid) = pid_opt {
                        unsafe {
                            if let Ok(h_proc) = OpenProcess(
                                PROCESS_QUERY_LIMITED_INFORMATION,
                                false,
                                game_pid,
                            ) {
                                let mut exit_code = 0u32;
                                if GetExitCodeProcess(h_proc, &mut exit_code).is_ok() {
                                    let _ = windows::Win32::Foundation::CloseHandle(h_proc);
                                    exit_code == 259 // STILL_ACTIVE
                                } else {
                                    let _ = windows::Win32::Foundation::CloseHandle(h_proc);
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    } else {
                        false
                    }
                };

                if !is_game_alive {
                    // Game process has closed/terminated -> turn off HDR immediately! Zero debounce delay!
                    self.turn_off_hdr_now(&conf);
                } else if !conf.exit_only_hdr {
                    // Game is still alive, and user configured to switch back to SDR on Alt+Tab:
                    self.schedule_turn_off(&conf);
                }
                // If conf.exit_only_hdr is true and game is still alive: keep HDR active (zero flicker on Alt+Tab)!
            } else {
                let _ = self.app_handle.emit(
                    "hdr-status-changed",
                    HdrStatePayload {
                        is_hdr_active: display::is_any_hdr_active(),
                        current_app_name: None,
                        current_exe: None,
                        switched_by_app: false,
                        steam_id: None,
                        launcher: None,
                        hdr_type: None,
                    },
                );
            }
        }
    }

    fn turn_on_hdr(&self, conf: &crate::config::AppConfig, exe: &str, pid: u32) {
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

        if pid != 0 {
            *self.active_game_pid.lock().unwrap() = Some(pid);

            // Spawn background process watcher to immediately revert to SDR when the game process terminates
            let conf_clone = conf.clone();
            thread::spawn(move || {
                unsafe {
                    if let Ok(h_proc) = OpenProcess(
                        PROCESS_SYNCHRONIZE,
                        false,
                        pid,
                    ) {
                        let _ = windows::Win32::System::Threading::WaitForSingleObject(
                            h_proc,
                            windows::Win32::System::Threading::INFINITE,
                        );
                        let _ = windows::Win32::Foundation::CloseHandle(h_proc);

                        // Game process terminated!
                        if let Some(service) = GLOBAL_SERVICE.get() {
                            if let Ok(guard) = service.active_game_pid.lock() {
                                if *guard == Some(pid) {
                                    drop(guard);
                                    service.turn_off_hdr_now(&conf_clone);
                                }
                            }
                        }
                    }
                }
            });
        }

        let app_info = self.config_mgr.find_app(exe);
        let app_display_name = app_info.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| exe.to_string());
        let steam_id = app_info.as_ref().and_then(|a| a.steam_id.clone());
        let launcher = app_info.as_ref().and_then(|a| a.launcher.clone());
        let hdr_type = app_info.as_ref().map(|a| a.hdr_type.as_str().to_string());

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
                steam_id,
                launcher,
                hdr_type,
            },
        );
    }

    pub fn turn_off_hdr_now(&self, conf: &crate::config::AppConfig) {
        // Cancel any pending debounce timer
        if let Ok(mut sender_opt) = self.debounce_sender.lock() {
            if let Some(sender) = sender_opt.take() {
                let _ = sender.send(());
            }
        }

        match conf.switch_method {
            SwitchMethod::Native => {
                if conf.target_monitor == "all" {
                    display::set_all_hdr(false);
                } else {
                    let monitors = display::get_monitors();
                    if let Some(m) = monitors.iter().find(|m| m.id == conf.target_monitor) {
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

        self.hdr_switched_by_us.store(false, Ordering::SeqCst);
        *self.active_game_pid.lock().unwrap() = None;

        if conf.notifications_enabled {
            let _ = self
                .app_handle
                .notification()
                .builder()
                .title("HDR Auto-Switch")
                .body("HDR vypnuto (návrat do SDR obsahu)")
                .show();
        }

        let _ = self.app_handle.emit(
            "hdr-status-changed",
            HdrStatePayload {
                is_hdr_active: display::is_any_hdr_active(),
                current_app_name: None,
                current_exe: None,
                switched_by_app: false,
                steam_id: None,
                launcher: None,
                hdr_type: None,
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
                        if let Some(service) = GLOBAL_SERVICE.get() {
                            *service.active_game_pid.lock().unwrap() = None;
                        }

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
                                steam_id: None,
                                launcher: None,
                                hdr_type: None,
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
                steam_id: None,
                launcher: None,
                hdr_type: None,
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

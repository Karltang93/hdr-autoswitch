use serde::{Deserialize, Serialize};
use std::mem;
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes,
    QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
    DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::LUID;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VK_B, VK_LWIN, VK_MENU,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub adapter_id_low: u32,
    pub adapter_id_high: i32,
    pub target_id: u32,
    pub is_hdr_supported: bool,
    pub is_hdr_enabled: bool,
    pub is_primary: bool,
}

pub fn get_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();

    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;

        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count).is_err() {
            return monitors;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
        .is_err()
        {
            return monitors;
        }

        for (idx, path) in paths.iter().take(path_count as usize).enumerate() {
            // Get target device name (friendly monitor name)
            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };

            let name = if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                let end = target_name
                    .monitorFriendlyDeviceName
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(target_name.monitorFriendlyDeviceName.len());
                String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName[..end])
            } else {
                format!("Monitor {}", idx + 1)
            };

            let friendly_name = if name.trim().is_empty() {
                format!("Monitor {}", idx + 1)
            } else {
                name
            };

            // Query Advanced Color (HDR) status
            let mut color_info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                    size: mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };

            let mut is_hdr_supported = false;
            let mut is_hdr_enabled = false;

            if DisplayConfigGetDeviceInfo(&mut color_info.header) == 0 {
                is_hdr_supported = color_info.Anonymous.value & 0x1 != 0; // advancedColorSupported
                is_hdr_enabled = color_info.Anonymous.value & 0x2 != 0;   // advancedColorEnabled
            }

            let monitor_id = format!(
                "{}_{}_{}",
                path.targetInfo.adapterId.LowPart,
                path.targetInfo.adapterId.HighPart,
                path.targetInfo.id
            );

            monitors.push(MonitorInfo {
                id: monitor_id,
                name: friendly_name,
                adapter_id_low: path.targetInfo.adapterId.LowPart,
                adapter_id_high: path.targetInfo.adapterId.HighPart,
                target_id: path.targetInfo.id,
                is_hdr_supported,
                is_hdr_enabled,
                is_primary: idx == 0,
            });
        }
    }

    monitors
}

pub fn set_monitor_hdr(adapter_low: u32, adapter_high: i32, target_id: u32, enable: bool) -> bool {
    unsafe {
        let adapter_id = LUID {
            LowPart: adapter_low,
            HighPart: adapter_high,
        };

        let mut set_color_state = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
                size: mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32,
                adapterId: adapter_id,
                id: target_id,
            },
            Anonymous: DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0 {
                value: if enable { 1 } else { 0 },
            },
        };

        let res = DisplayConfigSetDeviceInfo(&mut set_color_state.header);
        res == 0
    }
}

pub fn set_all_hdr(enable: bool) -> bool {
    let monitors = get_monitors();
    let mut any_changed = false;

    for m in &monitors {
        if m.is_hdr_supported {
            if m.is_hdr_enabled != enable {
                if set_monitor_hdr(m.adapter_id_low, m.adapter_id_high, m.target_id, enable) {
                    any_changed = true;
                }
            }
        }
    }

    any_changed
}

pub fn is_any_hdr_active() -> bool {
    let monitors = get_monitors();
    monitors.iter().any(|m| m.is_hdr_enabled)
}

pub fn simulate_win_alt_b() {
    unsafe {
        let mut inputs = [INPUT::default(); 6];

        let keys = [VK_LWIN, VK_MENU, VK_B];

        // Key down
        for (i, &key) in keys.iter().enumerate() {
            inputs[i] = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
        }

        // Key up in reverse order
        for (i, &key) in keys.iter().rev().enumerate() {
            inputs[3 + i] = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
        }

        SendInput(&inputs, mem::size_of::<INPUT>() as i32);
    }
}

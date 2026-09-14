use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_name: String,
    pub title: String,
    pub path: String,
}

struct EnumContext {
    seen_exes: HashSet<String>,
    processes: Vec<RunningProcessInfo>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumContext);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let length = GetWindowTextLengthW(hwnd);
    if length == 0 {
        return BOOL(1);
    }

    let mut title_buf = vec![0u16; (length + 1) as usize];
    let len = GetWindowTextW(hwnd, &mut title_buf);
    if len == 0 {
        return BOOL(1);
    }

    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
    let title_trim = title.trim();
    if title_trim.is_empty() || title_trim == "Program Manager" {
        return BOOL(1);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return BOOL(1);
    }

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
            let exe_name = Path::new(&full_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            let exe_lower = exe_name.to_lowercase();
            if !exe_lower.is_empty() && !ctx.seen_exes.contains(&exe_lower) {
                // Friendly name from exe (without .exe)
                let stem = Path::new(&exe_name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| exe_name.clone());

                ctx.seen_exes.insert(exe_lower.clone());
                ctx.processes.push(RunningProcessInfo {
                    pid,
                    name: stem,
                    exe_name: exe_lower,
                    title: title_trim.to_string(),
                    path: full_path,
                });
            }
        }
    }

    BOOL(1)
}

pub fn get_running_processes() -> Vec<RunningProcessInfo> {
    let mut ctx = EnumContext {
        seen_exes: HashSet::new(),
        processes: Vec::new(),
    };

    unsafe {
        let lparam = LPARAM(&mut ctx as *mut _ as isize);
        let _ = EnumWindows(Some(enum_windows_proc), lparam);
    }

    ctx.processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    ctx.processes
}

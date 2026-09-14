use crate::config::{HdrApp, HdrType};
use crate::database;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_steam_library_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();

    // Default primary steam path
    let default_steam = PathBuf::from(r"C:\Program Files (x86)\Steam");
    let vdf_path = default_steam.join(r"steamapps\libraryfolders.vdf");

    if vdf_path.exists() {
        folders.push(default_steam.join("steamapps\\common"));

        if let Ok(content) = fs::read_to_string(&vdf_path) {
            // Simple VDF parser for "path" "..."
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 4 {
                        let path_str = parts[3].replace(r"\\", r"\");
                        let p = PathBuf::from(path_str).join("steamapps\\common");
                        if p.exists() && !folders.contains(&p) {
                            folders.push(p);
                        }
                    }
                }
            }
        }
    }

    // Also check standard common game drives: D:\Games, E:\Games, etc.
    for drive in ["C", "D", "E", "F", "G"] {
        let steam_lib = PathBuf::from(format!(r"{}:\SteamLibrary\steamapps\common", drive));
        if steam_lib.exists() && !folders.contains(&steam_lib) {
            folders.push(steam_lib);
        }
    }

    folders
}

pub fn scan_installed_games() -> Vec<HdrApp> {
    let catalog = database::get_default_catalog();
    let library_folders = find_steam_library_folders();
    let mut detected = Vec::new();

    for lib_dir in library_folders {
        let entries = match fs::read_dir(&lib_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let game_dir = entry.path();
            if !game_dir.is_dir() {
                continue;
            }

            let folder_name = entry.file_name().to_string_lossy().to_string();

            // Find all .exe files inside the game directory (depth up to 3)
            let mut exe_files = Vec::new();
            collect_exes(&game_dir, 0, 3, &mut exe_files);

            for (exe_name, exe_path) in exe_files {
                let exe_lower = exe_name.to_lowercase();

                // Check if this exe matches anything in our HDR catalog
                if let Some(cat_item) = catalog.iter().find(|c| c.exe_name.to_lowercase() == exe_lower) {
                    if !detected.iter().any(|d: &HdrApp| d.exe_name.to_lowercase() == exe_lower) {
                        detected.push(HdrApp {
                            name: cat_item.name.clone(),
                            exe_name: exe_lower,
                            enabled: true,
                            hdr_type: cat_item.hdr_type.clone(),
                            path: Some(exe_path),
                        });
                    }
                } else {
                    // Check if folder name closely matches a catalog game name
                    let folder_lower = folder_name.to_lowercase().replace(' ', "");
                    if let Some(cat_item) = catalog.iter().find(|c| {
                        let cat_clean = c.name.to_lowercase().replace(' ', "");
                        folder_lower.contains(&cat_clean) || cat_clean.contains(&folder_lower)
                    }) {
                        // If it's the main exe (e.g. not crash_reporter, unity_crash, etc.)
                        if !exe_lower.contains("crash") && !exe_lower.contains("redlauncher") && !exe_lower.contains("unity") {
                            if !detected.iter().any(|d: &HdrApp| d.exe_name.to_lowercase() == exe_lower) {
                                detected.push(HdrApp {
                                    name: cat_item.name.clone(),
                                    exe_name: exe_lower,
                                    enabled: true,
                                    hdr_type: cat_item.hdr_type.clone(),
                                    path: Some(exe_path),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Also check standard media players (mpv, vlc)
    for (prog_name, exe_str, default_path) in [
        ("VLC Media Player", "vlc.exe", r"C:\Program Files\VideoLAN\VLC\vlc.exe"),
        ("MPC-HC", "mpc-hc64.exe", r"C:\Program Files\MPC-HC\mpc-hc64.exe"),
        ("mpv", "mpv.exe", r"C:\Program Files\mpv\mpv.exe"),
    ] {
        if Path::new(default_path).exists() {
            detected.push(HdrApp {
                name: prog_name.to_string(),
                exe_name: exe_str.to_string(),
                enabled: true,
                hdr_type: HdrType::Media,
                path: Some(default_path.to_string()),
            });
        }
    }

    detected
}

fn collect_exes(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<(String, String)>) {
    if depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            // skip common non-game subfolders
            if dir_name != "_CommonRedist" && dir_name != "Engine" && dir_name != "EasyAntiCheat" && dir_name != "BattlEye" {
                collect_exes(&path, depth + 1, max_depth, out);
            }
        } else if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("exe") {
                if let Some(file_name) = path.file_name() {
                    out.push((
                        file_name.to_string_lossy().to_string(),
                        path.to_string_lossy().to_string(),
                    ));
                }
            }
        }
    }
}

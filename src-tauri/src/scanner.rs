use crate::config::{HdrApp, HdrType};
use crate::database;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_all_game_library_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();

    // 1. Steam VDF parsing
    for steam_base in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Steam",
        r"D:\Steam",
        r"E:\Steam",
    ] {
        let vdf_path = PathBuf::from(steam_base).join(r"steamapps\libraryfolders.vdf");
        if vdf_path.exists() {
            let common = PathBuf::from(steam_base).join(r"steamapps\common");
            if common.exists() && !folders.contains(&common) {
                folders.push(common);
            }

            if let Ok(content) = fs::read_to_string(&vdf_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("\"path\"") {
                        let parts: Vec<&str> = trimmed.split('"').collect();
                        if parts.len() >= 4 {
                            let raw_path = parts[3].replace(r"\\", r"\");
                            let p = PathBuf::from(raw_path).join(r"steamapps\common");
                            if p.exists() && !folders.contains(&p) {
                                folders.push(p);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Scan all drive roots for common game directories
    for drive in ["C", "D", "E", "F", "G"] {
        for sub in [
            r"SteamLibrary\steamapps\common",
            r"SteamHry\steamapps\common",
            r"Steam\steamapps\common",
            r"Games",
            r"Hry",
            r"XboxGames",
            r"Epic Games",
        ] {
            let candidate = PathBuf::from(format!(r"{}:\{}", drive, sub));
            if candidate.exists() && !folders.contains(&candidate) {
                folders.push(candidate);
            }
        }
    }

    folders
}

pub fn scan_installed_games() -> Vec<HdrApp> {
    let catalog = database::get_full_catalog();
    let library_folders = find_all_game_library_folders();

    // Map by game name or folder to deduplicate (e.g. Bodycam.exe and Bodycam-Win64-Shipping.exe)
    let mut detected_map: HashMap<String, HdrApp> = HashMap::new();

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
            let folder_clean = folder_name.to_lowercase().replace([' ', '_', '-'], "");

            // Collect .exe files in game folder
            let mut exe_files = Vec::new();
            collect_exes(&game_dir, 0, 3, &mut exe_files);

            for (exe_name, exe_path) in exe_files {
                let exe_lower = exe_name.to_lowercase();

                // Skip non-game executables
                if exe_lower.contains("crash")
                    || exe_lower.contains("setup")
                    || exe_lower.contains("installer")
                    || exe_lower.contains("unity")
                    || exe_lower.contains("redlauncher")
                    || exe_lower.contains("launcher.exe")
                    || exe_lower.contains("protocol")
                    || exe_lower.contains("helper")
                    || exe_lower.contains("eula")
                    || exe_lower.contains("reporter")
                {
                    continue;
                }

                // 1. Direct match with catalog entry exe
                let mut matched_entry = catalog.iter().find(|c| c.exe_name.to_lowercase() == exe_lower);

                // 2. If no direct exe match, match by game folder name
                if matched_entry.is_none() {
                    matched_entry = catalog.iter().find(|c| {
                        let cat_clean = c.name.to_lowercase().replace([' ', '_', '-'], "");
                        folder_clean == cat_clean
                            || folder_clean.starts_with(&cat_clean)
                            || cat_clean.starts_with(&folder_clean)
                    });
                }

                if let Some(cat) = matched_entry {
                    if let Some(existing) = detected_map.get_mut(&cat.name) {
                        // Already found this game, add this exe to alternate_exes if not already present
                        if existing.exe_name.to_lowercase() != exe_lower
                            && !existing.alternate_exes.contains(&exe_lower)
                        {
                            existing.alternate_exes.push(exe_lower);
                        }
                    } else {
                        detected_map.insert(
                            cat.name.clone(),
                            HdrApp {
                                name: cat.name.clone(),
                                exe_name: exe_lower,
                                enabled: true,
                                hdr_type: cat.hdr_type.clone(),
                                path: Some(exe_path),
                                alternate_exes: Vec::new(),
                            },
                        );
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
            detected_map.insert(
                prog_name.to_string(),
                HdrApp {
                    name: prog_name.to_string(),
                    exe_name: exe_str.to_string(),
                    enabled: true,
                    hdr_type: HdrType::Media,
                    path: Some(default_path.to_string()),
                    alternate_exes: Vec::new(),
                },
            );
        }
    }

    let mut result: Vec<HdrApp> = detected_map.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
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
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // skip common non-game subfolders
            if dir_name != "_CommonRedist"
                && dir_name != "Engine"
                && dir_name != "EasyAntiCheat"
                && dir_name != "BattlEye"
                && dir_name != "CrashReportClient"
            {
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

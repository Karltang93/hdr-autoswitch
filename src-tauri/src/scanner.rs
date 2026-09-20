use crate::config::{HdrApp, HdrType};
use crate::database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickedGameInfo {
    pub name: String,
    pub exe_name: String,
    pub path: String,
    pub hdr_type: HdrType,
    pub is_hdr_supported: bool,
    pub notes: Option<String>,
    pub launcher: Option<String>,
}

pub fn scan_installed_games() -> Vec<HdrApp> {
    let catalog = database::get_full_catalog();
    let mut detected_map: HashMap<String, HdrApp> = HashMap::new();

    // 1. Scan Steam via appmanifest_*.acf (across all drives and libraryfolders)
    scan_steam_manifests(&catalog, &mut detected_map);

    // 2. Scan Epic Games Launcher via Manifests/*.item
    scan_epic_manifests(&catalog, &mut detected_map);

    // 3. Scan GOG Galaxy games registry
    scan_gog_registry(&catalog, &mut detected_map);

    // 4. Scan Windows Registry (EA App, Ubisoft Connect, standalone installers)
    scan_windows_registry(&catalog, &mut detected_map);

    // 5. Scan Xbox Games folders (C:\XboxGames, D:\XboxGames, etc.)
    scan_xbox_games(&catalog, &mut detected_map);

    // 6. Scan common media players
    scan_media_players(&mut detected_map);

    let mut result: Vec<HdrApp> = detected_map.into_values().collect();

    // Sort: HDR-enabled games first (alphabetically), then SDR games (alphabetically)
    result.sort_by(|a, b| {
        match (a.enabled, b.enabled) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    result
}

// --------------------------------------------------------------------------------------
// Native File Dialog & Path Inspection
// --------------------------------------------------------------------------------------

pub fn pick_game_exe_dialog(czech: bool) -> Result<Option<PickedGameInfo>, String> {
    let file = rfd::FileDialog::new()
        .add_filter(if czech { "Spustitelný soubor (*.exe)" } else { "Executable (*.exe)" }, &["exe"])
        .set_title(if czech { "Vybrat herní soubor (.exe)" } else { "Select Game Executable (.exe)" })
        .pick_file();

    match file {
        Some(path_buf) => {
            let path_str = path_buf.to_string_lossy().to_string();
            inspect_exe_path(&path_str).map(Some)
        }
        None => Ok(None),
    }
}

pub fn inspect_exe_path(path: &str) -> Result<PickedGameInfo, String> {
    let p = PathBuf::from(path);
    if !p.exists() || !p.is_file() {
        return Err("Selected file does not exist.".to_string());
    }

    let exe_name = match p.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return Err("Invalid file name.".to_string()),
    };

    if !exe_name.to_lowercase().ends_with(".exe") {
        return Err("Selected file is not an .exe.".to_string());
    }

    let catalog = database::get_full_catalog();

    // Determine smart game folder name by traversing up past "Win64", "Binaries", "x64", etc.
    let folder_name = p
        .parent()
        .and_then(|mut curr| {
            loop {
                let name = curr.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let lower = name.to_lowercase();
                if lower == "win64" || lower == "binaries" || lower == "bin" || lower == "x64" || lower == "shipping" {
                    if let Some(parent) = curr.parent() {
                        curr = parent;
                        continue;
                    }
                }
                break Some(name);
            }
        })
        .unwrap_or_default();

    // Check catalog match by exe name, alternate exes, or folder title
    let matched_cat = catalog.iter().find(|c| {
        c.exe_name.eq_ignore_ascii_case(&exe_name)
            || c.alternate_exes.iter().any(|a| a.eq_ignore_ascii_case(&exe_name))
            || (!folder_name.is_empty() && is_title_match(&c.name, &folder_name))
    });


    let (display_name, hdr_type, is_hdr, notes) = if let Some(cat) = matched_cat {
        (cat.name.clone(), cat.hdr_type.clone(), true, cat.notes.clone())
    } else {
        let clean_name = if !folder_name.is_empty() && !folder_name.eq_ignore_ascii_case("common") {
            clean_title_from_folder(&folder_name)
        } else {
            exe_name.trim_end_matches(".exe").trim_end_matches(".EXE").to_string()
        };
        (clean_name, HdrType::Native, false, None)
    };

    let path_lower = path.to_lowercase();
    let launcher = if path_lower.contains("steamapps") {
        Some("Steam".to_string())
    } else if path_lower.contains("epic games") || path_lower.contains(".egstore") {
        Some("Epic Games".to_string())
    } else if path_lower.contains("gog galaxy") || path_lower.contains("gog games") {
        Some("GOG".to_string())
    } else if path_lower.contains("xboxgames") {
        Some("Xbox".to_string())
    } else {
        None
    };

    Ok(PickedGameInfo {
        name: display_name,
        exe_name: exe_name.to_lowercase(),
        path: path.to_string(),
        hdr_type,
        is_hdr_supported: is_hdr,
        notes,
        launcher,
    })
}

// --------------------------------------------------------------------------------------
// 1. Steam scanner using appmanifest_*.acf
// --------------------------------------------------------------------------------------
fn scan_steam_manifests(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    let mut steam_roots = Vec::new();

    // Check registry keys for Steam
    let reg_checks = [
        (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
        (HKEY_CURRENT_USER, r"Software\Valve\Steam", "InstallPath"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
    ];

    for (root, subkey, val_name) in reg_checks {
        if let Ok(reg_val) = read_registry_string(root, subkey, val_name) {
            let normalized = reg_val.replace('/', "\\").trim_end_matches('\\').to_string();
            let p = PathBuf::from(normalized);
            if p.exists() && !steam_roots.contains(&p) {
                steam_roots.push(p);
            }
        }
    }

    // Check common drive roots for Steam
    for drive in ['C', 'D', 'E', 'F', 'G', 'H'] {
        for sub in [
            r"Program Files (x86)\Steam",
            r"Program Files\Steam",
            r"Steam",
            r"SteamLibrary",
            r"SteamHry",
            r"Games\Steam",
            r"Games\SteamLibrary",
        ] {
            let p = PathBuf::from(format!(r"{}:\{}", drive, sub));
            if p.exists() && !steam_roots.contains(&p) {
                steam_roots.push(p);
            }
        }
    }

    let mut library_paths = Vec::new();

    for steam_root in steam_roots {
        let direct_steamapps = steam_root.join("steamapps");
        if direct_steamapps.exists() && !library_paths.contains(&direct_steamapps) {
            library_paths.push(direct_steamapps);
        }

        let vdf = steam_root.join(r"steamapps\libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(&vdf) {
            for line in content.lines() {
                let trimmed = line.trim();
                // Find lines with "path" followed by library path
                if trimmed.contains("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    for (idx, &part) in parts.iter().enumerate() {
                        if part.eq_ignore_ascii_case("path") && idx + 2 < parts.len() {
                            let path_candidate = parts[idx + 2].replace(r"\\", r"\").replace('/', r"\");
                            let p = PathBuf::from(path_candidate).join("steamapps");
                            if p.exists() && !library_paths.contains(&p) {
                                library_paths.push(p);
                            }
                        }
                    }
                }
            }
        }
    }

    // Blacklisted Steam app IDs (Redistributables, runtimes, VR, dedicated servers)
    let ignored_app_ids = [
        "228980",  // Steamworks Common Redistributables
        "1070560", // Steam Linux Runtime
        "1391110", // Proton
        "1493710", // Proton Hotfix
        "221410",  // SteamVR
        "250820",  // SteamVR
    ];

    // Scan each library's appmanifest_*.acf
    for steamapps in library_paths {
        let entries = match fs::read_dir(&steamapps) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                let steam_appid = file_name
                    .trim_start_matches("appmanifest_")
                    .trim_end_matches(".acf")
                    .to_string();

                if ignored_app_ids.contains(&steam_appid.as_str()) {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&path) {
                    let mut game_name = String::new();
                    let mut install_dir_name = String::new();

                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("\"name\"") {
                            let parts: Vec<&str> = trimmed.split('"').collect();
                            if parts.len() >= 4 {
                                game_name = parts[3].replace(r#"™"#, "").replace(r#"®"#, "").trim().to_string();
                            }
                        } else if trimmed.starts_with("\"installdir\"") {
                            let parts: Vec<&str> = trimmed.split('"').collect();
                            if parts.len() >= 4 {
                                install_dir_name = parts[3].trim().to_string();
                            }
                        }
                    }

                    // Skip non-game Steam tools
                    let lower_name = game_name.to_lowercase();
                    if lower_name.contains("steamworks")
                        || lower_name.contains("proton")
                        || lower_name.contains("steam linux runtime")
                        || lower_name.contains("soundtrack")
                        || lower_name.contains("dedicated server")
                        || lower_name.contains("benchmark tool")
                    {
                        continue;
                    }

                    if !install_dir_name.is_empty() {
                        let full_game_dir = steamapps.join("common").join(&install_dir_name);
                        if full_game_dir.exists() {
                            let clean_game_name = if let Some(slash_idx) = game_name.find(" / ") {
                                game_name[..slash_idx].trim().to_string()
                            } else {
                                game_name.clone()
                            };

                            match_and_insert_game(
                                catalog,
                                &clean_game_name,
                                &full_game_dir,
                                None,
                                map,
                                Some("Steam"),
                                Some(&steam_appid),
                            );
                        }
                    }

                }
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// 2. Epic Games manifests scanner
// --------------------------------------------------------------------------------------
fn scan_epic_manifests(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    let mut manifest_dirs = Vec::new();
    if let Ok(prog_data) = std::env::var("ProgramData") {
        manifest_dirs.push(PathBuf::from(prog_data).join(r"Epic\EpicGamesLauncher\Data\Manifests"));
    }
    manifest_dirs.push(PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests"));

    for manifest_dir in manifest_dirs {
        if !manifest_dir.exists() {
            continue;
        }

        let entries = match fs::read_dir(&manifest_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "item").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Skip incomplete installations
                        if val["bIsIncompleteInstall"].as_bool().unwrap_or(false) {
                            continue;
                        }

                        let display_name = val["DisplayName"].as_str().unwrap_or_default();
                        let install_loc = val["InstallLocation"].as_str().unwrap_or_default();
                        let launch_exe = val["LaunchExecutable"].as_str().unwrap_or_default();

                        if display_name.is_empty() || install_loc.is_empty() {
                            continue;
                        }

                        let game_dir = PathBuf::from(install_loc);
                        if game_dir.exists() {
                            match_and_insert_game(
                                catalog,
                                display_name,
                                &game_dir,
                                if !launch_exe.is_empty() { Some(launch_exe) } else { None },
                                map,
                                Some("Epic Games"),
                                None,
                            );
                        }
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// 3. GOG Galaxy Registry Scanner
// --------------------------------------------------------------------------------------
fn scan_gog_registry(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    let gog_keys = [
        r"SOFTWARE\GOG.com\Games",
        r"SOFTWARE\WOW6432Node\GOG.com\Games",
    ];

    for key_path in gog_keys {
        let subkey_wide = to_wide(key_path);
        let mut h_key = HKEY::default();

        unsafe {
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(subkey_wide.as_ptr()), Some(0), KEY_READ, &mut h_key) != WIN32_ERROR(0) {
                continue;
            }

            let mut index = 0;
            let mut key_name_buf = [0u16; 256];

            loop {
                let mut key_name_len = key_name_buf.len() as u32;
                let status = RegEnumKeyExW(
                    h_key,
                    index,
                    Some(windows::core::PWSTR(key_name_buf.as_mut_ptr())),
                    &mut key_name_len,
                    None,
                    None,
                    None,
                    None,
                );

                if status != WIN32_ERROR(0) {
                    break;
                }

                index += 1;

                let child_id = String::from_utf16_lossy(&key_name_buf[..key_name_len as usize]);
                let child_path = format!(r"{}\{}", key_path, child_id);

                if let Ok(game_name) = read_registry_string(HKEY_LOCAL_MACHINE, &child_path, "gameName") {
                    if let Ok(path_str) = read_registry_string(HKEY_LOCAL_MACHINE, &child_path, "path") {
                        let game_dir = PathBuf::from(&path_str);
                        if game_dir.exists() && game_dir.is_dir() {
                            let launch_exe = read_registry_string(HKEY_LOCAL_MACHINE, &child_path, "exe").ok();
                            match_and_insert_game(
                                catalog,
                                &game_name,
                                &game_dir,
                                launch_exe.as_deref(),
                                map,
                                Some("GOG"),
                                None,
                            );
                        }
                    }
                }
            }

            let _ = RegCloseKey(h_key);
        }
    }
}

// --------------------------------------------------------------------------------------
// 4. Windows Registry Uninstall Keys (EA App, Ubisoft, Custom Installers)
// --------------------------------------------------------------------------------------
fn scan_windows_registry(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    let reg_paths = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];

    for (root, subkey) in reg_paths {
        scan_registry_uninstall_hive(root, subkey, catalog, map);
    }
}

fn scan_registry_uninstall_hive(
    root: HKEY,
    subkey: &str,
    catalog: &[database::CatalogEntry],
    map: &mut HashMap<String, HdrApp>,
) {
    let subkey_wide = to_wide(subkey);
    let mut h_key = HKEY::default();

    unsafe {
        if RegOpenKeyExW(root, PCWSTR(subkey_wide.as_ptr()), Some(0), KEY_READ, &mut h_key) != WIN32_ERROR(0) {
            return;
        }

        let mut index = 0;
        let mut key_name_buf = [0u16; 256];

        loop {
            let mut key_name_len = key_name_buf.len() as u32;
            let status = RegEnumKeyExW(
                h_key,
                index,
                Some(windows::core::PWSTR(key_name_buf.as_mut_ptr())),
                &mut key_name_len,
                None,
                None,
                None,
                None,
            );

            if status != WIN32_ERROR(0) {
                break;
            }

            index += 1;

            let child_name = String::from_utf16_lossy(&key_name_buf[..key_name_len as usize]);
            let child_path = format!(r"{}\{}", subkey, child_name);

            if let Ok(display_name) = read_registry_string(root, &child_path, "DisplayName") {
                let lower_name = display_name.to_lowercase();
                // Filter out non-game software and launcher clients
                if lower_name.contains("update")
                    || lower_name.contains("redistributable")
                    || lower_name.contains("driver")
                    || lower_name.contains("visual c++")
                    || lower_name.contains("directx")
                    || lower_name.contains("sdk")
                    || lower_name.contains("microsoft")
                    || lower_name.contains("windows")
                    || lower_name.contains("security")
                    || lower_name.contains("antivirus")
                    || lower_name.contains("vpn")
                    || lower_name.contains("launcher")
                    || lower_name.contains("game center")
                    || lower_name.contains("anti-cheat")
                    || lower_name.contains("service")
                    || lower_name == "steam"
                    || lower_name == "gog galaxy"
                    || lower_name == "ubisoft connect"
                    || lower_name == "epic games launcher"
                {
                    continue;
                }

                let mut direct_exe: Option<PathBuf> = None;
                if let Ok(icon) = read_registry_string(root, &child_path, "DisplayIcon") {
                    let raw = icon.split(',').next().unwrap_or(&icon).trim().trim_matches('"');
                    if raw.to_lowercase().ends_with(".exe") {
                        let icon_p = PathBuf::from(raw);
                        if icon_p.exists() && icon_p.is_file() {
                            direct_exe = Some(icon_p);
                        }
                    }
                }

                if let Ok(install_location) = read_registry_string(root, &child_path, "InstallLocation") {
                    let clean_dir = install_location.trim().trim_matches('"');
                    let p = PathBuf::from(clean_dir);
                    // NEVER scan entire Program Files or Drive root!
                    if p.exists() && p.is_dir() && p.parent().is_some() && p.components().count() >= 3 {
                        let path_lower = clean_dir.to_lowercase();
                        if !path_lower.ends_with(r"program files")
                            && !path_lower.ends_with(r"program files (x86)")
                            && !path_lower.ends_with(r"windows")
                        {
                            let is_cat = catalog.iter().any(|c| {
                                (!display_name.is_empty() && is_title_match(&c.name, &display_name))
                                    || direct_exe.as_ref().map(|e| e.file_name().unwrap_or_default().to_string_lossy().eq_ignore_ascii_case(&c.exe_name)).unwrap_or(false)
                            });

                            let is_game_path = path_lower.contains(r"\games")
                                || path_lower.contains(r"\hry")
                                || path_lower.contains(r"\steam")
                                || path_lower.contains(r"\epic")
                                || path_lower.contains(r"\ubisoft")
                                || path_lower.contains(r"\ea ")
                                || path_lower.contains(r"\electronic arts")
                                || path_lower.contains(r"\riot")
                                || path_lower.contains(r"\gog")
                                || path_lower.contains(r"\battle.net")
                                || path_lower.contains(r"\battlestate")
                                || path_lower.contains(r"\wargaming")
                                || path_lower.contains(r"\vintage story")
                                || path_lower.contains(r"\tarkov");

                            if is_cat || is_game_path {
                                let pref_exe = direct_exe.as_ref().and_then(|e| e.file_name().map(|n| n.to_string_lossy().to_string()));
                                match_and_insert_game(catalog, &display_name, &p, pref_exe.as_deref(), map, Some("Windows"), None);
                                continue;
                            }
                        }
                    }
                }

                if let Some(exe_path) = direct_exe {
                    if let Some(parent_dir) = exe_path.parent() {
                        let exe_name = exe_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let is_cat = catalog.iter().any(|c| c.exe_name.eq_ignore_ascii_case(&exe_name) || is_title_match(&c.name, &display_name));
                        let pl = exe_path.to_string_lossy().to_lowercase();
                        let is_game_path = pl.contains("games")
                            || pl.contains("hry")
                            || pl.contains("steam")
                            || pl.contains("epic")
                            || pl.contains("ubisoft")
                            || pl.contains("ea")
                            || pl.contains("riot")
                            || pl.contains("gog")
                            || pl.contains("battle.net")
                            || pl.contains("battlestate")
                            || pl.contains("wargaming")
                            || pl.contains("tarkov");
                        if is_cat || is_game_path {
                            match_and_insert_game(catalog, &display_name, parent_dir, Some(&exe_name), map, Some("Windows"), None);
                        }
                    }
                }
            }
        }

        let _ = RegCloseKey(h_key);
    }
}

// --------------------------------------------------------------------------------------
// 5. Xbox Games Folders
// --------------------------------------------------------------------------------------
fn scan_xbox_games(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    for drive in ["C", "D", "E", "F", "G"] {
        let xbox_path = PathBuf::from(format!(r"{}:\XboxGames", drive));
        if !xbox_path.exists() {
            continue;
        }

        let entries = match fs::read_dir(&xbox_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                match_and_insert_game(catalog, &name, &p, None, map, Some("Xbox"), None);
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// 6. Media Players
// --------------------------------------------------------------------------------------
fn scan_media_players(map: &mut HashMap<String, HdrApp>) {
    for (prog_name, exe_str, paths) in [
        ("VLC Media Player", "vlc.exe", vec![r"C:\Program Files\VideoLAN\VLC\vlc.exe", r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe"]),
        ("MPC-HC", "mpc-hc64.exe", vec![r"C:\Program Files\MPC-HC\mpc-hc64.exe", r"C:\Program Files (x86)\MPC-HC\mpc-hc.exe"]),
        ("MPC-BE", "mpc-be64.exe", vec![r"C:\Program Files\MPC-BE\mpc-be64.exe", r"C:\Program Files (x86)\MPC-BE\mpc-be.exe"]),
        ("mpv Media Player", "mpv.exe", vec![r"C:\Program Files\mpv\mpv.exe", r"C:\mpv\mpv.exe", r"D:\mpv\mpv.exe", r"E:\mpv\mpv.exe"]),
        ("PotPlayer", "potplayer64.exe", vec![r"C:\Program Files\DAUM\PotPlayer\PotPlayer64.exe", r"C:\Program Files (x86)\DAUM\PotPlayer\PotPlayer.exe"]),
        ("Kodi", "kodi.exe", vec![r"C:\Program Files\Kodi\kodi.exe"]),
    ] {
        for p in paths {
            if Path::new(p).exists() {
                map.insert(
                    prog_name.to_string(),
                    HdrApp {
                        name: prog_name.to_string(),
                        exe_name: exe_str.to_string(),
                        enabled: true,
                        hdr_type: HdrType::Media,
                        path: Some(p.to_string()),
                        alternate_exes: Vec::new(),
                        steam_id: None,
                        launcher: Some("Média".to_string()),
                    },
                );
                break;
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// Matching and Insertion Helpers
// --------------------------------------------------------------------------------------
fn match_and_insert_game(
    catalog: &[database::CatalogEntry],
    hint_name: &str,
    game_dir: &Path,
    preferred_exe: Option<&str>,
    map: &mut HashMap<String, HdrApp>,
    launcher: Option<&str>,
    steam_id: Option<&str>,
) {
    // 1. Collect all executable candidates in directory hierarchy (depth up to 5)
    let mut exes = Vec::new();
    collect_exes(game_dir, 0, 5, &mut exes);

    // Filter out helper/diagnostic/installer exes
    let game_exes: Vec<(String, String)> = exes
        .into_iter()
        .filter(|(name, _)| {
            let lower = name.to_lowercase();
            !lower.contains("crash")
                && !lower.contains("setup")
                && !lower.contains("installer")
                && !lower.contains("unins")
                && !lower.contains("unitycrashhandler")
                && !lower.contains("crashreportclient")
                && !lower.contains("protocol")
                && !lower.contains("helper")
                && !lower.contains("eula")
                && !lower.contains("reporter")
                && !lower.contains("prereq")
                && !lower.contains("redist")
                && !lower.contains("benchmark")
                && !lower.contains("dxsetup")
                && !lower.contains("vcredist")
                && !lower.contains("touchup")
                && !lower.contains("config")
        })
        .collect();

    if game_exes.is_empty() {
        return;
    }

    // 2. Check for catalog match
    let mut matched_cat = None;
    let mut primary_exe_pair = None;

    // Strategy 0: Direct Steam AppID match against catalog
    if let Some(s_id) = steam_id {
        if let Some(cat) = catalog.iter().find(|c| c.steam_id.as_deref() == Some(s_id)) {
            matched_cat = Some(cat);
        }
    }

    // Strategy A: Exe match against catalog
    if matched_cat.is_none() {
        for (exe_name, exe_path) in &game_exes {
            if let Some(cat) = catalog.iter().find(|c| {
                c.exe_name.eq_ignore_ascii_case(exe_name)
                    || c.alternate_exes.iter().any(|alt| alt.eq_ignore_ascii_case(exe_name))
            }) {
                matched_cat = Some(cat);
                primary_exe_pair = Some((exe_name.clone(), exe_path.clone()));
                break;
            }
        }
    }

    // Strategy B: Match by game title
    if matched_cat.is_none() && !hint_name.trim().is_empty() {
        if let Some(cat) = catalog.iter().find(|c| is_title_match(&c.name, hint_name)) {
            matched_cat = Some(cat);
        }
    }

    // If matched via Strategy 0 or B, try to find preferred exe from catalog entry
    if primary_exe_pair.is_none() {
        if let Some(cat) = matched_cat {
            for (exe_name, exe_path) in &game_exes {
                if cat.exe_name.eq_ignore_ascii_case(exe_name)
                    || cat.alternate_exes.iter().any(|alt| alt.eq_ignore_ascii_case(exe_name))
                {
                    primary_exe_pair = Some((exe_name.clone(), exe_path.clone()));
                    break;
                }
            }
        }
    }


    // 3. Determine the primary executable
    let (main_exe_name, main_exe_path) = if let Some(pair) = primary_exe_pair {
        pair
    } else if let Some(pref) = preferred_exe {
        // If preferred exe exists in game_exes, pick it
        if let Some(found) = game_exes.iter().find(|(name, _)| name.eq_ignore_ascii_case(pref)) {
            found.clone()
        } else {
            pick_best_primary_exe(&game_exes, hint_name, game_dir)
        }
    } else {
        pick_best_primary_exe(&game_exes, hint_name, game_dir)
    };

    let mut alternate_exes = Vec::new();
    for (e_name, _) in &game_exes {
        let e_lower = e_name.to_lowercase();
        if e_lower != main_exe_name.to_lowercase() && !alternate_exes.contains(&e_lower) {
            alternate_exes.push(e_lower);
        }
    }

    // 4. Construct HdrApp based on catalog match
    let (final_name, is_hdr_supported, final_hdr_type) = if let Some(cat) = matched_cat {
        (cat.name.clone(), true, cat.hdr_type.clone())
    } else {
        // Installed SDR game (not in catalog, but detected as installed on PC)
        let clean_name = if !hint_name.trim().is_empty() {
            hint_name.trim().to_string()
        } else {
            clean_title_from_folder(&game_dir.file_name().unwrap_or_default().to_string_lossy())
        };
        (clean_name, false, HdrType::Custom)
    };

    let key = final_name.clone();

    if let Some(existing) = map.get_mut(&key) {
        if existing.steam_id.is_none() && steam_id.is_some() {
            existing.steam_id = steam_id.map(|s| s.to_string());
        }
        if existing.launcher.is_none() && launcher.is_some() {
            existing.launcher = launcher.map(|s| s.to_string());
        }
        for alt in alternate_exes {
            if !existing.alternate_exes.contains(&alt) && existing.exe_name.to_lowercase() != alt {
                existing.alternate_exes.push(alt);
            }
        }
        if existing.path.is_none() {
            existing.path = Some(main_exe_path);
        }
    } else {
        map.insert(
            key,
            HdrApp {
                name: final_name,
                exe_name: main_exe_name.to_lowercase(),
                enabled: is_hdr_supported, // HDR games are enabled by default (top), SDR games are disabled by default (bottom)
                hdr_type: final_hdr_type,
                path: Some(main_exe_path),
                alternate_exes,
                steam_id: steam_id.map(|s| s.to_string()),
                launcher: launcher.map(|s| s.to_string()),
            },
        );
    }
}

fn pick_best_primary_exe(
    candidates: &[(String, String)],
    hint_name: &str,
    game_dir: &Path,
) -> (String, String) {
    let clean_hint = clean_string(hint_name);
    let folder_name = game_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let clean_folder = clean_string(&folder_name);

    // 1. Unreal Engine Shipping binary (e.g. *-Win64-Shipping.exe)
    for (name, path) in candidates {
        let lower = name.to_lowercase();
        if lower.ends_with("-win64-shipping.exe") || lower.ends_with("_win64_shipping.exe") {
            return (name.clone(), path.clone());
        }
    }

    // 2. Exe name strictly contains game title or folder title
    for (name, path) in candidates {
        let clean_exe = clean_string(name.trim_end_matches(".exe"));
        if (!clean_hint.is_empty() && (clean_exe == clean_hint || clean_hint.contains(&clean_exe) || clean_exe.contains(&clean_hint)))
            || (!clean_folder.is_empty() && (clean_exe == clean_folder || clean_folder.contains(&clean_exe) || clean_exe.contains(&clean_folder)))
        {
            return (name.clone(), path.clone());
        }
    }

    // 3. Executable directly in root folder
    for (name, path) in candidates {
        let p = Path::new(path);
        if p.parent() == Some(game_dir) {
            return (name.clone(), path.clone());
        }
    }

    // 4. Largest executable by file size
    let mut largest = &candidates[0];
    let mut max_size = 0u64;

    for candidate in candidates {
        if let Ok(meta) = fs::metadata(&candidate.1) {
            if meta.len() > max_size {
                max_size = meta.len();
                largest = candidate;
            }
        }
    }

    largest.clone()
}

fn collect_exes(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<(String, String)>) {
    if depth > max_depth || out.len() >= 12 {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
                .to_lowercase();

            let should_skip = dir_name.starts_with('.')
                || dir_name == "content"
                || dir_name == "assets"
                || dir_name == "data"
                || dir_name == "paks"
                || dir_name == "pak"
                || dir_name == "vehicles"
                || dir_name == "levels"
                || dir_name == "maps"
                || dir_name == "textures"
                || dir_name == "shaders"
                || dir_name == "movies"
                || dir_name == "audio"
                || dir_name == "sound"
                || dir_name == "sounds"
                || dir_name == "music"
                || dir_name == "plugins"
                || dir_name == "dlc"
                || dir_name == "localization"
                || dir_name == "languages"
                || dir_name == "ui"
                || dir_name == "scripts"
                || dir_name == "mods"
                || dir_name == "user_data"
                || dir_name == "temp"
                || dir_name == "cache"
                || dir_name == "docs"
                || dir_name == "documentation"
                || dir_name == "_commonredist"
                || dir_name == "engine"
                || dir_name == "easyanticheat"
                || dir_name == "battleye"
                || dir_name == "crashreportclient"
                || dir_name == "directx"
                || dir_name == "dotnet"
                || dir_name == "installers"
                || dir_name == "support"
                || dir_name == "prerequisites"
                || dir_name == "$recycle.bin"
                || dir_name == "saved"
                || dir_name == "save";

            if !should_skip {
                let is_binary_folder = dir_name == "bin"
                    || dir_name == "binaries"
                    || dir_name == "bin64"
                    || dir_name == "bin32"
                    || dir_name == "win64"
                    || dir_name == "win32"
                    || dir_name == "x64"
                    || dir_name == "x86";

                if depth == 0 || is_binary_folder {
                    subdirs.push((path, is_binary_folder));
                }
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

    // Prioritize binary directories first!
    subdirs.sort_by_key(|(_, is_bin)| if *is_bin { 0 } else { 1 });

    for (sub, _) in subdirs {
        collect_exes(&sub, depth + 1, max_depth, out);
        if out.len() >= 10 {
            break;
        }
    }
}

// --------------------------------------------------------------------------------------
// String & Title Helpers
// --------------------------------------------------------------------------------------

fn strip_editions(s: &str) -> String {
    let mut lower = s.to_lowercase();
    let editions = [
        "director's cut",
        "directors cut",
        "game of the year edition",
        "goty edition",
        "goty",
        "enhanced edition",
        "definitive edition",
        "ultimate edition",
        "digital deluxe edition",
        "deluxe edition",
        "complete edition",
        "royal edition",
        "anniversary edition",
        "special edition",
        "remastered",
        "remaster",
        "standard edition",
        "gold edition",
        "legendary edition",
        "vr edition",
    ];
    for ed in editions {
        lower = lower.replace(ed, " ");
    }
    lower
}

fn extract_title_base(title: &str) -> &str {
    if let Some(pos) = title.find(':') {
        return &title[..pos];
    }
    if let Some(pos) = title.find(" - ") {
        return &title[..pos];
    }
    if let Some(pos) = title.find(" – ") {
        return &title[..pos];
    }
    if let Some(pos) = title.find(" — ") {
        return &title[..pos];
    }
    title
}

pub fn is_title_match(cat_name: &str, candidate_name: &str) -> bool {
    // If either name contains " / " (e.g. bilingual Steam Capcom titles), test both full and primary part
    if let Some(pos) = candidate_name.find(" / ") {
        if is_title_match(cat_name, candidate_name[..pos].trim()) {
            return true;
        }
    }
    if let Some(pos) = cat_name.find(" / ") {
        if is_title_match(cat_name[..pos].trim(), candidate_name) {
            return true;
        }
    }

    let norm_cat = normalize_game_title(cat_name);
    let norm_cand = normalize_game_title(candidate_name);

    if norm_cat.is_empty() || norm_cand.is_empty() {
        return false;
    }

    // 1. Direct equality on normalized titles
    if norm_cat == norm_cand {
        return true;
    }

    // 2. Direct equality after stripping edition suffixes (e.g. "Director's Cut", "Remastered", "GOTY")
    let stripped_cat = normalize_game_title(&strip_editions(cat_name));
    let stripped_cand = normalize_game_title(&strip_editions(candidate_name));

    if !stripped_cat.is_empty() && !stripped_cand.is_empty() && stripped_cat == stripped_cand {
        return true;
    }

    // 3. One title includes a subtitle (e.g. "The Witcher 3: Wild Hunt"), while the other is just the base ("The Witcher 3").
    // Never match if both have subtitles with different endings (e.g. "Star Wars: Squadrons" vs "Star Wars: Outlaws")!
    let cat_base = normalize_game_title(&strip_editions(extract_title_base(cat_name)));
    let cand_base = normalize_game_title(&strip_editions(extract_title_base(candidate_name)));

    let cat_has_sub = cat_base != stripped_cat;
    let cand_has_sub = cand_base != stripped_cand;

    if cat_has_sub != cand_has_sub {
        if cat_has_sub && cat_base.len() >= 4 && cat_base == stripped_cand {
            return true;
        }
        if cand_has_sub && cand_base.len() >= 4 && cand_base == stripped_cat {
            return true;
        }
    }

    false
}

fn strip_year_parens(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j + 4 <= chars.len() && chars[j..j + 4].iter().all(|c| c.is_ascii_digit()) {
                let mut k = j + 4;
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if k < chars.len() && chars[k] == ')' {
                    i = k + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn normalize_game_title(s: &str) -> String {
    let stripped = strip_year_parens(s);
    let mut lower = stripped.to_lowercase();

    // Normalize Roman numerals commonly used in game titles
    lower = lower
        .replace(" viii", " 8")
        .replace(" vii", " 7")
        .replace(" vi", " 6")
        .replace(" iv", " 4")
        .replace(" v", " 5")
        .replace(" iii", " 3")
        .replace(" ii", " 2");

    clean_string(&lower)
}


fn clean_string(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn clean_title_from_folder(folder: &str) -> String {
    folder
        .replace('_', " ")
        .replace('-', " ")
        .replace("Win64", "")
        .replace("Shipping", "")
        .trim()
        .to_string()
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn read_registry_string(root: HKEY, subkey: &str, value_name: &str) -> Result<String, ()> {
    let subkey_wide = to_wide(subkey);
    let val_name_wide = to_wide(value_name);
    let mut h_key = HKEY::default();

    unsafe {
        if RegOpenKeyExW(root, PCWSTR(subkey_wide.as_ptr()), Some(0), KEY_READ, &mut h_key) != WIN32_ERROR(0) {
            return Err(());
        }

        let mut buf = [0u16; 1024];
        let mut buf_size = (buf.len() * 2) as u32;
        let mut val_type = REG_SZ;

        let status = RegQueryValueExW(
            h_key,
            PCWSTR(val_name_wide.as_ptr()),
            None,
            Some(&mut val_type),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_size),
        );

        let _ = RegCloseKey(h_key);

        if status == WIN32_ERROR(0) {
            let len = (buf_size / 2) as usize;
            let end = buf[..len].iter().position(|&c| c == 0).unwrap_or(len);
            Ok(String::from_utf16_lossy(&buf[..end]))
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assetto_corsa_does_not_match_competizione() {
        assert!(!is_title_match("Assetto Corsa Competizione", "Assetto Corsa"));
        assert!(!is_title_match("Assetto Corsa", "Assetto Corsa Competizione"));
        assert!(is_title_match("Assetto Corsa", "Assetto Corsa"));
        assert!(is_title_match("Assetto Corsa Competizione", "Assetto Corsa Competizione"));
    }

    #[test]
    fn test_editions_and_subtitles() {
        assert!(is_title_match("Death Stranding Director's Cut", "Death Stranding"));
        assert!(is_title_match("The Witcher 3: Wild Hunt", "The Witcher 3"));
        assert!(is_title_match("Cyberpunk 2077: Phantom Liberty", "Cyberpunk 2077"));
        assert!(is_title_match("Ghost of Tsushima DIRECTOR'S CUT", "Ghost of Tsushima"));
    }

    #[test]
    fn test_sequels_do_not_match() {
        assert!(!is_title_match("Doom Eternal", "Doom"));
        assert!(!is_title_match("Marvel's Spider-Man Remastered", "Marvel's Spider-Man 2"));
        assert!(!is_title_match("Alan Wake 2", "Alan Wake"));
        assert!(!is_title_match("Star Wars: Squadrons", "Star Wars: Outlaws"));
    }

    #[test]
    fn test_bilingual_capcom_titles() {
        assert!(is_title_match(
            "Resident Evil 7: Biohazard",
            "RESIDENT EVIL 7 biohazard / BIOHAZARD 7 resident evil"
        ));
        assert!(is_title_match(
            "RESIDENT EVIL 7 biohazard / BIOHAZARD 7 resident evil",
            "Resident Evil 7: Biohazard"
        ));
    }

    #[test]
    fn test_year_parens_stripping() {
        assert!(is_title_match("Silent Hill 2 (2024)", "SILENT HILL 2"));
        assert!(is_title_match("SILENT HILL 2", "Silent Hill 2 (2024)"));
        assert!(is_title_match("Alone in the Dark (2024)", "Alone in the Dark"));
    }

    #[test]
    fn test_alternate_exes_and_catalog_lookup() {
        let cat = database::get_full_catalog();

        // Check Silent Hill 2
        let sh2 = cat.iter().find(|c| c.steam_id.as_deref() == Some("2124490"));
        assert!(sh2.is_some(), "Silent Hill 2 must exist with Steam AppID 2124490");
        let sh2_entry = sh2.unwrap();
        assert_eq!(sh2_entry.hdr_type, HdrType::Native);

        // Check lookup by alternate or main exes
        let by_shipping = database::find_in_catalog("shproto-win64-shipping.exe");
        assert!(by_shipping.is_some(), "Should find SH2 by shproto-win64-shipping.exe");

        let by_eos = database::find_in_catalog("game_f_x64_eos.exe");
        assert!(by_eos.is_some(), "Should find Alan Wake Remastered by game_f_x64_eos.exe");

        let by_goty = database::find_in_catalog("borderlandsgoty.exe");
        assert!(by_goty.is_some(), "Should find Borderlands GOTY by borderlandsgoty.exe");

        let by_re7 = database::find_in_catalog("re7.exe");
        assert!(by_re7.is_some(), "Should find Resident Evil 7 by re7.exe");
    }
}



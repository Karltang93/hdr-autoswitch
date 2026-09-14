use crate::config::{HdrApp, HdrType};
use crate::database;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
};

pub fn scan_installed_games() -> Vec<HdrApp> {
    let catalog = database::get_full_catalog();
    let mut detected_map: HashMap<String, HdrApp> = HashMap::new();

    // 1. Scan Steam via appmanifest_*.acf (guarantees ONLY ACTUALLY INSTALLED games, no uninstalled leftovers)
    scan_steam_manifests(&catalog, &mut detected_map);

    // 2. Scan Epic Games Launcher via Manifests/*.item
    scan_epic_manifests(&catalog, &mut detected_map);

    // 3. Scan Windows Registry (EA App, Ubisoft Connect, GOG Galaxy, standalone installers)
    scan_windows_registry(&catalog, &mut detected_map);

    // 4. Scan Xbox Games folders (C:\XboxGames, D:\XboxGames, etc.)
    scan_xbox_games(&catalog, &mut detected_map);

    // 5. Scan common media players
    scan_media_players(&mut detected_map);

    let mut result: Vec<HdrApp> = detected_map.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

// --------------------------------------------------------------------------------------
// 1. Steam scanner using appmanifest_*.acf
// --------------------------------------------------------------------------------------
fn scan_steam_manifests(catalog: &[database::CatalogEntry], map: &mut HashMap<String, HdrApp>) {
    let mut steam_roots = Vec::new();

    // Check registry for SteamPath
    if let Ok(reg_steam) = read_registry_string(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath") {
        let p = PathBuf::from(reg_steam);
        if p.exists() {
            steam_roots.push(p);
        }
    }

    // Default paths
    for def in [r"C:\Program Files (x86)\Steam", r"C:\Steam", r"D:\Steam", r"E:\Steam"] {
        let p = PathBuf::from(def);
        if p.exists() && !steam_roots.contains(&p) {
            steam_roots.push(p);
        }
    }

    let mut library_paths = Vec::new();

    for steam_root in steam_roots {
        library_paths.push(steam_root.join("steamapps"));

        let vdf = steam_root.join(r"steamapps\libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(&vdf) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 4 {
                        let path_str = parts[3].replace(r"\\", r"\");
                        let p = PathBuf::from(path_str).join("steamapps");
                        if p.exists() && !library_paths.contains(&p) {
                            library_paths.push(p);
                        }
                    }
                }
            }
        }
    }

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

                    if !install_dir_name.is_empty() {
                        let full_game_dir = steamapps.join("common").join(&install_dir_name);
                        if full_game_dir.exists() {
                            match_and_insert_game(catalog, &game_name, &full_game_dir, map);
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
    let manifest_dir = PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    if !manifest_dir.exists() {
        return;
    }

    let entries = match fs::read_dir(&manifest_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "item").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let display_name = val["DisplayName"].as_str().unwrap_or_default();
                    let install_loc = val["InstallLocation"].as_str().unwrap_or_default();
                    let launch_exe = val["LaunchExecutable"].as_str().unwrap_or_default();

                    let game_dir = PathBuf::from(install_loc);
                    if game_dir.exists() {
                        // Check if launch_exe or display_name matches catalog
                        let mut found_cat = catalog.iter().find(|c| {
                            c.exe_name.eq_ignore_ascii_case(launch_exe)
                                || clean_string(&c.name) == clean_string(display_name)
                        });

                        if found_cat.is_none() {
                            // Check exes in game dir
                            let mut exes = Vec::new();
                            collect_exes(&game_dir, 0, 3, &mut exes);
                            for (e_name, _) in &exes {
                                if let Some(cat) = catalog.iter().find(|c| c.exe_name.eq_ignore_ascii_case(e_name)) {
                                    found_cat = Some(cat);
                                    break;
                                }
                            }
                        }

                        if let Some(cat) = found_cat {
                            insert_or_merge_game(cat, display_name, &game_dir, launch_exe, map);
                        }
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// 3. Windows Registry Uninstall Keys (EA App, Ubisoft, GOG, Custom Installers)
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
                if let Ok(install_location) = read_registry_string(root, &child_path, "InstallLocation") {
                    let clean_dir = install_location.trim().trim_matches('"');
                    if !clean_dir.is_empty() {
                        let p = PathBuf::from(clean_dir);
                        if p.exists() && p.is_dir() {
                            match_and_insert_game(catalog, &display_name, &p, map);
                        }
                    }
                }
            }
        }

        let _ = RegCloseKey(h_key);
    }
}

// --------------------------------------------------------------------------------------
// 4. Xbox Games Folders
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
                match_and_insert_game(catalog, &name, &p, map);
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// 5. Media Players
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
    map: &mut HashMap<String, HdrApp>,
) {
    let clean_hint = clean_string(hint_name);

    // Collect exes in directory
    let mut exes = Vec::new();
    collect_exes(game_dir, 0, 3, &mut exes);

    // Filter out obvious non-game helper exes
    let game_exes: Vec<(String, String)> = exes
        .into_iter()
        .filter(|(name, _)| {
            let lower = name.to_lowercase();
            !lower.contains("crash")
                && !lower.contains("setup")
                && !lower.contains("installer")
                && !lower.contains("unity")
                && !lower.contains("redlauncher")
                && !lower.contains("launcher.exe")
                && !lower.contains("protocol")
                && !lower.contains("helper")
                && !lower.contains("eula")
                && !lower.contains("reporter")
                && !lower.contains("prereq")
        })
        .collect();

    // Try finding match in catalog:
    // A) Direct exe match
    let mut matched_cat = None;
    let mut primary_exe = None;

    for (exe_name, exe_path) in &game_exes {
        if let Some(cat) = catalog.iter().find(|c| c.exe_name.eq_ignore_ascii_case(exe_name)) {
            matched_cat = Some(cat);
            primary_exe = Some((exe_name.clone(), exe_path.clone()));
            break;
        }
    }

    // B) If no exe match, try matching game name from hint
    if matched_cat.is_none() && !clean_hint.is_empty() {
        if let Some(cat) = catalog.iter().find(|c| {
            let clean_cat = clean_string(&c.name);
            clean_hint == clean_cat || clean_hint.contains(&clean_cat) || clean_cat.contains(&clean_hint)
        }) {
            matched_cat = Some(cat);
            if let Some(first) = game_exes.first() {
                primary_exe = Some(first.clone());
            }
        }
    }

    if let Some(cat) = matched_cat {
        let (main_exe_name, main_exe_path) = match primary_exe {
            Some(pe) => pe,
            None => (cat.exe_name.clone(), game_dir.to_string_lossy().to_string()),
        };

        let mut alternate_exes = Vec::new();
        for (e_name, _) in game_exes {
            let e_lower = e_name.to_lowercase();
            if e_lower != main_exe_name.to_lowercase() && !alternate_exes.contains(&e_lower) {
                alternate_exes.push(e_lower);
            }
        }

        // Always use clean catalog title for consistency
        let key = cat.name.clone();

        if let Some(existing) = map.get_mut(&key) {
            for alt in alternate_exes {
                if !existing.alternate_exes.contains(&alt) && existing.exe_name.to_lowercase() != alt {
                    existing.alternate_exes.push(alt);
                }
            }
        } else {
            map.insert(
                key,
                HdrApp {
                    name: cat.name.clone(),
                    exe_name: main_exe_name.to_lowercase(),
                    enabled: true,
                    hdr_type: cat.hdr_type.clone(),
                    path: Some(main_exe_path),
                    alternate_exes,
                },
            );
        }
    }
}

fn insert_or_merge_game(
    cat: &database::CatalogEntry,
    display_name: &str,
    game_dir: &Path,
    launch_exe: &str,
    map: &mut HashMap<String, HdrApp>,
) {
    let key = cat.name.clone();
    let main_exe = if !launch_exe.is_empty() {
        launch_exe.to_lowercase()
    } else {
        cat.exe_name.to_lowercase()
    };

    let mut exes = Vec::new();
    collect_exes(game_dir, 0, 3, &mut exes);
    let mut alternates = Vec::new();
    for (e_name, _) in exes {
        let el = e_name.to_lowercase();
        if el != main_exe && !alternates.contains(&el) && !el.contains("crash") && !el.contains("installer") {
            alternates.push(el);
        }
    }

    let game_path = game_dir.join(&main_exe);
    let path_str = if game_path.exists() {
        game_path.to_string_lossy().to_string()
    } else {
        game_dir.to_string_lossy().to_string()
    };

    if let Some(existing) = map.get_mut(&key) {
        for alt in alternates {
            if !existing.alternate_exes.contains(&alt) && existing.exe_name != alt {
                existing.alternate_exes.push(alt);
            }
        }
    } else {
        map.insert(
            key,
            HdrApp {
                name: if cat.name.is_empty() { display_name.to_string() } else { cat.name.clone() },
                exe_name: main_exe,
                enabled: true,
                hdr_type: cat.hdr_type.clone(),
                path: Some(path_str),
                alternate_exes: alternates,
            },
        );
    }
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

fn clean_string(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
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

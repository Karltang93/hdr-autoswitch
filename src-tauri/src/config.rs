use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HdrType {
    Native,
    AutoHdr,
    Media,
    Custom,
}

impl HdrType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HdrType::Native => "native",
            HdrType::AutoHdr => "autohdr",
            HdrType::Media => "media",
            HdrType::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdrApp {
    pub name: String,
    pub exe_name: String, // lowercase, e.g. "cyberpunk2077.exe"
    pub enabled: bool,
    pub hdr_type: HdrType,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub alternate_exes: Vec<String>,
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub launcher: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwitchMethod {
    Native,
    Shortcut,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub target_monitor: String, // "all" or monitor id
    pub alt_tab_delay_seconds: u64, // e.g. 2 seconds
    pub notifications_enabled: bool,
    pub autostart: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub auto_detect_new_games: bool,
    #[serde(default = "default_true")]
    pub auto_sync_database: bool,
    #[serde(default)]
    pub last_sync_timestamp: Option<u64>,
    #[serde(default = "default_true")]
    pub exit_only_hdr: bool, // When true, keeps HDR on during Alt+Tab while game is running; switches to SDR immediately on game exit!
    pub switch_method: SwitchMethod,
    pub blacklist: Vec<String>, // lowercase exe names to never trigger HDR (e.g. chrome.exe)
    pub apps: Vec<HdrApp>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_monitor: "all".to_string(),
            alt_tab_delay_seconds: 2,
            notifications_enabled: true,
            autostart: false,
            start_minimized: false,
            auto_detect_new_games: true,
            auto_sync_database: true,
            last_sync_timestamp: None,
            exit_only_hdr: true,
            switch_method: SwitchMethod::Native,
            blacklist: vec![
                "chrome.exe".to_string(),
                "msedge.exe".to_string(),
                "firefox.exe".to_string(),
                "brave.exe".to_string(),
                "opera.exe".to_string(),
                "vivaldi.exe".to_string(),
                "discord.exe".to_string(),
                "spotify.exe".to_string(),
                "steam.exe".to_string(),
                "steamwebhelper.exe".to_string(),
                "epicgameslauncher.exe".to_string(),
                "explorer.exe".to_string(),
                "taskmgr.exe".to_string(),
                "code.exe".to_string(),
                "devenv.exe".to_string(),
                "slack.exe".to_string(),
                "telegram.exe".to_string(),
                "whatsapp.exe".to_string(),
                "snippingtool.exe".to_string(),
                "screensketch.exe".to_string(),
                "applicationframehost.exe".to_string(),
                "gamingservicesui.exe".to_string(),
                "systemsettings.exe".to_string(),
                "shellexperiencehost.exe".to_string(),
                "searchhost.exe".to_string(),
                "startmenuexperiencehost.exe".to_string(),
                "lockapp.exe".to_string(),
                "cmd.exe".to_string(),
                "powershell.exe".to_string(),
                "wt.exe".to_string(),
                "conhost.exe".to_string(),
            ],
            apps: Vec::new(),
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: Arc<Mutex<AppConfig>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let app_data = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("HDRAutoSwitch");

        if !app_data.exists() {
            let _ = fs::create_dir_all(&app_data);
        }

        let config_path = app_data.join("config.json");
        let mut config = AppConfig::default();

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(mut loaded) = serde_json::from_str::<AppConfig>(&content) {
                    for app in &mut loaded.apps {
                        if app.steam_id.is_none() {
                            let lower = app.name.to_lowercase();
                            let sid = match lower.as_str() {
                                s if s.contains("bodycam") => Some("2406770"),
                                s if s.contains("assetto corsa") => Some("244210"),
                                s if s.contains("beamng") => Some("284160"),
                                s if s.contains("enshrouded") => Some("1203620"),
                                s if s.contains("forza horizon") => Some("1551360"),
                                s if s.contains("vostok") => Some("1963620"),
                                s if s.contains("starfield") => Some("1716740"),
                                s if s.contains("teardown") => Some("1167630"),
                                s if s.contains("the finals") => Some("2073850"),
                                s if s.contains("indiana jones") => Some("2677660"),
                                s if s.contains("battlefield") => Some("1517290"),
                                s if s.contains("cyberpunk") => Some("1091500"),
                                s if s.contains("witcher") => Some("292030"),
                                s if s.contains("elden ring") => Some("1245620"),
                                s if s.contains("baldur") => Some("1086940"),
                                s if s.contains("helldivers") => Some("553850"),
                                s if s.contains("wukong") => Some("2358720"),
                                s if s.contains("god of war") => Some("1593500"),
                                s if s.contains("red dead") => Some("1174180"),
                                _ => None,
                            };
                            if let Some(id) = sid {
                                app.steam_id = Some(id.to_string());
                                if app.launcher.is_none() {
                                    app.launcher = Some("Steam".to_string());
                                }
                            }
                        }
                    }

                    // Automatically purge any blacklisted or system apps that might have been accidentally enrolled:
                    let is_blacklisted = |exe: &str| -> bool {
                        let lower = exe.to_lowercase();
                        matches!(
                            lower.as_str(),
                            "snippingtool.exe"
                                | "screensketch.exe"
                                | "explorer.exe"
                                | "chrome.exe"
                                | "msedge.exe"
                                | "firefox.exe"
                                | "brave.exe"
                                | "opera.exe"
                                | "vivaldi.exe"
                                | "discord.exe"
                                | "spotify.exe"
                                | "steam.exe"
                                | "steamwebhelper.exe"
                                | "epicgameslauncher.exe"
                                | "applicationframehost.exe"
                                | "gamingservicesui.exe"
                                | "systemsettings.exe"
                                | "shellexperiencehost.exe"
                                | "searchhost.exe"
                                | "startmenuexperiencehost.exe"
                                | "lockapp.exe"
                                | "taskmgr.exe"
                                | "cmd.exe"
                                | "powershell.exe"
                                | "wt.exe"
                                | "conhost.exe"
                                | "code.exe"
                                | "devenv.exe"
                        )
                    };
                    loaded.apps.retain(|a| !is_blacklisted(&a.exe_name));

                    // Deduplicate apps by name / steam_id / exe and merge alternate_exes
                    let mut unique_apps: Vec<HdrApp> = Vec::new();
                    for app in loaded.apps {
                        let app_name_clean = app.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
                        if let Some(existing) = unique_apps.iter_mut().find(|a| {
                            let a_clean = a.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
                            (a_clean.len() > 2 && a_clean == app_name_clean)
                                || (a.steam_id.is_some() && a.steam_id == app.steam_id)
                                || a.exe_name.eq_ignore_ascii_case(&app.exe_name)
                        }) {
                            let exe_lower = app.exe_name.to_lowercase();
                            if existing.exe_name.to_lowercase() != exe_lower
                                && !existing.alternate_exes.iter().any(|alt| alt.to_lowercase() == exe_lower)
                            {
                                existing.alternate_exes.push(app.exe_name);
                            }
                            for alt in app.alternate_exes {
                                let alt_lower = alt.to_lowercase();
                                if existing.exe_name.to_lowercase() != alt_lower
                                    && !existing.alternate_exes.iter().any(|e| e.to_lowercase() == alt_lower)
                                {
                                    existing.alternate_exes.push(alt);
                                }
                            }
                            if existing.steam_id.is_none() && app.steam_id.is_some() {
                                existing.steam_id = app.steam_id;
                            }
                            if existing.launcher.is_none() && app.launcher.is_some() {
                                existing.launcher = app.launcher;
                            }
                        } else {
                            unique_apps.push(app);
                        }
                    }
                    loaded.apps = unique_apps;

                    config = loaded;
                }
            }
        }

        Self {
            config_path,
            config: Arc::new(Mutex::new(config)),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let conf = self.config.lock().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&*conf).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update_config(&self, new_config: AppConfig) -> Result<(), String> {
        {
            let mut conf = self.config.lock().map_err(|e| e.to_string())?;
            *conf = new_config;
        }
        self.save()
    }

    pub fn is_hdr_app(&self, exe: &str) -> bool {
        let exe_lower = exe.to_lowercase();
        let conf = self.config.lock().unwrap();

        // If blacklisted, never treat as HDR app
        if conf.blacklist.iter().any(|b| b.to_lowercase() == exe_lower) {
            return false;
        }

        // Check if in active apps (primary exe or alternate exes)
        conf.apps.iter().any(|a| {
            a.enabled
                && (a.exe_name.to_lowercase() == exe_lower
                    || a.alternate_exes
                        .iter()
                        .any(|alt| alt.to_lowercase() == exe_lower))
        })
    }

    pub fn find_app(&self, exe: &str) -> Option<HdrApp> {
        let exe_lower = exe.to_lowercase();
        let conf = self.config.lock().unwrap();
        // 1. Exact match in exe_name or alternate_exes
        if let Some(app) = conf.apps.iter().find(|a| {
            a.exe_name.to_lowercase() == exe_lower
                || a.alternate_exes
                    .iter()
                    .any(|alt| alt.to_lowercase() == exe_lower)
        }) {
            return Some(app.clone());
        }

        // 2. Base stem matching (strip -win64-shipping, etc.)
        let clean_target = exe_lower
            .trim_end_matches(".exe")
            .replace("-win64-shipping", "")
            .replace("_win64_shipping", "")
            .replace("-shipping", "")
            .replace("_shipping", "")
            .replace("_dx12", "")
            .replace("_dx11", "")
            .replace("_vk", "");
        let clean_target_alphanumeric: String = clean_target.chars().filter(|c| c.is_alphanumeric()).collect();

        if clean_target_alphanumeric.len() >= 3 {
            if let Some(app) = conf.apps.iter().find(|a| {
                let a_clean = a.exe_name.to_lowercase()
                    .trim_end_matches(".exe")
                    .replace("-win64-shipping", "")
                    .replace("_win64_shipping", "")
                    .replace("-shipping", "")
                    .replace("_shipping", "");
                let a_alphanumeric: String = a_clean.chars().filter(|c| c.is_alphanumeric()).collect();
                let a_name_alphanumeric: String = a.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
                a_alphanumeric == clean_target_alphanumeric || a_name_alphanumeric == clean_target_alphanumeric
            }) {
                return Some(app.clone());
            }
        }

        None
    }

    pub fn add_app(&self, mut app: HdrApp) -> Result<(), String> {
        {
            let mut conf = self.config.lock().map_err(|e| e.to_string())?;
            let exe_lower = app.exe_name.to_lowercase();
            let app_name_clean: String = app.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();

            // Check if game already exists by name, steam_id, or exe
            if let Some(existing) = conf.apps.iter_mut().find(|a| {
                let a_clean: String = a.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
                (a_clean.len() > 2 && a_clean == app_name_clean)
                    || (a.steam_id.is_some() && a.steam_id == app.steam_id)
                    || a.exe_name.to_lowercase() == exe_lower
                    || a.alternate_exes.iter().any(|alt| alt.to_lowercase() == exe_lower)
            }) {
                // If it exists, add this exe as an alternate exe if not already present
                if existing.exe_name.to_lowercase() != exe_lower
                    && !existing.alternate_exes.iter().any(|alt| alt.to_lowercase() == exe_lower)
                {
                    existing.alternate_exes.push(app.exe_name);
                }
                if existing.steam_id.is_none() && app.steam_id.is_some() {
                    existing.steam_id = app.steam_id;
                }
                return self.save();
            }

            // If steam_id is missing, check known game titles:
            if app.steam_id.is_none() {
                let lower = app.name.to_lowercase();
                let sid = match lower.as_str() {
                    s if s.contains("bodycam") => Some("2406770"),
                    s if s.contains("assetto corsa") => Some("244210"),
                    s if s.contains("beamng") => Some("284160"),
                    s if s.contains("enshrouded") => Some("1203620"),
                    s if s.contains("forza horizon") => Some("1551360"),
                    s if s.contains("vostok") => Some("1963620"),
                    s if s.contains("starfield") => Some("1716740"),
                    s if s.contains("teardown") => Some("1167630"),
                    s if s.contains("the finals") => Some("2073850"),
                    s if s.contains("indiana jones") => Some("2677660"),
                    s if s.contains("battlefield") => Some("1517290"),
                    s if s.contains("cyberpunk") => Some("1091500"),
                    s if s.contains("witcher") => Some("292030"),
                    s if s.contains("elden ring") => Some("1245620"),
                    s if s.contains("baldur") => Some("1086940"),
                    s if s.contains("helldivers") => Some("553850"),
                    s if s.contains("wukong") => Some("2358720"),
                    s if s.contains("god of war") => Some("1593500"),
                    s if s.contains("red dead") => Some("1174180"),
                    _ => None,
                };
                if let Some(id) = sid {
                    app.steam_id = Some(id.to_string());
                    if app.launcher.is_none() {
                        app.launcher = Some("Steam".to_string());
                    }
                }
            }

            conf.apps.push(app);
        }
        self.save()
    }
}

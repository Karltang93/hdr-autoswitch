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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub target_monitor: String, // "all" or monitor id
    pub alt_tab_delay_seconds: u64, // e.g. 2 seconds
    pub notifications_enabled: bool,
    pub autostart: bool,
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
        conf.apps
            .iter()
            .find(|a| {
                a.exe_name.to_lowercase() == exe_lower
                    || a.alternate_exes
                        .iter()
                        .any(|alt| alt.to_lowercase() == exe_lower)
            })
            .cloned()
    }
}

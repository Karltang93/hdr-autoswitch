use crate::config_storage::{
    read_optional, Document, InstallSource, RestoreSourceError, Storage, StorageReport, StoreOutcome,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HdrType {
    #[serde(alias = "Native")]
    Native,
    #[serde(alias = "AutoHdr", alias = "AutoHDR")]
    AutoHdr,
    #[serde(alias = "Media")]
    Media,
    #[serde(alias = "Custom")]
    Custom,
}

impl HdrType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::AutoHdr => "autohdr",
            Self::Media => "media",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HdrApp {
    pub name: String,
    pub exe_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwitchMethod {
    #[serde(alias = "Native")]
    Native,
    #[serde(alias = "Shortcut")]
    Shortcut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetMonitor {
    All,
    Monitor {
        device_path: String,
        display_name: String,
    },
    NeedsConfirmation {
        legacy_runtime_id: String,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub target_monitor: TargetMonitor,
    pub alt_tab_delay_seconds: u64,
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
    pub exit_only_hdr: bool,
    pub switch_method: SwitchMethod,
    pub blacklist: Vec<String>,
    pub apps: Vec<HdrApp>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_monitor: TargetMonitor::All,
            alt_tab_delay_seconds: 2,
            notifications_enabled: true,
            autostart: false,
            start_minimized: false,
            auto_detect_new_games: true,
            auto_sync_database: true,
            last_sync_timestamp: None,
            exit_only_hdr: true,
            switch_method: SwitchMethod::Native,
            blacklist: [
                "chrome.exe",
                "msedge.exe",
                "firefox.exe",
                "brave.exe",
                "opera.exe",
                "vivaldi.exe",
                "discord.exe",
                "spotify.exe",
                "steam.exe",
                "steamwebhelper.exe",
                "epicgameslauncher.exe",
                "explorer.exe",
                "taskmgr.exe",
                "code.exe",
                "devenv.exe",
                "slack.exe",
                "telegram.exe",
                "whatsapp.exe",
                "snippingtool.exe",
                "screensketch.exe",
                "applicationframehost.exe",
                "gamingservicesui.exe",
                "systemsettings.exe",
                "shellexperiencehost.exe",
                "searchhost.exe",
                "startmenuexperiencehost.exe",
                "lockapp.exe",
                "cmd.exe",
                "powershell.exe",
                "wt.exe",
                "conhost.exe",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            apps: Vec::new(),
        }
    }
}

impl AppConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if let TargetMonitor::Monitor { device_path, .. } = &self.target_monitor {
            if device_path.trim().is_empty() || device_path.contains('\0') {
                return Err("A monitor selection requires a nonempty device-interface path".into());
            }
        }
        Ok(())
    }

    pub fn is_hdr_app(&self, exe: &str) -> bool {
        self.find_app(exe).is_some()
    }

    pub fn find_app(&self, exe: &str) -> Option<&HdrApp> {
        self.resolve_app(None, exe).matched()
    }

    pub fn resolve_app(&self, path: Option<&str>, exe: &str) -> crate::runtime_policy::Resolution<'_> {
        crate::runtime_policy::resolve(self, path, exe)
    }
}

pub(crate) fn decode_legacy(bytes: &[u8]) -> Result<AppConfig, String> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid legacy JSON: {e}"))?;
    let object = value
        .as_object_mut()
        .ok_or("Legacy settings must be a JSON object")?;
    if object.contains_key("schema_version") {
        return Err("A versioned file is not a supported legacy settings file".into());
    }
    let legacy_target = object
        .get("target_monitor")
        .and_then(serde_json::Value::as_str)
        .ok_or("Legacy settings require a string target_monitor")?;
    let target = if legacy_target == "all" {
        TargetMonitor::All
    } else {
        TargetMonitor::NeedsConfirmation {
            legacy_runtime_id: legacy_target.to_owned(),
        }
    };
    object.insert(
        "target_monitor".into(),
        serde_json::to_value(target).map_err(|e| e.to_string())?,
    );
    let settings: AppConfig =
        serde_json::from_value(value).map_err(|e| format!("Invalid legacy settings: {e}"))?;
    settings.validate()?;
    // Import is lossless for recognized fields. Cleanup/enrichment belongs to explicit mutations.
    Ok(settings)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigMode {
    Ready,
    FirstRun,
    ImportAvailable,
    RecoveryRequired,
    UnsupportedSchema,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigSnapshot {
    pub settings: AppConfig,
    pub mode: ConfigMode,
    pub store_id: Option<String>,
    pub revision: String,
    pub context_token: String,
    pub library_generation: String,
    /// Orders every in-process publication, including nonpersistent mode/controller changes.
    pub control_epoch: String,
    pub issue: Option<String>,
    pub controller_issue: Option<String>,
    pub candidates: Vec<RecoveryCandidate>,
    pub config_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    pub target_monitor: Option<TargetMonitor>,
    pub alt_tab_delay_seconds: Option<u64>,
    pub notifications_enabled: Option<bool>,
    pub autostart: Option<bool>,
    pub start_minimized: Option<bool>,
    pub auto_detect_new_games: Option<bool>,
    pub auto_sync_database: Option<bool>,
    pub exit_only_hdr: Option<bool>,
    pub switch_method: Option<SwitchMethod>,
    pub blacklist: Option<Vec<String>>,
}

struct WriterState {
    storage: Storage,
    committed: Option<Document>,
    legacy_preview: Option<(Vec<u8>, AppConfig)>,
}

struct ControlState {
    snapshot: ConfigSnapshot,
    epoch: u64,
}

pub struct ConfigManager {
    writer: Mutex<WriterState>,
    control: RwLock<ControlState>,
}

impl ConfigManager {
    pub fn load(local_dir: PathBuf, legacy_path: PathBuf) -> Result<Self, String> {
        let mut storage = Storage::open(local_dir, legacy_path.clone())?;
        let mut report = storage.load();
        let mut legacy_preview = None;
        let mut settings = report
            .document
            .as_ref()
            .map(|d| d.envelope.settings.clone())
            .unwrap_or_default();
        if report.mode == ConfigMode::FirstRun {
            match read_optional(&legacy_path) {
                Ok(None) => {}
                Ok(Some(bytes)) => match decode_legacy(&bytes) {
                    Ok(imported) => {
                        settings = imported.clone();
                        legacy_preview = Some((bytes, imported));
                        report.mode = ConfigMode::ImportAvailable;
                    }
                    Err(error) => {
                        report.mode = ConfigMode::RecoveryRequired;
                        report.issue = Some(format!(
                            "Read legacy settings '{}': {error}",
                            legacy_path.display()
                        ));
                    }
                },
                Err(error) => {
                    report.mode = ConfigMode::Unavailable;
                    report.issue = Some(error);
                }
            }
        }
        if let Some(issue) = &report.issue {
            eprintln!("Configuration load is {:?}: {issue}", report.mode);
        }
        let snapshot = ConfigSnapshot {
            settings,
            mode: report.mode,
            store_id: report
                .document
                .as_ref()
                .map(|d| d.envelope.store_id.clone()),
            revision: report
                .document
                .as_ref()
                .map(|d| d.envelope.revision.clone())
                .unwrap_or_else(|| "0".into()),
            context_token: Uuid::new_v4().to_string(),
            library_generation: "0".into(),
            control_epoch: "0".into(),
            issue: report.issue,
            controller_issue: None,
            candidates: report.candidates,
            config_path: storage.main_path().display().to_string(),
        };
        Ok(Self {
            writer: Mutex::new(WriterState {
                storage,
                committed: report.document,
                legacy_preview,
            }),
            control: RwLock::new(ControlState { snapshot, epoch: 0 }),
        })
    }

    pub fn snapshot(&self) -> Result<ConfigSnapshot, String> {
        self.with_control_snapshot(Clone::clone)
    }

    /// This is the authorization/publication boundary, not a disk or native-call guard.
    /// The closure must only inspect/capture state and must not reenter the manager.
    pub fn with_control_snapshot<T>(
        &self,
        inspect: impl FnOnce(&ConfigSnapshot) -> T,
    ) -> Result<T, String> {
        let control = self
            .control
            .read()
            .map_err(|_| "Configuration gate is poisoned")?;
        Ok(inspect(&control.snapshot))
    }

    pub fn set_controller_issue(&self, issue: Option<String>) -> Result<ConfigSnapshot, String> {
        self.publish(|snapshot| snapshot.controller_issue = issue)
    }

    fn publish(&self, update: impl FnOnce(&mut ConfigSnapshot)) -> Result<ConfigSnapshot, String> {
        let mut control = self
            .control
            .write()
            .map_err(|_| "Configuration gate is poisoned")?;
        let epoch = control
            .epoch
            .checked_add(1)
            .ok_or("Control epoch exhausted")?;
        update(&mut control.snapshot);
        control.epoch = epoch;
        control.snapshot.control_epoch = epoch.to_string();
        Ok(control.snapshot.clone())
    }

    pub fn patch(
        &self,
        expected_context: &str,
        patch: SettingsPatch,
    ) -> Result<ConfigSnapshot, String> {
        self.mutate(expected_context, None, false, move |settings| {
            if matches!(patch.switch_method, Some(SwitchMethod::Shortcut)) {
                return Err(
                    "Shortcut automation is not supported; explicitly accept Native".into(),
                );
            }
            macro_rules! assign {
                ($($field:ident),+ $(,)?) => {
                    $(if let Some(value) = patch.$field {
                        settings.$field = value;
                    })+
                };
            }
            assign!(
                target_monitor,
                alt_tab_delay_seconds,
                notifications_enabled,
                autostart,
                start_minimized,
                auto_detect_new_games,
                auto_sync_database,
                exit_only_hdr,
                switch_method,
                blacklist,
            );
            Ok(())
        })
    }

    /// A mutation operates on the latest committed settings under the writer mutex.
    /// The closure must not perform I/O, call save, or recursively enter a mutation.
    pub fn mutate<F>(
        &self,
        expected_context: &str,
        expected_library_generation: Option<&str>,
        changes_library: bool,
        update: F,
    ) -> Result<ConfigSnapshot, String>
    where
        F: FnOnce(&mut AppConfig) -> Result<(), String>,
    {
        self.mutate_inner(expected_context, expected_library_generation, changes_library, false, update)
            .map(|(snapshot, _)| snapshot)
    }

    pub fn mutate_if_changed(
        &self,
        expected_context: &str,
        expected_library_generation: Option<&str>,
        changes_library: bool,
        update: impl FnOnce(&mut AppConfig) -> Result<(), String>,
    ) -> Result<Option<ConfigSnapshot>, String> {
        self.mutate_inner(expected_context, expected_library_generation, changes_library, true, update)
            .map(|(snapshot, changed)| changed.then_some(snapshot))
    }

    fn mutate_inner(
        &self,
        expected_context: &str,
        expected_library_generation: Option<&str>,
        changes_library: bool,
        skip_unchanged: bool,
        update: impl FnOnce(&mut AppConfig) -> Result<(), String>,
    ) -> Result<(ConfigSnapshot, bool), String> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "Configuration writer is poisoned")?;
        let before = self.check_context(expected_context)?;
        if before.mode != ConfigMode::Ready {
            return Err(format!("Settings are read-only in {:?} mode", before.mode));
        }
        if expected_library_generation
            .is_some_and(|generation| generation != before.library_generation)
        {
            return Err("Conflict: the game library changed; refresh/reconfirm this work".into());
        }
        let predecessor = writer
            .committed
            .as_ref()
            .ok_or("No committed settings are available")?
            .clone();
        let mut settings = predecessor.envelope.settings.clone();
        update(&mut settings)?;
        settings.validate()?;
        if settings.switch_method == SwitchMethod::Shortcut
            && predecessor.envelope.settings.switch_method != SwitchMethod::Shortcut
        {
            return Err("Shortcut automation cannot be enabled".into());
        }
        if skip_unchanged && settings == predecessor.envelope.settings {
            if let Err(error) = writer.storage.verify_unchanged(&predecessor) {
                return self.block(&mut writer, error).map(|snapshot| (snapshot, false));
            }
            return self.snapshot().map(|snapshot| (snapshot, false));
        }
        // A caller cannot accidentally omit the library fence for an actual library edit.
        let changes_library =
            changes_library || settings.apps != predecessor.envelope.settings.apps;
        let generation = before
            .library_generation
            .parse::<u64>()
            .map_err(|_| "Invalid library generation")?
            .checked_add(u64::from(changes_library))
            .ok_or("Library generation exhausted")?;
        let candidate = predecessor.successor(settings)?;
        let outcome = writer.storage.commit(&predecessor, candidate);
        self.finish(&mut writer, outcome, false, generation)
            .map(|snapshot| (snapshot, true))
    }

    pub fn initialize(&self, expected_context: &str) -> Result<ConfigSnapshot, String> {
        self.install(expected_context, InstallRequest::Initialize)
    }

    pub fn import_legacy(&self, expected_context: &str) -> Result<ConfigSnapshot, String> {
        self.install(expected_context, InstallRequest::ImportLegacy)
    }

    pub fn restore(
        &self,
        expected_context: &str,
        candidate_id: &str,
    ) -> Result<ConfigSnapshot, String> {
        self.install(expected_context, InstallRequest::Restore(candidate_id))
    }

    pub fn reset(&self, expected_context: &str) -> Result<ConfigSnapshot, String> {
        self.install(expected_context, InstallRequest::Reset)
    }

    fn install(
        &self,
        expected_context: &str,
        request: InstallRequest<'_>,
    ) -> Result<ConfigSnapshot, String> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "Configuration writer is poisoned")?;
        let before = self.check_context(expected_context)?;
        if before.mode == ConfigMode::UnsupportedSchema {
            return Err(
                "A compatible app version is required; restore/reset cannot downgrade settings"
                    .into(),
            );
        }
        let (settings, source, fresh_only) = match request {
            InstallRequest::Initialize => {
                if before.mode != ConfigMode::FirstRun {
                    return Err("Initialization requires the current first-run context".into());
                }
                // A legacy file appearing after bootstrap is not an empty first run.
                match read_optional(writer.storage.legacy_path()) {
                    Ok(None) => {}
                    Ok(Some(_)) => {
                        return self.block(
                            &mut writer,
                            "Conflict: legacy settings appeared after first-run inspection".into(),
                        );
                    }
                    Err(error) => return self.block(&mut writer, error),
                }
                (AppConfig::default(), None, true)
            }
            InstallRequest::ImportLegacy => {
                if before.mode != ConfigMode::ImportAvailable {
                    return Err("Import requires the current legacy-import preview".into());
                }
                let (bytes, settings) = writer
                    .legacy_preview
                    .as_ref()
                    .ok_or("No legacy import preview is available")?;
                (
                    settings.clone(),
                    Some(InstallSource::Legacy {
                        bytes: bytes.clone(),
                    }),
                    true,
                )
            }
            InstallRequest::Restore(id) => {
                let (settings, source) = match writer.storage.restore_source(id) {
                    Ok(source) => source,
                    Err(RestoreSourceError::UnknownCandidate) => {
                        return Err("Unknown or expired recovery candidate".into());
                    }
                    Err(RestoreSourceError::Blocked(error)) => {
                        return self.block(&mut writer, error);
                    }
                };
                (settings, Some(source), false)
            }
            InstallRequest::Reset => (AppConfig::default(), None, false),
        };
        let outcome = writer.storage.install(settings, source, fresh_only);
        self.finish(&mut writer, outcome, true, 0)
    }

    fn check_context(&self, expected: &str) -> Result<ConfigSnapshot, String> {
        let snapshot = self.snapshot()?;
        if snapshot.context_token != expected {
            return Err(
                "Conflict: this configuration context was retired; refresh before retrying".into(),
            );
        }
        Ok(snapshot)
    }

    fn finish(
        &self,
        writer: &mut WriterState,
        outcome: StoreOutcome,
        new_history: bool,
        generation: u64,
    ) -> Result<ConfigSnapshot, String> {
        match outcome {
            StoreOutcome::Committed {
                document,
                candidates,
                issue,
            } => {
                let result = self.publish(|snapshot| {
                    snapshot.settings = document.envelope.settings.clone();
                    snapshot.mode = ConfigMode::Ready;
                    snapshot.store_id = Some(document.envelope.store_id.clone());
                    snapshot.revision = document.envelope.revision.clone();
                    snapshot.library_generation = generation.to_string();
                    snapshot.issue = issue;
                    snapshot.candidates = candidates;
                    if new_history {
                        snapshot.context_token = Uuid::new_v4().to_string();
                    }
                    // Controller exclusion may have changed while persistence was in flight.
                    // publish intentionally leaves the current controller_issue untouched.
                })?;
                writer.committed = Some(document);
                writer.legacy_preview = None;
                Ok(result)
            }
            StoreOutcome::NotCommitted(error) => {
                eprintln!("Configuration was not committed: {error}");
                self.publish(|snapshot| snapshot.issue = Some(error.clone()))?;
                Err(error)
            }
            StoreOutcome::Blocked(error) => self.block(writer, error),
        }
    }

    fn block(&self, writer: &mut WriterState, error: String) -> Result<ConfigSnapshot, String> {
        eprintln!("Configuration persistence is blocked: {error}");
        // Close automatic authority before potentially slow recovery inventory I/O.
        // Recovery details may refine this state, but cannot leave a known-uncertain
        // configuration authorizing operations while its files are inspected.
        self.publish(|snapshot| {
            snapshot.mode = ConfigMode::RecoveryRequired;
            snapshot.issue = Some(error.clone());
            snapshot.candidates.clear();
            snapshot.context_token = Uuid::new_v4().to_string();
        })?;
        let StorageReport {
            mode,
            candidates,
            issue,
            ..
        } = writer.storage.blocked_report(&error);
        self.publish(|snapshot| {
            snapshot.mode = mode;
            snapshot.issue = issue;
            snapshot.candidates = candidates;
        })?;
        Err(error)
    }
}

enum InstallRequest<'a> {
    Initialize,
    ImportLegacy,
    Restore(&'a str),
    Reset,
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::config_storage::TestFault;
    use std::fs;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    struct Fixture {
        root: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                root: tempfile::Builder::new()
                    .prefix(".config-test-")
                    .tempdir_in(std::env::current_dir().unwrap())
                    .unwrap(),
            }
        }

        fn local(&self) -> PathBuf {
            self.root.path().join("local")
        }

        fn main(&self) -> PathBuf {
            self.local().join("config-v2.json")
        }

        fn legacy(&self) -> PathBuf {
            self.root.path().join("legacy.json")
        }

        fn load(&self) -> ConfigManager {
            ConfigManager::load(self.local(), self.legacy()).unwrap()
        }

        fn ready(&self) -> (ConfigManager, ConfigSnapshot) {
            let manager = self.load();
            let first = manager.snapshot().unwrap();
            assert_eq!(first.mode, ConfigMode::FirstRun);
            let ready = manager.initialize(&first.context_token).unwrap();
            (manager, ready)
        }

        fn candidate_path(&self, manager: &ConfigManager, id: &str) -> PathBuf {
            let writer = manager.writer.lock().unwrap();
            let (_, source) = writer.storage.restore_source(id).unwrap();
            let InstallSource::Artifact { name, .. } = source else {
                panic!("Expected a registered recovery artifact");
            };
            self.local().join(name)
        }
    }

    fn app(exe: &str) -> HdrApp {
        HdrApp {
            name: "Example Game".into(),
            exe_name: exe.into(),
            enabled: false,
            hdr_type: HdrType::AutoHdr,
            path: Some(r"C:\Games\Example\game.exe".into()),
            alternate_exes: vec!["game-win64-shipping.exe".into()],
            steam_id: None,
            launcher: Some("Custom launcher".into()),
        }
    }

    #[test]
    fn unchanged_mutation_preserves_revision_generation_epoch_and_artifacts() {
        let fixture = Fixture::new();
        let (manager, ready) = fixture.ready();
        let bytes = artifact_bytes(&fixture);
        let result = manager.mutate_if_changed(
            &ready.context_token, Some(&ready.library_generation), true, |_| Ok(()),
        ).unwrap();
        assert!(result.is_none());
        assert_eq!(manager.snapshot().unwrap(), ready);
        assert_eq!(artifact_bytes(&fixture), bytes);
        let mut called = false;
        assert!(manager.mutate_if_changed(&ready.context_token, Some("999"), true, |_| {
            called = true;
            Ok(())
        }).is_err());
        assert!(!called);
    }

    #[test]
    fn unchanged_mutation_still_retires_authority_on_a_persistence_conflict() {
        let fixture = Fixture::new();
        let (manager, ready) = fixture.ready();
        fs::write(fixture.main(), b"external conflicting bytes").unwrap();
        let bytes = artifact_bytes(&fixture);
        assert!(manager.mutate_if_changed(
            &ready.context_token, None, true, |_| Ok(()),
        ).is_err());
        let blocked = manager.snapshot().unwrap();
        assert_eq!(blocked.mode, ConfigMode::RecoveryRequired);
        assert_ne!(blocked.context_token, ready.context_token);
        assert_eq!(artifact_bytes(&fixture), bytes);
    }

    #[test]
    fn no_op_detection_uses_latest_settings_under_writer_serialization() {
        let fixture = Fixture::new();
        let (manager, ready) = fixture.ready();
        let manager = Arc::new(manager);
        let (entered, waiting) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let writer = manager.clone();
        let context = ready.context_token.clone();
        let first = std::thread::spawn(move || {
            writer.mutate(&context, None, true, |settings| {
                settings.apps.push(app("game.exe"));
                entered.send(()).unwrap();
                released.recv().unwrap();
                Ok(())
            }).unwrap()
        });
        waiting.recv().unwrap();
        let next_writer = manager.clone();
        let context = ready.context_token;
        let second = std::thread::spawn(move || {
            next_writer.mutate_if_changed(&context, None, true, |settings| {
                assert_eq!(settings.apps, vec![app("game.exe")]);
                crate::library::enrich_existing(settings, &[app("game.exe")]);
                Ok(())
            }).unwrap()
        });
        release.send(()).unwrap();
        let committed = first.join().unwrap();
        assert!(second.join().unwrap().is_none());
        assert_eq!(manager.snapshot().unwrap(), committed);
        let changed = manager.mutate_if_changed(
            &committed.context_token, Some(&committed.library_generation), true,
            |settings| {
                settings.apps[0].alternate_exes.push("game-dx12.exe".into());
                Ok(())
            },
        ).unwrap().unwrap();
        assert_eq!(changed.revision.parse::<u64>().unwrap(), committed.revision.parse::<u64>().unwrap() + 1);
        assert_eq!(changed.library_generation.parse::<u64>().unwrap(), committed.library_generation.parse::<u64>().unwrap() + 1);
    }

    fn legacy_bytes(settings: &AppConfig, target: &str) -> Vec<u8> {
        let mut value = serde_json::to_value(settings).unwrap();
        value["target_monitor"] = target.into();
        serde_json::to_vec_pretty(&value).unwrap()
    }

    fn write_future_intent(fixture: &Fixture, quarantined: bool) -> (PathBuf, Vec<u8>) {
        let transaction = Uuid::new_v4().to_string();
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "protocol_version": 1,
            "candidate": {
                "schema_version": 3,
                "store_id": Uuid::new_v4().to_string(),
                "revision": "1",
                "transaction_id": transaction.clone(),
                "settings": AppConfig::default(),
            },
            "operation": { "kind": "install", "source": null, "preserved": [] },
        }))
        .unwrap();
        let role = if quarantined { "quarantine" } else { "intent" };
        let path = fixture
            .local()
            .join(format!("config-v2.{role}-{transaction}.json"));
        fs::create_dir_all(fixture.local()).unwrap();
        fs::write(&path, &bytes).unwrap();
        (path, bytes)
    }

    fn artifact_bytes(fixture: &Fixture) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        fs::read_dir(fixture.local())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("config-v2.")
            })
            .map(|path| {
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }

    #[test]
    fn legacy_import_preserves_preferences_rows_and_explicit_native_consent() {
        for target in ["all", "123_456_7"] {
            let fixture = Fixture::new();
            let mut original = AppConfig {
                switch_method: SwitchMethod::Shortcut,
                alt_tab_delay_seconds: 9,
                autostart: true,
                start_minimized: true,
                auto_detect_new_games: false,
                auto_sync_database: false,
                exit_only_hdr: false,
                notifications_enabled: false,
                last_sync_timestamp: Some(123456),
                ..AppConfig::default()
            };
            original.apps = vec![app("chrome.exe"), app("chrome.exe")];
            original.blacklist = vec!["custom.exe".into(), "chrome.exe".into()];
            let bytes = legacy_bytes(&original, target);
            fs::write(fixture.legacy(), &bytes).unwrap();
            let manager = fixture.load();
            let preview = manager.snapshot().unwrap();
            assert_eq!(preview.mode, ConfigMode::ImportAvailable);
            assert!(!fixture.main().exists());
            original.target_monitor = if target == "all" {
                TargetMonitor::All
            } else {
                TargetMonitor::NeedsConfirmation {
                    legacy_runtime_id: target.into(),
                }
            };
            assert_eq!(preview.settings, original);
            let imported = manager.import_legacy(&preview.context_token).unwrap();
            assert_eq!(imported.settings, original);
            assert_eq!(imported.revision, "1");
            assert_eq!(imported.mode, ConfigMode::Ready);
            assert_ne!(imported.context_token, preview.context_token);
            assert_eq!(fs::read(fixture.legacy()).unwrap(), bytes);
            assert!(manager
                .patch(
                    &imported.context_token,
                    SettingsPatch {
                        switch_method: Some(SwitchMethod::Shortcut),
                        ..SettingsPatch::default()
                    }
                )
                .is_err());
            let accepted = manager
                .patch(
                    &imported.context_token,
                    SettingsPatch {
                        switch_method: Some(SwitchMethod::Native),
                        ..SettingsPatch::default()
                    },
                )
                .unwrap();
            assert_eq!(accepted.settings.switch_method, SwitchMethod::Native);
            assert_eq!(accepted.settings.apps, original.apps);
            assert_eq!(fs::read(fixture.legacy()).unwrap(), bytes);
        }
    }

    #[test]
    fn recognized_legacy_defaults_are_applied_without_library_cleanup() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&legacy_bytes(&AppConfig::default(), "old-runtime-id")).unwrap();
        for key in [
            "start_minimized",
            "auto_detect_new_games",
            "auto_sync_database",
            "last_sync_timestamp",
            "exit_only_hdr",
        ] {
            value.as_object_mut().unwrap().remove(key);
        }
        let imported = decode_legacy(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            imported.auto_detect_new_games && imported.auto_sync_database && imported.exit_only_hdr
        );
        assert!(!imported.start_minimized);
        assert_eq!(imported.last_sync_timestamp, None);
        assert!(matches!(
            imported.target_monitor,
            TargetMonitor::NeedsConfirmation { .. }
        ));
    }

    #[test]
    fn strict_v2_legacy_omissions_still_import_to_complete_canonical_settings() {
        for target in ["all", "legacy-display-id"] {
            let fixture = Fixture::new();
            let bytes = serde_json::to_vec(&serde_json::json!({
                "target_monitor": target,
                "alt_tab_delay_seconds": 9,
                "notifications_enabled": false,
                "autostart": true,
                "switch_method": "Shortcut",
                "blacklist": ["custom.exe"],
                "apps": [{
                    "name": "Legacy game",
                    "exe_name": "legacy-game.exe",
                    "enabled": true,
                    "hdr_type": "AutoHDR",
                    "unknown_legacy_app_field": true,
                }],
                "unknown_legacy_setting": true,
            }))
            .unwrap();
            fs::write(fixture.legacy(), &bytes).unwrap();
            let manager = fixture.load();
            let preview = manager.snapshot().unwrap();
            assert_eq!(preview.mode, ConfigMode::ImportAvailable);
            let imported = manager.import_legacy(&preview.context_token).unwrap();
            assert_eq!(imported.mode, ConfigMode::Ready);
            assert_eq!(imported.settings, preview.settings);
            assert_eq!(imported.settings.switch_method, SwitchMethod::Shortcut);
            assert_eq!(imported.settings.apps[0].hdr_type, HdrType::AutoHdr);
            assert!(imported.settings.auto_detect_new_games);
            assert!(imported.settings.auto_sync_database);
            assert!(imported.settings.exit_only_hdr);
            assert!(!imported.settings.start_minimized);
            assert_eq!(imported.settings.last_sync_timestamp, None);
            assert_eq!(imported.settings.apps[0].path, None);
            assert_eq!(imported.settings.apps[0].steam_id, None);
            assert_eq!(imported.settings.apps[0].launcher, None);
            assert!(imported.settings.apps[0].alternate_exes.is_empty());
            let persisted: crate::config_storage::Envelope =
                serde_json::from_slice(&fs::read(fixture.main()).unwrap()).unwrap();
            assert_eq!(persisted.settings, imported.settings);
            assert_eq!(fs::read(fixture.legacy()).unwrap(), bytes);
            drop(manager);
            let reloaded = fixture.load().snapshot().unwrap();
            assert_eq!(reloaded.mode, ConfigMode::Ready);
            assert_eq!(reloaded.settings, imported.settings);
        }
    }

    #[test]
    fn loading_never_saves_or_silently_reimports_and_a_lock_filename_is_harmless() {
        let fixture = Fixture::new();
        let manager = fixture.load();
        assert_eq!(manager.snapshot().unwrap().mode, ConfigMode::FirstRun);
        assert!(!fixture.main().exists());
        drop(manager);
        let (manager, ready) = fixture.ready();
        let bytes = fs::read(fixture.main()).unwrap();
        let modified = fs::metadata(fixture.main()).unwrap().modified().unwrap();
        drop(manager);
        fs::write(
            fixture.legacy(),
            legacy_bytes(
                &AppConfig {
                    autostart: true,
                    ..AppConfig::default()
                },
                "all",
            ),
        )
        .unwrap();
        let loaded = fixture.load().snapshot().unwrap();
        assert_eq!(loaded.mode, ConfigMode::Ready);
        assert_eq!(loaded.settings, ready.settings);
        assert_eq!(loaded.store_id, ready.store_id);
        assert_eq!(loaded.revision, ready.revision);
        assert_eq!(fs::read(fixture.main()).unwrap(), bytes);
        assert_eq!(
            fs::metadata(fixture.main()).unwrap().modified().unwrap(),
            modified
        );
    }

    #[test]
    fn corrupt_unsupported_and_unreadable_files_are_blocked_not_defaulted() {
        for (bytes, expected_mode) in [
            (b"{broken json".to_vec(), ConfigMode::RecoveryRequired),
            (
                br#"{"schema_version":99}"#.to_vec(),
                ConfigMode::UnsupportedSchema,
            ),
        ] {
            let fixture = Fixture::new();
            fs::create_dir_all(fixture.local()).unwrap();
            fs::write(fixture.main(), &bytes).unwrap();
            let manager = fixture.load();
            let snapshot = manager.snapshot().unwrap();
            assert_eq!(snapshot.mode, expected_mode);
            assert!(manager
                .patch(&snapshot.context_token, SettingsPatch::default())
                .is_err());
            assert!(manager.initialize(&snapshot.context_token).is_err());
            if expected_mode == ConfigMode::UnsupportedSchema {
                assert!(manager.reset(&snapshot.context_token).is_err());
                assert!(manager
                    .restore(&snapshot.context_token, &Uuid::new_v4().to_string())
                    .is_err());
            }
            assert_eq!(fs::read(fixture.main()).unwrap(), bytes);
        }
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.main()).unwrap();
        let manager = fixture.load();
        let snapshot = manager.snapshot().unwrap();
        assert_eq!(snapshot.mode, ConfigMode::Unavailable);
        assert!(manager.reset(&snapshot.context_token).is_err());
        assert!(fixture.main().is_dir());

        let fixture = Fixture::new();
        fs::write(fixture.legacy(), b"invalid legacy").unwrap();
        let manager = fixture.load();
        assert_eq!(
            manager.snapshot().unwrap().mode,
            ConfigMode::RecoveryRequired
        );
        assert!(!fixture.main().exists());
        assert_eq!(fs::read(fixture.legacy()).unwrap(), b"invalid legacy");
    }

    #[test]
    fn future_intent_or_quarantined_intent_is_read_only_without_main_or_staging() {
        for quarantined in [false, true] {
            let fixture = Fixture::new();
            let (evidence, bytes) = write_future_intent(&fixture, quarantined);
            let original = artifact_bytes(&fixture);
            let manager = fixture.load();
            let snapshot = manager.snapshot().unwrap();
            assert_eq!(snapshot.mode, ConfigMode::UnsupportedSchema);
            assert!(manager.reset(&snapshot.context_token).is_err());
            assert!(manager
                .restore(&snapshot.context_token, &Uuid::new_v4().to_string())
                .is_err());
            assert_eq!(
                manager.snapshot().unwrap().mode,
                ConfigMode::UnsupportedSchema
            );
            assert_eq!(fs::read(evidence).unwrap(), bytes);
            assert_eq!(artifact_bytes(&fixture), original);
            assert!(!fixture.main().exists());
        }
    }

    #[test]
    fn future_intent_detected_after_bootstrap_blocks_reset_and_restore_admission() {
        for quarantined in [false, true] {
            for restoring in [false, true] {
                let fixture = Fixture::new();
                let (manager, initial) = fixture.ready();
                let latest = manager
                    .patch(
                        &initial.context_token,
                        SettingsPatch {
                            autostart: Some(true),
                            ..SettingsPatch::default()
                        },
                    )
                    .unwrap();
                let checkpoint = latest.candidates[0].id.clone();
                fs::remove_file(fixture.main()).unwrap();
                let (evidence, bytes) = write_future_intent(&fixture, quarantined);
                let original = artifact_bytes(&fixture);
                let result = if restoring {
                    manager.restore(&latest.context_token, &checkpoint)
                } else {
                    manager.reset(&latest.context_token)
                };
                assert!(result.is_err());
                let blocked = manager.snapshot().unwrap();
                assert_eq!(blocked.mode, ConfigMode::UnsupportedSchema);
                assert_eq!(blocked.settings, latest.settings);
                assert_eq!(blocked.revision, latest.revision);
                assert_eq!(fs::read(evidence).unwrap(), bytes);
                assert_eq!(artifact_bytes(&fixture), original);
                assert!(!fixture.main().exists());
            }
        }
    }

    #[test]
    fn restore_source_conflicts_retire_authority_and_preserve_evidence() {
        for future_schema in [true, false] {
            let fixture = Fixture::new();
            let (manager, initial) = fixture.ready();
            let latest = manager
                .patch(
                    &initial.context_token,
                    SettingsPatch {
                        autostart: Some(true),
                        ..SettingsPatch::default()
                    },
                )
                .unwrap();
            let candidate = &latest.candidates[0].id;
            let path = fixture.candidate_path(&manager, candidate);
            let replacement = if future_schema {
                br#"{"schema_version":3}"#.to_vec()
            } else {
                fs::read(fixture.main()).unwrap()
            };
            fs::write(&path, &replacement).unwrap();
            let original = artifact_bytes(&fixture);

            let error = manager
                .restore(&latest.context_token, candidate)
                .unwrap_err();
            let blocked = manager.snapshot().unwrap();
            assert_eq!(
                blocked.mode,
                if future_schema {
                    ConfigMode::UnsupportedSchema
                } else {
                    ConfigMode::RecoveryRequired
                }
            );
            assert_eq!(blocked.settings, latest.settings);
            assert_eq!(blocked.store_id, latest.store_id);
            assert_eq!(blocked.revision, latest.revision);
            assert_eq!(blocked.library_generation, latest.library_generation);
            assert_ne!(blocked.context_token, latest.context_token);
            assert!(
                blocked.control_epoch.parse::<u64>().unwrap()
                    > latest.control_epoch.parse().unwrap()
            );
            assert!(blocked.issue.as_ref().unwrap().contains(&error));
            assert!(blocked.candidates.iter().all(|candidate| {
                latest.candidates.iter().all(|prior| prior.id != candidate.id)
            }));
            let mut called = false;
            assert!(manager
                .mutate(&latest.context_token, None, false, |_| {
                    called = true;
                    Ok(())
                })
                .is_err());
            assert!(!called);
            assert_eq!(artifact_bytes(&fixture), original);

            drop(manager);
            assert_eq!(fixture.load().snapshot().unwrap().mode, blocked.mode);
            assert_eq!(artifact_bytes(&fixture), original);
        }
    }

    #[test]
    fn restore_source_read_failures_retire_authority_without_writing() {
        use std::os::windows::fs::OpenOptionsExt;

        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let latest = manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        let candidate = &latest.candidates[0].id;
        let path = fixture.candidate_path(&manager, candidate);
        let original = artifact_bytes(&fixture);
        let deny_read = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap();

        let error = manager
            .restore(&latest.context_token, candidate)
            .unwrap_err();
        let blocked = manager.snapshot().unwrap();
        assert_eq!(blocked.mode, ConfigMode::RecoveryRequired);
        assert_eq!(blocked.settings, latest.settings);
        assert_eq!(blocked.store_id, latest.store_id);
        assert_eq!(blocked.revision, latest.revision);
        assert_ne!(blocked.context_token, latest.context_token);
        assert!(
            blocked.control_epoch.parse::<u64>().unwrap()
                > latest.control_epoch.parse().unwrap()
        );
        assert!(blocked.candidates.is_empty());
        assert!(blocked.issue.as_ref().unwrap().contains(&error));

        drop(deny_read);
        assert_eq!(artifact_bytes(&fixture), original);
        drop(manager);
        assert_eq!(fixture.load().snapshot().unwrap().mode, ConfigMode::Ready);
        assert_eq!(artifact_bytes(&fixture), original);
    }

    #[test]
    fn restore_unknown_and_expired_candidates_leave_authority_unchanged() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let edited = manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        let expired = &edited.candidates[0].id;
        let latest = manager
            .patch(
                &edited.context_token,
                SettingsPatch {
                    start_minimized: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        let original = artifact_bytes(&fixture);
        let unknown = Uuid::new_v4().to_string();
        for candidate in [r"..\other-settings.json", &unknown, expired] {
            assert_eq!(
                manager
                    .restore(&latest.context_token, candidate)
                    .unwrap_err(),
                "Unknown or expired recovery candidate"
            );
            assert_eq!(manager.snapshot().unwrap(), latest);
            assert_eq!(artifact_bytes(&fixture), original);
        }
    }

    #[test]
    fn lifetime_lock_and_read_sharing_failures_are_real_windows_exclusion() {
        use std::os::windows::fs::OpenOptionsExt;
        let fixture = Fixture::new();
        let (manager, _) = fixture.ready();
        assert!(ConfigManager::load(fixture.local(), fixture.legacy()).is_err());
        drop(manager);
        let bytes = fs::read(fixture.main()).unwrap();
        let deny_read = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(fixture.main())
            .unwrap();
        let manager = fixture.load();
        let snapshot = manager.snapshot().unwrap();
        assert_eq!(snapshot.mode, ConfigMode::Unavailable);
        assert!(!snapshot.issue.unwrap().is_empty());
        drop(deny_read);
        drop(manager);
        assert_eq!(fixture.load().snapshot().unwrap().mode, ConfigMode::Ready);
        assert_eq!(fs::read(fixture.main()).unwrap(), bytes);
    }

    #[test]
    fn contexts_and_library_generations_fence_retired_work_before_running_closures() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let added = manager
            .mutate(
                &initial.context_token,
                Some(&initial.library_generation),
                true,
                |settings| {
                    settings.apps.push(app("game.exe"));
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(added.library_generation, "1");
        let mut called = false;
        assert!(manager
            .mutate(
                &initial.context_token,
                Some(&initial.library_generation),
                true,
                |_| {
                    called = true;
                    Ok(())
                },
            )
            .is_err());
        assert!(!called);
        let patched = manager
            .patch(
                &added.context_token,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        assert_eq!(patched.context_token, initial.context_token);
        assert_eq!(patched.library_generation, added.library_generation);
        assert_eq!(patched.settings.apps, added.settings.apps);
        let reset = manager.reset(&patched.context_token).unwrap();
        assert_ne!(reset.context_token, patched.context_token);
        assert_ne!(reset.store_id, patched.store_id);
        assert_eq!(reset.revision, "1");
        assert!(manager
            .mutate(&patched.context_token, None, true, |_| {
                called = true;
                Ok(())
            })
            .is_err());
        assert!(!called);
        assert!(reset.settings.apps.is_empty());
    }

    #[test]
    fn restoring_old_content_creates_a_new_history_and_retires_pending_writers() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let edited = manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        let candidate = &edited.candidates[0];
        let restored = manager
            .restore(&edited.context_token, &candidate.id)
            .unwrap();
        assert_eq!(restored.settings, initial.settings);
        assert_ne!(restored.store_id, edited.store_id);
        assert_ne!(restored.context_token, edited.context_token);
        assert_eq!(restored.revision, "1");
        assert!(manager
            .patch(
                &edited.context_token,
                SettingsPatch {
                    start_minimized: Some(true),
                    ..SettingsPatch::default()
                }
            )
            .is_err());
        drop(manager);
        assert_eq!(
            fixture.load().snapshot().unwrap().store_id,
            restored.store_id
        );
    }

    #[test]
    fn an_actual_library_change_cannot_omit_generation_invalidation() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let changed = manager
            .mutate(&initial.context_token, None, false, |settings| {
                settings.apps.push(app("game.exe"));
                Ok(())
            })
            .unwrap();
        assert_eq!(changed.library_generation, "1");
        assert!(manager
            .mutate(&initial.context_token, Some("0"), true, |_| Ok(()))
            .is_err());
    }

    #[test]
    fn independent_concurrent_patches_merge_into_latest_committed_settings() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let manager = Arc::new(manager);
        let barrier = Arc::new(Barrier::new(3));
        let threads: Vec<_> = [
            SettingsPatch {
                autostart: Some(true),
                ..SettingsPatch::default()
            },
            SettingsPatch {
                start_minimized: Some(true),
                ..SettingsPatch::default()
            },
        ]
        .into_iter()
        .map(|patch| {
            let manager = Arc::clone(&manager);
            let barrier = Arc::clone(&barrier);
            let context = initial.context_token.clone();
            std::thread::spawn(move || {
                barrier.wait();
                manager.patch(&context, patch).unwrap();
            })
        })
        .collect();
        barrier.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        let snapshot = manager.snapshot().unwrap();
        assert!(snapshot.settings.autostart && snapshot.settings.start_minimized);
        assert_eq!(snapshot.revision, "3");
        assert_eq!(snapshot.context_token, initial.context_token);
    }

    #[test]
    fn failed_commit_retains_memory_and_publishes_the_closed_gate() {
        for fault in [
            TestFault::StageWrite,
            TestFault::StageSync,
            TestFault::ReplaceNotCommitted,
        ] {
            let fixture = Fixture::new();
            let (manager, initial) = fixture.ready();
            let bytes = fs::read(fixture.main()).unwrap();
            let known_failure = matches!(fault, TestFault::ReplaceNotCommitted);
            manager.writer.lock().unwrap().storage.fault = Some(fault);
            assert!(manager
                .patch(
                    &initial.context_token,
                    SettingsPatch {
                        target_monitor: Some(TargetMonitor::Monitor {
                            device_path: r"\\?\DISPLAY#fixture".into(),
                            display_name: "Fixture display".into(),
                        }),
                        ..SettingsPatch::default()
                    }
                )
                .is_err());
            let failed = manager.snapshot().unwrap();
            assert_eq!(failed.settings, initial.settings);
            assert_eq!(failed.revision, initial.revision);
            assert_eq!(failed.library_generation, initial.library_generation);
            assert_eq!(fs::read(fixture.main()).unwrap(), bytes);
            assert!(
                failed.control_epoch.parse::<u64>().unwrap()
                    > initial.control_epoch.parse().unwrap()
            );
            assert_eq!(
                failed.mode,
                if known_failure {
                    ConfigMode::Ready
                } else {
                    ConfigMode::RecoveryRequired
                }
            );
            if known_failure {
                assert_eq!(failed.context_token, initial.context_token);
            }
        }
    }

    #[test]
    fn controller_conflict_publication_is_short_independent_and_not_persisted() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let bytes = fs::read(fixture.main()).unwrap();
        {
            let _disk_guard = manager.writer.lock().unwrap();
            let blocked = manager
                .set_controller_issue(Some("Legacy controller is running".into()))
                .unwrap();
            assert!(blocked.controller_issue.is_some());
            assert_ne!(blocked.control_epoch, initial.control_epoch);
        }
        assert_eq!(fs::read(fixture.main()).unwrap(), bytes);
        let saved = manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    start_minimized: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        assert_eq!(
            saved.controller_issue.as_deref(),
            Some("Legacy controller is running")
        );
        assert!(!String::from_utf8(fs::read(fixture.main()).unwrap())
            .unwrap()
            .contains("controller_issue"));
    }

    #[test]
    fn required_epoch_orders_nonpersistent_updates_and_new_histories() {
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let mut missing_epoch = serde_json::to_value(&initial).unwrap();
        missing_epoch
            .as_object_mut()
            .unwrap()
            .remove("control_epoch");
        assert!(serde_json::from_value::<ConfigSnapshot>(missing_epoch).is_err());

        let conflict = manager
            .set_controller_issue(Some("controller conflict".into()))
            .unwrap();
        assert_eq!(conflict.revision, initial.revision);
        assert_eq!(conflict.context_token, initial.context_token);
        let saved = manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
            .unwrap();
        assert_eq!(saved.context_token, initial.context_token);
        manager.writer.lock().unwrap().storage.fault = Some(TestFault::ReplaceNotCommitted);
        assert!(manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    start_minimized: Some(true),
                    ..SettingsPatch::default()
                }
            )
            .is_err());
        let failed = manager.snapshot().unwrap();
        assert_eq!(failed.revision, saved.revision);
        assert_eq!(failed.context_token, saved.context_token);
        let reset = manager.reset(&initial.context_token).unwrap();
        assert_ne!(reset.context_token, initial.context_token);
        assert_ne!(reset.store_id, initial.store_id);
        assert_eq!(reset.revision, "1");
        let cleared = manager.set_controller_issue(None).unwrap();
        assert_eq!(cleared.context_token, reset.context_token);
        let epochs: Vec<u64> = [initial, conflict, saved, failed, reset, cleared]
            .into_iter()
            .map(|snapshot| snapshot.control_epoch.parse().unwrap())
            .collect();
        assert!(epochs.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn uncertain_store_closes_authority_before_slow_recovery_inventory() {
        use std::sync::mpsc;
        use std::time::Duration;

        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        let manager = Arc::new(manager);
        fs::write(fixture.main(), b"external conflicting contents").unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        manager.writer.lock().unwrap().storage.fault = Some(TestFault::PauseReport {
            entered: entered_tx,
            release: release_rx,
        });
        let writer = Arc::clone(&manager);
        let context = initial.context_token.clone();
        let saving = std::thread::spawn(move || {
            writer.patch(
                &context,
                SettingsPatch {
                    autostart: Some(true),
                    ..SettingsPatch::default()
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let during_io = manager.snapshot().unwrap();
        assert_eq!(during_io.mode, ConfigMode::RecoveryRequired);
        assert_ne!(during_io.context_token, initial.context_token);
        assert_eq!(during_io.settings, initial.settings);
        assert_eq!(during_io.revision, initial.revision);
        manager
            .set_controller_issue(Some("controller conflict".into()))
            .unwrap();
        release_tx.send(()).unwrap();
        assert!(saving.join().unwrap().is_err());
        assert_eq!(
            manager.snapshot().unwrap().controller_issue.as_deref(),
            Some("controller conflict")
        );
    }

    #[test]
    fn gate_publication_waits_for_capture_but_not_for_the_captured_work() {
        use std::sync::mpsc;
        use std::time::Duration;
        let fixture = Fixture::new();
        let (manager, _) = fixture.ready();
        let manager = Arc::new(manager);
        let (captured_tx, captured_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let capture = Arc::clone(&manager);
        let reader = std::thread::spawn(move || {
            capture
                .with_control_snapshot(|snapshot| {
                    captured_tx.send(snapshot.control_epoch.clone()).unwrap();
                    release_rx.recv().unwrap();
                })
                .unwrap();
        });
        let prior_epoch = captured_rx.recv().unwrap();
        let (published_tx, published_rx) = mpsc::channel();
        let publisher = Arc::clone(&manager);
        let writer = std::thread::spawn(move || {
            published_tx
                .send(
                    publisher
                        .set_controller_issue(Some("conflict".into()))
                        .unwrap(),
                )
                .unwrap();
        });
        assert!(published_rx
            .recv_timeout(Duration::from_millis(30))
            .is_err());
        release_tx.send(()).unwrap();
        let published = published_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(published.control_epoch.parse::<u64>().unwrap() > prior_epoch.parse().unwrap());
        reader.join().unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn settings_dtos_reject_whole_library_writes_and_invalid_paths() {
        assert!(serde_json::from_str::<SettingsPatch>(r#"{"apps":[]}"#).is_err());
        let monitor = TargetMonitor::Monitor {
            device_path: r"\\?\DISPLAY#opaque\path".into(),
            display_name: "Same model".into(),
        };
        assert_eq!(serde_json::to_value(&monitor).unwrap()["kind"], "monitor");
        let fixture = Fixture::new();
        let (manager, initial) = fixture.ready();
        assert!(manager
            .patch(
                &initial.context_token,
                SettingsPatch {
                    target_monitor: Some(TargetMonitor::Monitor {
                        device_path: String::new(),
                        display_name: "Label".into()
                    }),
                    ..SettingsPatch::default()
                }
            )
            .is_err());
        assert_eq!(manager.snapshot().unwrap(), initial);
        assert!(manager
            .restore(&initial.context_token, r"..\other-settings.json")
            .is_err());
    }

    #[test]
    fn changed_import_source_is_not_silently_retargeted() {
        let fixture = Fixture::new();
        fs::write(fixture.legacy(), legacy_bytes(&AppConfig::default(), "all")).unwrap();
        let manager = fixture.load();
        let preview = manager.snapshot().unwrap();
        let replacement = legacy_bytes(
            &AppConfig {
                autostart: true,
                ..AppConfig::default()
            },
            "different-tuple",
        );
        fs::write(fixture.legacy(), &replacement).unwrap();
        assert!(manager.import_legacy(&preview.context_token).is_err());
        assert_eq!(
            manager.snapshot().unwrap().mode,
            ConfigMode::RecoveryRequired
        );
        assert!(!fixture.main().exists());
        assert_eq!(fs::read(fixture.legacy()).unwrap(), replacement);
    }

    #[test]
    fn matching_is_exact_enabled_unscoped_and_never_fuzzy() {
        let mut settings = AppConfig::default();
        let mut game = app("ExampleGame.exe");
        game.enabled = true;
        game.path = None;
        settings.apps.push(game);
        assert!(settings.is_hdr_app("EXAMPLEGAME.EXE"));
        assert!(settings.is_hdr_app("GAME-WIN64-SHIPPING.EXE"));
        assert!(settings.find_app("ExampleGame_dx12.exe").is_none());
        assert!(!settings.is_hdr_app("ExampleGame_dx12.exe"));
        settings.blacklist.push("EXAMPLEGAME.EXE".into());
        assert!(!settings.is_hdr_app("ExampleGame.exe"));
        assert!(settings.find_app("ExampleGame.exe").is_none());
        assert!(settings.find_app("ex.exe").is_none());
    }
}

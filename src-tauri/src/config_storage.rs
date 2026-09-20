use crate::config_v2::{
    decode_legacy, AppConfig, ConfigMode, HdrApp, HdrType, RecoveryCandidate, SwitchMethod,
    TargetMonitor,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAIN: &str = "config-v2.json";
const PREFIX: &str = "config-v2.";
const SCHEMA: u64 = 2;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedSettings {
    target_monitor: PersistedTargetMonitor,
    alt_tab_delay_seconds: u64,
    notifications_enabled: bool,
    autostart: bool,
    start_minimized: bool,
    auto_detect_new_games: bool,
    auto_sync_database: bool,
    #[serde(deserialize_with = "deserialize_present_option")]
    last_sync_timestamp: Option<u64>,
    exit_only_hdr: bool,
    switch_method: SwitchMethod,
    blacklist: Vec<String>,
    apps: Vec<PersistedApp>,
}

impl From<PersistedSettings> for AppConfig {
    fn from(settings: PersistedSettings) -> Self {
        Self {
            target_monitor: settings.target_monitor.into(),
            alt_tab_delay_seconds: settings.alt_tab_delay_seconds,
            notifications_enabled: settings.notifications_enabled,
            autostart: settings.autostart,
            start_minimized: settings.start_minimized,
            auto_detect_new_games: settings.auto_detect_new_games,
            auto_sync_database: settings.auto_sync_database,
            last_sync_timestamp: settings.last_sync_timestamp,
            exit_only_hdr: settings.exit_only_hdr,
            switch_method: settings.switch_method,
            blacklist: settings.blacklist,
            apps: settings.apps.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedApp {
    name: String,
    exe_name: String,
    enabled: bool,
    hdr_type: HdrType,
    #[serde(deserialize_with = "deserialize_present_option")]
    path: Option<String>,
    alternate_exes: Vec<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    steam_id: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    launcher: Option<String>,
}

impl From<PersistedApp> for HdrApp {
    fn from(app: PersistedApp) -> Self {
        Self {
            name: app.name,
            exe_name: app.exe_name,
            enabled: app.enabled,
            hdr_type: app.hdr_type,
            path: app.path,
            alternate_exes: app.alternate_exes,
            steam_id: app.steam_id,
            launcher: app.launcher,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PersistedTargetMonitor {
    All {},
    Monitor {
        device_path: String,
        display_name: String,
    },
    NeedsConfirmation {
        legacy_runtime_id: String,
    },
}

impl From<PersistedTargetMonitor> for TargetMonitor {
    fn from(target: PersistedTargetMonitor) -> Self {
        match target {
            PersistedTargetMonitor::All {} => Self::All,
            PersistedTargetMonitor::Monitor {
                device_path,
                display_name,
            } => Self::Monitor {
                device_path,
                display_name,
            },
            PersistedTargetMonitor::NeedsConfirmation { legacy_runtime_id } => {
                Self::NeedsConfirmation { legacy_runtime_id }
            }
        }
    }
}

fn deserialize_settings<'de, D>(deserializer: D) -> Result<AppConfig, D::Error>
where
    D: Deserializer<'de>,
{
    PersistedSettings::deserialize(deserializer).map(Into::into)
}

// deserialize_with prevents Serde from treating a missing Option field as an implicit null.
fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    pub schema_version: u64,
    pub store_id: String,
    pub revision: String,
    pub transaction_id: String,
    #[serde(deserialize_with = "deserialize_settings")]
    pub settings: AppConfig,
}

impl Envelope {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA {
            return Err("Unsupported envelope schema".into());
        }
        validate_uuid(&self.store_id)?;
        validate_uuid(&self.transaction_id)?;
        let revision = self
            .revision
            .parse::<u64>()
            .map_err(|_| "Invalid revision")?;
        if revision == 0 || revision.to_string() != self.revision {
            return Err("Revision must be a positive canonical decimal string".into());
        }
        self.settings.validate()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub envelope: Envelope,
    pub bytes: Vec<u8>,
}

impl Document {
    fn fresh(settings: AppConfig) -> Result<Self, String> {
        Self::canonical(Envelope {
            schema_version: SCHEMA,
            store_id: Uuid::new_v4().to_string(),
            revision: "1".into(),
            transaction_id: Uuid::new_v4().to_string(),
            settings,
        })
    }

    fn canonical(envelope: Envelope) -> Result<Self, String> {
        envelope.validate()?;
        let bytes = json_bytes(&envelope)?;
        Ok(Self { envelope, bytes })
    }

    pub fn successor(&self, settings: AppConfig) -> Result<Self, String> {
        let revision = self
            .envelope
            .revision
            .parse::<u64>()
            .map_err(|_| "Invalid committed revision")?
            .checked_add(1)
            .ok_or("Configuration revision exhausted")?;
        Self::canonical(Envelope {
            schema_version: SCHEMA,
            store_id: self.envelope.store_id.clone(),
            revision: revision.to_string(),
            transaction_id: Uuid::new_v4().to_string(),
            settings,
        })
    }

    fn decode(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| DecodeError::Invalid(e.to_string()))?;
        let schema = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| DecodeError::Invalid("Missing integer schema_version".into()))?;
        if schema != SCHEMA {
            return Err(DecodeError::Unsupported(schema));
        }
        let envelope: Envelope =
            serde_json::from_value(value).map_err(|e| DecodeError::Invalid(e.to_string()))?;
        envelope.validate().map_err(DecodeError::Invalid)?;
        Ok(Self { envelope, bytes })
    }
}

#[derive(Debug)]
enum DecodeError {
    Unsupported(u64),
    Invalid(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(schema) => write!(f, "Unsupported settings schema {schema}"),
            Self::Invalid(error) => write!(f, "Invalid settings: {error}"),
        }
    }
}

fn unsupported_schema_evidence(bytes: &[u8]) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    for version in [
        value.get("schema_version"),
        value
            .get("candidate")
            .and_then(|candidate| candidate.get("schema_version")),
    ]
    .into_iter()
    .flatten()
    .filter_map(serde_json::Value::as_u64)
    {
        if version != SCHEMA {
            return Some(version);
        }
    }

    // A journal can itself be quarantined or survive only in another journal's
    // exact-byte evidence. Inspect those records before validating their v2 shape.
    let operation = value.get("operation")?;
    let preserved = operation
        .get("preserved")
        .and_then(serde_json::Value::as_array);
    let embedded = operation
        .get("predecessor")
        .into_iter()
        .chain(
            operation
                .get("source")
                .and_then(|source| source.get("bytes")),
        )
        .chain(
            preserved
                .into_iter()
                .flatten()
                .filter_map(|file| file.get("bytes")),
        );
    for encoded in embedded {
        let Some(encoded) = encoded.as_array() else {
            continue;
        };
        let Some(bytes) = encoded
            .iter()
            .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        if let Some(version) = unsupported_schema_evidence(&bytes) {
            return Some(version);
        }
    }
    None
}

#[derive(Debug)]
pub(crate) struct StorageReport {
    pub mode: ConfigMode,
    pub document: Option<Document>,
    pub issue: Option<String>,
    pub candidates: Vec<RecoveryCandidate>,
}

#[derive(Debug)]
pub(crate) enum RestoreSourceError {
    UnknownCandidate,
    Blocked(String),
}

#[derive(Debug)]
pub(crate) enum StoreOutcome {
    Committed {
        document: Document,
        candidates: Vec<RecoveryCandidate>,
        issue: Option<String>,
    },
    NotCommitted(String),
    Blocked(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum InstallSource {
    Legacy { bytes: Vec<u8> },
    Artifact { name: String, bytes: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreservedFile {
    original: String,
    archive: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Replace {
        predecessor: Vec<u8>,
    },
    Install {
        #[serde(deserialize_with = "deserialize_present_option")]
        source: Option<InstallSource>,
        preserved: Vec<PreservedFile>,
    },
}

// Exact bytes make reconciliation independent of revision guesses, JSON formatting, or timestamps.
// An intent is immutable; a non-replacing rename to "committed" records verified displacement
// before disposable artifacts are removed. Both names are recognized on the next startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    protocol_version: u32,
    candidate: Envelope,
    operation: Operation,
}

impl Intent {
    fn validate(&self) -> Result<(), String> {
        if self.protocol_version != 1 {
            return Err("Unsupported transaction-artifact protocol".into());
        }
        self.candidate.validate()?;
        match &self.operation {
            Operation::Replace { predecessor } => {
                let previous = Document::decode(predecessor.clone()).map_err(|e| e.to_string())?;
                let previous_revision = previous
                    .envelope
                    .revision
                    .parse::<u64>()
                    .map_err(|_| "Invalid predecessor revision")?;
                if previous.envelope.store_id != self.candidate.store_id
                    || previous_revision.checked_add(1).map(|r| r.to_string())
                        != Some(self.candidate.revision.clone())
                    || previous.envelope.transaction_id == self.candidate.transaction_id
                {
                    return Err("Invalid transaction predecessor relationship".into());
                }
            }
            Operation::Install { source, preserved } => {
                if self.candidate.revision != "1" {
                    return Err("A new history must start at revision 1".into());
                }
                if let Some(source) = source {
                    let settings = source_settings(source)?;
                    if settings != self.candidate.settings {
                        return Err("Recovery candidate does not match its preserved source".into());
                    }
                }
                let mut names = BTreeSet::new();
                for file in preserved {
                    validate_name(&file.original)?;
                    if unsupported_schema_evidence(&file.bytes).is_some() {
                        return Err(
                            "A recovery transaction cannot replace unsupported schema evidence"
                                .into(),
                        );
                    }
                    if !matches!(Artifact::parse(&file.archive), Artifact::Quarantine) {
                        return Err("Invalid quarantine artifact name".into());
                    }
                    if file.original == stage_name(&self.candidate.transaction_id)
                        || file.original == intent_name(&self.candidate.transaction_id)
                        || file.original == completed_name(&self.candidate.transaction_id)
                        || !names.insert(&file.original)
                        || !names.insert(&file.archive)
                    {
                        return Err("Conflicting preservation artifact names".into());
                    }
                    if file.original == MAIN {
                        if let Ok(previous) = Document::decode(file.bytes.clone()) {
                            if previous.envelope.store_id == self.candidate.store_id {
                                return Err(
                                    "Recovery must create a distinct configuration history".into(),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn document(&self) -> Result<Document, String> {
        Document::canonical(self.candidate.clone())
    }

    fn checkpoint(&self) -> Result<Option<(String, Vec<u8>)>, String> {
        match &self.operation {
            Operation::Replace { predecessor } => {
                let previous = Document::decode(predecessor.clone()).map_err(|e| e.to_string())?;
                Ok(Some((
                    checkpoint_name(
                        &previous.envelope.transaction_id,
                        &self.candidate.transaction_id,
                    ),
                    predecessor.clone(),
                )))
            }
            Operation::Install { .. } => Ok(None),
        }
    }
}

#[derive(Debug, Clone)]
enum Artifact {
    Stage,
    Intent(String, bool),
    Displaced,
    Checkpoint {
        predecessor: String,
        successor: String,
    },
    Source,
    Quarantine,
    Unknown,
}

impl Artifact {
    fn parse(name: &str) -> Self {
        let Some(body) = name
            .strip_prefix(PREFIX)
            .and_then(|n| n.strip_suffix(".json"))
        else {
            return Self::Unknown;
        };
        for (prefix, kind) in [
            ("stage-", 0),
            ("intent-", 1),
            ("committed-", 2),
            ("displaced-", 3),
        ] {
            if let Some(id) = body
                .strip_prefix(prefix)
                .filter(|id| validate_uuid(id).is_ok())
            {
                return match kind {
                    0 => Self::Stage,
                    1 => Self::Intent(id.into(), false),
                    2 => Self::Intent(id.into(), true),
                    _ => Self::Displaced,
                };
            }
        }
        if let Some((predecessor, successor)) = body
            .strip_prefix("checkpoint-")
            .and_then(|s| s.split_once("-for-"))
        {
            if validate_uuid(predecessor).is_ok() && validate_uuid(successor).is_ok() {
                return Self::Checkpoint {
                    predecessor: predecessor.into(),
                    successor: successor.into(),
                };
            }
        }
        if body
            .strip_prefix("source-")
            .is_some_and(|id| validate_uuid(id).is_ok())
        {
            return Self::Source;
        }
        if body
            .strip_prefix("quarantine-")
            .is_some_and(|id| validate_uuid(id).is_ok())
        {
            return Self::Quarantine;
        }
        Self::Unknown
    }

    fn active(&self) -> bool {
        !matches!(
            self,
            Self::Checkpoint { .. } | Self::Source | Self::Quarantine
        )
    }
}

#[derive(Clone)]
struct Candidate {
    name: String,
    bytes: Vec<u8>,
}

pub(crate) struct Storage {
    directory: PathBuf,
    legacy: PathBuf,
    _controller_lock: File,
    candidates: BTreeMap<String, Candidate>,
    #[cfg(test)]
    pub(crate) fault: Option<TestFault>,
}

impl Storage {
    pub fn open(directory: PathBuf, legacy: PathBuf) -> Result<Self, String> {
        if !directory.is_absolute() {
            return Err(format!(
                "The local settings directory must be absolute: '{}'",
                directory.display()
            ));
        }
        if !legacy.is_absolute() {
            return Err(format!(
                "The legacy settings path must be absolute: '{}'",
                legacy.display()
            ));
        }
        fs::create_dir_all(&directory)
            .map_err(|e| io_error("Create settings directory", &directory, e))?;
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|e| io_error("Inspect settings directory", &directory, e))?;
        if !metadata.is_dir() || redirected(&metadata) {
            return Err(format!(
                "Settings directory is not an ordinary directory: '{}'",
                directory.display()
            ));
        }
        fs::read_dir(&directory).map_err(|e| {
            io_error(
                "Open settings directory for recovery inspection",
                &directory,
                e,
            )
        })?;
        let lock_path = directory.join("controller.lock");
        if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
            if !metadata.is_file() || redirected(&metadata) {
                return Err(format!(
                    "Controller lock is not an ordinary file: '{}'",
                    lock_path.display()
                ));
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0).custom_flags(0x0020_0000);
        }
        #[cfg(not(windows))]
        return Err("The HDR controller requires a Windows OS-backed lifetime lock".into());
        #[cfg(windows)]
        let lock = options.open(&lock_path).map_err(|e| {
            io_error(
                "Acquire exclusive controller lock (another instance may be running)",
                &lock_path,
                e,
            )
        })?;
        #[cfg(windows)]
        if redirected(
            &lock
                .metadata()
                .map_err(|e| io_error("Inspect opened controller lock", &lock_path, e))?,
        ) {
            return Err(format!(
                "Refusing a redirected controller lock: '{}'",
                lock_path.display()
            ));
        }
        #[cfg(windows)]
        Ok(Self {
            directory,
            legacy,
            _controller_lock: lock,
            candidates: BTreeMap::new(),
            #[cfg(test)]
            fault: None,
        })
    }

    pub fn main_path(&self) -> PathBuf {
        self.directory.join(MAIN)
    }

    pub fn legacy_path(&self) -> &Path {
        &self.legacy
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn entries(&self) -> Result<Vec<(String, Artifact)>, String> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.directory)
            .map_err(|e| io_error("List configuration artifacts", &self.directory, e))?
        {
            let entry = entry
                .map_err(|e| io_error("Read configuration directory entry", &self.directory, e))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                if artifact_prefix(&name.to_string_lossy()) {
                    return Err("A configuration artifact has an invalid filename".into());
                }
                continue;
            };
            if !name.eq_ignore_ascii_case(MAIN) && artifact_prefix(name) {
                validate_name(name)?;
                entries.push((name.to_owned(), Artifact::parse(name)));
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }

    pub fn load(&mut self) -> StorageReport {
        match self.load_inner() {
            Ok(report) => report,
            Err(error) => self.blocked_report(&error),
        }
    }

    fn load_inner(&mut self) -> Result<StorageReport, String> {
        let main = read_optional(&self.main_path())?;
        if let Some(bytes) = &main {
            if let Some(version) = unsupported_schema_evidence(bytes) {
                return Ok(StorageReport {
                    mode: ConfigMode::UnsupportedSchema,
                    document: None,
                    issue: Some(format!(
                        "Unsupported settings schema {version} at '{}'; use a compatible app",
                        self.main_path().display()
                    )),
                    candidates: Vec::new(),
                });
            }
        }
        let entries = self.entries()?;
        if let Some(issue) = self.unsupported_artifacts(&entries)? {
            return Ok(StorageReport {
                mode: ConfigMode::UnsupportedSchema,
                document: None,
                issue: Some(issue),
                candidates: Vec::new(),
            });
        }
        let intents: Vec<_> = entries
            .iter()
            .filter_map(|(name, role)| match role {
                Artifact::Intent(id, completed) => Some((name, id, *completed)),
                _ => None,
            })
            .collect();
        if intents.len() > 1 {
            return Err(
                "Multiple unresolved configuration transactions require explicit recovery".into(),
            );
        }
        if let Some((name, id, completed)) = intents.first() {
            let bytes = self.required(name)?;
            let intent: Intent = serde_json::from_slice(&bytes).map_err(|e| {
                format!(
                    "Read transaction artifact '{}': {e}",
                    self.path(name).display()
                )
            })?;
            intent.validate()?;
            if &intent.candidate.transaction_id != *id {
                return Err("Transaction artifact filename does not match its transaction".into());
            }
            let permitted = [stage_name(id), displaced_name(id), (*name).clone()];
            if entries
                .iter()
                .any(|(entry, role)| role.active() && !permitted.contains(entry))
            {
                return Err("Unclassified transaction artifacts require explicit recovery".into());
            }
            match self.classify(&intent, *completed)? {
                Classification::Committed => {
                    self.finish_artifacts(&intent, &bytes, *completed, true)?;
                }
                Classification::NotCommitted => {
                    self.finish_artifacts(&intent, &bytes, false, false)?;
                }
                Classification::Recovery(error) => return Err(error),
            }
        } else if entries.iter().any(|(_, role)| role.active()) {
            return Err(
                "Unconfirmed staging/replacement artifacts require explicit recovery".into(),
            );
        }
        let main = read_optional(&self.main_path())?;
        match main {
            Some(bytes) => {
                let document = Document::decode(bytes)
                    .map_err(|e| format!("Read '{}': {e}", self.main_path().display()))?;
                self.validate_immutable_artifacts()?;
                let issue = self.prune_checkpoints(&document).err();
                let candidates = self.collect_candidates(false)?;
                Ok(StorageReport {
                    mode: ConfigMode::Ready,
                    document: Some(document),
                    issue,
                    candidates,
                })
            }
            None => {
                if !self.entries()?.is_empty() {
                    return Err(
                        "Settings are missing but recovery/quarantine evidence exists".into(),
                    );
                }
                Ok(StorageReport {
                    mode: ConfigMode::FirstRun,
                    document: None,
                    issue: None,
                    candidates: Vec::new(),
                })
            }
        }
    }

    pub fn blocked_report(&mut self, error: &str) -> StorageReport {
        #[cfg(test)]
        match self.fault.take() {
            Some(TestFault::PauseReport { entered, release }) => {
                entered
                    .send(())
                    .expect("Recovery test listener disappeared");
                release
                    .recv()
                    .expect("Recovery test did not release inventory");
            }
            other => self.fault = other,
        }
        let mut mode = match read_optional(&self.main_path()) {
            Err(_) => ConfigMode::Unavailable,
            Ok(Some(bytes)) if unsupported_schema_evidence(&bytes).is_some() => {
                ConfigMode::UnsupportedSchema
            }
            _ => ConfigMode::RecoveryRequired,
        };
        if mode == ConfigMode::RecoveryRequired {
            if let Ok(entries) = self.entries() {
                if matches!(self.unsupported_artifacts(&entries), Ok(Some(_))) {
                    mode = ConfigMode::UnsupportedSchema;
                }
            }
        }
        let (candidates, issue) = match self.collect_candidates(true) {
            Ok(candidates) => (candidates, error.to_owned()),
            Err(extra) => {
                self.candidates.clear();
                (Vec::new(), format!("{error}; recovery inventory: {extra}"))
            }
        };
        StorageReport {
            mode,
            document: None,
            issue: Some(issue),
            candidates,
        }
    }

    fn validate_immutable_artifacts(&self) -> Result<(), String> {
        for (name, role) in self.entries()? {
            match role {
                Artifact::Checkpoint { predecessor, .. } => {
                    let document = Document::decode(self.required(&name)?).map_err(|e| {
                        format!("Validate checkpoint '{}': {e}", self.path(&name).display())
                    })?;
                    if document.envelope.transaction_id != predecessor {
                        return Err(format!(
                            "Checkpoint identity mismatch at '{}'",
                            self.path(&name).display()
                        ));
                    }
                }
                Artifact::Source => {
                    Document::decode(self.required(&name)?).map_err(|e| {
                        format!(
                            "Validate recovery source '{}': {e}",
                            self.path(&name).display()
                        )
                    })?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn unsupported_artifacts(
        &self,
        entries: &[(String, Artifact)],
    ) -> Result<Option<String>, String> {
        for (name, _) in entries {
            if let Some(version) = unsupported_schema_evidence(&self.required(name)?) {
                return Ok(Some(format!(
                    "Unsupported schema {version} is preserved at '{}'; a compatible app is required",
                    self.path(name).display()
                )));
            }
        }
        Ok(None)
    }

    fn collect_candidates(&mut self, include_main: bool) -> Result<Vec<RecoveryCandidate>, String> {
        let mut entries = self.entries()?;
        if include_main {
            entries.push((MAIN.into(), Artifact::Unknown));
        }
        let mut candidates = Vec::new();
        let mut registry = BTreeMap::new();
        for (name, role) in entries {
            if matches!(role, Artifact::Intent(_, _)) {
                continue;
            }
            let Some(bytes) = read_optional(&self.path(&name))? else {
                continue;
            };
            let Ok(document) = Document::decode(bytes.clone()) else {
                continue;
            };
            let label = match role {
                Artifact::Checkpoint {
                    ref predecessor, ..
                } if predecessor == &document.envelope.transaction_id => "Known-good checkpoint",
                Artifact::Checkpoint { .. } => continue,
                Artifact::Stage => "Unconfirmed interrupted save",
                Artifact::Displaced => "Preserved displaced settings (requires confirmation)",
                Artifact::Quarantine => "Preserved quarantine settings",
                Artifact::Source => "Preserved recovery source",
                _ if name == MAIN => "Current file (requires recovery confirmation)",
                _ => "Unclassified supported settings (requires confirmation)",
            };
            let id = Uuid::new_v4().to_string();
            candidates.push(RecoveryCandidate {
                id: id.clone(),
                label: format!(
                    "{label} — history {}, revision {}",
                    document.envelope.store_id, document.envelope.revision
                ),
            });
            registry.insert(id, Candidate { name, bytes });
        }
        self.candidates = registry;
        Ok(candidates)
    }

    pub fn restore_source(
        &self,
        id: &str,
    ) -> Result<(AppConfig, InstallSource), RestoreSourceError> {
        validate_uuid(id).map_err(|_| RestoreSourceError::UnknownCandidate)?;
        let candidate = self
            .candidates
            .get(id)
            .ok_or(RestoreSourceError::UnknownCandidate)?;
        self.exact(&candidate.name, &candidate.bytes)
            .map_err(RestoreSourceError::Blocked)?;
        let document = Document::decode(candidate.bytes.clone())
            .map_err(|e| RestoreSourceError::Blocked(e.to_string()))?;
        Ok((
            document.envelope.settings,
            InstallSource::Artifact {
                name: candidate.name.clone(),
                bytes: candidate.bytes.clone(),
            },
        ))
    }

    pub fn commit(&mut self, predecessor: &Document, candidate: Document) -> StoreOutcome {
        match self.commit_inner(predecessor, candidate) {
            Ok(outcome) => outcome,
            Err(error) => StoreOutcome::Blocked(error),
        }
    }

    fn commit_inner(
        &mut self,
        predecessor: &Document,
        candidate: Document,
    ) -> Result<StoreOutcome, String> {
        self.ensure_no_active_artifacts()?;
        if let Some(issue) = self.unsupported_artifacts(&self.entries()?)? {
            return Err(issue);
        }
        let predecessor_check = self.exact(MAIN, &predecessor.bytes);
        let candidate = self.unique_candidate(candidate, Some(predecessor))?;
        let intent = Intent {
            protocol_version: 1,
            candidate: candidate.envelope.clone(),
            operation: Operation::Replace {
                predecessor: predecessor.bytes.clone(),
            },
        };
        intent.validate()?;
        let (checkpoint, bytes) = intent
            .checkpoint()?
            .ok_or("Missing predecessor checkpoint")?;
        write_new(&self.path(&checkpoint), &bytes)?;
        if let Err(error) = predecessor_check {
            // Preserve both the last committed bytes and the conflict across a restart.
            // This intent never authorizes replacement of the unexpected main.
            write_new(
                &self.path(&intent_name(&candidate.envelope.transaction_id)),
                &json_bytes(&intent)?,
            )?;
            return Err(error);
        }
        self.phase(Phase::AfterCheckpoint)?;
        self.stage(&candidate)?;
        self.phase(Phase::AfterStage)?;
        let intent_bytes = json_bytes(&intent)?;
        write_new(
            &self.path(&intent_name(&candidate.envelope.transaction_id)),
            &intent_bytes,
        )?;
        self.phase(Phase::AfterIntent)?;
        // This precheck is not compare-and-swap. The displaced bytes are checked after ReplaceFileW.
        self.exact(MAIN, &predecessor.bytes)?;
        let api_result = self.replace(&intent);
        self.phase(Phase::AfterReplace)?;
        match self.classify(&intent, false)? {
            Classification::Committed => self.complete(intent, intent_bytes, api_result.err()),
            Classification::NotCommitted => {
                self.finish_artifacts(&intent, &intent_bytes, false, false)?;
                Ok(StoreOutcome::NotCommitted(api_result.err().unwrap_or_else(
                    || {
                        "Configuration replacement did not commit; the prior settings remain active"
                            .into()
                    },
                )))
            }
            Classification::Recovery(error) => Err(with_api_error(error, api_result.err())),
        }
    }

    pub fn install(
        &mut self,
        settings: AppConfig,
        source: Option<InstallSource>,
        fresh_only: bool,
    ) -> StoreOutcome {
        match self.install_inner(settings, source, fresh_only) {
            Ok(outcome) => outcome,
            Err(error) => StoreOutcome::Blocked(error),
        }
    }

    fn install_inner(
        &mut self,
        settings: AppConfig,
        mut source: Option<InstallSource>,
        fresh_only: bool,
    ) -> Result<StoreOutcome, String> {
        settings.validate()?;
        let main = read_optional(&self.main_path())?;
        if let Some(bytes) = &main {
            if let Some(version) = unsupported_schema_evidence(bytes) {
                return Err(format!(
                    "Unsupported settings schema {version}; restore/reset is forbidden"
                ));
            }
        }
        let entries = self.entries()?;
        if let Some(issue) = self.unsupported_artifacts(&entries)? {
            return Err(issue);
        }
        if fresh_only && (main.is_some() || !entries.is_empty()) {
            return Err(
                "Conflict: configuration evidence appeared after the import/first-run preview"
                    .into(),
            );
        }
        if let Some(source) = &source {
            self.verify_source(source)?;
            if source_settings(source)? != settings {
                return Err("Recovery/import settings no longer match the selected source".into());
            }
        }
        let candidate = self.unique_candidate(Document::fresh(settings)?, None)?;
        self.stage(&candidate)?;
        self.phase(Phase::AfterStage)?;
        if let Some(InstallSource::Artifact { name, bytes }) = &source {
            // Active staging/displacement names must be retired, but never at the expense of
            // the selected source: keep an independent, flushed immutable copy first.
            if Artifact::parse(name).active() || name == MAIN {
                let preserved_source = self.fresh_name("source")?;
                write_new(&self.path(&preserved_source), bytes)?;
                source = Some(InstallSource::Artifact {
                    name: preserved_source,
                    bytes: bytes.clone(),
                });
            }
        }
        let mut preserved = Vec::new();
        if let Some(bytes) = main {
            preserved.push(PreservedFile {
                original: MAIN.into(),
                archive: self.fresh_name("quarantine")?,
                bytes,
            });
        }
        for (name, role) in entries {
            let bytes = self.required(&name)?;
            let invalid_immutable = match &role {
                Artifact::Checkpoint { predecessor, .. } => Document::decode(bytes.clone())
                    .map(|document| document.envelope.transaction_id != *predecessor)
                    .unwrap_or(true),
                Artifact::Source => Document::decode(bytes.clone()).is_err(),
                _ => false,
            };
            if role.active() || invalid_immutable {
                preserved.push(PreservedFile {
                    bytes,
                    original: name,
                    archive: self.fresh_name("quarantine")?,
                });
            }
        }
        let intent = Intent {
            protocol_version: 1,
            candidate: candidate.envelope.clone(),
            operation: Operation::Install { source, preserved },
        };
        intent.validate()?;
        let intent_bytes = json_bytes(&intent)?;
        write_new(
            &self.path(&intent_name(&candidate.envelope.transaction_id)),
            &intent_bytes,
        )?;
        self.phase(Phase::AfterIntent)?;
        if let Operation::Install { preserved, .. } = &intent.operation {
            for file in preserved {
                self.exact(&file.original, &file.bytes)?;
                self.absent(&file.archive)?;
                move_absent(&self.path(&file.original), &self.path(&file.archive))?;
                self.exact(&file.archive, &file.bytes)?;
                self.absent(&file.original)?;
            }
        }
        self.phase(Phase::AfterQuarantine)?;
        self.absent(MAIN)?;
        self.validate_install_evidence(&intent, false)?;
        self.phase(Phase::BeforeInstall)?;
        let api_result = self.install_move(&candidate.envelope.transaction_id);
        self.phase(Phase::AfterInstall)?;
        match self.classify(&intent, false)? {
            Classification::Committed => self.complete(intent, intent_bytes, api_result.err()),
            Classification::NotCommitted => {
                Err("An uncommitted initialization requires explicit recovery".into())
            }
            Classification::Recovery(error) => Err(with_api_error(error, api_result.err())),
        }
    }

    fn complete(
        &mut self,
        intent: Intent,
        bytes: Vec<u8>,
        api_error: Option<String>,
    ) -> Result<StoreOutcome, String> {
        self.finish_artifacts(&intent, &bytes, false, true)?;
        let document = intent.document()?;
        let retention_issue = self.prune_checkpoints(&document).err();
        self.ensure_no_active_artifacts()?;
        self.validate_immutable_artifacts()?;
        self.exact(MAIN, &document.bytes)?;
        let candidates = self.collect_candidates(false)?;
        self.phase(Phase::BeforePublish)?;
        let issue = match (api_error, retention_issue) {
            (Some(api), retention) => {
                let message = with_api_error(
                    "The committed configuration was verified after a Windows API error".into(),
                    Some(api),
                );
                eprintln!("{message}");
                Some(with_api_error(message, retention))
            }
            (None, retention) => retention,
        };
        Ok(StoreOutcome::Committed {
            document,
            candidates,
            issue,
        })
    }

    fn classify(&self, intent: &Intent, completed: bool) -> Result<Classification, String> {
        intent.validate()?;
        let candidate = intent.document()?;
        let main = read_optional(&self.main_path())?;
        let stage = stage_name(&intent.candidate.transaction_id);
        self.optional_exact(&stage, &candidate.bytes)?;
        match &intent.operation {
            Operation::Replace { predecessor } => {
                let (checkpoint, expected) =
                    intent.checkpoint()?.ok_or("Missing checkpoint metadata")?;
                self.exact(&checkpoint, &expected)?;
                let displaced = displaced_name(&intent.candidate.transaction_id);
                let backup_exists = self.optional_exact(&displaced, predecessor)?;
                if main.as_deref() == Some(candidate.bytes.as_slice()) {
                    if !backup_exists && !completed {
                        return Ok(Classification::Recovery("The candidate is installed but its displaced predecessor is unaccounted for".into()));
                    }
                    Ok(Classification::Committed)
                } else if main.as_deref() == Some(predecessor.as_slice()) && !completed {
                    Ok(Classification::NotCommitted)
                } else {
                    Ok(Classification::Recovery("The main settings file is not conclusively the old or new transaction; preserve all recovery files".into()))
                }
            }
            Operation::Install { .. } => {
                if main.as_deref() != Some(candidate.bytes.as_slice()) {
                    return Ok(Classification::Recovery("Initialization/recovery was interrupted; staged and quarantined data require an explicit recovery decision".into()));
                }
                self.validate_install_evidence(intent, true)?;
                Ok(Classification::Committed)
            }
        }
    }

    fn validate_install_evidence(&self, intent: &Intent, installed: bool) -> Result<(), String> {
        let Operation::Install { source, preserved } = &intent.operation else {
            return Err("Not an installation transaction".into());
        };
        if let Some(source) = source {
            self.verify_source(source)?;
        }
        for file in preserved {
            self.exact(&file.archive, &file.bytes)?;
            if file.original != MAIN || !installed {
                self.absent(&file.original)?;
            }
        }
        let permitted = [
            stage_name(&intent.candidate.transaction_id),
            intent_name(&intent.candidate.transaction_id),
            completed_name(&intent.candidate.transaction_id),
        ];
        if self
            .entries()?
            .iter()
            .any(|(name, role)| role.active() && !permitted.contains(name))
        {
            return Err("Unexpected configuration artifacts appeared during recovery".into());
        }
        Ok(())
    }

    fn finish_artifacts(
        &self,
        intent: &Intent,
        bytes: &[u8],
        completed: bool,
        committed: bool,
    ) -> Result<(), String> {
        let id = &intent.candidate.transaction_id;
        let candidate = intent.document()?;
        let pending = intent_name(id);
        let done = completed_name(id);
        if committed && !completed {
            self.exact(&pending, bytes)?;
            self.absent(&done)?;
            move_absent(&self.path(&pending), &self.path(&done))?;
            self.exact(&done, bytes)?;
        }
        self.remove_exact_if_present(&stage_name(id), &candidate.bytes)?;
        if let Operation::Replace { predecessor } = &intent.operation {
            self.remove_exact_if_present(&displaced_name(id), predecessor)?;
            if !committed {
                self.exact(MAIN, predecessor)?;
            }
        }
        if committed {
            if !matches!(self.classify(intent, true)?, Classification::Committed) {
                return Err("Configuration changed during final commit reconciliation".into());
            }
            self.remove_exact_if_present(&done, bytes)?;
        } else {
            self.remove_exact_if_present(&pending, bytes)?;
            if let Some((checkpoint, bytes)) = intent.checkpoint()? {
                self.remove_exact_if_present(&checkpoint, &bytes)?;
            }
        }
        Ok(())
    }

    fn prune_checkpoints(&self, current: &Document) -> Result<(), String> {
        let mut by_successor: BTreeMap<String, Vec<(String, Document)>> = BTreeMap::new();
        for (name, role) in self.entries()? {
            if let Artifact::Checkpoint {
                predecessor,
                successor,
            } = role
            {
                let Ok(document) = Document::decode(self.required(&name)?) else {
                    continue;
                };
                if document.envelope.store_id == current.envelope.store_id
                    && document.envelope.transaction_id == predecessor
                {
                    by_successor
                        .entry(successor)
                        .or_default()
                        .push((name, document));
                }
            }
        }
        let mut chain = Vec::new();
        let mut next = current.clone();
        while let Some(previous) = by_successor.remove(&next.envelope.transaction_id) {
            if previous.len() != 1 {
                break;
            }
            let (name, document) = previous
                .into_iter()
                .next()
                .ok_or("Missing checkpoint link")?;
            let revision = document
                .envelope
                .revision
                .parse::<u64>()
                .map_err(|_| "Invalid checkpoint revision")?;
            if revision.checked_add(1).map(|r| r.to_string())
                != Some(next.envelope.revision.clone())
            {
                break;
            }
            next = document.clone();
            chain.push((name, document));
        }
        // Delete only the proven chain, oldest first. Foreign/orphan/uncertain evidence is exempt.
        for (name, document) in chain.into_iter().skip(2).rev() {
            self.remove_exact_if_present(&name, &document.bytes)?;
        }
        Ok(())
    }

    fn ensure_no_active_artifacts(&self) -> Result<(), String> {
        if self.entries()?.iter().any(|(_, role)| role.active()) {
            Err("Unresolved configuration artifacts block ordinary saves".into())
        } else {
            Ok(())
        }
    }

    fn unique_candidate(
        &self,
        mut candidate: Document,
        predecessor: Option<&Document>,
    ) -> Result<Document, String> {
        for _ in 0..32 {
            let id = &candidate.envelope.transaction_id;
            let mut names = vec![
                stage_name(id),
                intent_name(id),
                completed_name(id),
                displaced_name(id),
            ];
            if let Some(previous) = predecessor {
                names.push(checkpoint_name(&previous.envelope.transaction_id, id));
            }
            let mut collision = false;
            for name in names {
                match fs::symlink_metadata(self.path(&name)) {
                    Ok(_) => collision = true,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        return Err(io_error(
                            "Inspect transaction destination",
                            &self.path(&name),
                            e,
                        ))
                    }
                }
            }
            if !collision {
                return Ok(candidate);
            }
            candidate.envelope.transaction_id = Uuid::new_v4().to_string();
            candidate = Document::canonical(candidate.envelope)?;
        }
        Err("Cannot allocate unused transaction filenames".into())
    }

    fn fresh_name(&self, role: &str) -> Result<String, String> {
        for _ in 0..32 {
            let name = format!("{PREFIX}{role}-{}.json", Uuid::new_v4());
            match fs::symlink_metadata(self.path(&name)) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(name),
                Ok(_) => {}
                Err(e) => {
                    return Err(io_error(
                        "Inspect archive destination",
                        &self.path(&name),
                        e,
                    ))
                }
            }
        }
        Err("Cannot allocate an unused recovery filename".into())
    }

    fn required(&self, name: &str) -> Result<Vec<u8>, String> {
        read_optional(&self.path(name))?.ok_or_else(|| {
            format!(
                "Required configuration evidence is missing: '{}'",
                self.path(name).display()
            )
        })
    }

    fn exact(&self, name: &str, expected: &[u8]) -> Result<(), String> {
        if self.required(name)? != expected {
            return Err(format!(
                "Conflict: unexpected contents at '{}'; nothing may overwrite this evidence",
                self.path(name).display()
            ));
        }
        Ok(())
    }

    fn optional_exact(&self, name: &str, expected: &[u8]) -> Result<bool, String> {
        match read_optional(&self.path(name))? {
            None => Ok(false),
            Some(bytes) if bytes == expected => Ok(true),
            Some(_) => Err(format!(
                "Conflict: unexpected displaced/staged contents at '{}'",
                self.path(name).display()
            )),
        }
    }

    fn absent(&self, name: &str) -> Result<(), String> {
        match fs::symlink_metadata(self.path(name)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error("Inspect absent destination", &self.path(name), e)),
            Ok(_) => Err(format!(
                "Conflict: destination already exists at '{}'",
                self.path(name).display()
            )),
        }
    }

    fn remove_exact_if_present(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.optional_exact(name, bytes)? {
            fs::remove_file(self.path(name)).map_err(|e| {
                io_error(
                    "Remove reconciled transaction artifact",
                    &self.path(name),
                    e,
                )
            })?;
        }
        Ok(())
    }

    fn verify_source(&self, source: &InstallSource) -> Result<(), String> {
        match source {
            InstallSource::Artifact { name, bytes } => {
                validate_name(name)?;
                self.exact(name, bytes)
            }
            InstallSource::Legacy { bytes } => {
                if read_optional(&self.legacy)?.as_deref() != Some(bytes.as_slice()) {
                    return Err(format!(
                        "Conflict: legacy import source changed at '{}'",
                        self.legacy.display()
                    ));
                }
                Ok(())
            }
        }
    }

    fn stage(&mut self, candidate: &Document) -> Result<(), String> {
        let path = self.path(&stage_name(&candidate.envelope.transaction_id));
        #[cfg(test)]
        if matches!(
            self.fault,
            Some(TestFault::StageWrite | TestFault::StageSync)
        ) {
            let fault = self.fault.take().unwrap();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| io_error("Create test staging file", &path, e))?;
            let bytes = if matches!(fault, TestFault::StageWrite) {
                &candidate.bytes[..candidate.bytes.len() / 2]
            } else {
                &candidate.bytes
            };
            file.write_all(bytes)
                .map_err(|e| io_error("Write test staging file", &path, e))?;
            return Err("Injected staging write/sync failure".into());
        }
        write_new(&path, &candidate.bytes)?;
        Document::decode(self.required(&stage_name(&candidate.envelope.transaction_id))?)
            .map_err(|e| format!("Validate staged configuration: {e}"))?;
        Ok(())
    }

    fn replace(&mut self, intent: &Intent) -> Result<(), String> {
        let id = &intent.candidate.transaction_id;
        let stage = self.path(&stage_name(id));
        let displaced = self.path(&displaced_name(id));
        self.absent(&displaced_name(id))?;
        #[cfg(test)]
        match self.fault.take() {
            Some(TestFault::ReplaceNotCommitted) => {
                return Err("Injected replacement/sharing failure".into())
            }
            Some(TestFault::ReplaceCommittedError) => {
                replace_file(&self.main_path(), &stage, &displaced)?;
                return Err("Injected API error after actual replacement".into());
            }
            Some(TestFault::DisplacedConflict(bytes)) => {
                fs::write(self.main_path(), bytes).map_err(|e| e.to_string())?;
            }
            Some(TestFault::DisplaceWithoutInstall) => {
                move_absent(&self.main_path(), &displaced)?;
                return Err("Injected displaced-main replacement failure".into());
            }
            other => self.fault = other,
        }
        replace_file(&self.main_path(), &stage, &displaced)
    }

    fn install_move(&mut self, id: &str) -> Result<(), String> {
        let stage = self.path(&stage_name(id));
        #[cfg(test)]
        match self.fault.take() {
            Some(TestFault::InstallCommittedError) => {
                move_absent(&stage, &self.main_path())?;
                return Err("Injected API error after installation".into());
            }
            Some(TestFault::InstallDestination(bytes)) => {
                write_new(&self.main_path(), &bytes)?;
            }
            other => self.fault = other,
        }
        move_absent(&stage, &self.main_path())
    }

    fn phase(&mut self, phase: Phase) -> Result<(), String> {
        #[cfg(test)]
        if matches!(&self.fault, Some(TestFault::Interrupt(at)) if *at == phase) {
            self.fault = None;
            return Err(format!("Injected interruption at {phase:?}"));
        }
        let _ = phase;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    AfterCheckpoint,
    AfterStage,
    AfterIntent,
    AfterReplace,
    AfterQuarantine,
    BeforeInstall,
    AfterInstall,
    BeforePublish,
}

#[cfg(test)]
pub(crate) enum TestFault {
    Interrupt(Phase),
    StageWrite,
    StageSync,
    ReplaceNotCommitted,
    ReplaceCommittedError,
    DisplacedConflict(Vec<u8>),
    DisplaceWithoutInstall,
    InstallCommittedError,
    InstallDestination(Vec<u8>),
    PauseReport {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    },
}

enum Classification {
    Committed,
    NotCommitted,
    Recovery(String),
}

fn source_settings(source: &InstallSource) -> Result<AppConfig, String> {
    match source {
        InstallSource::Legacy { bytes } => decode_legacy(bytes),
        InstallSource::Artifact { name, bytes } => {
            validate_name(name)?;
            Ok(Document::decode(bytes.clone())
                .map_err(|e| e.to_string())?
                .envelope
                .settings)
        }
    }
}

fn validate_uuid(value: &str) -> Result<(), String> {
    if Uuid::parse_str(value)
        .map_err(|_| "Invalid UUID")?
        .to_string()
        != value
    {
        return Err("UUID must use canonical lowercase hyphenated form".into());
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if !artifact_prefix(name)
        || name.contains(['\\', '/', ':'])
        || name.chars().any(|c| c.is_control())
        || name.ends_with(['.', ' '])
    {
        return Err("Invalid configuration artifact name".into());
    }
    Ok(())
}

fn artifact_prefix(name: &str) -> bool {
    name.get(..PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
}

fn stage_name(id: &str) -> String {
    format!("{PREFIX}stage-{id}.json")
}
fn intent_name(id: &str) -> String {
    format!("{PREFIX}intent-{id}.json")
}
fn completed_name(id: &str) -> String {
    format!("{PREFIX}committed-{id}.json")
}
fn displaced_name(id: &str) -> String {
    format!("{PREFIX}displaced-{id}.json")
}
fn checkpoint_name(predecessor: &str, successor: &str) -> String {
    format!("{PREFIX}checkpoint-{predecessor}-for-{successor}.json")
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn io_error(operation: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!("{operation} '{}': {error}", path.display())
}

fn with_api_error(message: String, api_error: Option<String>) -> String {
    match api_error {
        Some(error) => format!("{message}; {error}"),
        None => message,
    }
}

fn redirected(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error("Inspect settings file", path, e)),
        Ok(metadata) => {
            if !metadata.is_file() || redirected(&metadata) {
                return Err(format!(
                    "Refusing a nonregular/redirected configuration file: '{}'",
                    path.display()
                ));
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.custom_flags(0x0020_0000);
            }
            let mut file = options
                .open(path)
                .map_err(|e| io_error("Open settings file", path, e))?;
            let opened = file
                .metadata()
                .map_err(|e| io_error("Inspect opened settings file", path, e))?;
            if !opened.is_file() || redirected(&opened) {
                return Err(format!(
                    "Refusing an opened nonregular/redirected configuration file: '{}'",
                    path.display()
                ));
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(|e| io_error("Read settings file", path, e))?;
            Ok(Some(bytes))
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error("Exclusively create configuration artifact", path, e))?;
    file.write_all(bytes)
        .map_err(|e| io_error("Write configuration artifact", path, e))?;
    file.sync_all()
        .map_err(|e| io_error("Flush configuration artifact", path, e))?;
    drop(file);
    if read_optional(path)?.as_deref() != Some(bytes) {
        return Err(format!(
            "Configuration artifact validation failed: '{}'",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn replace_file(main: &Path, stage: &Path, backup: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{ReplaceFileW, REPLACE_FILE_FLAGS};
    let main_wide = wide_path(main)?;
    let stage_wide = wide_path(stage)?;
    let backup_wide = wide_path(backup)?;
    unsafe {
        ReplaceFileW(
            PCWSTR(main_wide.as_ptr()),
            PCWSTR(stage_wide.as_ptr()),
            PCWSTR(backup_wide.as_ptr()),
            REPLACE_FILE_FLAGS(0),
            None,
            None,
        )
    }
    .map_err(|e| io_error("ReplaceFileW", main, e))
}

#[cfg(windows)]
fn move_absent(from: &Path, to: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    let from_wide = wide_path(from)?;
    let to_wide = wide_path(to)?;
    // No REPLACE_EXISTING or COPY_ALLOWED: all managed moves stay on this volume.
    unsafe {
        MoveFileExW(
            PCWSTR(from_wide.as_ptr()),
            PCWSTR(to_wide.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| {
        format!(
            "MoveFileExW '{}' -> '{}': {e}",
            from.display(),
            to.display()
        )
    })
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Result<Vec<u16>, String> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err("Configuration path contains a NUL character".into());
    }
    wide.push(0);
    Ok(wide)
}

#[cfg(not(windows))]
fn replace_file(_: &Path, _: &Path, _: &Path) -> Result<(), String> {
    Err("Windows replacement semantics are required".into())
}

#[cfg(not(windows))]
fn move_absent(_: &Path, _: &Path) -> Result<(), String> {
    Err("Windows non-replacing move semantics are required".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::config_v2::{HdrApp, HdrType, TargetMonitor};
    use tempfile::TempDir;

    struct Fixture {
        root: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                root: tempfile::Builder::new()
                    .prefix(".storage-test-")
                    .tempdir_in(std::env::current_dir().unwrap())
                    .unwrap(),
            }
        }

        fn open(&self) -> Storage {
            Storage::open(
                self.root.path().join("local"),
                self.root.path().join("legacy.json"),
            )
            .unwrap()
        }

        fn ready(&self) -> (Storage, Document) {
            let mut storage = self.open();
            assert_eq!(storage.load().mode, ConfigMode::FirstRun);
            let document = committed(storage.install(AppConfig::default(), None, true));
            (storage, document)
        }
    }

    fn committed(outcome: StoreOutcome) -> Document {
        match outcome {
            StoreOutcome::Committed { document, .. } => document,
            other => panic!("Expected a committed transaction, got {other:?}"),
        }
    }

    fn changed(previous: &Document) -> Document {
        let mut settings = previous.envelope.settings.clone();
        settings.alt_tab_delay_seconds += 1;
        previous.successor(settings).unwrap()
    }

    fn has_exact_file(storage: &Storage, bytes: &[u8]) -> bool {
        fs::read_dir(&storage.directory).unwrap().any(|entry| {
            let entry = entry.unwrap();
            read_optional(&entry.path()).ok().flatten().as_deref() == Some(bytes)
        })
    }

    fn schema_targets() -> [TargetMonitor; 3] {
        [
            TargetMonitor::All,
            TargetMonitor::Monitor {
                device_path: r"\\?\DISPLAY#schema-test".into(),
                display_name: "Schema test display".into(),
            },
            TargetMonitor::NeedsConfirmation {
                legacy_runtime_id: "legacy-display-id".into(),
            },
        ]
    }

    fn schema_document(target_monitor: TargetMonitor) -> Document {
        Document::fresh(AppConfig {
            target_monitor,
            apps: vec![HdrApp {
                name: "Schema test game".into(),
                exe_name: "schema-game.exe".into(),
                enabled: true,
                hdr_type: HdrType::AutoHdr,
                path: None,
                alternate_exes: Vec::new(),
                steam_id: None,
                launcher: None,
            }],
            ..AppConfig::default()
        })
        .unwrap()
    }

    fn without_field(
        value: &serde_json::Value,
        object: &str,
        field: &str,
    ) -> serde_json::Value {
        let mut value = value.clone();
        assert!(value
            .pointer_mut(object)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field)
            .is_some());
        value
    }

    fn with_unknown_field(value: &serde_json::Value, object: &str) -> serde_json::Value {
        let mut value = value.clone();
        assert!(value
            .pointer_mut(object)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown_v2_field".into(), serde_json::json!({"keep": true}))
            .is_none());
        value
    }

    fn stored_evidence(storage: &Storage) -> BTreeMap<String, Vec<u8>> {
        let mut files: BTreeMap<_, _> = storage
            .entries()
            .unwrap()
            .into_iter()
            .map(|(name, _)| {
                let bytes = storage.required(&name).unwrap();
                (name, bytes)
            })
            .collect();
        if let Some(bytes) = read_optional(&storage.main_path()).unwrap() {
            files.insert(MAIN.into(), bytes);
        }
        files
    }

    fn load_main_without_rewriting(bytes: &[u8]) -> StorageReport {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        write_new(&storage.main_path(), bytes).unwrap();
        let before = stored_evidence(&storage);
        let report = storage.load();
        assert_eq!(stored_evidence(&storage), before);
        report
    }

    #[test]
    fn strict_v2_requires_all_settings_app_and_target_fields() {
        let mut accepted = Vec::new();
        for target in schema_targets() {
            let document = schema_document(target);
            let value = serde_json::to_value(&document.envelope).unwrap();
            for object in ["/settings", "/settings/apps/0", "/settings/target_monitor"] {
                for field in value.pointer(object).unwrap().as_object().unwrap().keys() {
                    let missing = without_field(&value, object, field);
                    let bytes = serde_json::to_vec(&missing).unwrap();
                    let report = load_main_without_rewriting(&bytes);
                    if Document::decode(bytes).is_ok()
                        || report.mode != ConfigMode::RecoveryRequired
                        || report.document.is_some()
                        || !report.candidates.is_empty()
                    {
                        accepted.push(format!("{object}/{field}: {:?}", report.mode));
                    }
                }
            }
        }
        assert!(accepted.is_empty(), "Accepted missing fields: {accepted:?}");
    }

    #[test]
    fn strict_v2_requires_nullable_fields_even_in_journal_candidates() {
        let document = schema_document(TargetMonitor::All);
        let value = serde_json::to_value(&document.envelope).unwrap();
        let mut nullable_fields = Vec::new();
        let mut accepted = Vec::new();
        for object in ["/settings", "/settings/apps/0"] {
            for (field, field_value) in value.pointer(object).unwrap().as_object().unwrap() {
                if !field_value.is_null() {
                    continue;
                }
                nullable_fields.push(format!("{object}/{field}"));
                let missing = without_field(&value, object, field);
                let journal = serde_json::json!({
                    "protocol_version": 1,
                    "candidate": missing,
                    "operation": { "kind": "install", "source": null, "preserved": [] },
                });
                if serde_json::from_value::<Intent>(journal).is_ok() {
                    accepted.push(format!("{object}/{field}"));
                }
            }
        }
        assert!(!nullable_fields.is_empty());
        assert!(accepted.is_empty(), "Accepted absent nullable fields: {accepted:?}");
    }

    #[test]
    fn strict_v2_rejects_unknown_nested_settings_app_and_target_fields() {
        let mut accepted = Vec::new();
        for target in schema_targets() {
            let document = schema_document(target);
            let value = serde_json::to_value(&document.envelope).unwrap();
            for object in ["/settings", "/settings/apps/0", "/settings/target_monitor"] {
                let bytes = serde_json::to_vec(&with_unknown_field(&value, object)).unwrap();
                let report = load_main_without_rewriting(&bytes);
                if Document::decode(bytes).is_ok()
                    || report.mode != ConfigMode::RecoveryRequired
                    || report.document.is_some()
                    || !report.candidates.is_empty()
                {
                    accepted.push(format!("{object}: {:?}", report.mode));
                }
            }
        }
        assert!(accepted.is_empty(), "Accepted unknown fields: {accepted:?}");
    }

    #[test]
    fn strict_v2_complete_documents_and_explicit_nulls_round_trip() {
        for target in schema_targets() {
            for populated in [false, true] {
                let mut settings = schema_document(target.clone()).envelope.settings;
                if populated {
                    settings.last_sync_timestamp = Some(123456);
                    settings.apps[0].path = Some(r"C:\Games\schema-game.exe".into());
                    settings.apps[0].steam_id = Some("123".into());
                    settings.apps[0].launcher = Some("Test launcher".into());
                }
                let fixture = Fixture::new();
                let mut storage = fixture.open();
                let document = committed(storage.install(settings.clone(), None, true));
                let decoded = Document::decode(document.bytes.clone()).unwrap();
                assert_eq!(decoded.envelope, document.envelope);
                assert_eq!(decoded.bytes, document.bytes);
                let intent = Intent {
                    protocol_version: 1,
                    candidate: document.envelope.clone(),
                    operation: Operation::Install {
                        source: None,
                        preserved: Vec::new(),
                    },
                };
                let decoded: Intent = serde_json::from_slice(&json_bytes(&intent).unwrap()).unwrap();
                decoded.validate().unwrap();
                let before = stored_evidence(&storage);
                let report = storage.load();
                assert_eq!(report.mode, ConfigMode::Ready);
                assert_eq!(report.document.unwrap().envelope.settings, settings);
                assert_eq!(stored_evidence(&storage), before);
            }
        }
    }

    #[test]
    fn strict_v2_artifacts_cannot_be_advertised_or_installed_as_recovery_sources() {
        for role in ["checkpoint", "stage", "source", "quarantine", "displaced", "unknown"] {
            let previous = schema_document(TargetMonitor::All);
            let current = changed(&previous);
            let value = serde_json::to_value(&previous.envelope).unwrap();
            for malformed in [
                without_field(&value, "/settings", "last_sync_timestamp"),
                with_unknown_field(&value, "/settings/apps/0"),
            ] {
                let fixture = Fixture::new();
                let mut storage = fixture.open();
                let name = match role {
                    "checkpoint" => checkpoint_name(
                        &previous.envelope.transaction_id,
                        &current.envelope.transaction_id,
                    ),
                    "stage" => stage_name(&previous.envelope.transaction_id),
                    "displaced" => displaced_name(&previous.envelope.transaction_id),
                    _ => format!("config-v2.{role}-{}.json", Uuid::new_v4()),
                };
                let bytes = serde_json::to_vec(&malformed).unwrap();
                write_new(&storage.path(&name), &bytes).unwrap();
                if matches!(role, "checkpoint" | "source") {
                    write_new(&storage.main_path(), &current.bytes).unwrap();
                }
                let before = stored_evidence(&storage);
                let report = storage.load();
                assert_eq!(report.mode, ConfigMode::RecoveryRequired, "{role}");
                assert!(storage.candidates.values().all(|candidate| candidate.name != name));
                let source = InstallSource::Artifact { name, bytes };
                assert!(source_settings(&source).is_err(), "{role}");
                assert!(matches!(
                    storage.install(previous.envelope.settings.clone(), Some(source), false),
                    StoreOutcome::Blocked(_)
                ), "{role}");
                assert_eq!(stored_evidence(&storage), before, "{role}");
            }
        }
    }

    #[test]
    fn strict_v2_journal_candidates_and_nullable_source_must_be_complete() {
        let document = schema_document(TargetMonitor::All);
        let intent = Intent {
            protocol_version: 1,
            candidate: document.envelope.clone(),
            operation: Operation::Install {
                source: None,
                preserved: Vec::new(),
            },
        };
        let value = serde_json::to_value(&intent).unwrap();
        for malformed in [
            without_field(&value, "/candidate/settings", "auto_detect_new_games"),
            without_field(&value, "/candidate/settings", "last_sync_timestamp"),
            without_field(&value, "/candidate/settings/apps/0", "path"),
            with_unknown_field(&value, "/candidate/settings"),
            with_unknown_field(&value, "/candidate/settings/apps/0"),
            with_unknown_field(&value, "/candidate/settings/target_monitor"),
            without_field(&value, "/operation", "source"),
        ] {
            let fixture = Fixture::new();
            let mut storage = fixture.open();
            write_new(&storage.main_path(), &document.bytes).unwrap();
            write_new(
                &storage.path(&intent_name(&document.envelope.transaction_id)),
                &serde_json::to_vec(&malformed).unwrap(),
            )
            .unwrap();
            let before = stored_evidence(&storage);
            let report = storage.load();
            assert_eq!(report.mode, ConfigMode::RecoveryRequired);
            assert!(report.document.is_none());
            assert_eq!(stored_evidence(&storage), before);
        }
    }

    #[test]
    fn strict_v2_journal_predecessors_and_artifact_sources_cannot_bypass_validation() {
        let previous = schema_document(TargetMonitor::All);
        let value = serde_json::to_value(&previous.envelope).unwrap();
        for malformed in [
            without_field(&value, "/settings", "last_sync_timestamp"),
            with_unknown_field(&value, "/settings/apps/0"),
        ] {
            for replacing in [false, true] {
                let fixture = Fixture::new();
                let mut storage = fixture.open();
                let candidate = if replacing {
                    changed(&previous)
                } else {
                    Document::fresh(previous.envelope.settings.clone()).unwrap()
                };
                let bytes = serde_json::to_vec(&malformed).unwrap();
                let operation = if replacing {
                    let checkpoint = checkpoint_name(
                        &previous.envelope.transaction_id,
                        &candidate.envelope.transaction_id,
                    );
                    write_new(&storage.path(&checkpoint), &bytes).unwrap();
                    Operation::Replace { predecessor: bytes }
                } else {
                    let name = storage.fresh_name("source").unwrap();
                    write_new(&storage.path(&name), &bytes).unwrap();
                    Operation::Install {
                        source: Some(InstallSource::Artifact { name, bytes }),
                        preserved: Vec::new(),
                    }
                };
                let intent = Intent {
                    protocol_version: 1,
                    candidate: candidate.envelope.clone(),
                    operation,
                };
                write_new(&storage.main_path(), &candidate.bytes).unwrap();
                write_new(
                    &storage.path(&completed_name(&candidate.envelope.transaction_id)),
                    &json_bytes(&intent).unwrap(),
                )
                .unwrap();
                let before = stored_evidence(&storage);
                let report = storage.load();
                assert_eq!(report.mode, ConfigMode::RecoveryRequired);
                assert!(report.document.is_none());
                assert_eq!(stored_evidence(&storage), before);
            }
        }
    }

    #[test]
    fn schema_and_revision_validation_are_exact_and_do_not_use_javascript_numbers() {
        let original = Document::fresh(AppConfig::default()).unwrap();
        let mut value = serde_json::to_value(&original.envelope).unwrap();
        value["revision"] = "9007199254740993".into();
        let decoded = Document::decode(serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(decoded.envelope.revision, "9007199254740993");
        assert_eq!(changed(&decoded).envelope.revision, "9007199254740994");
        for revision in [
            serde_json::json!(1),
            serde_json::json!("01"),
            serde_json::json!("0"),
            serde_json::json!("-1"),
        ] {
            value["revision"] = revision;
            assert!(Document::decode(serde_json::to_vec(&value).unwrap()).is_err());
        }
        value["schema_version"] = 123.into();
        assert!(matches!(
            Document::decode(serde_json::to_vec(&value).unwrap()),
            Err(DecodeError::Unsupported(123))
        ));
    }

    #[test]
    fn directory_and_controller_lock_failures_are_fatal_load_errors() {
        let fixture = Fixture::new();
        let local = fixture.root.path().join("not-a-directory");
        write_new(&local, b"ordinary file").unwrap();
        assert!(Storage::open(local.clone(), fixture.root.path().join("legacy.json")).is_err());
        assert_eq!(read_optional(&local).unwrap().unwrap(), b"ordinary file");
        let local = fixture.root.path().join("local");
        fs::create_dir_all(local.join("controller.lock")).unwrap();
        assert!(Storage::open(local, fixture.root.path().join("legacy.json")).is_err());
        assert!(
            Storage::open(PathBuf::from("."), fixture.root.path().join("legacy.json")).is_err()
        );
    }

    #[test]
    fn real_windows_replace_and_reported_error_after_commit_are_reconciled() {
        let fixture = Fixture::new();
        let (mut storage, first) = fixture.ready();
        let second = committed(storage.commit(&first, changed(&first)));
        assert_eq!(second.envelope.revision, "2");
        storage.fault = Some(TestFault::ReplaceCommittedError);
        let third = committed(storage.commit(&second, changed(&second)));
        assert_eq!(third.envelope.revision, "3");
        assert_eq!(storage.required(MAIN).unwrap(), third.bytes);
        assert!(has_exact_file(&storage, &first.bytes));
        assert!(has_exact_file(&storage, &second.bytes));
        let fourth = committed(storage.commit(&third, changed(&third)));
        let checkpoints: Vec<_> = storage
            .entries()
            .unwrap()
            .into_iter()
            .filter(|(_, role)| matches!(role, Artifact::Checkpoint { .. }))
            .collect();
        assert_eq!(checkpoints.len(), 2);
        assert!(!has_exact_file(&storage, &first.bytes));
        assert!(has_exact_file(&storage, &second.bytes));
        assert!(has_exact_file(&storage, &third.bytes));
        drop(storage);
        let reloaded = fixture.open().load();
        assert_eq!(reloaded.mode, ConfigMode::Ready);
        assert_eq!(reloaded.document.unwrap().bytes, fourth.bytes);
    }

    #[test]
    fn known_noncommit_keeps_the_exact_predecessor_and_cleans_only_known_roles() {
        let fixture = Fixture::new();
        let (mut storage, previous) = fixture.ready();
        let unrelated = storage.directory.join("unrelated.json");
        fs::write(&unrelated, b"not ours").unwrap();
        storage.fault = Some(TestFault::ReplaceNotCommitted);
        assert!(matches!(
            storage.commit(&previous, changed(&previous)),
            StoreOutcome::NotCommitted(_)
        ));
        assert_eq!(storage.required(MAIN).unwrap(), previous.bytes);
        assert!(storage.entries().unwrap().is_empty());
        assert_eq!(fs::read(unrelated).unwrap(), b"not ours");
        drop(storage);
        assert_eq!(fixture.open().load().mode, ConfigMode::Ready);
    }

    #[test]
    fn normal_save_interruptions_reconcile_roles_not_just_a_valid_main() {
        for (phase, expected_mode, candidate_committed) in [
            (Phase::AfterCheckpoint, ConfigMode::Ready, false),
            (Phase::AfterStage, ConfigMode::RecoveryRequired, false),
            (Phase::AfterIntent, ConfigMode::Ready, false),
            (Phase::AfterReplace, ConfigMode::Ready, true),
            (Phase::BeforePublish, ConfigMode::Ready, true),
        ] {
            let fixture = Fixture::new();
            let (mut storage, previous) = fixture.ready();
            let candidate = changed(&previous);
            storage.fault = Some(TestFault::Interrupt(phase));
            assert!(matches!(
                storage.commit(&previous, candidate.clone()),
                StoreOutcome::Blocked(_)
            ));
            assert!(
                has_exact_file(&storage, &previous.bytes),
                "Lost predecessor at {phase:?}"
            );
            drop(storage);
            let mut storage = fixture.open();
            let loaded = storage.load();
            assert_eq!(loaded.mode, expected_mode, "{phase:?}: {:?}", loaded.issue);
            if expected_mode == ConfigMode::Ready {
                assert_eq!(
                    loaded.document.unwrap().bytes,
                    if candidate_committed {
                        candidate.bytes
                    } else {
                        previous.bytes
                    }
                );
            } else {
                assert!(!loaded.candidates.is_empty());
            }
        }
    }

    #[test]
    fn first_creation_interruption_never_becomes_accidental_first_run() {
        for phase in [
            Phase::AfterStage,
            Phase::AfterIntent,
            Phase::AfterQuarantine,
            Phase::BeforeInstall,
            Phase::AfterInstall,
            Phase::BeforePublish,
        ] {
            let fixture = Fixture::new();
            let mut storage = fixture.open();
            storage.fault = Some(TestFault::Interrupt(phase));
            assert!(matches!(
                storage.install(AppConfig::default(), None, true),
                StoreOutcome::Blocked(_)
            ));
            drop(storage);
            let loaded = fixture.open().load();
            let committed = matches!(phase, Phase::AfterInstall | Phase::BeforePublish);
            assert_eq!(
                loaded.mode,
                if committed {
                    ConfigMode::Ready
                } else {
                    ConfigMode::RecoveryRequired
                },
                "{phase:?}: {:?}",
                loaded.issue
            );
            if !committed {
                assert!(!loaded.candidates.is_empty());
            }
        }
    }

    #[test]
    fn recovery_interruptions_keep_source_corrupt_main_and_unclassified_evidence() {
        for phase in [
            Phase::AfterStage,
            Phase::AfterIntent,
            Phase::AfterQuarantine,
            Phase::BeforeInstall,
            Phase::AfterInstall,
            Phase::BeforePublish,
        ] {
            let fixture = Fixture::new();
            let (mut storage, previous) = fixture.ready();
            let newer = committed(storage.commit(&previous, changed(&previous)));
            fs::write(storage.main_path(), b"broken main").unwrap();
            fs::write(
                storage.path("config-v2.unclassified.json"),
                b"interrupted evidence",
            )
            .unwrap();
            let report = storage.load();
            assert_eq!(report.mode, ConfigMode::RecoveryRequired);
            let candidate = report
                .candidates
                .iter()
                .find(|c| c.label.contains("checkpoint"))
                .unwrap();
            let (settings, source) = storage.restore_source(&candidate.id).unwrap();
            storage.fault = Some(TestFault::Interrupt(phase));
            assert!(matches!(
                storage.install(settings, Some(source), false),
                StoreOutcome::Blocked(_)
            ));
            assert!(has_exact_file(&storage, &previous.bytes));
            assert!(has_exact_file(&storage, b"broken main"));
            assert!(has_exact_file(&storage, b"interrupted evidence"));
            drop(storage);
            let mut storage = fixture.open();
            let loaded = storage.load();
            let installed = matches!(phase, Phase::AfterInstall | Phase::BeforePublish);
            assert_eq!(
                loaded.mode,
                if installed {
                    ConfigMode::Ready
                } else {
                    ConfigMode::RecoveryRequired
                },
                "{phase:?}: {:?}",
                loaded.issue
            );
            if installed {
                let restored = loaded.document.unwrap();
                assert_eq!(restored.envelope.settings, previous.envelope.settings);
                assert_ne!(restored.envelope.store_id, newer.envelope.store_id);
                assert_eq!(restored.envelope.revision, "1");
            }
            assert!(has_exact_file(&storage, &previous.bytes));
            assert!(has_exact_file(&storage, b"broken main"));
        }
    }

    #[test]
    fn displaced_same_revision_with_different_contents_is_a_preserved_conflict() {
        let fixture = Fixture::new();
        let (mut storage, previous) = fixture.ready();
        let mut external = previous.envelope.clone();
        external.settings.start_minimized = true;
        let external = Document::canonical(external).unwrap();
        let candidate = changed(&previous);
        storage.fault = Some(TestFault::DisplacedConflict(external.bytes.clone()));
        assert!(matches!(
            storage.commit(&previous, candidate.clone()),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(MAIN).unwrap(), candidate.bytes);
        assert!(has_exact_file(&storage, &external.bytes));
        assert!(has_exact_file(&storage, &previous.bytes));
        drop(storage);
        let mut storage = fixture.open();
        let report = storage.load();
        assert_eq!(report.mode, ConfigMode::RecoveryRequired);
        assert!(report.candidates.len() >= 3);
        assert!(has_exact_file(&storage, &external.bytes));
        assert!(has_exact_file(&storage, &previous.bytes));
    }

    #[test]
    fn predecessor_precheck_conflict_remains_visible_after_restart() {
        let fixture = Fixture::new();
        let (mut storage, previous) = fixture.ready();
        let competing = Document::fresh(AppConfig {
            autostart: true,
            ..AppConfig::default()
        })
        .unwrap();
        fs::write(storage.main_path(), &competing.bytes).unwrap();
        assert!(matches!(
            storage.commit(&previous, changed(&previous)),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(MAIN).unwrap(), competing.bytes);
        assert!(has_exact_file(&storage, &previous.bytes));
        drop(storage);
        let loaded = fixture.open().load();
        assert_eq!(loaded.mode, ConfigMode::RecoveryRequired);
        assert!(loaded
            .candidates
            .iter()
            .any(|candidate| candidate.label.contains("checkpoint")));
    }

    #[test]
    fn replacement_losing_main_keeps_a_valid_independent_checkpoint() {
        let fixture = Fixture::new();
        let (mut storage, previous) = fixture.ready();
        storage.fault = Some(TestFault::DisplaceWithoutInstall);
        assert!(matches!(
            storage.commit(&previous, changed(&previous)),
            StoreOutcome::Blocked(_)
        ));
        assert!(read_optional(&storage.main_path()).unwrap().is_none());
        assert!(has_exact_file(&storage, &previous.bytes));
        drop(storage);
        let loaded = fixture.open().load();
        assert_eq!(loaded.mode, ConfigMode::RecoveryRequired);
        assert!(loaded
            .candidates
            .iter()
            .any(|candidate| candidate.label.contains("checkpoint")));
    }

    #[test]
    fn restoring_staging_keeps_an_independent_source_and_retires_active_artifact_names() {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        let staged = Document::fresh(AppConfig {
            autostart: true,
            ..AppConfig::default()
        })
        .unwrap();
        let old_stage = stage_name(&staged.envelope.transaction_id);
        write_new(&storage.path(&old_stage), &staged.bytes).unwrap();
        let report = storage.load();
        assert_eq!(report.mode, ConfigMode::RecoveryRequired);
        let candidate = report
            .candidates
            .iter()
            .find(|candidate| candidate.label.contains("Unconfirmed"))
            .unwrap();
        let (settings, source) = storage.restore_source(&candidate.id).unwrap();
        let restored = committed(storage.install(settings, Some(source), false));
        assert_eq!(restored.envelope.settings, staged.envelope.settings);
        assert_ne!(restored.envelope.store_id, staged.envelope.store_id);
        assert!(has_exact_file(&storage, &staged.bytes));
        assert!(storage
            .entries()
            .unwrap()
            .iter()
            .any(|(_, role)| matches!(role, Artifact::Source)));
        assert!(storage
            .entries()
            .unwrap()
            .iter()
            .any(|(_, role)| matches!(role, Artifact::Quarantine)));
        assert!(!storage.path(&old_stage).exists());
        drop(storage);
        assert_eq!(fixture.open().load().mode, ConfigMode::Ready);
    }

    #[test]
    fn quarantine_alone_is_prior_data_and_reset_preserves_it() {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        let archive = storage.fresh_name("quarantine").unwrap();
        write_new(&storage.path(&archive), b"damaged prior settings").unwrap();
        assert_eq!(storage.load().mode, ConfigMode::RecoveryRequired);
        committed(storage.install(AppConfig::default(), None, false));
        assert_eq!(
            storage.required(&archive).unwrap(),
            b"damaged prior settings"
        );
        drop(storage);
        assert_eq!(fixture.open().load().mode, ConfigMode::Ready);
    }

    #[test]
    fn windows_filename_case_cannot_hide_interrupted_settings_evidence() {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        let document = Document::fresh(AppConfig::default()).unwrap();
        let renamed = stage_name(&document.envelope.transaction_id).to_uppercase();
        write_new(&storage.path(&renamed), &document.bytes).unwrap();
        let report = storage.load();
        assert_eq!(report.mode, ConfigMode::RecoveryRequired);
        assert!(!report.candidates.is_empty());
        assert!(matches!(
            storage.install(AppConfig::default(), None, true),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(&renamed).unwrap(), document.bytes);
        committed(storage.install(AppConfig::default(), None, false));
        assert!(has_exact_file(&storage, &document.bytes));
    }

    #[test]
    fn reset_quarantines_invalid_checkpoint_roles_instead_of_reentering_recovery() {
        let fixture = Fixture::new();
        let (mut storage, previous) = fixture.ready();
        let checkpoint = checkpoint_name(
            &previous.envelope.transaction_id,
            &Uuid::new_v4().to_string(),
        );
        write_new(&storage.path(&checkpoint), b"corrupted checkpoint").unwrap();
        assert_eq!(storage.load().mode, ConfigMode::RecoveryRequired);
        committed(storage.install(AppConfig::default(), None, false));
        assert!(has_exact_file(&storage, b"corrupted checkpoint"));
        assert!(!storage.path(&checkpoint).exists());
        drop(storage);
        assert_eq!(fixture.open().load().mode, ConfigMode::Ready);
    }

    #[test]
    fn recovery_does_not_overwrite_an_unexpected_destination() {
        let fixture = Fixture::new();
        let (mut storage, old) = fixture.ready();
        let competing = Document::fresh(AppConfig {
            start_minimized: true,
            ..AppConfig::default()
        })
        .unwrap();
        storage.fault = Some(TestFault::InstallDestination(competing.bytes.clone()));
        assert!(matches!(
            storage.install(AppConfig::default(), None, false),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(MAIN).unwrap(), competing.bytes);
        assert!(has_exact_file(&storage, &old.bytes));
        drop(storage);
        assert_eq!(fixture.open().load().mode, ConfigMode::RecoveryRequired);
    }

    #[test]
    fn reported_install_error_with_exact_candidate_and_evidence_can_commit() {
        let fixture = Fixture::new();
        let (mut storage, old) = fixture.ready();
        storage.fault = Some(TestFault::InstallCommittedError);
        let new = committed(storage.install(
            AppConfig {
                autostart: true,
                ..AppConfig::default()
            },
            None,
            false,
        ));
        assert_ne!(old.envelope.store_id, new.envelope.store_id);
        assert!(has_exact_file(&storage, &old.bytes));
        assert_eq!(storage.required(MAIN).unwrap(), new.bytes);
    }

    #[test]
    fn schema_changes_after_bootstrap_cannot_be_bypassed_by_reset() {
        let fixture = Fixture::new();
        let (mut storage, _) = fixture.ready();
        let future = br#"{"schema_version":500,"settings":{"important":"future"}}"#;
        fs::write(storage.main_path(), future).unwrap();
        assert!(matches!(
            storage.install(AppConfig::default(), None, false),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(MAIN).unwrap(), future);
        assert_eq!(
            storage.blocked_report("conflict").mode,
            ConfigMode::UnsupportedSchema
        );
    }

    #[test]
    fn preserved_future_schema_is_not_an_indirect_reset_or_downgrade_route() {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        let name = storage.fresh_name("quarantine").unwrap();
        let future = br#"{"schema_version":9,"future_setting":true}"#;
        write_new(&storage.path(&name), future).unwrap();
        assert_eq!(storage.load().mode, ConfigMode::UnsupportedSchema);
        assert!(matches!(
            storage.install(AppConfig::default(), None, false),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(storage.required(&name).unwrap(), future);
        assert!(read_optional(&storage.main_path()).unwrap().is_none());
    }

    #[test]
    fn future_intent_schema_is_detected_inside_preserved_transaction_bytes() {
        let fixture = Fixture::new();
        let mut storage = fixture.open();
        let supported = Document::fresh(AppConfig::default()).unwrap();
        let mut future = Intent {
            protocol_version: 1,
            candidate: supported.envelope.clone(),
            operation: Operation::Install {
                source: None,
                preserved: Vec::new(),
            },
        };
        assert_eq!(
            unsupported_schema_evidence(&json_bytes(&future).unwrap()),
            None
        );
        future.candidate.schema_version = 3;
        let future_bytes = json_bytes(&future).unwrap();
        assert_eq!(unsupported_schema_evidence(&future_bytes), Some(3));

        let inherited = Intent {
            protocol_version: 1,
            candidate: Document::fresh(AppConfig::default()).unwrap().envelope,
            operation: Operation::Install {
                source: None,
                preserved: vec![PreservedFile {
                    original: intent_name(&future.candidate.transaction_id),
                    archive: storage.fresh_name("quarantine").unwrap(),
                    bytes: future_bytes,
                }],
            },
        };
        assert!(inherited.validate().is_err());
        let bytes = json_bytes(&inherited).unwrap();
        assert_eq!(unsupported_schema_evidence(&bytes), Some(3));
        let archive = storage.fresh_name("quarantine").unwrap();
        write_new(&storage.path(&archive), &bytes).unwrap();
        assert_eq!(storage.load().mode, ConfigMode::UnsupportedSchema);
        assert!(matches!(
            storage.install(AppConfig::default(), None, false),
            StoreOutcome::Blocked(_)
        ));
        assert_eq!(
            storage.blocked_report("future transaction").mode,
            ConfigMode::UnsupportedSchema
        );
        assert_eq!(storage.required(&archive).unwrap(), bytes);
        assert_eq!(storage.entries().unwrap().len(), 1);
        assert!(read_optional(&storage.main_path()).unwrap().is_none());

        let mut malformed = serde_json::to_value(&inherited).unwrap();
        malformed["operation"]["predecessor"] = serde_json::json!("not encoded bytes");
        malformed["operation"]["source"] = serde_json::json!({ "bytes": [999] });
        assert_eq!(
            unsupported_schema_evidence(&json_bytes(&malformed).unwrap()),
            Some(3)
        );
        assert_eq!(
            unsupported_schema_evidence(br#"{"candidate":{"schema_version":3},"operation":null}"#),
            Some(3)
        );
    }

    #[test]
    fn exclusive_creation_and_move_collisions_preserve_existing_bytes() {
        let fixture = Fixture::new();
        let storage = fixture.open();
        let candidate = Document::fresh(AppConfig::default()).unwrap();
        let occupied = storage.path(&stage_name(&candidate.envelope.transaction_id));
        write_new(&occupied, b"do not overwrite").unwrap();
        assert!(write_new(&occupied, &candidate.bytes).is_err());
        let unique = storage.unique_candidate(candidate.clone(), None).unwrap();
        assert_ne!(
            unique.envelope.transaction_id,
            candidate.envelope.transaction_id
        );
        let other = storage.path(&stage_name(&unique.envelope.transaction_id));
        write_new(&other, &unique.bytes).unwrap();
        assert!(move_absent(&other, &occupied).is_err());
        assert_eq!(
            read_optional(&occupied).unwrap().unwrap(),
            b"do not overwrite"
        );
        assert_eq!(read_optional(&other).unwrap().unwrap(), unique.bytes);
    }

    #[test]
    fn candidate_registry_does_not_accept_paths_and_revalidates_exact_bytes() {
        let fixture = Fixture::new();
        let (mut storage, original) = fixture.ready();
        let current = committed(storage.commit(&original, changed(&original)));
        let snapshot = storage.load();
        let id = &snapshot.candidates[0].id;
        assert!(storage.restore_source(r"..\legacy.json").is_err());
        assert!(storage.restore_source(&Uuid::new_v4().to_string()).is_err());
        let name = storage.candidates[id].name.clone();
        fs::write(storage.path(&name), &current.bytes).unwrap();
        assert!(storage.restore_source(id).is_err());
        assert_eq!(storage.required(MAIN).unwrap(), current.bytes);
    }
}

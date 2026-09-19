use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem;
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes,
    QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
    DISPLAYCONFIG_DEVICE_INFO_TYPE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_PATH_SOURCE_INFO,
    DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::LUID;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP,
    DISPLAY_DEVICE_PRIMARY_DEVICE,
};

const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const ENUMERATION_ATTEMPTS: usize = 3;
const VERIFICATION_ATTEMPTS: usize = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigGetAdvancedColorInfo2 {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    value: u32,
    color_encoding: u32,
    bits_per_color_channel: u32,
    active_color_mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigSetHdrState {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    value: u32,
}

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "CompareStringOrdinal"]
    fn compare_string_ordinal(
        left: *const u16,
        left_len: i32,
        right: *const u16,
        right_len: i32,
        ignore_case: i32,
    ) -> i32;
}

pub fn identity_eq(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() || left.contains('\0') || right.contains('\0') {
        return false;
    }
    let left: Vec<_> = left.encode_utf16().collect();
    let right: Vec<_> = right.encode_utf16().collect();
    let (Ok(left_len), Ok(right_len)) = (i32::try_from(left.len()), i32::try_from(right.len()))
    else {
        return false;
    };
    // Device-interface paths are opaque; Windows ordinal comparison is not a locale transform.
    unsafe { compare_string_ordinal(left.as_ptr(), left_len, right.as_ptr(), right_len, 1) == 2 }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetStatus {
    Ready,
    Disconnected,
    NotHdrCapable,
    NeedsConfirmation,
    IdentityUnavailable,
    Ambiguous,
    EnumerationFailed,
    StateUnavailable,
    AutomationPaused,
    ControllerConflict,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeHdrState {
    Hdr,
    Sdr,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAddress {
    pub adapter_id_low: u32,
    pub adapter_id_high: i32,
    pub target_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Empty when identity is unavailable; never a runtime LUID/target identifier.
    pub id: String,
    pub device_path: Option<String>,
    pub identity_status: TargetStatus,
    pub identity_error: Option<String>,
    pub name: String,
    pub adapter_id_low: u32,
    pub adapter_id_high: i32,
    pub target_id: u32,
    pub is_hdr_supported: bool,
    pub is_hdr_enabled: bool,
    pub hdr_state_known: bool,
    pub state_error: Option<String>,
    pub is_primary: bool,
}

impl MonitorInfo {
    pub(crate) fn address(&self) -> RuntimeAddress {
        RuntimeAddress {
            adapter_id_low: self.adapter_id_low,
            adapter_id_high: self.adapter_id_high,
            target_id: self.target_id,
        }
    }
}

pub fn monitor_is_selected(
    monitor: &MonitorInfo,
    target: &crate::config::TargetMonitor,
) -> bool {
    if monitor.identity_status != TargetStatus::Ready {
        return false;
    }
    let Some(identity) = &monitor.device_path else {
        return false;
    };
    match target {
        crate::config::TargetMonitor::All => monitor.is_hdr_supported,
        crate::config::TargetMonitor::Monitor { device_path, .. } => {
            identity_eq(identity, device_path)
        }
        crate::config::TargetMonitor::NeedsConfirmation { .. } => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    AlreadyInDesiredState,
    Changed,
    OutcomeUnknown,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Disconnected,
    NotHdrCapable,
    NeedsConfirmation,
    IdentityUnavailable,
    Ambiguous,
    EnumerationFailed,
    StateUnavailable,
    IdentityChanged,
    NativeRejected,
    ExternalChange,
    AuthorityDenied,
    AttemptBudgetExhausted,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorOutcome {
    pub device_path: Option<String>,
    pub display_name: Option<String>,
    pub requested_hdr: bool,
    pub outcome: OutcomeKind,
    pub failure: Option<FailureKind>,
    pub message: Option<String>,
    pub previous_hdr: Option<bool>,
    pub observed_hdr: Option<bool>,
    pub previous_hdr_user_enabled: Option<bool>,
    pub observed_hdr_user_enabled: Option<bool>,
    pub attempts: usize,
}

impl MonitorOutcome {
    pub(crate) fn failed(path: Option<String>, enable: bool, failure: DisplayFailure) -> Self {
        Self {
            device_path: path,
            display_name: None,
            requested_hdr: enable,
            outcome: OutcomeKind::Failed,
            failure: Some(failure.kind),
            message: Some(failure.message),
            previous_hdr: None,
            observed_hdr: None,
            previous_hdr_user_enabled: None,
            observed_hdr_user_enabled: None,
            attempts: 0,
        }
    }

    pub(crate) fn is_verified(&self) -> bool {
        matches!(
            self.outcome,
            OutcomeKind::Changed | OutcomeKind::AlreadyInDesiredState
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DisplayFailure {
    pub kind: FailureKind,
    pub message: String,
}

impl DisplayFailure {
    pub(crate) fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeApi {
    Hdr,
    LegacyAdvancedColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePurpose {
    AutomaticEnable,
    Manual,
    Cleanup,
}

#[derive(Debug, Clone)]
pub(crate) struct Endpoint {
    pub identity: String,
    pub name: String,
    pub address: RuntimeAddress,
    pub supported: bool,
    pub enabled: bool,
    pub user_enabled: bool,
    pub api: NativeApi,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeError(pub i32);

impl NativeError {
    fn is_unsupported(self) -> bool {
        matches!(self.0, 50 | 87 | 120)
    }
}

pub(crate) trait DisplayBackend {
    fn inventory(&mut self) -> Result<Vec<MonitorInfo>, String>;
    fn probe(&mut self, address: RuntimeAddress) -> Result<Endpoint, DisplayFailure>;
    fn set(
        &mut self,
        address: RuntimeAddress,
        api: NativeApi,
        enable: bool,
    ) -> Result<(), NativeError>;
    fn verification_pause(&mut self) {}
}

pub(crate) struct WindowsDisplay;

pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    WindowsDisplay.inventory()
}

fn retry_enumeration<T>(mut enumerate: impl FnMut() -> Result<T, u32>) -> Result<T, String> {
    for attempt in 0..ENUMERATION_ATTEMPTS {
        match enumerate() {
            Ok(value) => return Ok(value),
            Err(ERROR_INSUFFICIENT_BUFFER) if attempt + 1 < ENUMERATION_ATTEMPTS => {}
            Err(code) => return Err(format!("Display enumeration failed (Windows error {code})")),
        }
    }
    unreachable!()
}

fn enumerate_paths() -> Result<Vec<DISPLAYCONFIG_PATH_INFO>, String> {
    retry_enumeration(|| unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        let result =
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count);
        if result.0 != 0 {
            return Err(result.0);
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        let result = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        );
        if result.0 != 0 {
            return Err(result.0);
        }
        paths.truncate(path_count as usize);
        Ok(paths)
    })
}

fn decode_gdi_device_name(value: &[u16]) -> Result<String, String> {
    let end = value
        .iter()
        .position(|&character| character == 0)
        .ok_or_else(|| "GDI display name is not NUL-terminated".to_string())?;
    if end == 0 {
        return Err("Windows returned an empty GDI display name".into());
    }
    String::from_utf16(&value[..end]).map_err(|_| "GDI display name is not valid UTF-16".into())
}

fn primary_from_gdi_devices(
    source_name: &[u16],
    devices: impl IntoIterator<Item = DISPLAY_DEVICEW>,
) -> Result<bool, String> {
    let source_name = decode_gdi_device_name(source_name)?;
    for device in devices {
        let name = decode_gdi_device_name(&device.DeviceName)?;
        if name.eq_ignore_ascii_case(&source_name) {
            if !device
                .StateFlags
                .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
            {
                return Err(format!(
                    "GDI display source {source_name} is no longer attached to the desktop"
                ));
            }
            return Ok(device.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE));
        }
    }
    Err(format!(
        "GDI primary metadata is unavailable for display source {source_name}"
    ))
}

fn read_source_primary(source: &DISPLAYCONFIG_PATH_SOURCE_INFO) -> Result<bool, String> {
    let mut name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: source.adapterId,
            id: source.id,
        },
        ..Default::default()
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut name.header) };
    if result != 0 {
        return Err(format!(
            "Display source GDI name query failed (Windows error {result})"
        ));
    }
    let devices = (0..).map_while(|index| {
        let mut device = DISPLAY_DEVICEW {
            cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        unsafe { EnumDisplayDevicesW(None, index, &mut device, 0) }
            .as_bool()
            .then_some(device)
    });
    primary_from_gdi_devices(&name.viewGdiDeviceName, devices)
}

fn primary_source_labels(
    paths: &[DISPLAYCONFIG_PATH_INFO],
    mut read_primary: impl FnMut(&DISPLAYCONFIG_PATH_SOURCE_INFO) -> Result<bool, String>,
    mut report_error: impl FnMut(String),
) -> Vec<bool> {
    let mut sources = HashMap::new();
    paths
        .iter()
        .map(|path| {
            let source = &path.sourceInfo;
            let key = (
                source.adapterId.LowPart,
                source.adapterId.HighPart,
                source.id,
            );
            // Clone targets share a logical source, including any failed metadata query.
            *sources
                .entry(key)
                .or_insert_with(|| match read_primary(source) {
                    Ok(is_primary) => is_primary,
                    Err(error) => {
                        report_error(format!(
                            "Primary monitor label unavailable for source {key:?}: {error}"
                        ));
                        false
                    }
                })
        })
        .collect()
}

fn header(
    address: RuntimeAddress,
    info_type: DISPLAYCONFIG_DEVICE_INFO_TYPE,
    size: usize,
) -> DISPLAYCONFIG_DEVICE_INFO_HEADER {
    DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: info_type,
        size: size as u32,
        adapterId: LUID {
            LowPart: address.adapter_id_low,
            HighPart: address.adapter_id_high,
        },
        id: address.target_id,
    }
}

fn read_target_name(address: RuntimeAddress) -> Result<(String, String), DisplayFailure> {
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: header(
            address,
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(),
        ),
        ..Default::default()
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
    if result != 0 {
        return Err(DisplayFailure::new(
            FailureKind::IdentityUnavailable,
            format!("Monitor identity query failed (Windows error {result})"),
        ));
    }
    let decode = |value: &[u16]| {
        let end = value.iter().position(|&c| c == 0).unwrap_or(value.len());
        String::from_utf16(&value[..end])
    };
    let identity = decode(&target.monitorDevicePath).map_err(|_| {
        DisplayFailure::new(
            FailureKind::IdentityUnavailable,
            "Monitor identity is not valid UTF-16",
        )
    })?;
    if identity.is_empty() {
        return Err(DisplayFailure::new(
            FailureKind::IdentityUnavailable,
            "Windows returned an empty monitor device-interface path",
        ));
    }
    let name = decode(&target.monitorFriendlyDeviceName).unwrap_or_default();
    Ok((identity, name))
}

fn modern_hdr_state(value: u32, active_color_mode: u32) -> (bool, bool) {
    // The general advanced-color flag and user preference bits also describe WCG/inactive HDR.
    (value & (1 << 4) != 0, active_color_mode == 2)
}

fn read_color_state(
    address: RuntimeAddress,
) -> Result<(bool, bool, bool, NativeApi), DisplayFailure> {
    let mut modern = DisplayConfigGetAdvancedColorInfo2 {
        header: header(
            address,
            DISPLAYCONFIG_DEVICE_INFO_TYPE(15),
            mem::size_of::<DisplayConfigGetAdvancedColorInfo2>(),
        ),
        value: 0,
        color_encoding: 0,
        bits_per_color_channel: 0,
        active_color_mode: 0,
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut modern.header) };
    if result == 0 {
        let (supported, enabled) = modern_hdr_state(modern.value, modern.active_color_mode);
        return Ok((
            supported,
            enabled,
            modern.value & (1 << 5) != 0,
            NativeApi::Hdr,
        ));
    }
    if !NativeError(result).is_unsupported() {
        return Err(DisplayFailure::new(
            FailureKind::StateUnavailable,
            format!("HDR state query failed (Windows error {result})"),
        ));
    }
    let mut legacy = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
        header: header(
            address,
            DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
            mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>(),
        ),
        ..Default::default()
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut legacy.header) };
    if result != 0 {
        return Err(DisplayFailure::new(
            FailureKind::StateUnavailable,
            format!("Legacy advanced-color query failed (Windows error {result})"),
        ));
    }
    let value = unsafe { legacy.Anonymous.value };
    Ok((
        value & 1 != 0,
        value & 2 != 0,
        value & 2 != 0,
        NativeApi::LegacyAdvancedColor,
    ))
}

impl DisplayBackend for WindowsDisplay {
    fn inventory(&mut self) -> Result<Vec<MonitorInfo>, String> {
        let paths = enumerate_paths()?;
        let primary_labels =
            primary_source_labels(&paths, read_source_primary, |error| eprintln!("{error}"));
        let mut monitors: Vec<MonitorInfo> = Vec::new();
        for (index, (path, is_primary)) in paths.into_iter().zip(primary_labels).enumerate() {
            let address = RuntimeAddress {
                adapter_id_low: path.targetInfo.adapterId.LowPart,
                adapter_id_high: path.targetInfo.adapterId.HighPart,
                target_id: path.targetInfo.id,
            };
            if monitors.iter().any(|monitor| monitor.address() == address) {
                continue;
            }
            let (identity, name, identity_error) = match read_target_name(address) {
                Ok((identity, name)) => (Some(identity), name, None),
                Err(error) => (None, String::new(), Some(error.message)),
            };
            let (supported, enabled, state_error) = match read_color_state(address) {
                Ok((supported, enabled, _, _)) => (supported, enabled, None),
                Err(error) => (false, false, Some(error.message)),
            };
            monitors.push(MonitorInfo {
                id: identity.clone().unwrap_or_default(),
                identity_status: if identity.is_some() {
                    TargetStatus::Ready
                } else {
                    TargetStatus::IdentityUnavailable
                },
                device_path: identity,
                identity_error,
                name: if name.trim().is_empty() {
                    format!("Monitor {}", index + 1)
                } else {
                    name
                },
                adapter_id_low: address.adapter_id_low,
                adapter_id_high: address.adapter_id_high,
                target_id: address.target_id,
                is_hdr_supported: supported,
                is_hdr_enabled: enabled,
                hdr_state_known: state_error.is_none(),
                state_error,
                is_primary,
            });
        }
        for index in 0..monitors.len() {
            let ambiguous = monitors[index]
                .device_path
                .as_ref()
                .is_some_and(|identity| {
                    monitors.iter().enumerate().any(|(other_index, other)| {
                        other_index != index
                            && other
                                .device_path
                                .as_ref()
                                .is_some_and(|other| identity_eq(identity, other))
                            && other.address() != monitors[index].address()
                    })
                });
            if ambiguous {
                monitors[index].identity_status = TargetStatus::Ambiguous;
            }
        }
        Ok(monitors)
    }

    fn probe(&mut self, address: RuntimeAddress) -> Result<Endpoint, DisplayFailure> {
        let (identity, name) = read_target_name(address)?;
        let (supported, enabled, user_enabled, api) = read_color_state(address)?;
        // State querying may race a topology change too; validate again before returning the tuple.
        let (after, _) = read_target_name(address)?;
        if !identity_eq(&identity, &after) {
            return Err(DisplayFailure::new(
                FailureKind::IdentityChanged,
                "Display identity changed while reading its HDR state",
            ));
        }
        Ok(Endpoint {
            identity,
            name,
            address,
            supported,
            enabled,
            user_enabled,
            api,
        })
    }

    fn set(
        &mut self,
        address: RuntimeAddress,
        api: NativeApi,
        enable: bool,
    ) -> Result<(), NativeError> {
        let result = match api {
            NativeApi::Hdr => {
                let mut value = DisplayConfigSetHdrState {
                    header: header(
                        address,
                        DISPLAYCONFIG_DEVICE_INFO_TYPE(16),
                        mem::size_of::<DisplayConfigSetHdrState>(),
                    ),
                    value: u32::from(enable),
                };
                unsafe { DisplayConfigSetDeviceInfo(&mut value.header) }
            }
            NativeApi::LegacyAdvancedColor => {
                let mut value = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
                    header: header(
                        address,
                        DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
                        mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>(),
                    ),
                    Anonymous: DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0 {
                        value: u32::from(enable),
                    },
                };
                unsafe { DisplayConfigSetDeviceInfo(&mut value.header) }
            }
        };
        if result == 0 {
            Ok(())
        } else {
            Err(NativeError(result))
        }
    }

    fn verification_pause(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub(crate) fn resolve_identity<'a>(
    monitors: &'a [MonitorInfo],
    identity: &str,
) -> Result<&'a MonitorInfo, DisplayFailure> {
    if identity.is_empty() || identity.contains('\0') {
        return Err(DisplayFailure::new(
            FailureKind::IdentityUnavailable,
            "A durable monitor device-interface path is required",
        ));
    }
    let mut matches = monitors.iter().filter(|monitor| {
        monitor
            .device_path
            .as_ref()
            .is_some_and(|path| identity_eq(path, identity))
    });
    let selected = matches.next().ok_or_else(|| {
        DisplayFailure::new(
            FailureKind::Disconnected,
            "The selected monitor is not connected",
        )
    })?;
    if selected.identity_status == TargetStatus::Ambiguous
        || matches.any(|other| other.address() != selected.address())
    {
        return Err(DisplayFailure::new(
            FailureKind::Ambiguous,
            "Multiple display endpoints claim this monitor identity",
        ));
    }
    Ok(selected)
}

pub(crate) fn resolve_monitor<'a>(
    monitors: &'a [MonitorInfo],
    identity: &str,
) -> Result<&'a MonitorInfo, DisplayFailure> {
    let selected = resolve_identity(monitors, identity)?;
    if !selected.hdr_state_known {
        return Err(DisplayFailure::new(
            FailureKind::StateUnavailable,
            selected
                .state_error
                .clone()
                .unwrap_or_else(|| "HDR state is unavailable".into()),
        ));
    }
    if !selected.is_hdr_supported {
        return Err(DisplayFailure::new(
            FailureKind::NotHdrCapable,
            "The selected monitor does not report HDR support",
        ));
    }
    Ok(selected)
}

fn fresh_endpoint(
    backend: &mut impl DisplayBackend,
    identity: &str,
) -> Result<Endpoint, DisplayFailure> {
    for retry in 0..=1 {
        let monitors = backend
            .inventory()
            .map_err(|message| DisplayFailure::new(FailureKind::EnumerationFailed, message))?;
        let selected = resolve_monitor(&monitors, identity)?;
        match backend.probe(selected.address()) {
            Ok(endpoint) if identity_eq(identity, &endpoint.identity) => {
                if !endpoint.supported {
                    return Err(DisplayFailure::new(
                        FailureKind::NotHdrCapable,
                        "The monitor no longer reports HDR support",
                    ));
                }
                return Ok(endpoint);
            }
            Err(error)
                if retry == 1
                    || !matches!(
                        error.kind,
                        FailureKind::IdentityChanged
                            | FailureKind::IdentityUnavailable
                            | FailureKind::Disconnected
                    ) =>
            {
                return Err(error);
            }
            _ if retry == 0 => {}
            _ => {
                return Err(DisplayFailure::new(
                    FailureKind::IdentityChanged,
                    "The current display address no longer identifies the selected monitor",
                ));
            }
        }
    }
    unreachable!()
}

#[derive(Debug, Clone)]
pub(crate) struct NativeAttempt {
    pub device_path: String,
    pub address: RuntimeAddress,
    pub api: NativeApi,
    pub requested_hdr: bool,
    pub previous_hdr: bool,
    pub previous_hdr_user_enabled: bool,
    pub purpose: NativePurpose,
}

fn desired_state(endpoint: &Endpoint, enable: bool) -> bool {
    endpoint.enabled == enable && endpoint.user_enabled == enable
}

/// Called only by the controller. Each compatibility attempt repeats resolution and authorization.
pub(crate) fn set_hdr(
    backend: &mut impl DisplayBackend,
    identity: &str,
    enable: bool,
    purpose: NativePurpose,
    mut authorize: impl FnMut(&NativeAttempt) -> Result<(), DisplayFailure>,
) -> MonitorOutcome {
    let mut outcome = MonitorOutcome {
        device_path: Some(identity.to_string()),
        display_name: None,
        requested_hdr: enable,
        outcome: OutcomeKind::Failed,
        failure: None,
        message: None,
        previous_hdr: None,
        observed_hdr: None,
        previous_hdr_user_enabled: None,
        observed_hdr_user_enabled: None,
        attempts: 0,
    };
    let mut compatibility_retry = false;
    loop {
        let endpoint = match fresh_endpoint(backend, identity) {
            Ok(endpoint) => endpoint,
            Err(error) => {
                outcome.failure = Some(error.kind);
                outcome.message = Some(error.message);
                return outcome;
            }
        };
        outcome.display_name = Some(endpoint.name.clone());
        outcome.previous_hdr.get_or_insert(endpoint.enabled);
        outcome
            .previous_hdr_user_enabled
            .get_or_insert(endpoint.user_enabled);
        outcome.observed_hdr = Some(endpoint.enabled);
        outcome.observed_hdr_user_enabled = Some(endpoint.user_enabled);
        if desired_state(&endpoint, enable)
            || (purpose == NativePurpose::AutomaticEnable && endpoint.enabled)
        {
            outcome.outcome = OutcomeKind::AlreadyInDesiredState;
            return outcome;
        }
        if purpose == NativePurpose::Cleanup && !endpoint.enabled {
            outcome.failure = Some(FailureKind::ExternalChange);
            outcome.message = Some(
                "HDR became inactive outside this operation; ownership was relinquished without changing the remaining Windows HDR preference".into(),
            );
            return outcome;
        }
        if enable && endpoint.user_enabled && !endpoint.enabled {
            outcome.failure = Some(FailureKind::StateUnavailable);
            outcome.message = Some(
                "Windows already has HDR enabled by user preference, but HDR is not active (possibly limited by system policy)".into(),
            );
            return outcome;
        }
        let api = if compatibility_retry {
            NativeApi::LegacyAdvancedColor
        } else {
            endpoint.api
        };
        let attempt = NativeAttempt {
            device_path: identity.to_string(),
            address: endpoint.address,
            api,
            requested_hdr: enable,
            previous_hdr: endpoint.enabled,
            previous_hdr_user_enabled: endpoint.user_enabled,
            purpose,
        };
        if let Err(error) = authorize(&attempt) {
            outcome.failure = Some(error.kind);
            outcome.message = Some(error.message);
            return outcome;
        }
        outcome.attempts += 1;
        let result = backend.set(endpoint.address, api, enable);
        let mut observed = None;
        let mut observed_user_enabled = None;
        let mut verification_error = None;
        for verification in 0..VERIFICATION_ATTEMPTS {
            if verification != 0 {
                backend.verification_pause();
            }
            match fresh_endpoint(backend, identity) {
                Ok(current) => {
                    observed = Some(current.enabled);
                    observed_user_enabled = Some(current.user_enabled);
                    if desired_state(&current, enable) {
                        outcome.observed_hdr = observed;
                        outcome.observed_hdr_user_enabled = observed_user_enabled;
                        outcome.outcome = OutcomeKind::Changed;
                        if let Err(error) = result {
                            outcome.message = Some(format!(
                                "Native error {} reconciled against the requested HDR state",
                                error.0
                            ));
                        }
                        return outcome;
                    }
                }
                Err(error) => verification_error = Some(error.message),
            }
        }
        outcome.observed_hdr = observed;
        outcome.observed_hdr_user_enabled = observed_user_enabled;
        if let Err(error) = result {
            if error.is_unsupported()
                && observed == Some(endpoint.enabled)
                && observed_user_enabled == Some(endpoint.user_enabled)
            {
                if api == NativeApi::Hdr && !compatibility_retry {
                    compatibility_retry = true;
                    continue;
                }
                outcome.failure = Some(FailureKind::NativeRejected);
                outcome.message = Some(format!(
                    "Native HDR API is unsupported or rejected the request (Windows error {})",
                    error.0
                ));
                return outcome;
            }
        }
        outcome.outcome = OutcomeKind::OutcomeUnknown;
        let native_error = result
            .err()
            .map(|error| format!(" (Windows error {})", error.0))
            .unwrap_or_default();
        outcome.message = Some(format!(
            "HDR may have changed; the requested state could not be verified{native_error}{}",
            verification_error
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        ));
        return outcome;
    }
}

pub(crate) fn status_for_failure(kind: FailureKind) -> TargetStatus {
    match kind {
        FailureKind::Disconnected => TargetStatus::Disconnected,
        FailureKind::NotHdrCapable => TargetStatus::NotHdrCapable,
        FailureKind::NeedsConfirmation => TargetStatus::NeedsConfirmation,
        FailureKind::IdentityUnavailable | FailureKind::IdentityChanged => {
            TargetStatus::IdentityUnavailable
        }
        FailureKind::Ambiguous => TargetStatus::Ambiguous,
        FailureKind::EnumerationFailed => TargetStatus::EnumerationFailed,
        FailureKind::StateUnavailable | FailureKind::NativeRejected => {
            TargetStatus::StateUnavailable
        }
        FailureKind::AuthorityDenied
        | FailureKind::AttemptBudgetExhausted
        | FailureKind::ExternalChange => TargetStatus::AutomationPaused,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn display_path(source_id: u32, target_id: u32) -> DISPLAYCONFIG_PATH_INFO {
        let mut path = DISPLAYCONFIG_PATH_INFO::default();
        path.sourceInfo.adapterId = LUID {
            LowPart: 20,
            HighPart: -1,
        };
        path.sourceInfo.id = source_id;
        path.targetInfo.adapterId = LUID {
            LowPart: 10,
            HighPart: 0,
        };
        path.targetInfo.id = target_id;
        path
    }

    fn gdi_device(name: &str, primary: bool) -> DISPLAY_DEVICEW {
        let mut device = DISPLAY_DEVICEW {
            cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
            StateFlags: if primary {
                DISPLAY_DEVICE_ATTACHED_TO_DESKTOP | DISPLAY_DEVICE_PRIMARY_DEVICE
            } else {
                DISPLAY_DEVICE_ATTACHED_TO_DESKTOP
            },
            ..Default::default()
        };
        let name: Vec<_> = name.encode_utf16().collect();
        assert!(name.len() < device.DeviceName.len());
        device.DeviceName[..name.len()].copy_from_slice(&name);
        device
    }

    #[test]
    fn primary_source_need_not_be_the_first_path_or_gdi_device() {
        let secondary = gdi_device(r"\\.\DISPLAY1", false);
        let primary = gdi_device(r"\\.\DISPLAY2", true);
        let labels = primary_source_labels(
            &[display_path(7, 1), display_path(9, 44)],
            |source| {
                let name = if source.id == 9 {
                    gdi_device(r"\\.\display2", false).DeviceName
                } else {
                    secondary.DeviceName
                };
                primary_from_gdi_devices(&name, [secondary, primary])
            },
            |error| panic!("Unexpected primary metadata failure: {error}"),
        );
        assert_eq!(labels, [false, true]);
    }

    #[test]
    fn primary_labels_follow_sources_when_paths_are_reordered() {
        for paths in [
            [display_path(7, 1), display_path(9, 44)],
            [display_path(9, 44), display_path(7, 1)],
        ] {
            let labels = primary_source_labels(
                &paths,
                |source| Ok(source.id == 9),
                |error| panic!("Unexpected primary metadata failure: {error}"),
            );
            let labelled_targets: Vec<_> = paths
                .iter()
                .zip(labels)
                .filter_map(|(path, primary)| primary.then_some(path.targetInfo.id))
                .collect();
            assert_eq!(labelled_targets, [44]);
        }
    }

    #[test]
    fn clone_targets_share_one_primary_source_metadata_query() {
        let mut queries = Vec::new();
        let labels = primary_source_labels(
            &[display_path(7, 1), display_path(9, 44), display_path(9, 45)],
            |source| {
                assert!(!queries.contains(&source.id));
                queries.push(source.id);
                Ok(source.id == 9)
            },
            |error| panic!("Unexpected primary metadata failure: {error}"),
        );
        assert_eq!(labels, [false, true, true]);
        assert_eq!(queries, [7, 9]);
    }

    #[test]
    fn source_ids_on_different_adapters_do_not_share_primary_metadata() {
        let primary = display_path(9, 44);
        let mut other_low = display_path(9, 45);
        other_low.sourceInfo.adapterId.LowPart = 21;
        let mut other_high = display_path(9, 46);
        other_high.sourceInfo.adapterId.HighPart = 0;
        let mut queries = 0;
        let labels = primary_source_labels(
            &[primary, other_low, other_high],
            |source| {
                queries += 1;
                Ok(source.adapterId.LowPart == 20 && source.adapterId.HighPart == -1)
            },
            |error| panic!("Unexpected primary metadata failure: {error}"),
        );
        assert_eq!(labels, [true, false, false]);
        assert_eq!(queries, 3);
    }

    #[test]
    fn absent_or_failing_primary_metadata_omits_badges_and_reports_diagnostics() {
        let name = gdi_device(r"\\.\DISPLAY1", false).DeviceName;
        let mut diagnostics = Vec::new();
        let labels = primary_source_labels(
            &[display_path(7, 1), display_path(9, 44), display_path(9, 45)],
            |source| {
                if source.id == 7 {
                    primary_from_gdi_devices(&name, [])
                } else {
                    Err("Display source GDI name query failed (Windows error 5)".into())
                }
            },
            |error| diagnostics.push(error),
        );
        assert_eq!(labels, [false, false, false]);
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].contains("GDI primary metadata is unavailable"));
        assert!(diagnostics[1].contains("Windows error 5"));
    }

    #[test]
    fn invalid_missing_or_detached_gdi_primary_metadata_is_rejected() {
        let primary = gdi_device(r"\\.\DISPLAY2", true);
        let name = primary.DeviceName;
        assert!(primary_from_gdi_devices(&[0; 32], [primary]).is_err());
        assert!(primary_from_gdi_devices(&[0xd800, 0], [primary]).is_err());
        assert!(primary_from_gdi_devices(&[65; 32], [primary]).is_err());
        assert!(primary_from_gdi_devices(&name, [gdi_device(r"\\.\DISPLAY1", true)]).is_err());
        let mut detached = primary;
        detached.StateFlags = DISPLAY_DEVICE_PRIMARY_DEVICE;
        assert!(primary_from_gdi_devices(&name, [detached]).is_err());
        let mut malformed = primary;
        malformed.DeviceName[0] = 0xd800;
        assert!(primary_from_gdi_devices(&name, [malformed]).is_err());
    }

    pub(crate) fn monitor(path: &str, target: u32, enabled: bool) -> MonitorInfo {
        MonitorInfo {
            id: path.into(),
            device_path: Some(path.into()),
            identity_status: TargetStatus::Ready,
            identity_error: None,
            name: "Same model name".into(),
            adapter_id_low: 10,
            adapter_id_high: 0,
            target_id: target,
            is_hdr_supported: true,
            is_hdr_enabled: enabled,
            hdr_state_known: true,
            state_error: None,
            is_primary: target == 1,
        }
    }

    #[derive(Default)]
    pub(crate) struct MockDisplay {
        pub monitors: Vec<MonitorInfo>,
        pub writes: Vec<(RuntimeAddress, NativeApi, bool)>,
        pub results: VecDeque<Result<(), NativeError>>,
        pub update_state: bool,
        pub mismatched_path: Option<String>,
        pub after_set: Option<Box<dyn FnMut(&mut Vec<MonitorInfo>)>>,
        pub enumeration_error: Option<String>,
        pub probes_fail_after_write: bool,
        pub user_enabled_override: Option<bool>,
    }

    impl MockDisplay {
        pub(crate) fn new(monitors: Vec<MonitorInfo>) -> Self {
            Self {
                monitors,
                update_state: true,
                ..Self::default()
            }
        }
    }

    impl DisplayBackend for MockDisplay {
        fn inventory(&mut self) -> Result<Vec<MonitorInfo>, String> {
            if let Some(error) = &self.enumeration_error {
                Err(error.clone())
            } else {
                Ok(self.monitors.clone())
            }
        }

        fn probe(&mut self, address: RuntimeAddress) -> Result<Endpoint, DisplayFailure> {
            if self.probes_fail_after_write && !self.writes.is_empty() {
                return Err(DisplayFailure::new(
                    FailureKind::StateUnavailable,
                    "mock query failure",
                ));
            }
            let monitor = self
                .monitors
                .iter()
                .find(|monitor| monitor.address() == address)
                .ok_or_else(|| DisplayFailure::new(FailureKind::Disconnected, "disconnected"))?;
            Ok(Endpoint {
                identity: self
                    .mismatched_path
                    .clone()
                    .unwrap_or_else(|| monitor.id.clone()),
                name: monitor.name.clone(),
                address,
                supported: monitor.is_hdr_supported,
                enabled: monitor.is_hdr_enabled,
                user_enabled: self.user_enabled_override.unwrap_or(monitor.is_hdr_enabled),
                api: NativeApi::Hdr,
            })
        }

        fn set(
            &mut self,
            address: RuntimeAddress,
            api: NativeApi,
            enable: bool,
        ) -> Result<(), NativeError> {
            self.writes.push((address, api, enable));
            let result = self.results.pop_front().unwrap_or(Ok(()));
            if self.update_state && result.is_ok() {
                if let Some(monitor) = self
                    .monitors
                    .iter_mut()
                    .find(|monitor| monitor.address() == address)
                {
                    monitor.is_hdr_enabled = enable;
                }
                if self.user_enabled_override.is_some() {
                    self.user_enabled_override = Some(enable);
                }
            }
            if let Some(after_set) = &mut self.after_set {
                after_set(&mut self.monitors);
            }
            result
        }
    }

    #[test]
    fn primary_labels_do_not_change_identity_based_hdr_targeting() {
        for metadata_available in [true, false] {
            let paths = [display_path(7, 1), display_path(9, 44)];
            let mut diagnostics = Vec::new();
            let labels = primary_source_labels(
                &paths,
                |source| {
                    if metadata_available {
                        Ok(source.id == 7)
                    } else {
                        Err("Primary metadata query failed".into())
                    }
                },
                |error| diagnostics.push(error),
            );
            let mut monitors = vec![monitor("primary", 1, false), monitor("selected", 44, false)];
            for (monitor, is_primary) in monitors.iter_mut().zip(labels) {
                monitor.is_primary = is_primary;
                assert_eq!(monitor.identity_status, TargetStatus::Ready);
                assert!(monitor.identity_error.is_none());
                assert!(monitor.hdr_state_known);
                assert!(monitor.state_error.is_none());
            }
            assert_eq!(monitors[0].is_primary, metadata_available);
            assert!(!monitors[1].is_primary);
            assert_eq!(diagnostics.len(), if metadata_available { 0 } else { 2 });
            let selected_address = monitors[1].address();
            let target = crate::config::TargetMonitor::Monitor {
                device_path: "selected".into(),
                display_name: monitors[0].name.clone(),
            };
            assert!(!monitor_is_selected(&monitors[0], &target));
            assert!(monitor_is_selected(&monitors[1], &target));
            let mut backend = MockDisplay::new(monitors);
            let result = set_hdr(
                &mut backend,
                "SELECTED",
                true,
                NativePurpose::Manual,
                |_| Ok(()),
            );
            assert_eq!(result.outcome, OutcomeKind::Changed);
            assert_eq!(backend.writes, [(selected_address, NativeApi::Hdr, true)]);
            assert!(!backend.monitors[0].is_hdr_enabled);
            assert_eq!(
                set_hdr(
                    &mut backend,
                    "missing",
                    true,
                    NativePurpose::Manual,
                    |_| Ok(())
                )
                .failure,
                Some(FailureKind::Disconnected)
            );
            assert_eq!(backend.writes.len(), 1);
        }
    }

    #[test]
    fn same_path_resolves_after_reboot_and_reordering() {
        let mut selected = monitor(r"\\?\DISPLAY#Selected", 44, false);
        selected.adapter_id_low = 987;
        let mut backend = MockDisplay::new(vec![monitor("other", 2, true), selected.clone()]);
        let result = set_hdr(
            &mut backend,
            r"\\?\display#selected",
            true,
            NativePurpose::Manual,
            |_| Ok(()),
        );
        assert_eq!(result.outcome, OutcomeKind::Changed);
        assert_eq!(backend.writes.len(), 1);
        assert_eq!(backend.writes[0].0, selected.address());
    }

    #[test]
    fn missing_ambiguous_and_mismatched_identities_never_write() {
        let mut backend = MockDisplay::new(vec![monitor("other", 1, false)]);
        assert_eq!(
            set_hdr(
                &mut backend,
                "missing",
                true,
                NativePurpose::Manual,
                |_| Ok(())
            )
            .failure,
            Some(FailureKind::Disconnected)
        );
        backend.monitors = vec![monitor("same", 1, false), monitor("SAME", 2, false)];
        assert_eq!(
            set_hdr(
                &mut backend,
                "same",
                true,
                NativePurpose::Manual,
                |_| Ok(())
            )
            .failure,
            Some(FailureKind::Ambiguous)
        );
        backend.monitors.truncate(1);
        backend.mismatched_path = Some("replacement".into());
        assert_eq!(
            set_hdr(
                &mut backend,
                "same",
                true,
                NativePurpose::Manual,
                |_| Ok(())
            )
            .failure,
            Some(FailureKind::IdentityChanged)
        );
        assert!(backend.writes.is_empty());
    }

    #[test]
    fn compatibility_attempt_resolves_a_new_tuple_and_authorizes_again() {
        let mut backend = MockDisplay::new(vec![monitor("chosen", 1, false)]);
        backend.results.push_back(Err(NativeError(50)));
        backend.after_set = Some(Box::new(|monitors| monitors[0].target_id = 92));
        let mut authorizations = 0;
        let result = set_hdr(&mut backend, "chosen", true, NativePurpose::Manual, |_| {
            authorizations += 1;
            Ok(())
        });
        assert_eq!(result.outcome, OutcomeKind::Changed);
        assert_eq!(authorizations, 2);
        assert_eq!(backend.writes[0].0.target_id, 1);
        assert_eq!(backend.writes[1].0.target_id, 92);
        assert_eq!(backend.writes[1].1, NativeApi::LegacyAdvancedColor);
    }

    #[test]
    fn native_uncertainty_never_uses_a_compatibility_or_global_fallback() {
        let mut backend = MockDisplay::new(vec![monitor("chosen", 1, false)]);
        backend.update_state = false;
        backend.results.push_back(Err(NativeError(31)));
        assert_eq!(
            set_hdr(&mut backend, "chosen", true, NativePurpose::Manual, |_| Ok(
                ()
            ))
            .outcome,
            OutcomeKind::OutcomeUnknown
        );
        assert_eq!(backend.writes.len(), 1);
    }

    #[test]
    fn enumeration_has_three_attempts_and_does_not_hide_failures() {
        let mut attempts = 0;
        let result = retry_enumeration::<()>(|| {
            attempts += 1;
            Err(ERROR_INSUFFICIENT_BUFFER)
        });
        assert!(result.is_err());
        assert_eq!(attempts, 3);
        attempts = 0;
        assert!(retry_enumeration::<()>(|| {
            attempts += 1;
            Err(5)
        })
        .is_err());
        assert_eq!(attempts, 1);
        assert_eq!(
            retry_enumeration(|| Ok(Vec::<u8>::new())).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn wcg_and_inactive_hdr_preferences_are_not_active_hdr() {
        assert_eq!(modern_hdr_state(1 | (1 << 6), 1), (false, false));
        assert_eq!(modern_hdr_state((1 << 4) | (1 << 5), 0), (true, false));
        assert_eq!(modern_hdr_state(1 << 4, 2), (true, true));
        assert_eq!(mem::size_of::<DisplayConfigGetAdvancedColorInfo2>(), 36);
        assert_eq!(mem::size_of::<DisplayConfigSetHdrState>(), 24);
    }

    #[test]
    fn manual_off_clears_an_inactive_hdr_user_preference_instead_of_claiming_already_off() {
        let mut backend = MockDisplay::new(vec![monitor("chosen", 1, false)]);
        backend.user_enabled_override = Some(true);
        let outcome = set_hdr(&mut backend, "chosen", false, NativePurpose::Manual, |_| {
            Ok(())
        });
        assert_eq!(outcome.outcome, OutcomeKind::Changed);
        assert_eq!(outcome.previous_hdr, Some(false));
        assert_eq!(outcome.previous_hdr_user_enabled, Some(true));
        assert_eq!(outcome.observed_hdr_user_enabled, Some(false));
        assert_eq!(backend.writes.len(), 1);
    }

    #[test]
    fn inactive_user_enabled_hdr_does_not_manufacture_an_automatic_change() {
        let mut backend = MockDisplay::new(vec![monitor("chosen", 1, false)]);
        backend.user_enabled_override = Some(true);
        let outcome = set_hdr(
            &mut backend,
            "chosen",
            true,
            NativePurpose::AutomaticEnable,
            |_| Ok(()),
        );
        assert_eq!(outcome.failure, Some(FailureKind::StateUnavailable));
        assert!(backend.writes.is_empty());
    }

    #[test]
    fn automatic_control_does_not_override_an_in_progress_windows_preference_change() {
        let mut backend = MockDisplay::new(vec![monitor("chosen", 1, true)]);
        backend.user_enabled_override = Some(false);
        let outcome = set_hdr(
            &mut backend,
            "chosen",
            true,
            NativePurpose::AutomaticEnable,
            |_| Ok(()),
        );
        assert_eq!(outcome.outcome, OutcomeKind::AlreadyInDesiredState);
        assert!(backend.writes.is_empty());
        backend.monitors[0].is_hdr_enabled = false;
        backend.user_enabled_override = Some(true);
        let outcome = set_hdr(
            &mut backend,
            "chosen",
            false,
            NativePurpose::Cleanup,
            |_| Ok(()),
        );
        assert_eq!(outcome.failure, Some(FailureKind::ExternalChange));
        assert!(backend.writes.is_empty());
    }

    #[test]
    fn selection_badges_use_durable_ordinal_identity_and_reject_unavailable_identities() {
        let mut selected = monitor("DEVICE-PATH", 1, false);
        let target = crate::config::TargetMonitor::Monitor {
            device_path: "device-path".into(),
            display_name: "A cached label is not an identity".into(),
        };
        assert!(monitor_is_selected(&selected, &target));
        selected.target_id = 900;
        assert!(monitor_is_selected(&selected, &target));
        selected.identity_status = TargetStatus::Ambiguous;
        assert!(!monitor_is_selected(&selected, &target));
        assert!(!monitor_is_selected(
            &selected,
            &crate::config::TargetMonitor::All
        ));
        selected.identity_status = TargetStatus::IdentityUnavailable;
        selected.device_path = None;
        assert!(!monitor_is_selected(&selected, &target));
    }
}

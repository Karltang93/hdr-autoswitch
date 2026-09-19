use crate::config::{
    AppConfig, ConfigManager, ConfigMode, ConfigSnapshot, HdrApp, SwitchMethod, TargetMonitor,
};
pub use crate::display::ScopeHdrState;
use crate::display::{
    DisplayFailure, FailureKind, MonitorInfo, MonitorOutcome, NativeAttempt, NativePurpose,
    OutcomeKind, TargetStatus, WindowsDisplay,
};
use crate::hdr_controller::{same_target, HdrController, ProcessIdentity, WriteAuthority};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use windows::core::PWSTR;
use windows::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WAIT_TIMEOUT, WPARAM,
};
use windows::Win32::System::Threading::{
    CreateEventW, GetCurrentThreadId, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
    SetEvent, WaitForMultipleObjects, WaitForSingleObject, INFINITE, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PeekMessageW,
    PostThreadMessageW, TranslateMessage, EVENT_SYSTEM_FOREGROUND, MSG, PM_NOREMOVE,
    WINEVENT_OUTOFCONTEXT, WM_QUIT,
};

const SHUTDOWN_ATTEMPT_BUDGET: usize = 32;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ManualControl {
    Available,
    Blocked { reason: String },
}

impl ManualControl {
    fn require(self) -> Result<(), String> {
        match self {
            Self::Available => Ok(()),
            Self::Blocked { reason } => Err(reason),
        }
    }
}

fn manual_admission(snapshot: &ConfigSnapshot, admitted: bool) -> ManualControl {
    let reason = if !admitted {
        Some("The HDR controller is shutting down".into())
    } else if let Some(issue) = &snapshot.controller_issue {
        Some(issue.clone())
    } else if snapshot.mode == ConfigMode::Unavailable {
        Some("Configuration authority is unavailable; manual HDR is blocked".into())
    } else {
        None
    };
    match reason {
        Some(reason) => ManualControl::Blocked { reason },
        None => ManualControl::Available,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HdrStatePayload {
    /// Observed frozen activation scope, or the saved scope when no activation exists.
    pub is_hdr_active: bool,
    pub scope_hdr_state: ScopeHdrState,
    pub manual_control: ManualControl,
    pub current_app_name: Option<String>,
    pub current_exe: Option<String>,
    pub switched_by_app: bool,
    pub steam_id: Option<String>,
    pub launcher: Option<String>,
    pub hdr_type: Option<String>,
    pub warning: Option<String>,
    /// Availability of the saved target for the next activation.
    pub target_status: TargetStatus,
    pub active_target: Option<TargetMonitor>,
    pub target_deferred: bool,
    pub any_hdr_active: bool,
    pub inventory_stale: bool,
    pub uncertain_targets: Vec<String>,
    pub operation_outcomes: Vec<MonitorOutcome>,
}

impl HdrStatePayload {
    fn unavailable(message: String) -> Self {
        Self {
            is_hdr_active: false,
            scope_hdr_state: ScopeHdrState::Unknown,
            manual_control: ManualControl::Blocked { reason: message.clone() },
            current_app_name: None,
            current_exe: None,
            switched_by_app: false,
            steam_id: None,
            launcher: None,
            hdr_type: None,
            warning: Some(message),
            target_status: TargetStatus::AutomationPaused,
            active_target: None,
            target_deferred: false,
            any_hdr_active: false,
            inventory_stale: true,
            uncertain_targets: Vec::new(),
            operation_outcomes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManualSetResult {
    pub outcomes: Vec<MonitorOutcome>,
    pub partial: bool,
    pub status: HdrStatePayload,
}

enum Command {
    ForegroundObserved,
    ConfigCommitted,
    Refresh(Sender<Result<Vec<MonitorInfo>, String>>),
    ManualSet(TargetMonitor, bool, Sender<Result<ManualSetResult, String>>),
    Status(Sender<Result<HdrStatePayload, String>>),
    GameExited {
        generation: u64,
        process: ProcessIdentity,
    },
    WatcherFailed {
        generation: u64,
        process: ProcessIdentity,
        message: String,
    },
    HookUnavailable(String),
    Shutdown,
}

#[derive(Clone)]
struct EventSink {
    sender: Sender<Command>,
    admitted: Arc<AtomicBool>,
    foreground_pending: Arc<AtomicBool>,
    config_pending: Arc<AtomicBool>,
    hook_available: Arc<AtomicBool>,
    shutdown_budget: Arc<AtomicUsize>,
}

impl EventSink {
    fn foreground(&self) {
        if self.admitted.load(Ordering::Acquire)
            && !self.foreground_pending.swap(true, Ordering::AcqRel)
            && self.sender.send(Command::ForegroundObserved).is_err()
        {
            self.foreground_pending.store(false, Ordering::Release);
        }
    }
}

struct HookThread {
    id: u32,
    thread: JoinHandle<()>,
}

#[derive(Default)]
struct Threads {
    actor: Option<JoinHandle<()>>,
    hook: Option<HookThread>,
}

pub struct MonitorService {
    config: Arc<ConfigManager>,
    events: EventSink,
    threads: Mutex<Threads>,
}

static HOOK_EVENTS: OnceLock<Mutex<Option<EventSink>>> = OnceLock::new();

pub struct PendingRequest<T> {
    receiver: Receiver<Result<T, String>>,
}

impl<T: Send + 'static> PendingRequest<T> {
    pub async fn resolve(self) -> Result<T, String> {
        tauri::async_runtime::spawn_blocking(move || {
            self.receiver
                .recv()
                .map_err(|_| "The HDR controller stopped before responding".to_string())?
        })
        .await
        .map_err(|error| format!("Cannot receive HDR controller response: {error}"))?
    }
}

impl MonitorService {
    pub fn new(config_mgr: Arc<ConfigManager>, app_handle: AppHandle) -> Arc<Self> {
        let (sender, receiver) = channel();
        let events = EventSink {
            sender,
            admitted: Arc::new(AtomicBool::new(true)),
            foreground_pending: Arc::new(AtomicBool::new(false)),
            config_pending: Arc::new(AtomicBool::new(false)),
            hook_available: Arc::new(AtomicBool::new(false)),
            shutdown_budget: Arc::new(AtomicUsize::new(SHUTDOWN_ATTEMPT_BUDGET)),
        };
        let actor_events = events.clone();
        let actor_config = config_mgr.clone();
        let actor = thread::Builder::new()
            .name("hdr-controller".into())
            .spawn(move || {
                Actor::new(actor_config, app_handle, actor_events).run(receiver);
            })
            .expect("Unable to start the HDR controller");
        Arc::new(Self {
            config: config_mgr,
            events,
            threads: Mutex::new(Threads {
                actor: Some(actor),
                hook: None,
            }),
        })
    }

    pub fn start_hook(self: &Arc<Self>) {
        let mut threads = self
            .threads
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.events.admitted.load(Ordering::Acquire) || threads.hook.is_some() {
            return;
        }
        let global = HOOK_EVENTS.get_or_init(|| Mutex::new(None));
        *global.lock().unwrap_or_else(|error| error.into_inner()) = Some(self.events.clone());
        let (ready, ready_receiver) = channel();
        let events = self.events.clone();
        let hook = thread::Builder::new()
            .name("hdr-foreground-hook".into())
            .spawn(move || unsafe {
                let id = GetCurrentThreadId();
                let mut message = MSG::default();
                let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                let hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                if hook.is_invalid() {
                    let _ = ready.send(Err(
                        "Unable to install the foreground window hook".to_string()
                    ));
                    return;
                }
                events.hook_available.store(true, Ordering::Release);
                let _ = ready.send(Ok(id));
                events.foreground();
                loop {
                    let result = GetMessageW(&mut message, None, 0, 0);
                    if result.0 <= 0 {
                        break;
                    }
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let _ = UnhookWinEvent(hook);
                events.hook_available.store(false, Ordering::Release);
                if events.admitted.load(Ordering::Acquire) {
                    let _ = events.sender.send(Command::HookUnavailable(
                        "The foreground hook stopped; automatic HDR is paused".into(),
                    ));
                }
            });
        match hook {
            Ok(thread) => match ready_receiver.recv() {
                Ok(Ok(id)) => threads.hook = Some(HookThread { id, thread }),
                result => {
                    let _ = thread.join();
                    let message = match result {
                        Ok(Err(error)) => error,
                        _ => "The foreground hook stopped during initialization".into(),
                    };
                    let _ = self.events.sender.send(Command::HookUnavailable(message));
                }
            },
            Err(error) => {
                let _ = self.events.sender.send(Command::HookUnavailable(format!(
                    "Unable to start the foreground hook: {error}"
                )));
            }
        }
        self.events.foreground();
    }

    fn enqueue<T: Send + 'static>(
        &self,
        command: impl FnOnce(Sender<Result<T, String>>) -> Command,
    ) -> Result<PendingRequest<T>, String> {
        if !self.events.admitted.load(Ordering::Acquire) {
            return Err("The HDR controller is shutting down".into());
        }
        let (reply, receive) = channel();
        self.events
            .sender
            .send(command(reply))
            .map_err(|_| "The HDR controller is unavailable".to_string())?;
        Ok(PendingRequest { receiver: receive })
    }

    pub fn refresh(&self) -> Result<PendingRequest<Vec<MonitorInfo>>, String> {
        self.enqueue(Command::Refresh)
    }

    pub fn manual_set(
        &self,
        scope: TargetMonitor,
        enable: bool,
    ) -> Result<PendingRequest<ManualSetResult>, String> {
        manual_admission(&self.config.snapshot()?, self.events.admitted.load(Ordering::Acquire))
            .require()?;
        self.enqueue(|reply| Command::ManualSet(scope, enable, reply))
    }

    pub fn status(&self) -> Result<PendingRequest<HdrStatePayload>, String> {
        self.enqueue(Command::Status)
    }

    pub fn config_committed(&self) {
        if self.events.admitted.load(Ordering::Acquire)
            && !self.events.config_pending.swap(true, Ordering::AcqRel)
            && self.events.sender.send(Command::ConfigCommitted).is_err()
        {
            self.events.config_pending.store(false, Ordering::Release);
        }
    }

    pub fn shutdown(&self) {
        let mut threads = self
            .threads
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.events.admitted.store(false, Ordering::Release);
        self.events.hook_available.store(false, Ordering::Release);
        if let Some(global) = HOOK_EVENTS.get() {
            *global.lock().unwrap_or_else(|error| error.into_inner()) = None;
        }
        if let Some(hook) = threads.hook.take() {
            let _ = unsafe { PostThreadMessageW(hook.id, WM_QUIT, WPARAM(0), LPARAM(0)) };
            let _ = hook.thread.join();
        }
        if let Some(actor) = threads.actor.take() {
            let _ = self.events.sender.send(Command::Shutdown);
            // Never detach a timed-out actor: a synchronous native call cannot be cancelled.
            // The caller retains controller exclusion until this join has completed.
            if actor.join().is_err() {
                eprintln!("HDR controller exited unexpectedly; hardware state may require manual verification");
            }
        }
    }
}

impl Drop for MonitorService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if let Some(events) = HOOK_EVENTS.get() {
        if let Ok(events) = events.lock() {
            if let Some(events) = events.as_ref() {
                events.foreground();
            }
        }
    }
}

struct OwnedHandle(HANDLE);

// Kernel process/event handles support cross-thread waits; one RAII owner closes each handle.
unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[derive(Clone)]
struct TrackedProcess {
    identity: ProcessIdentity,
    exe: String,
    handle: Arc<OwnedHandle>,
}

impl TrackedProcess {
    fn is_alive(&self) -> bool {
        unsafe { WaitForSingleObject(self.handle.0, 0) == WAIT_TIMEOUT }
    }

    fn is_foreground(&self) -> bool {
        self.is_alive() && foreground_pid() == self.identity.pid
    }
}

fn foreground_pid() -> u32 {
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return 0;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut pid));
    }
    pid
}

fn observe_foreground() -> Result<Option<TrackedProcess>, String> {
    let pid = foreground_pid();
    if pid == 0 {
        return Ok(None);
    }
    let handle = Arc::new(OwnedHandle(
        unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }
        .map_err(|error| format!("Unable to inspect foreground process {pid}: {error}"))?,
    ));
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_FORMAT(0),
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|error| format!("Unable to read foreground executable: {error}"))?;
    let path = String::from_utf16_lossy(&path[..length as usize]);
    let exe = Path::new(&path)
        .file_name()
        .ok_or("Foreground executable has no filename")?
        .to_string_lossy()
        .to_lowercase();
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) }
        .map_err(|error| format!("Unable to identify foreground process lifetime: {error}"))?;
    let process = TrackedProcess {
        identity: ProcessIdentity {
            pid,
            created_at: (u64::from(created.dwHighDateTime) << 32)
                | u64::from(created.dwLowDateTime),
        },
        exe,
        handle,
    };
    if process.is_foreground() {
        Ok(Some(process))
    } else {
        Ok(None)
    }
}

struct ProcessWatcher {
    generation: u64,
    process: ProcessIdentity,
    cancelled: Arc<OwnedHandle>,
    thread: Option<JoinHandle<()>>,
}

impl ProcessWatcher {
    fn new(process: TrackedProcess, generation: u64, events: EventSink) -> Result<Self, String> {
        let cancelled = Arc::new(OwnedHandle(
            unsafe { CreateEventW(None, true, false, None) }.map_err(|error| {
                format!("Unable to create process-watcher cancellation event: {error}")
            })?,
        ));
        let cancellation = cancelled.clone();
        let identity = process.identity;
        let thread = thread::Builder::new()
            .name("hdr-game-exit".into())
            .spawn(move || {
                let result = unsafe {
                    WaitForMultipleObjects(&[process.handle.0, cancellation.0], false, INFINITE)
                };
                if result == WAIT_OBJECT_0
                    && events.admitted.load(Ordering::Acquire)
                    && unsafe { WaitForSingleObject(cancellation.0, 0) } == WAIT_TIMEOUT
                {
                    let _ = events.sender.send(Command::GameExited {
                        generation,
                        process: identity,
                    });
                } else if result.0 != WAIT_OBJECT_0.0 + 1
                    && result != WAIT_OBJECT_0
                    && events.admitted.load(Ordering::Acquire)
                {
                    let _ = events.sender.send(Command::WatcherFailed {
                        generation,
                        process: identity,
                        message: format!(
                            "The process-exit watcher failed (wait result {:#x})",
                            result.0
                        ),
                    });
                }
            })
            .map_err(|error| format!("Unable to start process-exit watcher: {error}"))?;
        Ok(Self {
            generation,
            process: identity,
            cancelled,
            thread: Some(thread),
        })
    }
}

impl Drop for ProcessWatcher {
    fn drop(&mut self) {
        let _ = unsafe { SetEvent(self.cancelled.0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone, Copy)]
struct Debounce {
    deadline: Instant,
    generation: u64,
    process: ProcessIdentity,
    seconds: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    AutomaticEnable,
    Manual,
    Cleanup,
}

fn matches_app(app: &HdrApp, exe: &str) -> bool {
    app.exe_name.eq_ignore_ascii_case(exe)
        || app
            .alternate_exes
            .iter()
            .any(|alternate| alternate.eq_ignore_ascii_case(exe))
}

fn eligible_app<'a>(config: &'a AppConfig, exe: &str) -> Option<&'a HdrApp> {
    if config
        .blacklist
        .iter()
        .any(|blocked| blocked.eq_ignore_ascii_case(exe))
    {
        return None;
    }
    config
        .apps
        .iter()
        .find(|app| app.enabled && matches_app(app, exe))
}

fn automatic_pause(snapshot: &ConfigSnapshot, exe: &str, origin_context: &str) -> Option<String> {
    if let Some(issue) = &snapshot.controller_issue {
        return Some(issue.clone());
    }
    if snapshot.mode != ConfigMode::Ready {
        return Some(
            snapshot
                .issue
                .clone()
                .unwrap_or_else(|| "Automation is paused until configuration is ready".into()),
        );
    }
    if snapshot.context_token != origin_context {
        return Some("The originating configuration history has been retired".into());
    }
    if !matches!(snapshot.settings.switch_method, SwitchMethod::Native) {
        return Some(
            "Native HDR consent is required; keyboard-shortcut automation is disabled".into(),
        );
    }
    if eligible_app(&snapshot.settings, exe).is_none() {
        return Some("The tracked application is no longer eligible for automatic HDR".into());
    }
    None
}

fn publish_enrollment_result(
    result: Result<ConfigSnapshot, String>,
    latest_snapshot: impl FnOnce() -> Result<ConfigSnapshot, String>,
    emit: impl FnOnce(&ConfigSnapshot) -> Result<(), String>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let snapshot = match result {
        Ok(snapshot) => Ok(snapshot),
        Err(error) => {
            diagnostics.push(format!("Automatic game enrollment was skipped: {error}"));
            latest_snapshot()
        }
    };
    match snapshot {
        Ok(snapshot) => {
            if let Err(error) = emit(&snapshot) {
                diagnostics.push(format!(
                    "Unable to publish the canonical configuration after automatic enrollment: {error}"
                ));
            }
        }
        Err(error) => diagnostics.push(format!(
            "Unable to read the canonical configuration after automatic enrollment failed: {error}"
        )),
    }
    diagnostics
}

struct Authority {
    config: Arc<ConfigManager>,
    admitted: Arc<AtomicBool>,
    kind: OperationKind,
    process: Option<TrackedProcess>,
    context_token: Option<String>,
    hook_available: Arc<AtomicBool>,
    shutdown_budget: Arc<AtomicUsize>,
}

impl WriteAuthority for Authority {
    fn authorize(
        &mut self,
        attempt: &NativeAttempt,
        mark_issued: &mut dyn FnMut(),
    ) -> Result<(), DisplayFailure> {
        let valid_purpose = match self.kind {
            OperationKind::AutomaticEnable => {
                attempt.purpose == NativePurpose::AutomaticEnable && attempt.requested_hdr
            }
            OperationKind::Manual => attempt.purpose == NativePurpose::Manual,
            OperationKind::Cleanup => {
                attempt.purpose == NativePurpose::Cleanup && !attempt.requested_hdr
            }
        };
        if !valid_purpose {
            return Err(DisplayFailure::new(
                FailureKind::AuthorityDenied,
                "The HDR operation does not match its authorization purpose",
            ));
        }
        let live = self.process.as_ref().is_some_and(TrackedProcess::is_alive);
        let foreground = self
            .process
            .as_ref()
            .is_some_and(TrackedProcess::is_foreground);
        self.config
            .with_control_snapshot(|snapshot| {
                let denied =
                    |message| Err(DisplayFailure::new(FailureKind::AuthorityDenied, message));
                if let Some(issue) = &snapshot.controller_issue {
                    return denied(issue.clone());
                }
                if self.kind == OperationKind::Manual {
                    if let Err(reason) = manual_admission(
                        snapshot,
                        self.admitted.load(Ordering::Acquire),
                    ).require() {
                        return denied(reason);
                    }
                }
                if self.kind != OperationKind::Cleanup && !self.admitted.load(Ordering::Acquire) {
                    return denied("The HDR controller is shutting down".into());
                }
                if self.kind == OperationKind::AutomaticEnable {
                    if !self.hook_available.load(Ordering::Acquire) {
                        return denied(
                            "Automatic HDR is paused because the foreground hook is unavailable"
                                .into(),
                        );
                    }
                    let Some(process) = &self.process else {
                        return denied("There is no tracked application process".into());
                    };
                    if !live || (!snapshot.settings.exit_only_hdr && !foreground) {
                        return denied(
                            "The tracked game no longer satisfies the foreground/exit policy"
                                .into(),
                        );
                    }
                    if let Some(reason) = automatic_pause(
                        snapshot,
                        &process.exe,
                        self.context_token.as_deref().unwrap_or(""),
                    ) {
                        return denied(reason);
                    }
                }
                if self.kind == OperationKind::Cleanup
                    && !self.admitted.load(Ordering::Acquire)
                    && self
                        .shutdown_budget
                        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                            remaining.checked_sub(1)
                        })
                        .is_err()
                {
                    return Err(DisplayFailure::new(
                        FailureKind::AttemptBudgetExhausted,
                        "The bounded shutdown cleanup attempt budget has been exhausted",
                    ));
                }
                // No native call, process wait, or disk I/O is made while the publication lock is held.
                mark_issued();
                Ok(())
            })
            .map_err(|error| DisplayFailure::new(FailureKind::AuthorityDenied, error))?
    }
}

struct Actor {
    config: Arc<ConfigManager>,
    app: AppHandle,
    events: EventSink,
    controller: HdrController<WindowsDisplay>,
    tracked: Option<TrackedProcess>,
    watcher: Option<ProcessWatcher>,
    debounce: Option<Debounce>,
    last_outcomes: Vec<MonitorOutcome>,
}

impl Actor {
    fn new(config: Arc<ConfigManager>, app: AppHandle, events: EventSink) -> Self {
        Self {
            config,
            app,
            events,
            controller: HdrController::new(WindowsDisplay),
            tracked: None,
            watcher: None,
            debounce: None,
            last_outcomes: Vec::new(),
        }
    }

    fn authority(&self, kind: OperationKind) -> Authority {
        Authority {
            config: self.config.clone(),
            admitted: self.events.admitted.clone(),
            kind,
            process: self.tracked.clone(),
            context_token: self
                .controller
                .activation()
                .map(|active| active.context_token.clone()),
            hook_available: self.events.hook_available.clone(),
            shutdown_budget: self.events.shutdown_budget.clone(),
        }
    }

    fn run(mut self, receiver: Receiver<Command>) {
        loop {
            if self
                .debounce
                .is_some_and(|timer| timer.deadline <= Instant::now())
            {
                let expired = self.debounce.take();
                if self.events.admitted.load(Ordering::Acquire) {
                    let _ = self.observe(expired, true);
                    self.publish();
                }
                continue;
            }
            let received = if let Some(timer) = self.debounce {
                receiver.recv_timeout(timer.deadline.saturating_duration_since(Instant::now()))
            } else {
                receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
            };
            let command = match received {
                Ok(command) => command,
                Err(RecvTimeoutError::Timeout) => {
                    let expired = self.debounce.take();
                    if self.events.admitted.load(Ordering::Acquire) {
                        let _ = self.observe(expired, true);
                        self.publish();
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            };
            match command {
                Command::Shutdown => break,
                Command::ForegroundObserved => {
                    self.events
                        .foreground_pending
                        .store(false, Ordering::Release);
                    if self.events.admitted.load(Ordering::Acquire) {
                        let _ = self.observe(None, true);
                        self.publish();
                    }
                }
                Command::ConfigCommitted => {
                    self.events.config_pending.store(false, Ordering::Release);
                    if self.events.admitted.load(Ordering::Acquire) {
                        let _ = self.observe(None, true);
                        self.publish();
                    }
                }
                Command::Refresh(reply) => {
                    let result = if self.events.admitted.load(Ordering::Acquire) {
                        self.observe(None, true).and_then(|_| {
                            if let Some(error) = self.controller.inventory_error() {
                                Err(error.to_string())
                            } else {
                                Ok(self.controller.inventory().to_vec())
                            }
                        })
                    } else {
                        Err("The HDR controller is shutting down".into())
                    };
                    self.publish();
                    let _ = reply.send(result);
                }
                Command::ManualSet(scope, enable, reply) => {
                    let result = if self.events.admitted.load(Ordering::Acquire) {
                        self.manual_set(scope, enable)
                    } else {
                        Err("The HDR controller is shutting down".into())
                    };
                    self.publish();
                    let _ = reply.send(result);
                }
                Command::Status(reply) => {
                    if !self.events.admitted.load(Ordering::Acquire) {
                        let _ = reply.send(Err("The HDR controller is shutting down".into()));
                        continue;
                    }
                    self.reconcile_gate();
                    let result = self
                        .config
                        .snapshot()
                        .map(|snapshot| self.payload(&snapshot));
                    self.publish();
                    let _ = reply.send(result);
                }
                Command::GameExited {
                    generation,
                    process,
                } => {
                    if self.controller.matches_activation(generation, process)
                        && self.events.admitted.load(Ordering::Acquire)
                    {
                        let _ = self.observe(None, true);
                        self.publish();
                    }
                }
                Command::WatcherFailed {
                    generation,
                    process,
                    message,
                } => {
                    if self.events.admitted.load(Ordering::Acquire)
                        && self.controller.matches_activation(generation, process)
                    {
                        self.controller.warn(message);
                        self.finish_activation(usize::MAX);
                        self.publish();
                    }
                }
                Command::HookUnavailable(message) => {
                    self.events.hook_available.store(false, Ordering::Release);
                    self.controller.warn(message);
                    if self.events.admitted.load(Ordering::Acquire) {
                        self.finish_activation(usize::MAX);
                        self.publish();
                    }
                }
            }
        }
        self.events.admitted.store(false, Ordering::Release);
        self.finish_activation(SHUTDOWN_ATTEMPT_BUDGET);
        self.publish();
    }

    fn enroll(&mut self, origin: &ConfigSnapshot, process: &TrackedProcess) {
        if !self.events.admitted.load(Ordering::Acquire)
            || !self.events.hook_available.load(Ordering::Acquire)
            || origin.mode != ConfigMode::Ready
            || origin.controller_issue.is_some()
            || !matches!(origin.settings.switch_method, SwitchMethod::Native)
            || !origin.settings.auto_detect_new_games
            || origin
                .settings
                .blacklist
                .iter()
                .any(|exe| exe.eq_ignore_ascii_case(&process.exe))
            || origin
                .settings
                .apps
                .iter()
                .any(|app| matches_app(app, &process.exe))
        {
            return;
        }
        let Some(entry) = crate::database::find_in_catalog(&process.exe) else {
            return;
        };
        if !process.is_foreground() {
            return;
        }
        let app = HdrApp {
            name: entry.name,
            exe_name: process.exe.clone(),
            enabled: true,
            hdr_type: entry.hdr_type,
            path: None,
            alternate_exes: Vec::new(),
            steam_id: entry.steam_id,
            launcher: None,
        };
        let admitted = self.events.admitted.clone();
        let hook_available = self.events.hook_available.clone();
        let result = self.config.mutate(
            &origin.context_token,
            Some(&origin.library_generation),
            true,
            |current| {
                if !admitted.load(Ordering::Acquire)
                    || !hook_available.load(Ordering::Acquire)
                    || !process.is_foreground()
                    || !current.auto_detect_new_games
                    || !matches!(current.switch_method, SwitchMethod::Native)
                    || current
                        .blacklist
                        .iter()
                        .any(|exe| exe.eq_ignore_ascii_case(&process.exe))
                    || current.apps.iter().any(|existing| {
                        matches_app(existing, &process.exe)
                            || existing.name.eq_ignore_ascii_case(&app.name)
                            || (app.steam_id.is_some() && existing.steam_id == app.steam_id)
                    })
                {
                    return Err("Automatic enrollment is no longer eligible".into());
                }
                current.apps.push(app.clone());
                Ok(())
            },
        );
        let diagnostics = publish_enrollment_result(
            result,
            || self.config.snapshot(),
            |snapshot| {
                self.app
                    .emit("config-changed", snapshot.clone())
                    .map_err(|error| error.to_string())
            },
        );
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
            self.controller.warn(diagnostic);
        }
    }

    fn ensure_watcher(&mut self) -> Result<(), String> {
        let (Some(active), Some(process)) = (self.controller.activation(), self.tracked.clone())
        else {
            return Ok(());
        };
        if self.watcher.as_ref().is_some_and(|watcher| {
            watcher.generation == active.generation && watcher.process == active.process
        }) {
            return Ok(());
        }
        let generation = active.generation;
        self.watcher = None;
        self.watcher = Some(ProcessWatcher::new(
            process,
            generation,
            self.events.clone(),
        )?);
        Ok(())
    }

    fn observe(&mut self, expired: Option<Debounce>, enable_automatic: bool) -> Result<(), String> {
        let _ = self.controller.refresh_inventory();
        let origin = match self.config.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.controller.warn(error.clone());
                self.finish_activation(usize::MAX);
                return Err(error);
            }
        };
        let foreground = match observe_foreground() {
            Ok(foreground) => foreground,
            Err(error) => {
                self.controller.warn(error);
                self.tracked
                    .as_ref()
                    .filter(|process| process.is_foreground())
                    .cloned()
            }
        };
        if let Some(process) = &foreground {
            self.enroll(&origin, process);
        }
        let snapshot = self.config.snapshot()?;
        let game = foreground.filter(|process| {
            automatic_pause(&snapshot, &process.exe, &snapshot.context_token).is_none()
                && process.is_foreground()
                && self.events.hook_available.load(Ordering::Acquire)
        });
        if let Some(active) = self.controller.activation().cloned() {
            let transferable =
                game.as_ref().is_some() && active.context_token == snapshot.context_token;
            if transferable {
                let process = game.as_ref().expect("transfer requires a game");
                self.controller
                    .transfer(process.identity, process.exe.clone());
                self.tracked = Some(process.clone());
                self.debounce = None;
            } else if automatic_pause(&snapshot, &active.exe, &active.context_token).is_some()
                || !self.tracked.as_ref().is_some_and(TrackedProcess::is_alive)
            {
                self.finish_activation(usize::MAX);
            } else if snapshot.settings.exit_only_hdr {
                self.debounce = None;
            } else if expired.is_some_and(|timer| {
                self.controller
                    .matches_activation(timer.generation, timer.process)
            }) {
                self.finish_activation(usize::MAX);
            } else {
                let seconds = snapshot.settings.alt_tab_delay_seconds;
                if !self.debounce.is_some_and(|timer| {
                    timer.generation == active.generation && timer.seconds == seconds
                }) {
                    let delay = Duration::from_secs(seconds);
                    let deadline = Instant::now()
                        .checked_add(delay)
                        .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400));
                    self.debounce = Some(Debounce {
                        deadline,
                        generation: active.generation,
                        process: active.process,
                        seconds,
                    });
                }
            }
        }
        let mut outcomes = Vec::new();
        if self.controller.activation().is_none() {
            if let Some(process) = game {
                match self.controller.begin(
                    process.identity,
                    process.exe.clone(),
                    snapshot.context_token.clone(),
                    snapshot.settings.target_monitor.clone(),
                ) {
                    Ok(skipped) => {
                        outcomes = skipped;
                        self.tracked = Some(process);
                    }
                    Err(error) => self.controller.warn(error.message),
                }
            }
        }
        if self.controller.activation().is_some() {
            if let Err(error) = self.ensure_watcher() {
                self.controller.warn(error);
                self.finish_activation(usize::MAX);
            } else if enable_automatic {
                let mut authority = self.authority(OperationKind::AutomaticEnable);
                outcomes.extend(self.controller.enable_activation(&mut authority));
                if !outcomes.is_empty() {
                    self.last_outcomes = outcomes.clone();
                }
                let _ = self.controller.refresh_inventory();
                self.reconcile_gate();
                if self.controller.activation().is_some()
                    && self
                        .controller
                        .scope_is_active(&snapshot.settings.target_monitor)
                {
                    self.notify_verified(&outcomes, true);
                }
            }
        }
        Ok(())
    }

    fn reconcile_gate(&mut self) {
        let Some(active) = self.controller.activation() else {
            return;
        };
        let reason = match self.config.snapshot() {
            Ok(snapshot) => automatic_pause(&snapshot, &active.exe, &active.context_token),
            Err(error) => Some(error),
        };
        let reason = reason.or_else(|| {
            (!self.events.hook_available.load(Ordering::Acquire))
                .then(|| "The foreground hook is unavailable; automatic HDR is paused".into())
        });
        if let Some(reason) = reason {
            self.controller.warn(reason);
            self.finish_activation(usize::MAX);
        }
    }

    fn finish_activation(&mut self, budget: usize) {
        self.debounce = None;
        self.watcher = None;
        let mut authority = self.authority(OperationKind::Cleanup);
        let budget = if self.events.admitted.load(Ordering::Acquire) {
            budget
        } else {
            budget.min(SHUTDOWN_ATTEMPT_BUDGET)
        };
        let outcomes = self.controller.end(&mut authority, budget);
        if !outcomes.is_empty() {
            self.last_outcomes = outcomes.clone();
        }
        self.tracked = None;
        let _ = self.controller.refresh_inventory();
        self.notify_verified(&outcomes, false);
    }

    fn manual_set(
        &mut self,
        scope: TargetMonitor,
        enable: bool,
    ) -> Result<ManualSetResult, String> {
        let snapshot = self.config.snapshot()?;
        manual_admission(&snapshot, self.events.admitted.load(Ordering::Acquire)).require()?;
        // Establish the logical interval without enabling HDR first. A manual Off while a game is
        // foreground must not produce an automatic On followed by a second hidden setter.
        let _ = self.controller.refresh_inventory();
        if let Ok(Some(process)) = observe_foreground() {
            if automatic_pause(&snapshot, &process.exe, &snapshot.context_token).is_none() {
                if self.controller.activation().is_none() {
                    match self.controller.begin(
                        process.identity,
                        process.exe.clone(),
                        snapshot.context_token.clone(),
                        snapshot.settings.target_monitor.clone(),
                    ) {
                        Ok(_) => self.tracked = Some(process),
                        Err(error) => self.controller.warn(error.message),
                    }
                } else if self
                    .controller
                    .activation()
                    .is_some_and(|active| active.context_token == snapshot.context_token)
                {
                    self.controller
                        .transfer(process.identity, process.exe.clone());
                    self.tracked = Some(process);
                    self.debounce = None;
                }
                if let Err(error) = self.ensure_watcher() {
                    self.controller.warn(error);
                }
            }
        }
        let mut authority = self.authority(OperationKind::Manual);
        let outcomes = self.controller.manual_set(&scope, enable, &mut authority);
        self.last_outcomes = outcomes.clone();
        for outcome in &outcomes {
            if outcome.outcome == OutcomeKind::Failed {
                if let Some(message) = &outcome.message {
                    self.controller
                        .warn(format!("The last manual HDR request failed: {message}"));
                }
            }
        }
        let _ = self.controller.refresh_inventory();
        self.reconcile_gate();
        let snapshot = self.config.snapshot()?;
        let partial = outcomes.is_empty() || outcomes.iter().any(|outcome| !outcome.is_verified());
        if !partial {
            self.notify_verified(&outcomes, enable);
        }
        Ok(ManualSetResult {
            outcomes,
            partial,
            status: self.payload(&snapshot),
        })
    }

    fn notify_verified(&self, outcomes: &[MonitorOutcome], enabled: bool) {
        if outcomes.is_empty()
            || outcomes.iter().any(|outcome| !outcome.is_verified())
            || !outcomes
                .iter()
                .any(|outcome| outcome.outcome == OutcomeKind::Changed)
        {
            return;
        }
        let Ok(snapshot) = self.config.snapshot() else {
            return;
        };
        if !snapshot.settings.notifications_enabled {
            return;
        }
        let body = if enabled {
            "The requested HDR state was verified"
        } else {
            "The requested SDR state was verified"
        };
        let _ = self
            .app
            .notification()
            .builder()
            .title("HDR Auto-Switch")
            .body(body)
            .show();
    }

    fn payload(&self, snapshot: &ConfigSnapshot) -> HdrStatePayload {
        let active = self.controller.activation();
        let app = active.and_then(|active| {
            snapshot
                .settings
                .apps
                .iter()
                .find(|app| matches_app(app, &active.exe))
        });
        let mut warnings: Vec<String> = self.controller.warning().into_iter().collect();
        if let Some(issue) = &snapshot.issue {
            warnings.push(issue.clone());
        }
        let mut target_status = self
            .controller
            .target_status(&snapshot.settings.target_monitor);
        if snapshot.mode != ConfigMode::Ready {
            target_status = TargetStatus::AutomationPaused;
            warnings.push("Automatic HDR is paused until configuration is ready".into());
        }
        if !matches!(snapshot.settings.switch_method, SwitchMethod::Native) {
            target_status = TargetStatus::AutomationPaused;
            warnings.push(
                "Native HDR consent is required; keyboard-shortcut automation is disabled".into(),
            );
        }
        if let Some(issue) = &snapshot.controller_issue {
            target_status = TargetStatus::ControllerConflict;
            warnings.push(issue.clone());
        }
        if let Some(error) = self.controller.inventory_error() {
            warnings.push(format!("Display inventory is stale: {error}"));
        }
        if !self.events.hook_available.load(Ordering::Acquire)
            && snapshot.controller_issue.is_none()
        {
            target_status = TargetStatus::AutomationPaused;
            warnings.push("The foreground hook is unavailable; automatic HDR is paused".into());
        }
        let target_deferred = snapshot.mode == ConfigMode::Ready && active
            .is_some_and(|active| !same_target(&active.target, &snapshot.settings.target_monitor));
        if target_deferred {
            warnings.push("The saved monitor target will apply to the next activation".into());
        }
        let observed_scope = active.map(|active| &active.target).or_else(|| {
            (snapshot.mode == ConfigMode::Ready).then_some(&snapshot.settings.target_monitor)
        });
        let scope_hdr_state = observed_scope
            .map(|scope| self.controller.scope_hdr_state(scope))
            .unwrap_or(ScopeHdrState::Unknown);
        HdrStatePayload {
            is_hdr_active: scope_hdr_state == ScopeHdrState::Hdr,
            scope_hdr_state,
            manual_control: manual_admission(snapshot, self.events.admitted.load(Ordering::Acquire)),
            current_app_name: active.map(|active| {
                app.map(|app| app.name.clone())
                    .unwrap_or_else(|| active.exe.clone())
            }),
            current_exe: active.map(|active| active.exe.clone()),
            switched_by_app: self.controller.has_ownership(),
            steam_id: app.and_then(|app| app.steam_id.clone()),
            launcher: app.and_then(|app| app.launcher.clone()),
            hdr_type: app.map(|app| app.hdr_type.as_str().to_string()),
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join("\n"))
            },
            target_status,
            active_target: active.map(|active| active.target.clone()),
            target_deferred,
            any_hdr_active: self.controller.inventory_error().is_none()
                && self
                    .controller
                    .inventory()
                    .iter()
                    .any(|monitor| monitor.hdr_state_known && monitor.is_hdr_enabled),
            inventory_stale: self.controller.inventory_error().is_some(),
            uncertain_targets: self.controller.uncertain_targets(),
            operation_outcomes: self.last_outcomes.clone(),
        }
    }

    fn publish(&mut self) {
        let mut payload = match self.config.snapshot() {
            Ok(snapshot) => self.payload(&snapshot),
            Err(error) => {
                let warning = self
                    .controller
                    .warning()
                    .map(|warning| format!("{warning}\n{error}"))
                    .unwrap_or(error);
                HdrStatePayload::unavailable(warning)
            }
        };
        if let Err(error) = self.app.emit("hdr-status-changed", payload.clone()) {
            let warning = format!("Unable to publish HDR status: {error}");
            eprintln!("{warning}");
            self.controller.warn(warning.clone());
            payload.warning = Some(match payload.warning.take() {
                Some(existing) => format!("{existing}\n{warning}"),
                None => warning,
            });
        }
        crate::tray::update_status(&self.app, &payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HdrType;
    use crate::display::{NativeApi, RuntimeAddress};

    fn ready_snapshot() -> ConfigSnapshot {
        let mut settings = AppConfig::default();
        settings.apps = vec![HdrApp {
            name: "Game".into(),
            exe_name: "game.exe".into(),
            enabled: true,
            hdr_type: HdrType::Native,
            path: None,
            alternate_exes: Vec::new(),
            steam_id: None,
            launcher: None,
        }];
        ConfigSnapshot {
            settings,
            mode: ConfigMode::Ready,
            store_id: Some("store".into()),
            revision: "1".into(),
            context_token: "context".into(),
            library_generation: "0".into(),
            control_epoch: "0".into(),
            issue: None,
            controller_issue: None,
            candidates: Vec::new(),
            config_path: "isolated-test".into(),
        }
    }

    struct GateFixture {
        manager: Arc<ConfigManager>,
        _directory: tempfile::TempDir,
    }

    impl GateFixture {
        fn new() -> Self {
            let directory = tempfile::Builder::new()
                .prefix(".hdr-gate-test-")
                .tempdir_in(env!("CARGO_MANIFEST_DIR"))
                .unwrap();
            let manager = Arc::new(
                ConfigManager::load(
                    directory.path().join("local"),
                    directory.path().join("absent-legacy.json"),
                )
                .unwrap(),
            );
            Self {
                manager,
                _directory: directory,
            }
        }

        fn authority(&self, kind: OperationKind) -> Authority {
            Authority {
                config: self.manager.clone(),
                admitted: Arc::new(AtomicBool::new(true)),
                kind,
                process: None,
                context_token: None,
                hook_available: Arc::new(AtomicBool::new(true)),
                shutdown_budget: Arc::new(AtomicUsize::new(SHUTDOWN_ATTEMPT_BUDGET)),
            }
        }

        fn ready(&self) -> ConfigSnapshot {
            let first = self.manager.snapshot().unwrap();
            let ready = self.manager.initialize(&first.context_token).unwrap();
            self.manager
                .mutate(&ready.context_token, None, true, |settings| {
                    settings.apps = ready_snapshot().settings.apps;
                    Ok(())
                })
                .unwrap()
        }
    }

    fn idle_service(config: Arc<ConfigManager>) -> (MonitorService, Receiver<Command>) {
        let (sender, receiver) = channel();
        let service = MonitorService {
            config,
            events: EventSink {
                sender,
                admitted: Arc::new(AtomicBool::new(true)),
                foreground_pending: Arc::new(AtomicBool::new(false)),
                config_pending: Arc::new(AtomicBool::new(false)),
                hook_available: Arc::new(AtomicBool::new(true)),
                shutdown_budget: Arc::new(AtomicUsize::new(SHUTDOWN_ATTEMPT_BUDGET)),
            },
            threads: Mutex::new(Threads::default()),
        };
        (service, receiver)
    }

    fn mock_attempt() -> NativeAttempt {
        NativeAttempt {
            device_path: "mock-monitor".into(),
            address: RuntimeAddress {
                adapter_id_low: 1,
                adapter_id_high: 0,
                target_id: 2,
            },
            api: NativeApi::Hdr,
            requested_hdr: true,
            previous_hdr: false,
            previous_hdr_user_enabled: false,
            purpose: NativePurpose::Manual,
        }
    }

    fn manual_fixture(mode: ConfigMode) -> GateFixture {
        let directory = tempfile::Builder::new()
            .prefix(".hdr-manual-test-")
            .tempdir_in(env!("CARGO_MANIFEST_DIR"))
            .unwrap();
        let local = directory.path().join("local");
        let legacy = directory.path().join("legacy.json");
        std::fs::create_dir(&local).unwrap();
        match mode {
            ConfigMode::RecoveryRequired => {
                std::fs::write(local.join("config-v2.json"), b"unreadable settings").unwrap();
            }
            ConfigMode::UnsupportedSchema => {
                std::fs::write(local.join("config-v2.json"), br#"{"schema_version":3}"#).unwrap();
            }
            ConfigMode::Unavailable => std::fs::create_dir(&legacy).unwrap(),
            ConfigMode::Ready | ConfigMode::ImportAvailable => {
                let mut settings = serde_json::to_value(AppConfig::default()).unwrap();
                settings["switch_method"] = "shortcut".into();
                settings["target_monitor"] = "all".into();
                std::fs::write(&legacy, serde_json::to_vec(&settings).unwrap()).unwrap();
            }
            ConfigMode::FirstRun => {}
        }
        let manager = Arc::new(ConfigManager::load(local, legacy).unwrap());
        if mode == ConfigMode::Ready {
            let snapshot = manager.snapshot().unwrap();
            manager.import_legacy(&snapshot.context_token).unwrap();
        }
        assert_eq!(manager.snapshot().unwrap().mode, mode);
        GateFixture { manager, _directory: directory }
    }

    #[test]
    fn scoped_manual_hdr_in_paused_modes_does_not_grant_automatic_consent() {
        use crate::display::tests::{monitor, MockDisplay};

        for mode in [
            ConfigMode::FirstRun, ConfigMode::ImportAvailable, ConfigMode::RecoveryRequired,
            ConfigMode::UnsupportedSchema, ConfigMode::Ready,
        ] {
            let fixture = manual_fixture(mode);
            let before = fixture.manager.snapshot().unwrap();
            assert_eq!(manual_admission(&before, true), ManualControl::Available);
            assert!(automatic_pause(&before, "game.exe", &before.context_token).is_some());
            let mut authority = fixture.authority(OperationKind::Manual);
            let mut controller = HdrController::new(MockDisplay::new(vec![
                monitor("chosen", 1, false), monitor("other", 2, true),
            ]));
            let scope = TargetMonitor::Monitor {
                device_path: "chosen".into(), display_name: "Chosen display".into(),
            };
            for enabled in [true, false] {
                let outcomes = controller.manual_set(&scope, enabled, &mut authority);
                assert_eq!(outcomes.len(), 1);
                assert!(outcomes[0].is_verified());
                let inventory = controller.refresh_inventory().unwrap();
                assert_eq!(inventory[0].is_hdr_enabled, enabled);
                assert!(inventory[1].is_hdr_enabled);
                assert_eq!(fixture.manager.snapshot().unwrap(), before);
            }
            assert!(controller.activation().is_none());
            assert!(!controller.has_ownership());
        }
    }

    #[test]
    fn manual_entry_and_issuance_share_conflict_shutdown_and_unavailable_gates() {
        for mode in [ConfigMode::FirstRun, ConfigMode::Unavailable] {
            let fixture = manual_fixture(mode);
            let (service, receiver) = idle_service(fixture.manager.clone());
            if mode == ConfigMode::Unavailable {
                assert!(service.manual_set(TargetMonitor::All, true).is_err());
                let mut authority = fixture.authority(OperationKind::Manual);
                assert!(authority.authorize(&mock_attempt(), &mut || panic!("issued")).is_err());
            } else {
                fixture.manager.set_controller_issue(Some("controller conflict".into())).unwrap();
                assert!(service.manual_set(TargetMonitor::All, true).is_err());
                fixture.manager.set_controller_issue(None).unwrap();
                service.events.admitted.store(false, Ordering::Release);
                assert!(service.manual_set(TargetMonitor::All, false).is_err());
            }
            assert!(receiver.try_recv().is_err());
        }
        let unknown = HdrStatePayload::unavailable("Unreadable authority".into());
        assert!(matches!(unknown.manual_control, ManualControl::Blocked { .. }));
    }

    #[test]
    fn safe_test_mode_blocks_manual_control_without_granting_consent() {
        for mode in [
            ConfigMode::FirstRun,
            ConfigMode::ImportAvailable,
            ConfigMode::RecoveryRequired,
            ConfigMode::UnsupportedSchema,
            ConfigMode::Ready,
            ConfigMode::Unavailable,
        ] {
            let fixture = manual_fixture(mode);
            let before = fixture.manager.snapshot().unwrap();
            let blocked = crate::reconcile_controller(&fixture.manager, true).unwrap();
            assert_eq!(blocked.controller_issue.as_deref(), Some(crate::SAFE_TEST_ISSUE));
            assert_eq!(
                manual_admission(&blocked, true),
                ManualControl::Blocked { reason: crate::SAFE_TEST_ISSUE.into() }
            );
            assert_eq!(
                automatic_pause(&blocked, "game.exe", &blocked.context_token).as_deref(),
                Some(crate::SAFE_TEST_ISSUE)
            );
            let (service, receiver) = idle_service(fixture.manager.clone());
            for scope in [
                TargetMonitor::All,
                TargetMonitor::Monitor {
                    device_path: "chosen".into(),
                    display_name: "Chosen display".into(),
                },
            ] {
                for enabled in [true, false] {
                    assert_eq!(
                        service.manual_set(scope.clone(), enabled).err().as_deref(),
                        Some(crate::SAFE_TEST_ISSUE)
                    );
                }
            }
            assert!(receiver.try_recv().is_err());
            let mut authority = fixture.authority(OperationKind::Manual);
            for enabled in [true, false] {
                let mut attempt = mock_attempt();
                attempt.requested_hdr = enabled;
                let error = authority
                    .authorize(&attempt, &mut || panic!("Safe test mode issued native HDR"))
                    .unwrap_err();
                assert_eq!(error.kind, FailureKind::AuthorityDenied);
                assert_eq!(error.message, crate::SAFE_TEST_ISSUE);
            }
            assert_eq!(fixture.manager.snapshot().unwrap(), blocked);
            assert_eq!(blocked.settings, before.settings);
            assert_eq!(blocked.mode, before.mode);
            assert_eq!(blocked.revision, before.revision);
            assert_eq!(blocked.context_token, before.context_token);
        }
    }

    #[test]
    fn manual_requests_enqueue_in_click_order_without_waiting_for_the_actor() {
        let fixture = GateFixture::new();
        let (service, receiver) = idle_service(fixture.manager.clone());
        let on = service.manual_set(TargetMonitor::All, true).unwrap();
        let off = service.manual_set(TargetMonitor::All, false).unwrap();
        for (expected, result) in [(true, "first result"), (false, "second result")] {
            let Command::ManualSet(scope, enabled, reply) = receiver.try_recv().unwrap() else {
                panic!("Expected manual request");
            };
            assert_eq!(scope, TargetMonitor::All);
            assert_eq!(enabled, expected);
            reply.send(Err(result.into())).unwrap();
        }
        // Awaiting in the opposite order cannot change the already-enqueued native order.
        assert_eq!(tauri::async_runtime::block_on(off.resolve()).unwrap_err(), "second result");
        assert_eq!(tauri::async_runtime::block_on(on.resolve()).unwrap_err(), "first result");
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn manual_waiter_reports_a_dropped_actor_response_without_hanging() {
        let fixture = GateFixture::new();
        let (service, receiver) = idle_service(fixture.manager.clone());
        let pending = service.manual_set(TargetMonitor::All, false).unwrap();
        drop(receiver);
        assert!(tauri::async_runtime::block_on(pending.resolve())
            .unwrap_err().contains("stopped before responding"));
        assert!(service.manual_set(TargetMonitor::All, true).is_err());
    }

    #[test]
    fn queued_manual_requests_cannot_bypass_a_later_conflict_or_shutdown() {
        use crate::display::tests::{monitor, MockDisplay};

        for conflict in [true, false] {
            let fixture = GateFixture::new();
            let (service, receiver) = idle_service(fixture.manager.clone());
            let pending = service.manual_set(TargetMonitor::All, true).unwrap();
            if conflict {
                fixture.manager.set_controller_issue(Some("new conflict".into())).unwrap();
            } else {
                service.events.admitted.store(false, Ordering::Release);
            }
            let Command::ManualSet(scope, enabled, reply) = receiver.try_recv().unwrap() else {
                panic!("Expected queued manual request");
            };
            let mut authority = fixture.authority(OperationKind::Manual);
            authority.admitted = service.events.admitted.clone();
            let mut controller = HdrController::new(MockDisplay::new(vec![
                monitor("chosen", 1, false),
            ]));
            let outcomes = controller.manual_set(&scope, enabled, &mut authority);
            assert_eq!(outcomes[0].failure, Some(FailureKind::AuthorityDenied));
            assert!(!controller.refresh_inventory().unwrap()[0].is_hdr_enabled);
            reply.send(Err(outcomes[0].message.clone().unwrap())).unwrap();
            assert!(tauri::async_runtime::block_on(pending.resolve()).is_err());
        }
    }

    #[test]
    fn unchanged_enrichment_is_quiet_but_real_changes_and_conflicts_wake_the_actor() {
        let fixture = GateFixture::new();
        let before = fixture.ready();
        let (service, receiver) = idle_service(fixture.manager.clone());
        let unchanged = fixture.manager.mutate_if_changed(
            &before.context_token, Some(&before.library_generation), true, |settings| {
                assert!(!crate::library::enrich_existing(settings, &before.settings.apps));
                Ok(())
            },
        );
        assert!(matches!(&unchanged, Ok(None)));
        crate::background::publish_enrichment_result(
            &fixture.manager, &service, unchanged, |_| panic!("no-op emitted"),
        );
        assert!(receiver.try_recv().is_err());
        assert_eq!(fixture.manager.snapshot().unwrap(), before);
        let changed = fixture.manager.mutate_if_changed(
            &before.context_token, Some(&before.library_generation), true, |settings| {
                let mut detected = settings.apps[0].clone();
                detected.alternate_exes.push("new-alias.exe".into());
                assert!(crate::library::enrich_existing(settings, &[detected]));
                Ok(())
            },
        );
        assert!(matches!(&changed, Ok(Some(_))));
        let mut emitted = None;
        crate::background::publish_enrichment_result(
            &fixture.manager, &service, changed, |snapshot| emitted = Some(snapshot.clone()),
        );
        assert!(matches!(receiver.try_recv(), Ok(Command::ConfigCommitted)));
        assert_eq!(emitted, Some(fixture.manager.snapshot().unwrap()));
        service.events.config_pending.store(false, Ordering::Release);
        std::fs::write(&before.config_path, b"external conflict").unwrap();
        let failed = fixture.manager.mutate_if_changed(
            &before.context_token, None, true, |_| Ok(()),
        );
        assert!(failed.is_err());
        crate::background::publish_enrichment_result(
            &fixture.manager, &service, failed, |snapshot| {
                assert_eq!(snapshot.mode, ConfigMode::RecoveryRequired);
            },
        );
        assert!(matches!(receiver.try_recv(), Ok(Command::ConfigCommitted)));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn failed_enrollment_publishes_the_latest_canonical_recovery_snapshot() {
        let origin = ready_snapshot();
        let mut current = origin.clone();
        current.mode = ConfigMode::RecoveryRequired;
        current.context_token = "replacement-load-authority".into();
        current.control_epoch = "2".into();
        current.issue = Some("Replacement outcome is uncertain".into());
        let mut emitted = None;
        let diagnostics = publish_enrollment_result(
            Err("Commit outcome is uncertain".into()),
            || Ok(current.clone()),
            |snapshot| {
                emitted = Some(snapshot.clone());
                Ok(())
            },
        );
        assert_eq!(emitted, Some(current));
        assert_ne!(
            emitted.as_ref().unwrap().context_token,
            origin.context_token
        );
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("Commit outcome is uncertain")));
    }

    #[test]
    fn failed_enrollment_surfaces_snapshot_errors_without_emitting_stale_configuration() {
        let mut emitted = false;
        let diagnostics = publish_enrollment_result(
            Err("Write failed".into()),
            || Err("Configuration gate is poisoned".into()),
            |_| {
                emitted = true;
                Ok(())
            },
        );
        assert!(!emitted);
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("Write failed")));
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("Configuration gate is poisoned")));
    }

    #[test]
    fn enrollment_surfaces_event_failures_on_both_commit_and_error_paths() {
        for failed_commit in [false, true] {
            let snapshot = ready_snapshot();
            let result = if failed_commit {
                Err("Write failed".into())
            } else {
                Ok(snapshot.clone())
            };
            let mut reads = 0;
            let diagnostics = publish_enrollment_result(
                result,
                || {
                    reads += 1;
                    Ok(snapshot.clone())
                },
                |_| Err("Event delivery failed".into()),
            );
            assert_eq!(reads, usize::from(failed_commit));
            assert!(diagnostics
                .iter()
                .any(|message| message.contains("Event delivery failed")));
        }
    }

    #[test]
    fn scope_hdr_state_is_mandatory_and_serializes_all_four_distinct_states() {
        for (state, expected) in [
            (ScopeHdrState::Hdr, "hdr"),
            (ScopeHdrState::Sdr, "sdr"),
            (ScopeHdrState::Mixed, "mixed"),
            (ScopeHdrState::Unknown, "unknown"),
        ] {
            assert_eq!(serde_json::to_value(state).unwrap(), expected);
        }
        let payload = HdrStatePayload::unavailable("Read failed".into());
        let serialized = serde_json::to_value(payload).unwrap();
        assert_eq!(serialized["scope_hdr_state"], "unknown");
        assert_eq!(serialized["is_hdr_active"], false);
    }

    #[test]
    fn eligibility_requires_enabled_exact_or_alternate_executable_and_no_blacklist() {
        let mut config = AppConfig::default();
        config.apps = vec![HdrApp {
            name: "Game".into(),
            exe_name: "game.exe".into(),
            enabled: true,
            hdr_type: HdrType::Native,
            path: None,
            alternate_exes: vec!["game-dx12.exe".into()],
            steam_id: None,
            launcher: None,
        }];
        assert!(eligible_app(&config, "GAME.EXE").is_some());
        assert!(eligible_app(&config, "game-dx12.exe").is_some());
        assert!(eligible_app(&config, "game-unlisted.exe").is_none());
        config.apps[0].enabled = false;
        assert!(eligible_app(&config, "game-dx12.exe").is_none());
        config.apps[0].enabled = true;
        config.blacklist.push("GAME-DX12.EXE".into());
        assert!(eligible_app(&config, "game-dx12.exe").is_none());
    }

    #[test]
    fn producer_hints_are_coalesced_and_stop_after_admission_closes() {
        let fixture = GateFixture::new();
        let (service, receiver) = idle_service(fixture.manager.clone());
        let events = &service.events;
        events.foreground();
        events.foreground();
        assert!(matches!(
            receiver.try_recv(),
            Ok(Command::ForegroundObserved)
        ));
        assert!(receiver.try_recv().is_err());
        events.foreground_pending.store(false, Ordering::Release);
        service.config_committed();
        service.config_committed();
        assert!(matches!(receiver.try_recv(), Ok(Command::ConfigCommitted)));
        assert!(receiver.try_recv().is_err());
        events.config_pending.store(false, Ordering::Release);
        events.admitted.store(false, Ordering::Release);
        events.foreground();
        service.config_committed();
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn background_enrichment_wakes_controller_without_a_foreground_event() {
        let fixture = GateFixture::new();
        let before = fixture.ready();
        let (service, receiver) = idle_service(fixture.manager.clone());
        let foreground_exe = "game-dx12.exe";
        assert!(automatic_pause(&before, foreground_exe, &before.context_token).is_some());
        let mut detected = before.settings.apps[0].clone();
        detected.exe_name = foreground_exe.into();
        let result = fixture.manager.mutate(
            &before.context_token,
            Some(&before.library_generation),
            true,
            |settings| {
                crate::library::enrich_existing(settings, &[detected]);
                Ok(())
            },
        );
        let committed = result.as_ref().unwrap().clone();
        let mut emitted = None;
        crate::background::publish_result(&fixture.manager, &service, result, |snapshot| {
            emitted = Some(snapshot.clone());
            assert!(receiver.try_recv().is_err());
        });
        assert_eq!(emitted, Some(committed.clone()));
        assert!(matches!(receiver.try_recv(), Ok(Command::ConfigCommitted)));
        assert!(receiver.try_recv().is_err());
        assert!(!service.events.foreground_pending.load(Ordering::Acquire));
        let current = fixture.manager.snapshot().unwrap();
        assert_eq!(current, committed);
        assert!(automatic_pause(&current, foreground_exe, &before.context_token).is_none());
    }

    #[test]
    fn background_recovery_wakes_controller_after_failed_commit() {
        let fixture = GateFixture::new();
        let before = fixture.ready();
        let (service, receiver) = idle_service(fixture.manager.clone());
        assert!(automatic_pause(&before, "game.exe", &before.context_token).is_none());
        let conflicting_bytes = b"unreadable settings";
        std::fs::write(&before.config_path, conflicting_bytes).unwrap();
        let result = fixture.manager.mutate(
            &before.context_token,
            None,
            false,
            |settings| {
                settings.last_sync_timestamp = Some(123);
                Ok(())
            },
        );
        assert!(result.is_err());
        let current = fixture.manager.snapshot().unwrap();
        assert_eq!(current.mode, ConfigMode::RecoveryRequired);
        assert_ne!(current.context_token, before.context_token);
        let mut emitted = None;
        crate::background::publish_result(&fixture.manager, &service, result, |snapshot| {
            emitted = Some(snapshot.clone());
            assert!(receiver.try_recv().is_err());
        });
        assert_eq!(emitted, Some(current.clone()));
        assert!(matches!(receiver.try_recv(), Ok(Command::ConfigCommitted)));
        assert!(receiver.try_recv().is_err());
        assert!(!service.events.foreground_pending.load(Ordering::Acquire));
        assert!(automatic_pause(&current, "game.exe", &before.context_token).is_some());
        assert_eq!(std::fs::read(&before.config_path).unwrap(), conflicting_bytes);
    }

    #[test]
    fn current_gate_rejects_retired_history_recovery_withdrawn_consent_and_disabled_apps() {
        let mut snapshot = ready_snapshot();
        assert!(automatic_pause(&snapshot, "game.exe", "context").is_none());
        assert!(automatic_pause(&snapshot, "game.exe", "retired").is_some());
        snapshot.mode = ConfigMode::RecoveryRequired;
        assert!(automatic_pause(&snapshot, "game.exe", "context").is_some());
        snapshot.mode = ConfigMode::Ready;
        snapshot.settings.switch_method = SwitchMethod::Shortcut;
        assert!(automatic_pause(&snapshot, "game.exe", "context").is_some());
        snapshot.settings.switch_method = SwitchMethod::Native;
        snapshot.settings.apps[0].enabled = false;
        assert!(automatic_pause(&snapshot, "game.exe", "context").is_some());
        snapshot.settings.apps[0].enabled = true;
        snapshot.controller_issue = Some("predecessor conflict".into());
        assert!(automatic_pause(&snapshot, "game.exe", "context").is_some());
    }

    #[test]
    fn controller_conflict_blocks_manual_and_cleanup_authorizations_without_native_calls() {
        let fixture = GateFixture::new();
        fixture
            .manager
            .set_controller_issue(Some("predecessor conflict".into()))
            .unwrap();
        for kind in [OperationKind::Manual, OperationKind::Cleanup] {
            let mut authority = fixture.authority(kind);
            let mut issued = false;
            let mut attempt = mock_attempt();
            if kind == OperationKind::Cleanup {
                attempt.purpose = NativePurpose::Cleanup;
                attempt.requested_hdr = false;
            }
            let result = authority.authorize(&attempt, &mut || issued = true);
            assert_eq!(result.unwrap_err().kind, FailureKind::AuthorityDenied);
            assert!(!issued);
        }
    }

    #[test]
    fn closing_admission_blocks_manual_but_allows_only_bounded_cleanup() {
        let fixture = GateFixture::new();
        let mut authority = fixture.authority(OperationKind::Manual);
        authority.admitted.store(false, Ordering::Release);
        let mut issued = 0;
        assert!(authority
            .authorize(&mock_attempt(), &mut || issued += 1)
            .is_err());
        authority.kind = OperationKind::Cleanup;
        authority.shutdown_budget.store(1, Ordering::Release);
        let mut cleanup = mock_attempt();
        cleanup.purpose = NativePurpose::Cleanup;
        cleanup.requested_hdr = false;
        assert!(authority.authorize(&cleanup, &mut || issued += 1).is_ok());
        assert_eq!(
            authority
                .authorize(&cleanup, &mut || issued += 1)
                .unwrap_err()
                .kind,
            FailureKind::AttemptBudgetExhausted,
        );
        assert_eq!(issued, 1);
    }

    #[test]
    fn cleanup_authority_cannot_be_reused_to_enable_hdr() {
        let fixture = GateFixture::new();
        let mut authority = fixture.authority(OperationKind::Cleanup);
        let mut attempt = mock_attempt();
        attempt.purpose = NativePurpose::Cleanup;
        assert_eq!(
            authority
                .authorize(&attempt, &mut || panic!("cleanup enabled HDR"))
                .unwrap_err()
                .kind,
            FailureKind::AuthorityDenied,
        );
    }

    #[test]
    fn issuing_and_gate_publication_share_the_same_short_boundary() {
        let fixture = GateFixture::new();
        let (entered_tx, entered_rx) = channel();
        let (release_tx, release_rx) = channel();
        let (publishing_tx, publishing_rx) = channel();
        let (published_tx, published_rx) = channel();
        let issued = Arc::new(AtomicBool::new(false));
        let gate_manager = fixture.manager.clone();
        let gate_issued = issued.clone();
        let authorization = thread::spawn(move || {
            gate_manager
                .with_control_snapshot(|_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    gate_issued.store(true, Ordering::Release);
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();
        let publisher_manager = fixture.manager.clone();
        let publisher = thread::spawn(move || {
            publishing_tx.send(()).unwrap();
            publisher_manager
                .set_controller_issue(Some("closed".into()))
                .unwrap();
            published_tx.send(issued.load(Ordering::Acquire)).unwrap();
        });
        publishing_rx.recv().unwrap();
        let published_before_release = published_rx.try_recv().is_ok();
        release_tx.send(()).unwrap();
        authorization.join().unwrap();
        publisher.join().unwrap();
        assert!(!published_before_release);
        assert!(published_rx.recv().unwrap());
        let mut authority = fixture.authority(OperationKind::Manual);
        assert!(authority
            .authorize(&mock_attempt(), &mut || panic!(
                "closed gate issued a request"
            ))
            .is_err());
    }
}

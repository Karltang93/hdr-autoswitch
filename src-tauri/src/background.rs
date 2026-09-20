use crate::{
    config::{ConfigManager, ConfigMode, ConfigSnapshot},
    database, emit_config, library, monitor_hook::MonitorService, scanner,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, Weak,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

pub struct BackgroundWork {
    cancelled: Arc<AtomicBool>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

fn ready_snapshot(config: &Weak<ConfigManager>) -> Option<ConfigSnapshot> {
    let manager = config.upgrade()?;
    match manager.snapshot() {
        Ok(snapshot) if matches!(snapshot.mode, ConfigMode::Ready) => Some(snapshot),
        Ok(_) => None,
        Err(error) => {
            eprintln!("Background configuration read failed: {error}");
            None
        }
    }
}

pub(crate) fn publish_result(
    manager: &ConfigManager,
    monitor_service: &MonitorService,
    result: Result<ConfigSnapshot, String>,
    emit: impl FnOnce(&ConfigSnapshot),
) {
    match result {
        Ok(snapshot) => emit(&snapshot),
        Err(error) => {
            eprintln!("Background result was not committed: {error}");
            match manager.snapshot() {
                Ok(snapshot) => emit(&snapshot),
                Err(error) => eprintln!("Cannot publish background configuration state: {error}"),
            }
        }
    }
    monitor_service.config_committed();
}

pub(crate) fn publish_enrichment_result(
    manager: &ConfigManager,
    monitor_service: &MonitorService,
    result: Result<Option<ConfigSnapshot>, String>,
    emit: impl FnOnce(&ConfigSnapshot),
) {
    match result {
        Ok(None) => {}
        Ok(Some(snapshot)) => publish_result(manager, monitor_service, Ok(snapshot), emit),
        Err(error) => publish_result(manager, monitor_service, Err(error), emit),
    }
}

impl BackgroundWork {
    pub fn start(
        manager: &Arc<ConfigManager>,
        monitor_service: Arc<MonitorService>,
        app: AppHandle,
    ) -> Self {
        let config = Arc::downgrade(manager);
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let task = tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(15)).await;
            if stop.load(Ordering::Acquire) {
                return;
            }
            let Some(origin) = ready_snapshot(&config) else {
                return;
            };
            let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs(),
                Err(error) => {
                    eprintln!("Cannot schedule catalog synchronization: {error}");
                    return;
                }
            };
            let should_sync = origin.settings.auto_sync_database
                && origin
                    .settings
                    .last_sync_timestamp
                    .is_none_or(|last| now.saturating_sub(last) > 7 * 86400);
            if should_sync {
                match database::fetch_online_database().await {
                    Ok(_) if !stop.load(Ordering::Acquire) => {
                        let Some(manager) = config.upgrade() else {
                            return;
                        };
                        let result =
                            manager.mutate(&origin.context_token, None, false, |settings| {
                                if stop.load(Ordering::Acquire) || !settings.auto_sync_database {
                                    return Err("Catalog synchronization was cancelled.".into());
                                }
                                settings.last_sync_timestamp = Some(now);
                                Ok(())
                            });
                        publish_result(&manager, &monitor_service, result, |snapshot| {
                            emit_config(&app, snapshot)
                        });
                    }
                    Ok(_) => return,
                    Err(error) => eprintln!("Online catalog synchronization failed: {error}"),
                }
            }

            let Some(scan_origin) = ready_snapshot(&config) else {
                return;
            };
            if stop.load(Ordering::Acquire) || scan_origin.context_token != origin.context_token {
                return;
            }
            let detected =
                match tauri::async_runtime::spawn_blocking(scanner::scan_installed_games).await {
                    Ok(detected) => detected,
                    Err(error) => {
                        eprintln!("Background game scan failed: {error}");
                        return;
                    }
                };
            if detected.is_empty() || stop.load(Ordering::Acquire) {
                return;
            }
            let Some(manager) = config.upgrade() else {
                return;
            };
            let result = manager.mutate_if_changed(
                &scan_origin.context_token,
                Some(&scan_origin.library_generation),
                true,
                |settings| {
                    if stop.load(Ordering::Acquire) {
                        return Err("Background enrichment was cancelled.".into());
                    }
                    library::enrich_existing(settings, &detected);
                    Ok(())
                },
            );
            publish_enrichment_result(&manager, &monitor_service, result, |snapshot| {
                emit_config(&app, snapshot)
            });
        });
        Self {
            cancelled,
            task: Mutex::new(Some(task)),
        }
    }

    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::Release);
        match self.task.lock() {
            Ok(mut slot) => {
                if let Some(task) = slot.take() {
                    task.abort();
                    if let Err(error) = tauri::async_runtime::block_on(task) {
                        eprintln!("Background task stopped: {error}");
                    }
                }
            }
            Err(error) => eprintln!("Cannot join background task: {error}"),
        }
    }
}

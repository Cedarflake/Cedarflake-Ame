#[cfg(windows)]
use std::collections::BTreeMap;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc::{self, Receiver, TryRecvError};
#[cfg(windows)]
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use crate::adapters::{
    SqliteCatalog, inspect_root_availability, production_library_change_source_factory,
};
use crate::domain::{LibrarySynchronizationSnapshot, ScanError};
#[cfg(windows)]
use crate::domain::{RecoverableScan, ScanEvent, ScanRequest};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use crate::application::{cancel_scan, run_scan, storage_paths};
#[cfg(windows)]
use crate::ports::CatalogRepository;

#[cfg(windows)]
static SYNCHRONIZATION_RUNTIME: OnceLock<Mutex<Option<ProductionSynchronization>>> =
    OnceLock::new();
#[cfg(windows)]
const RECOVERY_RETRY_INITIAL_MILLIS: i64 = 1_000;
#[cfg(windows)]
const RECOVERY_RETRY_MAXIMUM_MILLIS: i64 = 5 * 60 * 1_000;

#[cfg(windows)]
struct ProductionSynchronization {
    runtime: LibrarySynchronizationRuntime,
    recovery: Option<RecoveryTask>,
    checked_recoverable_scan: bool,
    recovery_retries: BTreeMap<String, RecoveryRetryState>,
}

#[cfg(windows)]
struct RecoveryTask {
    root_id: String,
    scan_id: String,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<(), ScanError>>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecoveryRetryState {
    failure_count: u32,
    next_attempt_unix_ms: i64,
}

pub(crate) fn start_production_library_synchronization()
-> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        let mut runtime_state = lock_runtime()?;
        let runtime = runtime_state.get_or_insert_with(|| ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(
                production_library_change_source_factory(),
            ),
            recovery: None,
            checked_recoverable_scan: false,
            recovery_retries: BTreeMap::new(),
        });
        poll_runtime(runtime)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn poll_production_library_synchronization()
-> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        let mut runtime_state = lock_runtime()?;
        let runtime = runtime_state.as_mut().ok_or_else(|| {
            ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            )
        })?;
        poll_runtime(runtime)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn stop_production_library_synchronization() -> Result<(), ScanError> {
    #[cfg(windows)]
    {
        let runtime_state = {
            let mut guarded = lock_runtime()?;
            guarded.take()
        };
        if let Some(mut runtime) = runtime_state {
            runtime.stop()?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(windows)]
fn poll_runtime(
    runtime: &mut ProductionSynchronization,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let storage = storage_paths()?;
    let now_unix_ms = now_unix_ms()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    runtime.poll_recovery(now_unix_ms)?;
    if !runtime.checked_recoverable_scan && runtime.recovery.is_none() {
        if let Some(recoverable) = catalog.load_recoverable_scan()? {
            let root_id = recovery_root_id(&recoverable);
            if runtime.recovery_is_due(&root_id, now_unix_ms) {
                runtime.start_recoverable_scan(recoverable)?;
                runtime.checked_recoverable_scan = true;
            }
        } else {
            runtime.checked_recoverable_scan = true;
        }
    }
    let snapshot = runtime
        .runtime
        .poll(&mut catalog, now_unix_ms, |root_path| {
            inspect_root_availability(root_path).availability
        })?;
    if runtime.recovery.is_none()
        && let Some(request) = runtime
            .runtime
            .pending_full_scan_requests()
            .into_iter()
            .find(|request| runtime.recovery_is_due(&request.root_id, now_unix_ms))
    {
        let scan_id = format!(
            "sync-recovery-{}-{}-{}",
            request.root_generation.value(),
            request.queue_high_watermark.value(),
            now_unix_ms,
        );
        runtime.start_recovery_scan(
            request.root_id.clone(),
            ScanRequest {
                scan_id,
                root_path: request.root_path,
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
        )?;
        runtime
            .runtime
            .acknowledge_full_scan_started(&request.root_id, request.queue_high_watermark);
    }
    Ok(snapshot)
}

#[cfg(windows)]
fn lock_runtime() -> Result<MutexGuard<'static, Option<ProductionSynchronization>>, ScanError> {
    SYNCHRONIZATION_RUNTIME
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| {
            ScanError::new(
                "library_synchronization_state_unavailable",
                "The library synchronization runtime state is unavailable",
            )
        })
}

#[cfg(windows)]
impl ProductionSynchronization {
    fn poll_recovery(&mut self, now_unix_ms: i64) -> Result<(), ScanError> {
        let Some(task) = self.recovery.as_mut() else {
            return Ok(());
        };
        let result = match task.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ScanError::new(
                "authoritative_recovery_worker_disconnected",
                "The authoritative recovery worker stopped without a result",
            ))),
        };
        let Some(result) = result else {
            return Ok(());
        };
        let root_id = task.root_id.clone();
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        self.recovery = None;
        match result {
            Ok(()) => {
                self.recovery_retries.remove(&root_id);
            }
            Err(error) => {
                self.record_recovery_failure(&root_id, now_unix_ms);
                self.checked_recoverable_scan = false;
                self.runtime.record_full_scan_failure(&root_id, error.code);
            }
        }
        Ok(())
    }

    fn start_recoverable_scan(&mut self, recoverable: RecoverableScan) -> Result<(), ScanError> {
        let root_id = recovery_root_id(&recoverable);
        self.start_recovery_scan(
            root_id,
            ScanRequest {
                scan_id: recoverable.scan_id,
                root_path: recoverable.root_path,
                max_items: recoverable.max_items,
                max_entries: recoverable.max_entries,
                preview_edge: recoverable.preview_edge,
            },
        )
    }

    fn start_recovery_scan(
        &mut self,
        root_id: String,
        request: ScanRequest,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Ok(());
        }
        let scan_id = request.scan_id.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-authoritative-recovery".to_owned())
            .spawn(move || {
                let result = run_recovery_scan(request, &worker_cancelled);
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start authoritative recovery: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id,
            scan_id,
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ScanError> {
        let runtime_result = self.runtime.stop();
        let recovery_result = self.stop_recovery();
        runtime_result.and(recovery_result)
    }

    fn stop_recovery(&mut self) -> Result<(), ScanError> {
        let Some(mut task) = self.recovery.take() else {
            return Ok(());
        };
        task.cancelled.store(true, Ordering::Release);
        let _ = cancel_scan(&task.scan_id);
        match task.receiver.recv_timeout(Duration::from_secs(2)) {
            Ok(_) => {
                if let Some(worker) = task.worker.take() {
                    let _ = worker.join();
                }
                Ok(())
            }
            Err(_) => Err(ScanError::new(
                "authoritative_recovery_stop_timeout",
                "Authoritative recovery did not stop within two seconds",
            )),
        }
    }

    fn recovery_is_due(&self, root_id: &str, now_unix_ms: i64) -> bool {
        self.recovery_retries
            .get(root_id)
            .is_none_or(|retry| now_unix_ms >= retry.next_attempt_unix_ms)
    }

    fn record_recovery_failure(&mut self, root_id: &str, now_unix_ms: i64) {
        let failure_count = self
            .recovery_retries
            .get(root_id)
            .map_or(1, |retry| retry.failure_count.saturating_add(1));
        let exponent = failure_count.saturating_sub(1).min(31);
        let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
        let delay = RECOVERY_RETRY_INITIAL_MILLIS
            .saturating_mul(multiplier)
            .min(RECOVERY_RETRY_MAXIMUM_MILLIS);
        self.recovery_retries.insert(
            root_id.to_owned(),
            RecoveryRetryState {
                failure_count,
                next_attempt_unix_ms: now_unix_ms.saturating_add(delay),
            },
        );
    }
}

#[cfg(windows)]
fn recovery_root_id(recoverable: &RecoverableScan) -> String {
    super::super::scan_library::stable_id("library-root-v1", &recoverable.root_path)
}

#[cfg(windows)]
fn run_recovery_scan(request: ScanRequest, cancelled: &AtomicBool) -> Result<(), ScanError> {
    let scan_id = request.scan_id.clone();
    let mut completed = false;
    let mut terminal_error = None;
    run_scan(request, |event| {
        match event {
            ScanEvent::Completed { was_limited, .. } if !was_limited => completed = true,
            ScanEvent::Completed { .. } => {
                terminal_error = Some(ScanError::new(
                    "authoritative_recovery_scan_limited",
                    "An authoritative recovery scan cannot publish a limited result",
                ));
            }
            ScanEvent::Cancelled { .. } => {
                terminal_error = Some(ScanError::new(
                    "authoritative_recovery_cancelled",
                    "Authoritative recovery was cancelled",
                ));
            }
            ScanEvent::Stale { .. } => {
                terminal_error = Some(ScanError::new(
                    "authoritative_recovery_stale",
                    "Source state changed before authoritative recovery could publish",
                ));
            }
            ScanEvent::Failed { code, message, .. } => {
                terminal_error = Some(ScanError::new(code, message));
            }
            _ => {}
        }
        !cancelled.load(Ordering::Acquire)
    })?;
    if completed {
        return Ok(());
    }
    Err(terminal_error.unwrap_or_else(|| {
        ScanError::new(
            "authoritative_recovery_incomplete",
            format!("Authoritative recovery scan {scan_id} ended without publication"),
        )
    }))
}

fn now_unix_ms() -> Result<i64, ScanError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is earlier than the Unix epoch",
        )
    })?;
    i64::try_from(elapsed.as_millis()).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is outside the supported range",
        )
    })
}

#[cfg(not(windows))]
fn unsupported_platform() -> ScanError {
    ScanError::new(
        "library_synchronization_unsupported",
        "Continuous library synchronization is currently supported only on Windows",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_is_representable() {
        assert!(now_unix_ms().expect("current time") > 0);
    }

    #[cfg(windows)]
    #[test]
    fn recovery_retry_is_bounded_exponential_and_isolated_by_root() {
        let factory = crate::adapters::production_library_change_source_factory();
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: None,
            checked_recoverable_scan: true,
            recovery_retries: BTreeMap::new(),
        };

        production.record_recovery_failure("root-a", 1_000);
        assert!(!production.recovery_is_due("root-a", 1_999));
        assert!(production.recovery_is_due("root-a", 2_000));
        assert!(production.recovery_is_due("root-b", 1_000));
        for _ in 0..40 {
            production.record_recovery_failure("root-a", 2_000);
        }
        let retry = production
            .recovery_retries
            .get("root-a")
            .expect("root retry state");
        assert_eq!(
            retry.next_attempt_unix_ms,
            2_000 + RECOVERY_RETRY_MAXIMUM_MILLIS
        );
    }
}

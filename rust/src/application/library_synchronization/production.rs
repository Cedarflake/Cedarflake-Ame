#[cfg(windows)]
use std::collections::BTreeMap;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
#[cfg(windows)]
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use crate::adapters::{
    SqliteCatalog, inspect_root_availability, production_library_change_catch_up_source,
    production_library_change_source_factory,
};
use crate::domain::{LibraryChangeCatchUpReport, LibrarySynchronizationSnapshot, ScanError};
#[cfg(windows)]
use crate::domain::{RecoverableScan, ScanEvent, ScanRequest};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use crate::application::library_change_catch_up::{
    LibraryChangeCatchUpExecution, process_library_change_catch_up,
};
#[cfg(windows)]
use crate::application::{
    AuthoritativeLibraryChangeReport, cancel_scan,
    process_ready_authoritative_library_change_cancellable, run_authoritative_scan, storage_paths,
};
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
    recovery_retries: BTreeMap<String, RecoveryRetryState>,
    recoverable_scan_cursor: Option<String>,
    is_stopping: bool,
}

#[cfg(windows)]
struct RecoveryTask {
    root_id: String,
    kind: RecoveryTaskKind,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<RecoveryTaskOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
enum RecoveryTaskKind {
    Authoritative,
    FullScan { scan_id: String },
    CatchUp { root_ids: Vec<String> },
}

#[cfg(windows)]
enum RecoveryTaskOutcome {
    Authoritative(AuthoritativeLibraryChangeReport),
    FullScan,
    CatchUp(LibraryChangeCatchUpReport),
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
        if let Some(runtime) = runtime_state.as_mut()
            && runtime.is_stopping
        {
            if runtime.finish_stopping()? {
                *runtime_state = None;
            } else {
                return Err(ScanError::new(
                    "library_synchronization_stop_in_progress",
                    "The prior synchronization runtime is still stopping",
                ));
            }
        }
        let runtime = runtime_state.get_or_insert_with(|| ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_production(
                production_library_change_source_factory(),
            ),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: false,
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
        if runtime.is_stopping {
            return Err(ScanError::new(
                "library_synchronization_stop_in_progress",
                "Library synchronization is still stopping",
            ));
        }
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
        let mut guarded = lock_runtime()?;
        if let Some(runtime) = guarded.as_mut() {
            runtime.is_stopping = true;
            runtime.stop()?;
        }
        guarded.take();
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
    let recovered_mutation_count = runtime.poll_recovery(now_unix_ms)?;
    let mut snapshot = runtime.runtime.poll_without_authoritative_recovery(
        &mut catalog,
        now_unix_ms,
        |root_path| inspect_root_availability(root_path).availability,
    )?;
    snapshot.applied_mutation_count = snapshot
        .applied_mutation_count
        .checked_add(recovered_mutation_count)
        .ok_or_else(|| {
            ScanError::new(
                "library_synchronization_count_overflow",
                "The synchronization mutation count exceeded the supported range",
            )
        })?;
    if runtime.recovery.is_none() {
        let pending_catch_up = runtime.runtime.pending_catch_up_roots();
        if !pending_catch_up.is_empty() {
            if pending_catch_up
                .iter()
                .all(|root| runtime.recovery_is_due(&root.root_id, now_unix_ms))
            {
                runtime.start_catch_up(pending_catch_up, now_unix_ms)?;
            }
            return Ok(snapshot);
        }
        if runtime.runtime.has_unready_catch_up_roots() {
            return Ok(snapshot);
        }
    }
    if runtime.recovery.is_none() {
        let recoverable_scan = runtime.next_authoritative_recoverable_scan(&catalog)?;
        if let Some(recoverable) = recoverable_scan
            && runtime.recovery_is_due(&recovery_root_id(&recoverable), now_unix_ms)
        {
            runtime.start_recoverable_scan(recoverable)?;
            return Ok(snapshot);
        }
        let mut pending_full_scan = None;
        for request in runtime.runtime.pending_full_scan_requests() {
            if runtime.recovery_is_due(&request.root_id, now_unix_ms)
                && !catalog.has_active_scan_for_root(&request.root_id)?
            {
                pending_full_scan = Some(request);
                break;
            }
        }
        if let Some(request) = pending_full_scan {
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
        } else if let Some((root_id, root_generation)) =
            ready_authoritative_root(runtime, &catalog, &snapshot, now_unix_ms)?
        {
            runtime.start_authoritative_recovery(root_id, root_generation, now_unix_ms)?;
        }
    }
    Ok(snapshot)
}

#[cfg(windows)]
fn ready_authoritative_root(
    runtime: &ProductionSynchronization,
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
) -> Result<Option<(String, crate::domain::LibraryRootGeneration)>, ScanError> {
    for root in &snapshot.roots {
        if root.availability != crate::domain::LibraryRootAvailability::Available
            || root.source_health != crate::domain::LibraryChangeSourceHealth::Healthy
            || !runtime.recovery_is_due(&root.root_id, now_unix_ms)
        {
            continue;
        }
        let root_generation = crate::domain::LibraryRootGeneration::new(root.root_generation)
            .ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
        if catalog.has_ready_authoritative_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? {
            return Ok(Some((root.root_id.clone(), root_generation)));
        }
    }
    Ok(None)
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
    fn next_authoritative_recoverable_scan(
        &mut self,
        catalog: &SqliteCatalog,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        let mut recoverable = catalog
            .load_authoritative_recoverable_scan_after(self.recoverable_scan_cursor.as_deref())?;
        if recoverable.is_none() && self.recoverable_scan_cursor.is_some() {
            self.recoverable_scan_cursor = None;
            recoverable = catalog.load_authoritative_recoverable_scan_after(None)?;
        }
        if let Some(scan) = recoverable.as_ref() {
            self.recoverable_scan_cursor = Some(scan.scan_id.clone());
        }
        Ok(recoverable)
    }

    fn poll_recovery(&mut self, now_unix_ms: i64) -> Result<u32, ScanError> {
        let Some(task) = self.recovery.as_mut() else {
            return Ok(0);
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
            return Ok(0);
        };
        let root_id = task.root_id.clone();
        let catch_up_root_ids = match &task.kind {
            RecoveryTaskKind::CatchUp { root_ids } => Some(root_ids.clone()),
            RecoveryTaskKind::Authoritative | RecoveryTaskKind::FullScan { .. } => None,
        };
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        self.recovery = None;
        match result {
            Ok(RecoveryTaskOutcome::Authoritative(report)) => {
                if let Some(request) = report.full_scan {
                    self.runtime.record_full_scan_request(request);
                } else {
                    self.recovery_retries.remove(&root_id);
                }
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::FullScan) => {
                self.recovery_retries.remove(&root_id);
                Ok(0)
            }
            Ok(RecoveryTaskOutcome::CatchUp(report)) => {
                for completed in &report.completed_roots {
                    self.recovery_retries.remove(&completed.root_id);
                }
                self.runtime.acknowledge_catch_up(&report);
                Ok(0)
            }
            Err(error) => {
                if let Some(root_ids) = catch_up_root_ids {
                    for affected_root_id in &root_ids {
                        self.record_recovery_failure(affected_root_id, now_unix_ms);
                    }
                    self.runtime.record_catch_up_failure(&root_ids, &error.code);
                } else {
                    self.record_recovery_failure(&root_id, now_unix_ms);
                    self.runtime.record_full_scan_failure(&root_id, error.code);
                }
                Ok(0)
            }
        }
    }

    fn start_catch_up(
        &mut self,
        roots: Vec<crate::domain::IncrementalCatalogRoot>,
        now_unix_ms: i64,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() || roots.is_empty() {
            return Ok(());
        }
        let root_ids = roots
            .iter()
            .map(|root| root.root_id.clone())
            .collect::<Vec<_>>();
        let catalog_path = storage_paths()?.catalog_path;
        let queue_policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-usn-downtime-catch-up".to_owned())
            .spawn(move || {
                let source = production_library_change_catch_up_source();
                let result = SqliteCatalog::open(catalog_path).and_then(|mut catalog| {
                    process_library_change_catch_up(
                        &source,
                        &mut catalog,
                        &roots,
                        LibraryChangeCatchUpExecution::at(now_unix_ms, queue_policy),
                        &worker_cancelled,
                    )
                    .map(RecoveryTaskOutcome::CatchUp)
                });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "library_change_catch_up_worker_start_failed",
                    format!("Could not start downtime catch-up: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id: root_ids[0].clone(),
            kind: RecoveryTaskKind::CatchUp { root_ids },
            cancelled,
            receiver,
            worker: Some(worker),
        });
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

    fn start_authoritative_recovery(
        &mut self,
        root_id: String,
        root_generation: crate::domain::LibraryRootGeneration,
        now_unix_ms: i64,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Ok(());
        }
        let catalog_path = storage_paths()?.catalog_path;
        let queue_policy = self.runtime.queue_policy();
        let recovery_policy = self.runtime.recovery_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-bounded-authoritative-recovery".to_owned())
            .spawn(move || {
                let result = SqliteCatalog::open(catalog_path).and_then(|mut catalog| {
                    process_ready_authoritative_library_change_cancellable(
                        &mut catalog,
                        &worker_root_id,
                        root_generation,
                        now_unix_ms,
                        queue_policy,
                        recovery_policy,
                        &worker_cancelled,
                    )
                    .map(RecoveryTaskOutcome::Authoritative)
                });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start bounded authoritative recovery: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id,
            kind: RecoveryTaskKind::Authoritative,
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
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
                let result = run_recovery_scan(request, &worker_cancelled)
                    .map(|()| RecoveryTaskOutcome::FullScan);
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
            kind: RecoveryTaskKind::FullScan { scan_id },
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ScanError> {
        self.stop_recovery()?;
        self.runtime.stop()
    }

    fn stop_recovery(&mut self) -> Result<(), ScanError> {
        self.stop_recovery_with_timeout(Duration::from_secs(2))
    }

    fn stop_recovery_with_timeout(&mut self, timeout: Duration) -> Result<(), ScanError> {
        let Some(mut task) = self.recovery.take() else {
            return Ok(());
        };
        task.cancelled.store(true, Ordering::Release);
        if let RecoveryTaskKind::FullScan { scan_id } = &task.kind {
            let _ = cancel_scan(scan_id);
        }
        match task.receiver.recv_timeout(timeout) {
            Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(worker) = task.worker.take() {
                    let _ = worker.join();
                }
                Ok(())
            }
            Err(RecvTimeoutError::Timeout) => {
                self.recovery = Some(task);
                Err(ScanError::new(
                    "authoritative_recovery_stop_timeout",
                    "Authoritative recovery did not stop within the bounded shutdown window",
                ))
            }
        }
    }

    fn finish_stopping(&mut self) -> Result<bool, ScanError> {
        if let Some(task) = self.recovery.as_mut() {
            match task.receiver.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    if let Some(worker) = task.worker.take() {
                        let _ = worker.join();
                    }
                    self.recovery = None;
                }
                Err(TryRecvError::Empty) => return Ok(false),
            }
        }
        self.runtime.stop()?;
        Ok(true)
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
    run_authoritative_scan(request, |event| {
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
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: false,
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

    #[cfg(windows)]
    #[test]
    fn full_scan_reescalation_preserves_retry_history_by_root() {
        let factory = crate::adapters::production_library_change_source_factory();
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: false,
        };
        production.record_recovery_failure("root-b", 5_000);
        let root_b_retry = production.recovery_retries["root-b"];

        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(Err(ScanError::new(
                "authoritative_recovery_stale",
                "The first full scan could not publish",
            )))
            .expect("first full scan failure result");
        production.recovery = Some(RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::FullScan {
                scan_id: "sync-recovery-a-1".to_owned(),
            },
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        });
        production
            .poll_recovery(1_000)
            .expect("first failed full scan is recorded");
        assert_eq!(
            production.recovery_retries["root-a"],
            RecoveryRetryState {
                failure_count: 1,
                next_attempt_unix_ms: 2_000,
            }
        );

        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(Ok(RecoveryTaskOutcome::Authoritative(
                AuthoritativeLibraryChangeReport {
                    full_scan: Some(crate::application::FullScanRecoveryRequest {
                        root_id: "root-a".to_owned(),
                        root_path: "C:\\RecoveryA".to_owned(),
                        root_generation: crate::domain::LibraryRootGeneration::initial(),
                        queue_high_watermark: crate::domain::LibraryChangeId::new(1)
                            .expect("queue high watermark"),
                    }),
                    ..AuthoritativeLibraryChangeReport::default()
                },
            )))
            .expect("bounded escalation result");
        production.recovery = Some(RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::Authoritative,
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        });

        production
            .poll_recovery(2_000)
            .expect("bounded recovery escalation");
        assert_eq!(production.recovery_retries["root-a"].failure_count, 1);

        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(Err(ScanError::new(
                "authoritative_recovery_stale",
                "The full scan could not publish",
            )))
            .expect("full scan failure result");
        production.recovery = Some(RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::FullScan {
                scan_id: "sync-recovery-a-2".to_owned(),
            },
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        });

        production
            .poll_recovery(2_000)
            .expect("failed full scan is recorded");
        assert_eq!(
            production.recovery_retries["root-a"],
            RecoveryRetryState {
                failure_count: 2,
                next_attempt_unix_ms: 4_000,
            }
        );
        assert_eq!(production.recovery_retries["root-b"], root_b_retry);
    }

    #[cfg(windows)]
    #[test]
    fn recoverable_scan_cursor_rotates_across_multiple_roots() {
        let directory = tempfile::tempdir().expect("catalog directory");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        for (scan_id, root_id, root_path) in [
            ("sync-recovery-a", "root-a", "C:\\RecoveryA"),
            ("sync-recovery-b", "root-b", "C:\\RecoveryB"),
        ] {
            let request = ScanRequest {
                scan_id: scan_id.to_owned(),
                root_path: root_path.to_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            };
            catalog
                .begin_authoritative_scan(&request, root_id, root_path)
                .expect("authoritative scan");
        }
        let factory = crate::adapters::production_library_change_source_factory();
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: false,
        };

        let first = production
            .next_authoritative_recoverable_scan(&catalog)
            .expect("first recovery")
            .expect("first scan");
        let second = production
            .next_authoritative_recoverable_scan(&catalog)
            .expect("second recovery")
            .expect("second scan");
        let wrapped = production
            .next_authoritative_recoverable_scan(&catalog)
            .expect("wrapped recovery")
            .expect("wrapped scan");

        assert_eq!(first.scan_id, "sync-recovery-a");
        assert_eq!(second.scan_id, "sync-recovery-b");
        assert_eq!(wrapped.scan_id, "sync-recovery-a");
    }

    #[cfg(windows)]
    #[test]
    fn stopping_bounded_recovery_signals_and_joins_the_background_worker() {
        let factory = crate::adapters::production_library_change_source_factory();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            while !worker_cancelled.load(Ordering::Acquire) {
                thread::yield_now();
            }
            let _ = sender.send(Ok(RecoveryTaskOutcome::Authoritative(
                AuthoritativeLibraryChangeReport::default(),
            )));
        });
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: Some(RecoveryTask {
                root_id: "root-a".to_owned(),
                kind: RecoveryTaskKind::Authoritative,
                cancelled: Arc::clone(&cancelled),
                receiver,
                worker: Some(worker),
            }),
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: false,
        };

        production.stop_recovery().expect("stop bounded recovery");

        assert!(cancelled.load(Ordering::Acquire));
        assert!(production.recovery.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn recovery_stop_timeout_retains_worker_ownership_until_a_later_join() {
        let factory = crate::adapters::production_library_change_source_factory();
        let cancelled = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let worker_release = Arc::clone(&release);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            while !worker_release.load(Ordering::Acquire) {
                thread::yield_now();
            }
            let _ = sender.send(Ok(RecoveryTaskOutcome::Authoritative(
                AuthoritativeLibraryChangeReport::default(),
            )));
        });
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: Some(RecoveryTask {
                root_id: "root-a".to_owned(),
                kind: RecoveryTaskKind::Authoritative,
                cancelled: Arc::clone(&cancelled),
                receiver,
                worker: Some(worker),
            }),
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            is_stopping: true,
        };

        let error = production
            .stop_recovery_with_timeout(Duration::from_millis(10))
            .expect_err("uncooperative worker must exceed the short stop window");

        assert_eq!(error.code, "authoritative_recovery_stop_timeout");
        assert!(cancelled.load(Ordering::Acquire));
        assert!(production.recovery.is_some());
        release.store(true, Ordering::Release);
        production
            .stop_recovery_with_timeout(Duration::from_secs(1))
            .expect("later stop joins retained worker");
        assert!(production.recovery.is_none());
    }
}

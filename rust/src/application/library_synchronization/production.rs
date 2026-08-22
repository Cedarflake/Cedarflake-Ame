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
    SqliteCatalog, inspect_root_availability, production_library_change_source_factory,
};
use crate::domain::{
    LeasedLibraryChange, LibrarySynchronizationPhase, LibrarySynchronizationSnapshot, ScanError,
};
#[cfg(windows)]
use crate::domain::{RecoverableScan, ScanEvent, ScanRequest};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use crate::application::scan_library::resume_authoritative_scan_with_storage;
#[cfg(windows)]
use crate::application::{
    AuthoritativeLibraryChangeReport, MetadataInventoryProgressPhase,
    MetadataInventoryRecoveryReport, MetadataInventoryWorkerControl, defer_authoritative_change,
    leased_change_requires_metadata_inventory,
    process_leased_authoritative_library_change_cancellable,
    process_leased_metadata_inventory_change_with_progress, storage_paths, suspend_scan,
};
#[cfg(windows)]
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

#[cfg(windows)]
static SYNCHRONIZATION_RUNTIME: OnceLock<Mutex<Option<ProductionSynchronization>>> =
    OnceLock::new();
#[cfg(windows)]
const RECOVERY_RETRY_INITIAL_MILLIS: i64 = 1_000;
#[cfg(windows)]
const RECOVERY_RETRY_MAXIMUM_MILLIS: i64 = 5 * 60 * 1_000;
#[cfg(windows)]
// The absolute 4096th queue slot remains the inventory authority while one candidate page drains.
const METADATA_INVENTORY_WORK_PAGE_ENTRIES: u32 = 4_095;
#[cfg(windows)]
struct ProductionSynchronization {
    runtime: LibrarySynchronizationRuntime,
    recovery: Option<RecoveryTask>,
    recovery_retries: BTreeMap<String, RecoveryRetryState>,
    recoverable_scan_cursor: Option<String>,
    authoritative_root_cursor: Option<String>,
    legacy_audits_retired: bool,
    is_stopping: bool,
}

#[cfg(windows)]
struct RecoveryTask {
    root_id: String,
    kind: RecoveryTaskKind,
    phase: Arc<Mutex<LibrarySynchronizationPhase>>,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<RecoveryTaskOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
enum RecoveryTaskKind {
    BoundedAuthoritative { continuity_revision: u64 },
    MetadataInventory { continuity_revision: u64 },
    FullScan { scan_id: String },
}

#[cfg(windows)]
enum RecoveryTaskOutcome {
    Authoritative(AuthoritativeLibraryChangeReport),
    MetadataInventory(MetadataInventoryRecoveryReport),
    FullScan,
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
        let runtime = runtime_state.get_or_insert_with(new_production_synchronization);
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
    poll_runtime_with_storage(runtime, &storage)
}

#[cfg(windows)]
fn poll_runtime_with_storage(
    runtime: &mut ProductionSynchronization,
    storage: &crate::application::storage::StoragePaths,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let now_unix_ms = now_unix_ms()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())?;
    if !runtime.legacy_audits_retired {
        catalog.retire_legacy_consistency_audits(now_unix_ms)?;
        runtime.legacy_audits_retired = true;
    }
    let recovered_mutation_count = runtime.poll_recovery(now_unix_ms)?;
    let mut snapshot = runtime.runtime.poll_without_authoritative_recovery(
        &mut catalog,
        now_unix_ms,
        |root_path| inspect_root_availability(root_path).availability,
    )?;
    runtime.cancel_stale_automatic_recovery();
    snapshot.applied_mutation_count = snapshot
        .applied_mutation_count
        .checked_add(recovered_mutation_count)
        .ok_or_else(|| {
            ScanError::new(
                "library_synchronization_count_overflow",
                "The synchronization mutation count exceeded the supported range",
            )
        })?;
    project_active_recovery_as_updating(runtime.recovery.as_ref(), &mut snapshot);
    if runtime.recovery.is_none() {
        let recoverable_scan = runtime.next_authoritative_recoverable_scan(&catalog)?;
        if let Some(recoverable) = recoverable_scan
            && runtime
                .runtime
                .root_is_ready_for_authoritative_recovery(&recovery_root_id(&recoverable))
            && runtime.recovery_is_due(&recovery_root_id(&recoverable), now_unix_ms)
        {
            runtime.start_recoverable_scan(recoverable, storage.clone())?;
            project_active_recovery_as_updating(runtime.recovery.as_ref(), &mut snapshot);
            return Ok(snapshot);
        }
        if let Some((root_id, root_generation)) =
            ready_authoritative_root(runtime, &catalog, &snapshot, now_unix_ms)?
        {
            let continuity_revision = runtime
                .runtime
                .root_continuity_revision(&root_id)
                .ok_or_else(|| {
                    ScanError::new(
                        "library_continuity_root_missing",
                        "The selected continuity root is no longer active",
                    )
                })?;
            if let Some(leased) = catalog.lease_authoritative_library_change(
                &root_id,
                root_generation,
                now_unix_ms,
                runtime.runtime.queue_policy(),
            )? {
                let leased_for_defer = leased.clone();
                if let Err(start_error) = runtime.start_automatic_recovery(
                    root_id,
                    root_generation,
                    continuity_revision,
                    leased,
                    now_unix_ms,
                    storage.catalog_path.clone(),
                ) {
                    if let Err(defer_error) = catalog.defer_library_change(
                        leased_for_defer.change.id,
                        leased_for_defer.lease_generation,
                        now_unix_ms,
                    ) {
                        return Err(ScanError::new(
                            "authoritative_recovery_worker_start_cleanup_failed",
                            format!(
                                "{}; leased work could not be deferred: {}",
                                start_error.message, defer_error.message
                            ),
                        ));
                    }
                    return Err(start_error);
                }
            }
        }
    }
    project_active_recovery_as_updating(runtime.recovery.as_ref(), &mut snapshot);
    Ok(snapshot)
}

#[cfg(windows)]
fn new_production_synchronization() -> ProductionSynchronization {
    ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_production(
            production_library_change_source_factory(),
        ),
        recovery: None,
        recovery_retries: BTreeMap::new(),
        recoverable_scan_cursor: None,
        authoritative_root_cursor: None,
        legacy_audits_retired: false,
        is_stopping: false,
    }
}

#[cfg(all(test, windows))]
pub(crate) struct ProductionSynchronizationTestHarness {
    runtime: ProductionSynchronization,
    storage: crate::application::storage::StoragePaths,
}

#[cfg(all(test, windows))]
impl ProductionSynchronizationTestHarness {
    pub(crate) fn new(storage: crate::application::storage::StoragePaths) -> Self {
        Self {
            runtime: new_production_synchronization(),
            storage,
        }
    }

    pub(crate) fn poll(&mut self) -> Result<LibrarySynchronizationSnapshot, ScanError> {
        poll_runtime_with_storage(&mut self.runtime, &self.storage)
    }

    pub(crate) fn stop(&mut self) -> Result<(), ScanError> {
        self.runtime.stop()
    }
}

#[cfg(windows)]
fn project_active_recovery_as_updating(
    recovery: Option<&RecoveryTask>,
    snapshot: &mut LibrarySynchronizationSnapshot,
) {
    let Some(recovery) = recovery else {
        return;
    };
    let root_ids = std::slice::from_ref(&recovery.root_id);
    let recovery_phase = recovery
        .phase
        .lock()
        .map_or_else(|_| recovery.default_phase(), |phase| *phase);
    for status in &mut snapshot.roots {
        if root_ids.contains(&status.root_id)
            && status.availability == crate::domain::LibraryRootAvailability::Available
            && status.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
            && !status.recovery_blocked
        {
            status.freshness = crate::domain::CatalogFreshnessState::Updating;
            status.freshness_cause = crate::domain::CatalogFreshnessCause::PendingChanges;
            status.phase = recovery_phase;
        }
    }
}

#[cfg(windows)]
impl RecoveryTask {
    const fn default_phase(&self) -> LibrarySynchronizationPhase {
        match &self.kind {
            RecoveryTaskKind::BoundedAuthoritative { .. } => {
                LibrarySynchronizationPhase::Reconciliation
            }
            RecoveryTaskKind::MetadataInventory { .. } => {
                LibrarySynchronizationPhase::InventoryEnumeration
            }
            RecoveryTaskKind::FullScan { .. } => LibrarySynchronizationPhase::FullScan,
        }
    }
}

#[cfg(windows)]
const fn synchronization_phase(
    phase: MetadataInventoryProgressPhase,
) -> LibrarySynchronizationPhase {
    match phase {
        MetadataInventoryProgressPhase::Enumeration => {
            LibrarySynchronizationPhase::InventoryEnumeration
        }
        MetadataInventoryProgressPhase::Comparison => {
            LibrarySynchronizationPhase::InventoryComparison
        }
        MetadataInventoryProgressPhase::QueuePublication => {
            LibrarySynchronizationPhase::QueuePublication
        }
    }
}

#[cfg(windows)]
fn ready_authoritative_root(
    runtime: &mut ProductionSynchronization,
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
) -> Result<Option<(String, crate::domain::LibraryRootGeneration)>, ScanError> {
    if snapshot.roots.is_empty() {
        return Ok(None);
    }
    let start_index = runtime
        .authoritative_root_cursor
        .as_ref()
        .and_then(|root_id| {
            snapshot
                .roots
                .iter()
                .position(|root| &root.root_id == root_id)
        })
        .map_or(0, |index| (index + 1) % snapshot.roots.len());
    for offset in 0..snapshot.roots.len() {
        let root = &snapshot.roots[(start_index + offset) % snapshot.roots.len()];
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
            runtime.authoritative_root_cursor = Some(root.root_id.clone());
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
        #[cfg(debug_assertions)]
        let recovery_kind = match &task.kind {
            RecoveryTaskKind::BoundedAuthoritative { .. } => "bounded-authoritative",
            RecoveryTaskKind::MetadataInventory { .. } => "metadata-inventory",
            RecoveryTaskKind::FullScan { .. } => "full-scan",
        };
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        self.recovery = None;
        match result {
            Ok(RecoveryTaskOutcome::Authoritative(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.incremental.applied_mutation_count
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_retries.remove(&root_id);
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::MetadataInventory(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok staged={} candidates={} mutations={}",
                    report.inventory.staged_entry_count,
                    report.inventory.candidate_count,
                    report.incremental.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_retries.remove(&root_id);
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::FullScan) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok"
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_retries.remove(&root_id);
                Ok(0)
            }
            Err(error) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=failed code={} message={}",
                    error.code,
                    one_line_message(&error.message)
                );
                self.record_recovery_failure(&root_id, now_unix_ms);
                self.runtime
                    .record_recovery_failure(&root_id, &error.code, now_unix_ms);
                Ok(0)
            }
        }
    }

    fn start_recoverable_scan(
        &mut self,
        recoverable: RecoverableScan,
        storage: crate::application::storage::StoragePaths,
    ) -> Result<(), ScanError> {
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
            storage,
        )
    }

    fn start_automatic_recovery(
        &mut self,
        root_id: String,
        root_generation: crate::domain::LibraryRootGeneration,
        continuity_revision: u64,
        leased: LeasedLibraryChange,
        now_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Err(ScanError::new(
                "authoritative_recovery_already_running",
                "Another authoritative recovery already owns the worker slot",
            ));
        }
        let is_inventory = leased_change_requires_metadata_inventory(&leased);
        let task_kind = if is_inventory {
            RecoveryTaskKind::MetadataInventory {
                continuity_revision,
            }
        } else {
            RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision,
            }
        };
        let recovery_kind = if is_inventory {
            "metadata-inventory"
        } else {
            "bounded-authoritative"
        };
        #[cfg(debug_assertions)]
        eprintln!("[Ame sync] recovery started kind={recovery_kind} root={root_id}");
        let queue_policy = self.runtime.queue_policy();
        let recovery_policy = self.runtime.recovery_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let initial_phase = if is_inventory {
            LibrarySynchronizationPhase::InventoryEnumeration
        } else {
            LibrarySynchronizationPhase::Reconciliation
        };
        let phase = Arc::new(Mutex::new(initial_phase));
        let worker_phase = Arc::clone(&phase);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name(format!("ame-{recovery_kind}-recovery"))
            .spawn(move || {
                let result = SqliteCatalog::open(catalog_path).and_then(|mut catalog| {
                    let Some(root) = catalog.load_incremental_catalog_root(&worker_root_id)? else {
                        return Ok(if is_inventory {
                            RecoveryTaskOutcome::MetadataInventory(
                                MetadataInventoryRecoveryReport::default(),
                            )
                        } else {
                            RecoveryTaskOutcome::Authoritative(
                                AuthoritativeLibraryChangeReport::default(),
                            )
                        });
                    };
                    if root.root_generation != root_generation
                        || root.active_scan_id.is_none()
                        || root.has_running_scan
                    {
                        let deferred = defer_authoritative_change(
                            &mut catalog,
                            &leased,
                            root.catalog_revision,
                            now_unix_ms,
                        )?;
                        return Ok(if is_inventory {
                            RecoveryTaskOutcome::MetadataInventory(
                                MetadataInventoryRecoveryReport {
                                    incremental: deferred.incremental,
                                    ..MetadataInventoryRecoveryReport::default()
                                },
                            )
                        } else {
                            RecoveryTaskOutcome::Authoritative(deferred)
                        });
                    }
                    if is_inventory {
                        let report_progress = |progress| {
                            if let Ok(mut phase) = worker_phase.lock() {
                                *phase = synchronization_phase(progress);
                            }
                        };
                        process_leased_metadata_inventory_change_with_progress(
                            &mut catalog,
                            &root,
                            &leased,
                            now_unix_ms,
                            METADATA_INVENTORY_WORK_PAGE_ENTRIES,
                            queue_policy,
                            MetadataInventoryWorkerControl::with_progress(
                                &worker_cancelled,
                                &report_progress,
                            ),
                        )
                        .map(RecoveryTaskOutcome::MetadataInventory)
                    } else {
                        process_leased_authoritative_library_change_cancellable(
                            &mut catalog,
                            &root,
                            &leased,
                            now_unix_ms,
                            queue_policy,
                            recovery_policy,
                            &worker_cancelled,
                        )
                        .map(RecoveryTaskOutcome::Authoritative)
                    }
                });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start {recovery_kind} recovery: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id,
            kind: task_kind,
            phase,
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
        storage: crate::application::storage::StoragePaths,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Ok(());
        }
        let scan_id = request.scan_id.clone();
        #[cfg(debug_assertions)]
        eprintln!("[Ame sync] recovery started kind=full-scan root={root_id} scan={scan_id}");
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let phase = Arc::new(Mutex::new(LibrarySynchronizationPhase::FullScan));
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-authoritative-recovery".to_owned())
            .spawn(move || {
                let result = run_recovery_scan(request, &worker_cancelled, storage)
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
            phase,
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn cancel_stale_automatic_recovery(&mut self) {
        let Some(task) = self.recovery.as_ref() else {
            return;
        };
        let expected_revision = match task.kind {
            RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision,
            }
            | RecoveryTaskKind::MetadataInventory {
                continuity_revision,
            } => continuity_revision,
            RecoveryTaskKind::FullScan { .. } => return,
        };
        let is_current = self
            .runtime
            .root_continuity_revision(&task.root_id)
            .is_some_and(|revision| revision == expected_revision)
            && self
                .runtime
                .root_is_ready_for_authoritative_recovery(&task.root_id);
        if !is_current {
            task.cancelled.store(true, Ordering::Release);
        }
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
        match &task.kind {
            RecoveryTaskKind::FullScan { scan_id } => {
                let _ = suspend_scan(scan_id);
            }
            RecoveryTaskKind::BoundedAuthoritative { .. }
            | RecoveryTaskKind::MetadataInventory { .. } => {
                task.cancelled.store(true, Ordering::Release);
            }
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

#[cfg(all(windows, debug_assertions))]
fn one_line_message(message: &str) -> String {
    message.replace(['\r', '\n'], " ")
}

#[cfg(windows)]
fn run_recovery_scan(
    request: ScanRequest,
    cancelled: &AtomicBool,
    storage: crate::application::storage::StoragePaths,
) -> Result<(), ScanError> {
    let scan_id = request.scan_id.clone();
    let mut completed = false;
    let mut terminal_error = None;
    #[cfg(debug_assertions)]
    let mut last_reported_entries = 0_u64;
    #[cfg(debug_assertions)]
    let mut isolated_finalization_races = 0_u32;
    resume_authoritative_scan_with_storage(
        request,
        |event| {
            #[cfg(debug_assertions)]
            match &event {
                ScanEvent::Progress {
                    visited_entries,
                    accepted_items,
                    issue_count,
                    ..
                } if *visited_entries == 1
                    || visited_entries.saturating_sub(last_reported_entries) >= 4_096 =>
                {
                    last_reported_entries = *visited_entries;
                    eprintln!(
                        "[Ame sync] recovery progress kind=full-scan scan={scan_id} visited={visited_entries} accepted={accepted_items} issues={issue_count}"
                    );
                }
                ScanEvent::Finalizing {
                    validated_items,
                    total_items,
                    visited_entries,
                    issue_count,
                    ..
                } => {
                    eprintln!(
                        "[Ame sync] recovery progress kind=full-scan scan={scan_id} phase=finalizing validated={validated_items}/{total_items} visited={visited_entries} issues={issue_count}"
                    );
                }
                ScanEvent::Issue { issue, .. }
                    if matches!(
                        issue.code.as_str(),
                        "source_changed_during_scan"
                            | "source_replaced_during_scan"
                            | "source_revalidation_failed"
                            | "source_became_unavailable"
                            | "source_identity_unavailable"
                    ) =>
                {
                    isolated_finalization_races = isolated_finalization_races.saturating_add(1);
                    eprintln!(
                        "[Ame sync] recovery progress kind=full-scan scan={scan_id} phase=isolating-file-race retry_paths={isolated_finalization_races} code={}",
                        issue.code
                    );
                }
                _ => {}
            }
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
        },
        storage,
    )?;
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
    #[cfg(windows)]
    use crate::ports::LibraryChangeQueue;

    #[cfg(windows)]
    #[derive(Clone, Copy)]
    struct HealthyFactory;

    #[cfg(windows)]
    struct HealthySource;

    #[cfg(windows)]
    impl crate::ports::LibraryChangeSourceFactory for HealthyFactory {
        type Source = HealthySource;

        fn start(
            &self,
            _request: &crate::ports::LibraryChangeSourceRequest,
        ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
            Ok(HealthySource)
        }
    }

    #[cfg(windows)]
    impl crate::ports::LibraryChangeSource for HealthySource {
        fn health(&self) -> crate::domain::LibraryChangeSourceHealth {
            crate::domain::LibraryChangeSourceHealth::Healthy
        }

        fn drain(
            &mut self,
            _max_observations: usize,
        ) -> Result<crate::domain::LibraryChangeSourceBatch, crate::domain::LibraryChangeSourceError>
        {
            Ok(crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 0,
                ignored_callback_count: 0,
                last_issue_code: None,
            })
        }

        fn stop(
            &mut self,
        ) -> Result<
            crate::domain::LibraryChangeSourceStopReport,
            crate::domain::LibraryChangeSourceError,
        > {
            Ok(crate::domain::LibraryChangeSourceStopReport::default())
        }
    }

    #[test]
    fn current_time_is_representable() {
        assert!(now_unix_ms().expect("current time") > 0);
    }

    #[cfg(windows)]
    #[test]
    fn active_recovery_projects_its_real_phase_during_transient_catalog_contention() {
        let (_sender, receiver) = mpsc::sync_channel(1);
        let recovery = RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        };
        let mut snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 1,
            applied_mutation_count: 0,
            roots: vec![crate::domain::LibraryRootSynchronizationStatus {
                root_id: "root-a".to_owned(),
                root_generation: 1,
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::Updating,
                freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
                phase: LibrarySynchronizationPhase::QueuePublication,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                pending_change_count: 1,
                retry_wait_count: 0,
                freshness_unknown_count: 0,
                recovery_blocked: false,
                last_issue_code: Some("catalog_database_busy".to_owned()),
            }],
        };

        project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Updating
        );
        assert_eq!(
            snapshot.roots[0].freshness_cause,
            crate::domain::CatalogFreshnessCause::PendingChanges
        );
        assert_eq!(
            snapshot.roots[0].phase,
            LibrarySynchronizationPhase::Reconciliation
        );
    }

    #[cfg(windows)]
    #[test]
    fn active_bounded_recovery_projects_updating_after_nominal_lease_expiry() {
        let (_sender, receiver) = mpsc::sync_channel(1);
        let recovery = RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        };
        let mut snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 1,
            applied_mutation_count: 0,
            roots: vec![crate::domain::LibraryRootSynchronizationStatus {
                root_id: "root-a".to_owned(),
                root_generation: 1,
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
                freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
                phase: LibrarySynchronizationPhase::Blocked,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Degraded,
                pending_change_count: 1,
                retry_wait_count: 0,
                freshness_unknown_count: 1,
                recovery_blocked: false,
                last_issue_code: None,
            }],
        };

        project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Updating
        );
        assert_eq!(
            snapshot.roots[0].phase,
            LibrarySynchronizationPhase::Reconciliation
        );
    }

    #[cfg(windows)]
    #[test]
    fn active_recovery_does_not_hide_durable_degraded_queue_state() {
        let (_sender, receiver) = mpsc::sync_channel(1);
        let recovery = RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        };
        let mut snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 1,
            applied_mutation_count: 0,
            roots: vec![crate::domain::LibraryRootSynchronizationStatus {
                root_id: "root-a".to_owned(),
                root_generation: 1,
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
                freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
                phase: LibrarySynchronizationPhase::Blocked,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Degraded,
                pending_change_count: 1,
                retry_wait_count: 1,
                freshness_unknown_count: 0,
                recovery_blocked: true,
                last_issue_code: Some("metadata_inventory_enumeration_failed".to_owned()),
            }],
        };

        project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::NeedsReconciliation
        );
        assert_eq!(
            snapshot.roots[0].last_issue_code.as_deref(),
            Some("metadata_inventory_enumeration_failed")
        );
    }

    #[cfg(windows)]
    #[test]
    fn newer_continuity_revision_cancels_an_older_automatic_worker() {
        let directory = tempfile::tempdir().expect("catalog directory");
        let source = tempfile::tempdir().expect("source directory");
        let root_path = source.path().to_string_lossy().into_owned();
        let root_id = "root-a";
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: "scan-a".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                root_id,
                &root_path,
            )
            .expect("begin root scan");
        catalog
            .publish_scan("scan-a", root_id, 0, 0)
            .expect("publish root scan");
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(
                crate::ports::erase_library_change_source_factory(HealthyFactory),
            ),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
            is_stopping: false,
        };
        production
            .runtime
            .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
                crate::domain::LibraryRootAvailability::Available
            })
            .expect("establish first continuity epoch");
        let first_revision = production
            .runtime
            .root_continuity_revision(root_id)
            .expect("first continuity revision");
        let cancelled = Arc::new(AtomicBool::new(false));
        let (_sender, receiver) = mpsc::sync_channel(1);
        production.recovery = Some(RecoveryTask {
            root_id: root_id.to_owned(),
            kind: RecoveryTaskKind::MetadataInventory {
                continuity_revision: first_revision,
            },
            phase: Arc::new(Mutex::new(
                LibrarySynchronizationPhase::InventoryEnumeration,
            )),
            cancelled: Arc::clone(&cancelled),
            receiver,
            worker: None,
        });
        production
            .runtime
            .roots
            .get_mut(root_id)
            .expect("runtime root")
            .needs_continuity_gap = true;
        production
            .runtime
            .poll_without_authoritative_recovery(&mut catalog, 2_000, |_| {
                crate::domain::LibraryRootAvailability::Available
            })
            .expect("establish newer continuity epoch");

        production.cancel_stale_automatic_recovery();

        assert!(cancelled.load(Ordering::Acquire));
        assert!(
            production
                .runtime
                .root_continuity_revision(root_id)
                .expect("newer continuity revision")
                > first_revision
        );
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
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
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
    fn bounded_authoritative_success_clears_only_its_root_retry_history() {
        let factory = crate::adapters::production_library_change_source_factory();
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
            is_stopping: false,
        };
        production.record_recovery_failure("root-a", 1_000);
        production.record_recovery_failure("root-b", 5_000);
        let root_b_retry = production.recovery_retries["root-b"];

        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(Ok(RecoveryTaskOutcome::Authoritative(
                AuthoritativeLibraryChangeReport::default(),
            )))
            .expect("bounded recovery result");
        production.recovery = Some(RecoveryTask {
            root_id: "root-a".to_owned(),
            kind: RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: Arc::new(AtomicBool::new(false)),
            receiver,
            worker: None,
        });

        production
            .poll_recovery(2_000)
            .expect("bounded recovery succeeds");
        assert!(!production.recovery_retries.contains_key("root-a"));
        assert_eq!(production.recovery_retries["root-b"], root_b_retry);
    }

    #[cfg(windows)]
    #[test]
    fn authoritative_root_cursor_prevents_a_busy_first_root_from_starving_peers() {
        let directory = tempfile::tempdir().expect("catalog directory");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        let generation = crate::domain::LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        for (root_id, root_path) in [("root-a", "C:\\RootA"), ("root-b", "C:\\RootB")] {
            let scan_id = format!("scan-{root_id}");
            catalog
                .begin_scan(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
                        root_path: root_path.to_owned(),
                        max_items: None,
                        max_entries: None,
                        preview_edge: 512,
                    },
                    root_id,
                    root_path,
                )
                .expect("begin root scan");
            catalog
                .publish_scan(&scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .enqueue_library_change_intents(
                    &[crate::domain::LibraryChangeIntent {
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                        scope: crate::domain::LibraryChangeScope::Root,
                        relative_path: String::new(),
                        previous_relative_path: None,
                        origin: crate::domain::LibraryChangeOrigin::StartupCatchUp,
                        first_observed_unix_ms: 1_000,
                        most_recent_observed_unix_ms: 1_000,
                        first_sequence: 1,
                        most_recent_sequence: 1,
                        coalesced_observation_count: 1,
                    }],
                    1_000,
                    policy,
                )
                .expect("enqueue authoritative work");
        }
        let snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 0,
            applied_mutation_count: 0,
            roots: ["root-a", "root-b"]
                .into_iter()
                .map(|root_id| crate::domain::LibraryRootSynchronizationStatus {
                    root_id: root_id.to_owned(),
                    root_generation: generation.value(),
                    availability: crate::domain::LibraryRootAvailability::Available,
                    freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
                    freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
                    phase: LibrarySynchronizationPhase::Blocked,
                    source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                    queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                    pending_change_count: 1,
                    retry_wait_count: 0,
                    freshness_unknown_count: 1,
                    recovery_blocked: false,
                    last_issue_code: None,
                })
                .collect(),
        };
        let factory = crate::adapters::production_library_change_source_factory();
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: None,
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
            is_stopping: false,
        };

        let first = ready_authoritative_root(&mut production, &catalog, &snapshot, 2_000)
            .expect("first ready root")
            .expect("first root");
        let second = ready_authoritative_root(&mut production, &catalog, &snapshot, 2_000)
            .expect("second ready root")
            .expect("second root");
        let wrapped = ready_authoritative_root(&mut production, &catalog, &snapshot, 2_000)
            .expect("wrapped ready root")
            .expect("wrapped root");

        assert_eq!(first.0, "root-a");
        assert_eq!(second.0, "root-b");
        assert_eq!(wrapped.0, "root-a");
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
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
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
                kind: RecoveryTaskKind::BoundedAuthoritative {
                    continuity_revision: 0,
                },
                phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
                cancelled: Arc::clone(&cancelled),
                receiver,
                worker: Some(worker),
            }),
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
            is_stopping: false,
        };

        production.stop_recovery().expect("stop bounded recovery");

        assert!(cancelled.load(Ordering::Acquire));
        assert!(production.recovery.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn stopping_full_scan_does_not_cancel_its_recoverable_scan_state() {
        let factory = crate::adapters::production_library_change_source_factory();
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(Err(ScanError::new(
                "authoritative_recovery_incomplete",
                "The suspended full scan remains recoverable",
            )))
            .expect("suspended full scan result");
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(factory),
            recovery: Some(RecoveryTask {
                root_id: "root-a".to_owned(),
                kind: RecoveryTaskKind::FullScan {
                    scan_id: "sync-recovery-a".to_owned(),
                },
                phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::FullScan)),
                cancelled: Arc::clone(&cancelled),
                receiver,
                worker: None,
            }),
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
            is_stopping: false,
        };

        production.stop_recovery().expect("stop full scan recovery");

        assert!(!cancelled.load(Ordering::Acquire));
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
                kind: RecoveryTaskKind::BoundedAuthoritative {
                    continuity_revision: 0,
                },
                phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
                cancelled: Arc::clone(&cancelled),
                receiver,
                worker: Some(worker),
            }),
            recovery_retries: BTreeMap::new(),
            recoverable_scan_cursor: None,
            authoritative_root_cursor: None,
            legacy_audits_retired: false,
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

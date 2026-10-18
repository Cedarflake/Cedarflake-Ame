#[cfg(all(test, windows))]
use std::collections::BTreeMap;
#[cfg(all(test, windows))]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(all(test, windows))]
use std::sync::Mutex;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc::RecvTimeoutError;
#[cfg(windows)]
use std::sync::mpsc::{self, Receiver, TryRecvError};
#[cfg(all(test, windows))]
use std::sync::{Condvar, OnceLock};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use crate::adapters::{
    SqliteCatalog, SqliteCatalogSession, inspect_root_availability,
    production_library_change_source_factory,
};
#[cfg(all(test, windows))]
use crate::domain::{IncrementalLibraryChangeReport, LibraryChangeId, LibrarySynchronizationPhase};
#[cfg(all(test, windows))]
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRecoveryAuthorityReason,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaselineStartRequest,
    PersistentJournalCapability, PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalEnrollmentReport, PersistentJournalFailure,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind,
    PersistentJournalVolumeIdentity,
};
use crate::domain::{
    LeasedLibraryChange, LibraryChangeLane, LibraryRootGeneration, LibrarySynchronizationSnapshot,
    ScanError,
};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use super::journal_status::JournalStatusOwner;
#[cfg(windows)]
use super::observation_diagnostics::measure_observation;
#[cfg(all(test, windows))]
use super::production_synchronization_cadence::ProductionSynchronizationCadence;
#[cfg(all(test, windows))]
use crate::application::AuthoritativeLibraryChangeReport;
#[cfg(windows)]
use crate::application::{MetadataInventoryRecoveryPage, storage_paths};
#[cfg(all(test, windows))]
use crate::journal_broker::{
    BrokerResponse, JournalCapability, PersistentChangeJournalOperationError,
    PersistentChangeJournalRead, PersistentChangeJournalSession, QueryJournalRequest,
    ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest, RootCapability,
    describe_production_persistent_journal_root,
};
#[cfg(windows)]
use crate::journal_broker::{
    PersistentChangeJournal, PersistentChangeJournalConnection,
    PersistentChangeJournalLiveOnlyReason, production_persistent_journal_caller_claim,
};
#[cfg(all(test, windows))]
use crate::ports::MetadataInventoryRepository;
#[cfg(windows)]
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSourceStarter,
    PersistentJournalRepository,
};

#[cfg(windows)]
mod poll_diagnostics;

#[cfg(windows)]
mod runtime_lifecycle;

#[cfg(all(test, windows))]
mod runtime_test_support;

#[cfg(all(test, windows, debug_assertions))]
mod ordinary_journal;

#[cfg(windows)]
mod catalog_scheduling;

#[cfg(windows)]
mod journal_volume;

#[cfg(windows)]
mod journal_lane;
#[cfg(windows)]
mod recovery_lane;
#[cfg(windows)]
mod recovery_sources;
#[cfg(windows)]
mod recovery_work;
#[cfg(windows)]
use journal_lane::JournalLane;
#[cfg(windows)]
use recovery_lane::{ReadyRecoveryWork, RecoveryLane};
#[cfg(windows)]
use recovery_sources::RecoverySources;
#[cfg(all(test, windows))]
use recovery_work::fail_next_recovery_worker_spawn;
#[cfg(windows)]
use recovery_work::{
    RecoveryTask, RecoveryTaskKind, RecoveryTaskOutcome, RecoveryWorkPolicy, RecoveryWorkRoot,
};

#[cfg(windows)]
mod journal_work;
#[cfg(windows)]
use journal_work::{JournalTask, JournalTaskOutcome};

#[cfg(windows)]
mod journal_recovery_admission;

#[cfg(windows)]
mod inventory_cleanup;

#[cfg(windows)]
mod root_location;

#[cfg(windows)]
mod poll_catalog;

#[cfg(windows)]
mod metadata_status;

#[cfg(windows)]
mod live_work;

#[cfg(windows)]
use live_work::LiveTask;
#[cfg(all(test, windows))]
use live_work::LiveWorkOutcome;

#[cfg(windows)]
use poll_catalog::PollCatalogOwner;

#[cfg(windows)]
use inventory_cleanup::InventoryCleanupOwner;

#[cfg(windows)]
use root_location::RootLocationRecoveryOwner;

#[cfg(windows)]
use poll_diagnostics::{
    ElapsedStageTimer, SynchronizationPollStageTimings, log_synchronization_poll_diagnostic,
};

#[cfg(all(test, windows))]
static LIVE_WORKER_AFTER_LEASE_PAUSES: OnceLock<
    Mutex<BTreeMap<String, Arc<LiveWorkerAfterLeasePauseState>>>,
> = OnceLock::new();

#[cfg(all(test, windows))]
struct LiveWorkerAfterLeasePauseState {
    reached: Mutex<bool>,
    reached_changed: Condvar,
    released: Mutex<bool>,
    released_changed: Condvar,
}

#[cfg(all(test, windows))]
struct LiveWorkerAfterLeasePause {
    root_id: String,
    state: Arc<LiveWorkerAfterLeasePauseState>,
}

#[cfg(all(test, windows))]
impl LiveWorkerAfterLeasePause {
    fn wait_until_reached(&self, timeout: Duration) {
        let reached = self.state.reached.lock().expect("live lease pause reached");
        let (reached, timeout_result) = self
            .state
            .reached_changed
            .wait_timeout_while(reached, timeout, |reached| !*reached)
            .expect("wait for live lease pause");
        assert!(
            *reached && !timeout_result.timed_out(),
            "production live worker did not reach the after-lease pause"
        );
    }

    fn release(&self) {
        let mut released = self
            .state
            .released
            .lock()
            .expect("live lease pause release");
        *released = true;
        self.state.released_changed.notify_all();
    }
}

#[cfg(all(test, windows))]
impl Drop for LiveWorkerAfterLeasePause {
    fn drop(&mut self) {
        self.release();
        let pauses = LIVE_WORKER_AFTER_LEASE_PAUSES.get_or_init(Default::default);
        let mut pauses = pauses.lock().expect("live lease pause registry");
        if pauses
            .get(&self.root_id)
            .is_some_and(|state| Arc::ptr_eq(state, &self.state))
        {
            pauses.remove(&self.root_id);
        }
    }
}

#[cfg(all(test, windows))]
fn install_live_worker_after_lease_pause(root_id: &str) -> LiveWorkerAfterLeasePause {
    let state = Arc::new(LiveWorkerAfterLeasePauseState {
        reached: Mutex::new(false),
        reached_changed: Condvar::new(),
        released: Mutex::new(false),
        released_changed: Condvar::new(),
    });
    let pauses = LIVE_WORKER_AFTER_LEASE_PAUSES.get_or_init(Default::default);
    let replaced = pauses
        .lock()
        .expect("live lease pause registry")
        .insert(root_id.to_owned(), Arc::clone(&state));
    assert!(replaced.is_none(), "live lease pause already installed");
    LiveWorkerAfterLeasePause {
        root_id: root_id.to_owned(),
        state,
    }
}

#[cfg(all(test, windows))]
fn pause_live_worker_after_lease(root_id: &str) {
    let state = LIVE_WORKER_AFTER_LEASE_PAUSES
        .get_or_init(Default::default)
        .lock()
        .expect("live lease pause registry")
        .get(root_id)
        .cloned();
    let Some(state) = state else {
        return;
    };
    {
        let mut reached = state.reached.lock().expect("live lease pause reached");
        *reached = true;
        state.reached_changed.notify_all();
    }
    let released = state.released.lock().expect("live lease pause release");
    let _released = state
        .released_changed
        .wait_while(released, |released| !*released)
        .expect("wait for live lease pause release");
}
#[cfg(windows)]
const SYNCHRONIZATION_STOP_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(windows)]
// The absolute 4096th queue slot remains the inventory authority while one candidate page drains.
const METADATA_INVENTORY_WORK_PAGE_ENTRIES: u32 = 4_095;
#[cfg(windows)]
struct ProductionSynchronization {
    runtime: LibrarySynchronizationRuntime<
        <SqliteCatalog as crate::ports::LibraryChangeIngress>::Reservation,
    >,
    catalog_session: Option<Arc<SqliteCatalogSession>>,
    poll_catalog: PollCatalogOwner,
    persistent_change_journal_factory: Option<Arc<dyn PersistentChangeJournal>>,
    _persistent_change_journal: PersistentChangeJournalConnection,
    persistent_change_journal_opened: bool,
    persistent_change_journal_caller: Option<crate::journal_broker::CallerClaim>,
    journal_status: JournalStatusOwner,
    journal: Option<JournalTask>,
    journal_lane: JournalLane,
    live: Option<LiveTask>,
    live_root_cursor: Option<String>,
    recovery: Option<RecoveryTask>,
    recovery_sources: RecoverySources,
    metadata_inventory_page_entries: u32,
    recovery_lane: RecoveryLane,
    legacy_automatic_scans_retired: bool,
    inventory_cleanup: InventoryCleanupOwner,
    root_location: RootLocationRecoveryOwner,
    is_stopping: bool,
    stop_requested: Arc<AtomicBool>,
    core_stopped: bool,
    journal_closed: bool,
    journal_close: Option<JournalCloseTask>,
    #[cfg(test)]
    drain_panic: Option<DrainPanicInjection>,
}

#[cfg(all(test, windows))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrainPanicPoint {
    RequestStop,
    P0Result,
    P1Result,
    P2Result,
    JournalCloseInstalled,
    JournalCloseResult,
}

#[cfg(all(test, windows))]
struct DrainPanicInjection {
    point: DrainPanicPoint,
    remaining: Option<usize>,
}

#[cfg(windows)]
struct JournalCloseTask {
    receiver: Receiver<Result<(), ScanError>>,
    worker: Option<JoinHandle<()>>,
    outcome: Option<Result<(), ScanError>>,
}

pub(crate) fn reserve_production_library_synchronization_start_ticket() -> Result<u64, ScanError> {
    #[cfg(windows)]
    {
        runtime_lifecycle::reserve_start_ticket()
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn reserve_production_library_synchronization_stop_fence() -> Result<u64, ScanError> {
    #[cfg(windows)]
    {
        runtime_lifecycle::reserve_stop_fence()
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn start_production_library_synchronization(
    owner_ticket: u64,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        runtime_lifecycle::start(owner_ticket)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn poll_production_library_synchronization(
    owner_ticket: u64,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        runtime_lifecycle::poll(owner_ticket)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

#[cfg(not(test))]
pub(crate) fn poll_production_first_import_change_capture(
    scan_id: &str,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    catalog_path: &std::path::Path,
) -> Result<bool, ScanError> {
    #[cfg(windows)]
    {
        let Some(owner_ticket) = runtime_lifecycle::first_import_owner_ticket()? else {
            return Ok(false);
        };
        let snapshot = match poll_production_library_synchronization(owner_ticket) {
            Ok(snapshot) => snapshot,
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "library_synchronization_start_in_progress"
                        | "library_synchronization_poll_in_progress"
                        | "library_synchronization_state_raced"
                ) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        let observer_is_healthy = snapshot.roots.iter().any(|root| {
            root.root_id == root_id
                && root.root_generation == root_generation.value()
                && root.availability == crate::domain::LibraryRootAvailability::Available
                && root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
        });
        if !observer_is_healthy {
            return Ok(false);
        }
        let catalog = crate::application::catalog_session::open_catalog(
            catalog_path,
            LibraryChangeLane::Journal,
        )?;
        catalog.first_import_change_capture_is_ready(scan_id, root_id, root_generation)
    }
    #[cfg(not(windows))]
    {
        let _ = (scan_id, root_id, root_generation, catalog_path);
        Err(unsupported_platform())
    }
}

pub(crate) fn stop_production_library_synchronization(
    cancellation_fence: u64,
) -> Result<(), ScanError> {
    #[cfg(windows)]
    {
        runtime_lifecycle::stop(cancellation_fence)
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
    let started = Instant::now();
    let mut timings = SynchronizationPollStageTimings::default();
    let result = poll_runtime_with_storage_inner(runtime, storage, &mut timings);
    if result.is_err() {
        runtime.runtime.release_ingress_admissions();
    }
    log_synchronization_poll_diagnostic(started.elapsed(), &timings, &result);
    result
}

#[cfg(windows)]
fn poll_runtime_with_storage_inner(
    runtime: &mut ProductionSynchronization,
    storage: &crate::application::storage::StoragePaths,
    timings: &mut SynchronizationPollStageTimings,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    timings.stage = "preflight";
    if runtime.stop_requested.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "library_synchronization_poll_cancelled",
            "Synchronization polling was cancelled before work began",
        ));
    }
    timings.stage = "catalog";
    let catalog_timer = ElapsedStageTimer::new(&mut timings.catalog_ms);
    let mut poll_unix_ms = now_unix_ms()?;
    let mut checkout = runtime.poll_catalog.checkout(&storage.catalog_path)?;
    runtime.catalog_session = Some(checkout.session());
    let catalog = &mut *checkout;
    catalog.enqueue_media_admission_rechecks(poll_unix_ms, runtime.runtime.queue_policy())?;
    catalog.enqueue_metadata_rechecks(poll_unix_ms, runtime.runtime.queue_policy())?;
    if !runtime.legacy_automatic_scans_retired {
        catalog.retire_legacy_automatic_full_scans(poll_unix_ms)?;
        runtime.legacy_automatic_scans_retired = true;
        poll_unix_ms = now_unix_ms()?;
    }
    drop(catalog_timer);
    timings.stage = "observation";
    let observation_timer = ElapsedStageTimer::new(&mut timings.observation_ms);
    let mut admissions = measure_observation("admission_load", || {
        super::admission::SynchronizationAdmissions::load(catalog)
    })?;
    let mut snapshot = measure_observation("observer_poll", || {
        runtime.runtime.poll_internal(
            catalog,
            admissions.observing_roots(),
            poll_unix_ms,
            |root_path| {
                measure_observation("root_availability", || {
                    inspect_root_availability(root_path).availability
                })
            },
            false,
            false,
        )
    })?;
    measure_observation("admission_revalidation", || {
        admissions.revalidate(catalog, &mut runtime.runtime, &mut snapshot)
    })?;
    if runtime.stop_requested.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "library_synchronization_poll_cancelled",
            "Synchronization polling was cancelled after root observation",
        ));
    }
    drop(observation_timer);
    timings.stage = "lanes";
    let lanes_timer = ElapsedStageTimer::new(&mut timings.lanes_ms);
    runtime.open_persistent_change_journal_after_watcher();
    if runtime.stop_requested.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "library_synchronization_poll_cancelled",
            "Synchronization polling was cancelled after journal connection",
        ));
    }
    let live_mutation_count = runtime.poll_live(poll_unix_ms);
    let journal_mutation_count = runtime.poll_journal();
    let recovered_mutation_count = runtime.poll_recovery(poll_unix_ms)?;
    runtime.recovery_sources.prune(catalog)?;
    runtime.cancel_stale_automatic_recovery();
    snapshot.applied_mutation_count = snapshot
        .applied_mutation_count
        .checked_add(recovered_mutation_count)
        .and_then(|count| count.checked_add(journal_mutation_count))
        .and_then(|count| count.checked_add(live_mutation_count))
        .ok_or_else(|| {
            ScanError::new(
                "library_synchronization_count_overflow",
                "The synchronization mutation count exceeded the supported range",
            )
        })?;
    drop(lanes_timer);
    timings.stage = "scheduling";
    let scheduling_timer = ElapsedStageTimer::new(&mut timings.scheduling_ms);
    let change_capture_unix_ms = now_unix_ms()?;
    project_active_recovery_as_updating(runtime.recovery.as_ref(), &mut snapshot);
    runtime.schedule_catalog_work(
        catalog,
        &snapshot,
        poll_unix_ms,
        change_capture_unix_ms,
        storage,
    )?;
    drop(scheduling_timer);
    timings.stage = "projection";
    let projection_timer = ElapsedStageTimer::new(&mut timings.projection_ms);
    project_active_recovery_as_updating(runtime.recovery.as_ref(), &mut snapshot);
    runtime.project_persistent_journal_continuity(catalog, &mut snapshot)?;
    admissions.revalidate(catalog, &mut runtime.runtime, &mut snapshot)?;
    metadata_status::project_pending_metadata(catalog, &mut snapshot)?;
    admissions.append_dormant_statuses(&mut snapshot);
    drop(projection_timer);
    runtime.inventory_cleanup.poll(
        runtime
            .catalog_session
            .as_ref()
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_cleanup_session_missing",
                    "The runtime lost its validated catalog before scheduling cleanup",
                )
            })?
            .clone(),
        Arc::clone(&runtime.stop_requested),
    )?;
    runtime.root_location.poll(
        checkout.session(),
        admissions.observing_roots(),
        &mut snapshot,
        Arc::clone(&runtime.stop_requested),
    )?;
    timings.stage = "checkout_return";
    let return_started = Instant::now();
    drop(admissions);
    drop(checkout);
    timings.checkout_return_ms = Some(return_started.elapsed().as_millis());
    timings.stage = "complete";
    Ok(snapshot)
}

#[cfg(windows)]
fn new_production_synchronization() -> ProductionSynchronization {
    let factory = crate::adapters::production_identity_journal_factory();
    new_production_synchronization_with_factory(production_library_change_source_factory(), factory)
}

#[cfg(windows)]
fn new_production_synchronization_with_factory(
    start_source: LibraryChangeSourceStarter,
    factory: Arc<dyn PersistentChangeJournal>,
) -> ProductionSynchronization {
    ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_production(start_source),
        catalog_session: None,
        poll_catalog: PollCatalogOwner::default(),
        persistent_change_journal_factory: Some(factory),
        _persistent_change_journal: PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
        ),
        persistent_change_journal_opened: false,
        persistent_change_journal_caller: None,
        journal: None,
        journal_status: JournalStatusOwner::default(),
        journal_lane: JournalLane::default(),
        live: None,
        live_root_cursor: None,
        recovery: None,
        recovery_sources: RecoverySources::default(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_lane: RecoveryLane::default(),
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
        root_location: RootLocationRecoveryOwner::default(),
        is_stopping: false,
        stop_requested: Arc::new(AtomicBool::new(false)),
        core_stopped: false,
        journal_closed: false,
        journal_close: None,
        #[cfg(test)]
        drain_panic: None,
    }
}

#[cfg(all(windows, test))]
fn new_production_synchronization_with_connection(
    start_source: LibraryChangeSourceStarter,
    persistent_change_journal: PersistentChangeJournalConnection,
) -> ProductionSynchronization {
    ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_production(start_source),
        catalog_session: None,
        poll_catalog: PollCatalogOwner::default(),
        persistent_change_journal_factory: None,
        _persistent_change_journal: persistent_change_journal,
        persistent_change_journal_opened: true,
        persistent_change_journal_caller: None,
        journal: None,
        journal_status: JournalStatusOwner::default(),
        journal_lane: JournalLane::default(),
        live: None,
        live_root_cursor: None,
        recovery: None,
        recovery_sources: RecoverySources::default(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_lane: RecoveryLane::default(),
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
        root_location: RootLocationRecoveryOwner::default(),
        is_stopping: false,
        stop_requested: Arc::new(AtomicBool::new(false)),
        core_stopped: false,
        journal_closed: false,
        journal_close: None,
        #[cfg(test)]
        drain_panic: None,
    }
}

#[cfg(windows)]
impl ProductionSynchronization {
    fn validated_catalog_session(
        &mut self,
        catalog_path: &std::path::Path,
    ) -> Result<Arc<SqliteCatalogSession>, ScanError> {
        if let Some(session) = &self.catalog_session {
            if session.path() != catalog_path {
                return Err(ScanError::new(
                    "library_synchronization_catalog_changed",
                    "The production runtime cannot switch catalogs within one epoch",
                ));
            }
            return Ok(session.clone());
        }
        let session = crate::application::catalog_session::validated_catalog_session(catalog_path)?;
        self.catalog_session = Some(Arc::clone(&session));
        Ok(session)
    }

    fn project_persistent_journal_continuity(
        &mut self,
        catalog: &SqliteCatalog,
        snapshot: &mut LibrarySynchronizationSnapshot,
    ) -> Result<(), ScanError> {
        self.journal_status
            .project(&self._persistent_change_journal, catalog, snapshot)
    }

    fn open_persistent_change_journal_after_watcher(&mut self) {
        if self.persistent_change_journal_opened {
            return;
        }
        let Some(factory) = self.persistent_change_journal_factory.as_ref() else {
            self.persistent_change_journal_opened = true;
            return;
        };
        self._persistent_change_journal = factory.connect();
        if matches!(
            self._persistent_change_journal,
            PersistentChangeJournalConnection::Connected(_)
        ) {
            match production_persistent_journal_caller_claim() {
                Ok(caller) => self.persistent_change_journal_caller = Some(caller),
                Err(_) => {
                    let connected = std::mem::replace(
                        &mut self._persistent_change_journal,
                        PersistentChangeJournalConnection::LiveOnly(
                            PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
                        ),
                    );
                    let _ = connected.close();
                }
            }
        }
        self.persistent_change_journal_opened = true;
    }
}

#[cfg(all(test, windows))]
pub(crate) struct ProductionSynchronizationTestHarness {
    runtime: ProductionSynchronization,
    storage: crate::application::storage::StoragePaths,
    cadence: ProductionSynchronizationCadence,
}

#[cfg(all(test, windows))]
impl ProductionSynchronizationTestHarness {
    pub(crate) fn with_change_source_for_contract(
        storage: crate::application::storage::StoragePaths,
        factory: crate::ports::LibraryChangeSourceStarter,
    ) -> Self {
        Self {
            runtime: new_production_synchronization_with_connection(
                factory,
                PersistentChangeJournalConnection::LiveOnly(
                    PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
                ),
            ),
            storage,
            cadence: ProductionSynchronizationCadence::from_shared_policy(),
        }
    }

    pub(crate) fn has_scheduled_work(&self) -> bool {
        self.runtime.live.is_some()
            || self.runtime.journal.is_some()
            || self.runtime.recovery.is_some()
    }

    pub(crate) fn new(storage: crate::application::storage::StoragePaths) -> Self {
        Self {
            runtime: new_production_synchronization(),
            storage,
            cadence: ProductionSynchronizationCadence::from_shared_policy(),
        }
    }

    pub(crate) fn from_policy_source_for_contract(
        storage: crate::application::storage::StoragePaths,
        source: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            runtime: new_production_synchronization(),
            storage,
            cadence: ProductionSynchronizationCadence::try_from_policy_source(source)?,
        })
    }

    pub(crate) fn with_persistent_journal_session(
        storage: crate::application::storage::StoragePaths,
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
    ) -> Self {
        let mut runtime = new_production_synchronization_with_connection(
            crate::adapters::production_library_change_source_factory(),
            PersistentChangeJournalConnection::Connected(session),
        );
        runtime.persistent_change_journal_caller = Some(caller);
        Self {
            runtime,
            storage,
            cadence: ProductionSynchronizationCadence::from_shared_policy(),
        }
    }

    pub(crate) const fn production_cadence(&self) -> ProductionSynchronizationCadence {
        self.cadence
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
    let recovery_phase = recovery.phase();
    for status in &mut snapshot.roots {
        if status.root_id == recovery.root_id()
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
fn ready_recovery_work(
    runtime: &mut ProductionSynchronization,
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
) -> Result<Option<ReadyRecoveryWork>, ScanError> {
    let journal_capacity_debt_roots = runtime.journal_lane.capacity_debt_roots(
        catalog,
        snapshot,
        &runtime._persistent_change_journal,
        runtime.persistent_change_journal_caller.as_ref(),
        runtime.runtime.queue_policy(),
    )?;
    if snapshot.roots.is_empty()
        || runtime.live.is_some()
        || runtime.journal.is_some()
        || snapshot_has_ready_live_work(
            catalog,
            snapshot,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )?
        || snapshot_has_ready_journal_work(
            catalog,
            snapshot,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )?
    {
        return Ok(None);
    }
    runtime.recovery_lane.select(
        catalog,
        snapshot,
        now_unix_ms,
        runtime.runtime.queue_policy(),
        &journal_capacity_debt_roots,
    )
}

#[cfg(windows)]
fn ready_live_root(
    runtime: &mut ProductionSynchronization,
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
) -> Result<Option<(String, LibraryRootGeneration)>, ScanError> {
    if snapshot.roots.is_empty() {
        return Ok(None);
    }
    let start_index = runtime
        .live_root_cursor
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
        if root.availability != crate::domain::LibraryRootAvailability::Available {
            continue;
        }
        let root_generation =
            LibraryRootGeneration::new(root.root_generation).ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
        if catalog
            .load_incremental_catalog_root(&root.root_id)?
            .is_none_or(|catalog_root| catalog_root.active_scan_id.is_none())
        {
            continue;
        }
        if catalog.has_ready_live_authoritative_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? || catalog.has_ready_live_path_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? {
            runtime.live_root_cursor = Some(root.root_id.clone());
            return Ok(Some((root.root_id.clone(), root_generation)));
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn snapshot_has_ready_live_work(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
    policy: crate::domain::LibraryChangeQueuePolicy,
) -> Result<bool, ScanError> {
    for root in &snapshot.roots {
        let Some(root_generation) = LibraryRootGeneration::new(root.root_generation) else {
            continue;
        };
        if catalog
            .load_incremental_catalog_root(&root.root_id)?
            .is_none_or(|catalog_root| catalog_root.active_scan_id.is_none())
        {
            continue;
        }
        if catalog.has_ready_live_authoritative_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            policy,
        )? || catalog.has_ready_live_path_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            policy,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn snapshot_has_ready_journal_work(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    now_unix_ms: i64,
    policy: crate::domain::LibraryChangeQueuePolicy,
) -> Result<bool, ScanError> {
    for root in &snapshot.roots {
        let Some(root_generation) = LibraryRootGeneration::new(root.root_generation) else {
            continue;
        };
        if catalog.has_ready_journal_path_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            policy,
        )? || catalog.has_ready_journal_authoritative_library_change(
            &root.root_id,
            root_generation,
            now_unix_ms,
            policy,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn select_rotated_key<'a>(
    keys: impl Iterator<Item = &'a String>,
    cursor: Option<&str>,
) -> Option<String> {
    let mut keys = keys.cloned().collect::<Vec<_>>();
    keys.sort();
    let start = cursor
        .and_then(|cursor| keys.iter().position(|key| key == cursor))
        .map_or(0, |index| (index + 1) % keys.len().max(1));
    keys.get(start).cloned()
}

#[cfg(windows)]
impl ProductionSynchronization {
    fn schedule_next_live_work(
        &mut self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        storage: &crate::application::storage::StoragePaths,
    ) -> Result<(), ScanError> {
        if self.live.is_some() || self.is_stopping {
            return Ok(());
        }
        let Some((root_id, root_generation)) =
            ready_live_root(self, catalog, snapshot, now_unix_ms)?
        else {
            return Ok(());
        };
        self.start_live_work(root_id, root_generation, storage.catalog_path.clone())
    }

    fn start_live_work(
        &mut self,
        root_id: String,
        root_generation: LibraryRootGeneration,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        if self.live.is_some() {
            return Err(ScanError::new(
                "live_reconciliation_already_running",
                "Another P0 live worker already owns the reserved slot",
            ));
        }
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        self.live = Some(LiveTask::start(
            catalog_session,
            root_id,
            root_generation,
            self.runtime.queue_policy(),
            self.runtime.recovery_policy(),
        )?);
        Ok(())
    }

    fn poll_live(&mut self, now_unix_ms: i64) -> u32 {
        let Some(task) = self.live.as_mut() else {
            return 0;
        };
        let Some(outcome) = task.poll() else {
            return 0;
        };
        let root_id = task.root_id().to_owned();
        self.live = None;
        if let Some(error) = outcome.failure {
            #[cfg(debug_assertions)]
            eprintln!(
                "[Ame sync] live worker failed code={} message={}",
                error.code,
                one_line_message(&error.message),
            );
            self.runtime
                .record_recovery_failure(&root_id, &error.code, now_unix_ms);
        }
        outcome.report.applied_mutation_count
    }

    fn poll_journal(&mut self) -> u32 {
        let Some(result) = self.journal.as_mut().and_then(JournalTask::poll) else {
            return 0;
        };
        self.journal = None;
        match result {
            Ok(JournalTaskOutcome::Page(window)) => {
                self.journal_status.record_window(&window);
                #[cfg(debug_assertions)]
                for _report in window.reports() {
                    eprintln!(
                        "[Ame sync] journal page finished roots={} failed={} observations={} checkpoints={} failure_codes={:?}",
                        _report.enrolled_root_count,
                        _report.failed_root_count,
                        _report.observation_count,
                        _report.advanced_checkpoint_count,
                        _report
                            .root_failures
                            .iter()
                            .map(|failure| failure.failure.code.as_str())
                            .collect::<Vec<_>>(),
                    );
                }
                0
            }
            Ok(JournalTaskOutcome::Drain(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] journal drain finished leased={} completed={} retried={} superseded={} mutations={}",
                    report.leased_count,
                    report.completed_count,
                    report.retried_count,
                    report.superseded_count,
                    report.applied_mutation_count,
                );
                report.applied_mutation_count
            }
            Ok(JournalTaskOutcome::BaselineBoundary) => 0,
            Err(_error) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] journal worker failed code={} message={}",
                    _error.code,
                    one_line_message(&_error.message),
                );
                0
            }
        }
    }

    fn schedule_next_journal_work(
        &mut self,
        catalog: &mut SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        storage: &crate::application::storage::StoragePaths,
    ) -> Result<(), ScanError> {
        if self.journal.is_some()
            || self.is_stopping
            || self.live.is_some()
            || snapshot_has_ready_live_work(
                catalog,
                snapshot,
                now_unix_ms,
                self.runtime.queue_policy(),
            )?
        {
            return Ok(());
        }
        let Some(work) = self.journal_lane.select(
            catalog,
            snapshot,
            now_unix_ms,
            &self._persistent_change_journal,
            self.persistent_change_journal_caller.as_ref(),
            self.runtime.queue_policy(),
        )?
        else {
            return Ok(());
        };
        let catalog_session = self.validated_catalog_session(&storage.catalog_path)?;
        self.journal = Some(JournalTask::start(
            work,
            catalog_session,
            self.runtime.queue_policy(),
            self.runtime.recovery_policy(),
            now_unix_ms,
        )?);
        Ok(())
    }

    fn poll_recovery(&mut self, now_unix_ms: i64) -> Result<u32, ScanError> {
        let Some(task) = self.recovery.as_mut() else {
            return Ok(0);
        };
        let Some(result) = task.poll() else {
            return Ok(0);
        };
        let root_id = task.root_id().to_owned();
        let metadata_inventory_change_id = match task.kind() {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => Some(*change_id),
            RecoveryTaskKind::BoundedAuthoritative { .. }
            | RecoveryTaskKind::CandidateDrain { .. }
            | RecoveryTaskKind::LegacyUnownedDrain { .. } => None,
        };
        #[cfg(debug_assertions)]
        let recovery_kind = match task.kind() {
            RecoveryTaskKind::BoundedAuthoritative { .. } => "bounded-authoritative",
            RecoveryTaskKind::MetadataInventory { .. } => "metadata-inventory",
            RecoveryTaskKind::CandidateDrain { .. } => "candidate-drain",
            RecoveryTaskKind::LegacyUnownedDrain { .. } => "legacy-unowned-drain",
        };
        self.recovery = None;
        match result {
            Ok(RecoveryTaskOutcome::Authoritative(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.incremental.applied_mutation_count
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_lane.record_success(&root_id);
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::MetadataInventory(page)) => {
                let change_id = metadata_inventory_change_id.ok_or_else(|| {
                    ScanError::new(
                        "metadata_inventory_recovery_task_mismatch",
                        "The metadata inventory result lost its immutable recovery change owner",
                    )
                })?;
                let MetadataInventoryRecoveryPage {
                    report,
                    retained_source,
                } = *page;
                self.recovery_sources.publish(change_id, retained_source);
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok leased={} completed={} retried={} deferred={} superseded={} staged={} candidates={} mutations={}",
                    report.incremental.leased_count,
                    report.incremental.completed_count,
                    report.incremental.retried_count,
                    report.incremental.deferred_count,
                    report.incremental.superseded_count,
                    report.inventory.staged_entry_count,
                    report.inventory.candidate_count,
                    report.incremental.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_lane.record_success(&root_id);
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::CandidateDrain(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_lane.record_success(&root_id);
                Ok(report.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::LegacyUnownedDrain(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_lane.record_success(&root_id);
                Ok(report.applied_mutation_count)
            }
            Err(error) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=failed code={} message={}",
                    error.code,
                    one_line_message(&error.message)
                );
                self.recovery_lane.record_failure(&root_id, now_unix_ms);
                self.runtime
                    .record_recovery_failure(&root_id, &error.code, now_unix_ms);
                Ok(0)
            }
        }
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
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        self.recovery = Some(recovery_work::start_automatic(
            RecoveryWorkRoot {
                root_id,
                root_generation,
                continuity_revision,
            },
            leased,
            now_unix_ms,
            catalog_session,
            RecoveryWorkPolicy {
                queue: self.runtime.queue_policy(),
                recovery: self.runtime.recovery_policy(),
                metadata_inventory_page_entries: self.metadata_inventory_page_entries,
            },
            &mut self.recovery_sources,
        )?);
        Ok(())
    }

    fn start_recovery_candidate_drain(
        &mut self,
        root_id: String,
        root_generation: LibraryRootGeneration,
        continuity_revision: u64,
        now_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Err(ScanError::new(
                "authoritative_recovery_already_running",
                "Another recovery page already owns the P2 worker slot",
            ));
        }
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        self.recovery = Some(recovery_work::start_candidates(
            RecoveryWorkRoot {
                root_id,
                root_generation,
                continuity_revision,
            },
            now_unix_ms,
            catalog_session,
            self.runtime.queue_policy(),
        )?);
        Ok(())
    }

    fn start_legacy_unowned_recovery_drain(
        &mut self,
        root_id: String,
        root_generation: LibraryRootGeneration,
        continuity_revision: u64,
        now_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        if self.recovery.is_some() {
            return Err(ScanError::new(
                "authoritative_recovery_already_running",
                "Another recovery page already owns the P2 worker slot",
            ));
        }
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        self.recovery = Some(recovery_work::start_legacy_unowned(
            RecoveryWorkRoot {
                root_id,
                root_generation,
                continuity_revision,
            },
            now_unix_ms,
            catalog_session,
            self.runtime.queue_policy(),
        )?);
        Ok(())
    }

    fn cancel_stale_automatic_recovery(&mut self) {
        let Some(task) = self.recovery.as_ref() else {
            return;
        };
        let expected_revision = task.continuity_revision();
        let is_current = self
            .runtime
            .root_continuity_revision(task.root_id())
            .is_some_and(|revision| revision == expected_revision)
            && self
                .runtime
                .root_is_ready_for_authoritative_recovery(task.root_id());
        if !is_current {
            task.request_stop();
        }
    }

    #[cfg(test)]
    fn stop(&mut self) -> Result<(), ScanError> {
        self.finish_stopping_until(Instant::now() + Duration::from_secs(2))
    }

    #[cfg(test)]
    fn inject_drain_panic(&mut self, point: DrainPanicPoint, remaining: Option<usize>) {
        self.drain_panic = Some(DrainPanicInjection { point, remaining });
    }

    #[cfg(test)]
    fn clear_drain_panic(&mut self) {
        self.drain_panic = None;
    }

    #[cfg(test)]
    fn maybe_panic_while_draining(&mut self, point: DrainPanicPoint) {
        let Some(injection) = self.drain_panic.as_mut() else {
            return;
        };
        if injection.point != point {
            return;
        }
        let should_panic = match injection.remaining.as_mut() {
            None => true,
            Some(remaining) if *remaining == 0 => false,
            Some(remaining) => {
                *remaining -= 1;
                true
            }
        };
        if should_panic {
            // Fault injection must unwind without running diagnostic hooks inside
            // the existing shutdown deadline, especially during assertion unwind.
            std::panic::resume_unwind(Box::new("controlled draining owner panic"));
        }
    }

    fn request_stop(&mut self) -> Result<(), ScanError> {
        if self.is_stopping {
            return Ok(());
        }
        self.is_stopping = true;
        self.stop_requested.store(true, Ordering::Release);
        if let Some(task) = &self.live {
            task.request_stop();
        }
        if let Some(task) = &self.journal {
            task.request_stop();
        }
        if let Some(task) = &self.recovery {
            task.request_stop();
        }
        self.inventory_cleanup.request_stop();
        self.root_location.request_stop();
        if let Err(error) = self.runtime.request_stop() {
            self.is_stopping = false;
            return Err(error);
        }
        Ok(())
    }

    fn finish_core_shutdown_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        if !self.core_stopped {
            self.runtime.request_stop()?;
            self.runtime.finish_stop_until(deadline)?;
            self.core_stopped = true;
        }
        if !self.journal_closed {
            if self.journal_close.is_none() {
                let connection = self._persistent_change_journal.clone();
                let poll_catalog = self.poll_catalog.clone();
                let (sender, receiver) = mpsc::channel();
                let worker = thread::Builder::new()
                    .name("ame-persistent-journal-close".to_owned())
                    .spawn(move || {
                        let catalog_retirement = poll_catalog.retire();
                        let result = connection.close().map_err(|error| {
                            ScanError::new(
                                "persistent_change_journal_close_failed",
                                format!(
                                    "The persistent change journal did not close cleanly: {error}"
                                ),
                            )
                        });
                        let _ = sender.send(catalog_retirement.and(result));
                    })
                    .map_err(|error| {
                        ScanError::new(
                            "persistent_change_journal_close_worker_start_failed",
                            format!("Could not start the persistent journal close worker: {error}"),
                        )
                    })?;
                self.journal_close = Some(JournalCloseTask {
                    receiver,
                    worker: Some(worker),
                    outcome: None,
                });
            }
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::JournalCloseInstalled);
            finish_journal_close_task_until(
                self.journal_close
                    .as_mut()
                    .expect("journal close task was just installed"),
                deadline,
            )?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::JournalCloseResult);
            self.journal_closed = true;
            self.journal_close = None;
        }
        Ok(())
    }

    #[cfg(test)]
    fn stop_recovery(&mut self) -> Result<(), ScanError> {
        self.stop_recovery_with_timeout(Duration::from_secs(2))
    }

    #[cfg(test)]
    fn stop_recovery_with_timeout(&mut self, timeout: Duration) -> Result<(), ScanError> {
        let Some(mut task) = self.recovery.take() else {
            return Ok(());
        };
        match task.stop_with_timeout(timeout) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.recovery = Some(task);
                Err(error)
            }
        }
    }

    fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        #[cfg(test)]
        self.maybe_panic_while_draining(DrainPanicPoint::RequestStop);
        self.request_stop()?;
        if let Some(task) = self.live.as_mut() {
            task.finish_stopping_until(deadline)?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P0Result);
            self.live = None;
        }
        if let Some(task) = self.journal.as_mut() {
            task.finish_stopping_until(deadline)?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P1Result);
            self.journal = None;
        }
        if let Some(task) = self.recovery.as_mut() {
            task.finish_stopping_until(deadline)?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P2Result);
            self.recovery = None;
        }
        self.inventory_cleanup.finish_stopping_until(deadline)?;
        self.root_location.finish_stopping_until(deadline)?;
        self.finish_core_shutdown_until(deadline)
    }
}

#[cfg(windows)]
fn finish_runtime_task_until<T>(
    receiver: &Receiver<T>,
    worker: &mut Option<JoinHandle<()>>,
    deadline: Instant,
    timeout_code: &'static str,
    timeout_message: &'static str,
) -> Result<(), ScanError> {
    match receiver.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {}
        Err(TryRecvError::Empty) => {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| ScanError::new(timeout_code, timeout_message))?;
            match receiver.recv_timeout(remaining) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => {}
                Err(RecvTimeoutError::Timeout) => {
                    return Err(ScanError::new(timeout_code, timeout_message));
                }
            }
        }
    }
    if let Some(worker_handle) = worker.as_ref() {
        while !worker_handle.is_finished() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| ScanError::new(timeout_code, timeout_message))?;
            thread::sleep(remaining.min(Duration::from_millis(2)));
        }
    }
    if let Some(worker) = worker.take() {
        let _ = worker.join();
    }
    Ok(())
}

#[cfg(windows)]
fn finish_journal_close_task_until(
    task: &mut JournalCloseTask,
    deadline: Instant,
) -> Result<(), ScanError> {
    if task.outcome.is_none() {
        match task.receiver.try_recv() {
            Ok(outcome) => task.outcome = Some(outcome),
            Err(TryRecvError::Disconnected) => {
                task.outcome = Some(Err(ScanError::new(
                    "persistent_change_journal_close_worker_failed",
                    "The persistent journal close worker disconnected before reporting its result",
                )));
            }
            Err(TryRecvError::Empty) => {
                let remaining = deadline.checked_duration_since(Instant::now()).ok_or_else(|| {
                    ScanError::new(
                        "library_synchronization_stop_timeout",
                        "Library synchronization did not stop within the bounded shutdown window",
                    )
                })?;
                match task.receiver.recv_timeout(remaining) {
                    Ok(outcome) => task.outcome = Some(outcome),
                    Err(RecvTimeoutError::Disconnected) => {
                        task.outcome = Some(Err(ScanError::new(
                            "persistent_change_journal_close_worker_failed",
                            "The persistent journal close worker disconnected before reporting its result",
                        )));
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        return Err(ScanError::new(
                            "library_synchronization_stop_timeout",
                            "Library synchronization did not stop within the bounded shutdown window",
                        ));
                    }
                }
            }
        }
    }
    if let Some(worker) = task.worker.as_ref() {
        while !worker.is_finished() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| {
                    ScanError::new(
                        "library_synchronization_stop_timeout",
                        "Library synchronization did not stop within the bounded shutdown window",
                    )
                })?;
            thread::sleep(remaining.min(Duration::from_millis(2)));
        }
    }
    if let Some(worker) = task.worker.take()
        && worker.join().is_err()
    {
        task.outcome = Some(Err(ScanError::new(
            "persistent_change_journal_close_worker_panicked",
            "The persistent journal close worker panicked",
        )));
    }
    task.outcome
        .as_ref()
        .expect("journal close outcome exists after its worker finished")
        .clone()
}

#[cfg(all(windows, debug_assertions))]
fn one_line_message(message: &str) -> String {
    message.replace(['\r', '\n'], " ")
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
mod tests;

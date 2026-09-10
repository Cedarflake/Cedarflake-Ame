#[cfg(all(test, windows))]
use std::cell::Cell;
#[cfg(windows)]
use std::collections::{BTreeMap, VecDeque};
#[cfg(windows)]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc::RecvTimeoutError;
#[cfg(windows)]
use std::sync::mpsc::{self, Receiver, TryRecvError};
#[cfg(windows)]
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
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
use crate::domain::{
    IncrementalLibraryChangeReport, LeasedLibraryChange, LibraryChangeId, LibraryChangeLane,
    LibraryRootGeneration, LibrarySynchronizationPhase, LibrarySynchronizationSnapshot,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalEnrollmentReport,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind, ScanError,
};
#[cfg(all(test, windows))]
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRecoveryAuthorityReason,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaselineStartRequest,
    PersistentJournalCapability, PersistentJournalFailure, PersistentJournalVolumeIdentity,
};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use super::observation_diagnostics::measure_observation;
#[cfg(all(test, windows))]
use super::production_synchronization_cadence::ProductionSynchronizationCadence;
#[cfg(windows)]
use crate::application::{
    AuthoritativeLibraryChangeReport, MetadataInventoryProgressPhase,
    MetadataInventoryRecoveryExecution, MetadataInventoryRecoveryPage,
    MetadataInventoryRecoveryReport, MetadataInventoryWorkerControl, PersistentJournalBrokerRoot,
    RetainedMetadataInventorySource, SessionBackedPersistentJournalVolumeReader,
    catch_up_persistent_journal_volume, classify_persistent_journal_operation_failure,
    defer_authoritative_change, leased_change_requires_metadata_inventory,
    process_leased_authoritative_library_change_cancellable,
    process_leased_metadata_inventory_change_with_retained_source,
    process_ready_library_changes_in_lane_cancellable,
    process_ready_metadata_inventory_recovery_candidates_cancellable,
    process_ready_unowned_recovery_paths_cancellable, storage_paths,
};
#[cfg(all(test, windows))]
use crate::journal_broker::{
    BrokerResponse, JournalCapability, PersistentChangeJournalRead, PersistentChangeJournalSession,
    QueryJournalRequest, ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest,
    RootCapability,
};
#[cfg(windows)]
use crate::journal_broker::{
    PersistentChangeJournal, PersistentChangeJournalConnection,
    PersistentChangeJournalLiveOnlyReason, PersistentChangeJournalOperationError,
    ProductionPersistentJournalRoot, describe_production_persistent_journal_root,
    production_client_allows_broker_activation, production_persistent_change_journal,
    production_persistent_journal_caller_claim,
};
#[cfg(windows)]
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSourceStarter,
    MetadataInventoryRepository, PersistentJournalRepository,
};

#[cfg(windows)]
mod poll_diagnostics;

#[cfg(windows)]
mod catalog_scheduling;

#[cfg(windows)]
mod inventory_cleanup;

#[cfg(windows)]
mod poll_catalog;

#[cfg(windows)]
use poll_catalog::PollCatalogOwner;

#[cfg(windows)]
use inventory_cleanup::InventoryCleanupOwner;

#[cfg(windows)]
use poll_diagnostics::{
    ElapsedStageTimer, SynchronizationPollStageTimings, log_synchronization_poll_diagnostic,
};

#[cfg(windows)]
static SYNCHRONIZATION_RUNTIME: OnceLock<SynchronizationRuntimeRegistry> = OnceLock::new();

#[cfg(all(test, windows))]
thread_local! {
    static FAIL_NEXT_RECOVERY_WORKER_SPAWN: Cell<bool> = const { Cell::new(false) };
}
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
const MAX_ADMITTED_STOP_FENCES: usize = 64;
#[cfg(windows)]
const RECOVERY_RETRY_INITIAL_MILLIS: i64 = 1_000;
#[cfg(windows)]
const RECOVERY_RETRY_MAXIMUM_MILLIS: i64 = 5 * 60 * 1_000;
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
    journal: Option<JournalTask>,
    journal_volume_cursor: Option<String>,
    journal_root_cursor: Option<String>,
    journal_next_action_is_read: bool,
    live: Option<LiveTask>,
    live_root_cursor: Option<String>,
    recovery: Option<RecoveryTask>,
    recovery_inventory_sources: BTreeMap<LibraryChangeId, RetainedMetadataInventorySource>,
    metadata_inventory_page_entries: u32,
    recovery_retries: BTreeMap<String, RecoveryRetryState>,
    authoritative_root_cursor: Option<String>,
    legacy_automatic_scans_retired: bool,
    inventory_cleanup: InventoryCleanupOwner,
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
struct SynchronizationRuntimeRegistry {
    state: Mutex<SynchronizationRuntimeState>,
    changed: Condvar,
    #[cfg(test)]
    panic_retention_barrier: Mutex<Option<PanicRetentionBarrier>>,
}

#[cfg(all(test, windows))]
#[derive(Clone)]
struct PanicRetentionBarrier {
    reached: Arc<std::sync::Barrier>,
    resume: Arc<std::sync::Barrier>,
}

#[cfg(windows)]
struct SynchronizationRuntimeState {
    next_epoch: u64,
    next_lifecycle_ordinal: LifecycleOrdinal,
    cancelled_through_ordinal: LifecycleOrdinal,
    admitted_stop_fences: VecDeque<AdmittedStopFence>,
    retired_through_epoch: u64,
    entry: SynchronizationRuntimeEntry,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct LifecycleOrdinal(u64);

#[cfg(windows)]
impl LifecycleOrdinal {
    const MAX: u64 = u64::MAX >> 1;

    fn checked_next(self) -> Result<Self, ScanError> {
        let next = self.0.checked_add(1).filter(|next| *next <= Self::MAX);
        next.map(Self).ok_or_else(|| {
            ScanError::new(
                "library_synchronization_lifecycle_ticket_exhausted",
                "The synchronization lifecycle ticket space is exhausted",
            )
        })
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeStartTicket(LifecycleOrdinal);

#[cfg(windows)]
impl RuntimeStartTicket {
    const KIND: u64 = 0;

    const fn from_ordinal(ordinal: LifecycleOrdinal) -> Self {
        Self(ordinal)
    }

    const fn ordinal(self) -> LifecycleOrdinal {
        self.0
    }

    const fn raw(self) -> u64 {
        (self.0.0 << 1) | Self::KIND
    }

    fn decode(raw: u64) -> Result<Self, ScanError> {
        let ordinal = raw >> 1;
        if raw & 1 != Self::KIND || ordinal == 0 {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_ticket_invalid",
                "The synchronization lifecycle ticket was not issued by this process",
            ));
        }
        Ok(Self(LifecycleOrdinal(ordinal)))
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeStopFence(LifecycleOrdinal);

#[cfg(windows)]
impl RuntimeStopFence {
    const KIND: u64 = 1;

    const fn from_ordinal(ordinal: LifecycleOrdinal) -> Self {
        Self(ordinal)
    }

    const fn ordinal(self) -> LifecycleOrdinal {
        self.0
    }

    const fn raw(self) -> u64 {
        (self.0.0 << 1) | Self::KIND
    }

    fn decode(raw: u64) -> Result<Self, ScanError> {
        let ordinal = raw >> 1;
        if raw & 1 != Self::KIND || ordinal == 0 {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_fence_invalid",
                "The synchronization stop fence was not admitted by this process",
            ));
        }
        Ok(Self(LifecycleOrdinal(ordinal)))
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeOwnerIdentity {
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmittedStopFenceUse {
    Pending,
    Consumed,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdmittedStopFence {
    fence: RuntimeStopFence,
    covered_owner: Option<RuntimeOwnerIdentity>,
    use_state: AdmittedStopFenceUse,
}

#[cfg(windows)]
enum SynchronizationRuntimeEntry {
    Empty,
    Starting {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
    },
    Ready {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
    Polling {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
    Stopping {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancellation_fence: RuntimeStopFence,
        deadline: Instant,
    },
    Draining {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancellation_fence: Option<RuntimeStopFence>,
        deadline: Instant,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

#[cfg(windows)]
impl SynchronizationRuntimeEntry {
    const fn owner_ticket(&self) -> Option<RuntimeStartTicket> {
        match self {
            Self::Empty => None,
            Self::Starting { owner_ticket, .. }
            | Self::Ready { owner_ticket, .. }
            | Self::Polling { owner_ticket, .. }
            | Self::Stopping { owner_ticket, .. }
            | Self::Draining { owner_ticket, .. } => Some(*owner_ticket),
        }
    }

    const fn owner_identity(&self) -> Option<RuntimeOwnerIdentity> {
        match self {
            Self::Empty => None,
            Self::Starting {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Ready {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Polling {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Stopping {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Draining {
                epoch,
                owner_ticket,
                ..
            } => Some(RuntimeOwnerIdentity {
                epoch: *epoch,
                owner_ticket: *owner_ticket,
            }),
        }
    }
}

#[cfg(windows)]
enum SynchronizationRuntimeClaim {
    Construct {
        epoch: u64,
        cancelled: Arc<AtomicBool>,
    },
    Poll {
        epoch: u64,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

#[cfg(windows)]
enum ConstructedRuntimeDisposition {
    Poll(Arc<Mutex<ProductionSynchronization>>),
    Drain {
        deadline: Instant,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

#[cfg(windows)]
enum PanickedRuntimeRetention {
    Retained(Instant),
    AlreadyDrained,
}

#[cfg(windows)]
type RuntimePollClaim = (u64, Arc<AtomicBool>, Arc<Mutex<ProductionSynchronization>>);

#[cfg(windows)]
impl Default for SynchronizationRuntimeRegistry {
    fn default() -> Self {
        Self {
            state: Mutex::new(SynchronizationRuntimeState {
                next_epoch: 0,
                next_lifecycle_ordinal: LifecycleOrdinal(0),
                cancelled_through_ordinal: LifecycleOrdinal(0),
                admitted_stop_fences: VecDeque::new(),
                retired_through_epoch: 0,
                entry: SynchronizationRuntimeEntry::Empty,
            }),
            changed: Condvar::new(),
            #[cfg(test)]
            panic_retention_barrier: Mutex::new(None),
        }
    }
}

#[cfg(windows)]
impl SynchronizationRuntimeState {
    fn allocate_lifecycle_ordinal(&mut self) -> Result<LifecycleOrdinal, ScanError> {
        let ordinal = self.next_lifecycle_ordinal.checked_next()?;
        self.next_lifecycle_ordinal = ordinal;
        Ok(ordinal)
    }

    fn allocate_epoch(&mut self) -> Result<u64, ScanError> {
        self.next_epoch = self.next_epoch.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "library_synchronization_epoch_exhausted",
                "The synchronization runtime epoch space is exhausted",
            )
        })?;
        Ok(self.next_epoch)
    }

    fn retire_epoch(&mut self, epoch: u64) {
        self.retired_through_epoch = self.retired_through_epoch.max(epoch);
    }

    fn prepare_stop_fence_capacity(&mut self) -> Result<(), ScanError> {
        if self.admitted_stop_fences.len() < MAX_ADMITTED_STOP_FENCES {
            return Ok(());
        }

        let active_owner = self.entry.owner_identity();
        let reclaimable = self.admitted_stop_fences.iter().position(|entry| {
            entry.use_state == AdmittedStopFenceUse::Consumed
                && !entry
                    .covered_owner
                    .is_some_and(|owner| Some(owner) == active_owner)
        });
        let Some(reclaimable) = reclaimable else {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_fence_capacity_exhausted",
                "The bounded synchronization stop-fence ledger is full",
            ));
        };
        self.admitted_stop_fences.remove(reclaimable);
        Ok(())
    }

    fn consume_admitted_stop_fence(
        &mut self,
        cancellation_fence: RuntimeStopFence,
    ) -> Result<(), ScanError> {
        if cancellation_fence.ordinal() > self.next_lifecycle_ordinal
            || cancellation_fence.ordinal() > self.cancelled_through_ordinal
        {
            return Err(invalid_stop_fence_error());
        }
        let Some(entry) = self
            .admitted_stop_fences
            .iter_mut()
            .find(|entry| entry.fence == cancellation_fence)
        else {
            return Err(invalid_stop_fence_error());
        };
        entry.use_state = AdmittedStopFenceUse::Consumed;
        Ok(())
    }
}

#[cfg(windows)]
struct JournalTask {
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<JournalTaskOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
struct JournalCloseTask {
    receiver: Receiver<Result<(), ScanError>>,
    worker: Option<JoinHandle<()>>,
    outcome: Option<Result<(), ScanError>>,
}

#[cfg(windows)]
struct LiveTask {
    root_id: String,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<AuthoritativeLibraryChangeReport, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
enum JournalTaskOutcome {
    Page(PersistentJournalEnrollmentReport),
    Drain(IncrementalLibraryChangeReport),
    BaselineBoundary,
}

#[cfg(windows)]
struct JournalVolumeWork {
    key: String,
    checkpoints: Vec<PersistentJournalCheckpoint>,
    roots: Vec<JournalRootWork>,
}

#[cfg(windows)]
struct JournalRootWork {
    root_id: String,
    root_generation: LibraryRootGeneration,
    root_path: std::path::PathBuf,
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
    BoundedAuthoritative {
        continuity_revision: u64,
    },
    MetadataInventory {
        continuity_revision: u64,
        change_id: LibraryChangeId,
    },
    CandidateDrain {
        continuity_revision: u64,
    },
    LegacyUnownedDrain {
        continuity_revision: u64,
    },
}

#[cfg(windows)]
enum RecoveryTaskOutcome {
    Authoritative(AuthoritativeLibraryChangeReport),
    MetadataInventory(Box<MetadataInventoryRecoveryPage>),
    CandidateDrain(IncrementalLibraryChangeReport),
    LegacyUnownedDrain(IncrementalLibraryChangeReport),
}

#[cfg(windows)]
fn take_transferred_inventory_source(
    transfer: &Arc<Mutex<Option<RetainedMetadataInventorySource>>>,
) -> Result<Option<RetainedMetadataInventorySource>, ScanError> {
    transfer
        .lock()
        .map_err(|_| {
            ScanError::new(
                "metadata_inventory_source_transfer_poisoned",
                "The retained metadata inventory source transfer became unavailable",
            )
        })
        .map(|mut source| source.take())
}

#[cfg(windows)]
fn spawn_recovery_worker(
    name: String,
    worker: impl FnOnce() + Send + 'static,
) -> std::io::Result<JoinHandle<()>> {
    #[cfg(test)]
    if FAIL_NEXT_RECOVERY_WORKER_SPAWN.with(|failure| failure.replace(false)) {
        return Err(std::io::Error::other(
            "injected recovery worker spawn failure",
        ));
    }
    thread::Builder::new().name(name).spawn(worker)
}

#[cfg(all(test, windows))]
fn fail_next_recovery_worker_spawn() {
    FAIL_NEXT_RECOVERY_WORKER_SPAWN.with(|failure| failure.set(true));
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecoveryRetryState {
    failure_count: u32,
    next_attempt_unix_ms: i64,
}

pub(crate) fn reserve_production_library_synchronization_start_ticket() -> Result<u64, ScanError> {
    #[cfg(windows)]
    {
        reserve_runtime_start_ticket(runtime_registry()).map(RuntimeStartTicket::raw)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn reserve_production_library_synchronization_stop_fence() -> Result<u64, ScanError> {
    #[cfg(windows)]
    {
        reserve_runtime_stop_fence(runtime_registry(), SYNCHRONIZATION_STOP_TIMEOUT)
            .map(RuntimeStopFence::raw)
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
        let owner_ticket = RuntimeStartTicket::decode(owner_ticket)?;
        start_runtime_with_ticket(
            runtime_registry(),
            owner_ticket,
            new_production_synchronization,
        )
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

#[cfg(all(test, windows))]
fn start_runtime_with(
    registry: &SynchronizationRuntimeRegistry,
    constructor: impl FnOnce() -> ProductionSynchronization,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let owner_ticket = reserve_runtime_start_ticket(registry)?;
    start_runtime_with_ticket(registry, owner_ticket, constructor)
}

#[cfg(windows)]
fn start_runtime_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
    constructor: impl FnOnce() -> ProductionSynchronization,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let (epoch, cancelled, runtime) =
        match claim_runtime_for_start_with_ticket(registry, owner_ticket)? {
            SynchronizationRuntimeClaim::Poll {
                epoch,
                cancelled,
                runtime,
            } => (epoch, cancelled, runtime),
            SynchronizationRuntimeClaim::Construct { epoch, cancelled } => {
                // Service identity, hashing, WinTrust and platform construction stay outside the
                // registry mutex. The epoch decides whether their late result may be installed.
                let mut candidate = match catch_unwind(AssertUnwindSafe(constructor)) {
                    Ok(candidate) => candidate,
                    Err(_) => {
                        recover_starting_runtime_panic(registry, epoch, owner_ticket)?;
                        return Err(runtime_owner_panic_error());
                    }
                };
                candidate.stop_requested = Arc::clone(&cancelled);
                match promote_constructed_runtime(
                    registry,
                    epoch,
                    owner_ticket,
                    Arc::clone(&cancelled),
                    candidate,
                )? {
                    ConstructedRuntimeDisposition::Poll(runtime) => (epoch, cancelled, runtime),
                    ConstructedRuntimeDisposition::Drain { deadline, runtime } => {
                        finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime)?;
                        return Err(ScanError::new(
                            "library_synchronization_start_cancelled",
                            "Synchronization startup completed after its epoch was stopped",
                        ));
                    }
                }
            }
        };
    execute_runtime_poll(registry, epoch, owner_ticket, cancelled, runtime)
}

pub(crate) fn poll_production_library_synchronization(
    owner_ticket: u64,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        let owner_ticket = RuntimeStartTicket::decode(owner_ticket)?;
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll_with_ticket(runtime_registry(), owner_ticket)?;
        execute_runtime_poll(runtime_registry(), epoch, owner_ticket, cancelled, runtime)
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
        let Some(owner_ticket) = first_import_runtime_owner_for_poll(runtime_registry())? else {
            return Ok(false);
        };
        let snapshot = match poll_production_library_synchronization(owner_ticket.raw()) {
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

#[cfg(windows)]
fn first_import_runtime_owner_for_poll(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<Option<RuntimeStartTicket>, ScanError> {
    let registry = lock_runtime_registry(registry)?;
    match &registry.entry {
        SynchronizationRuntimeEntry::Empty
        | SynchronizationRuntimeEntry::Starting { .. }
        | SynchronizationRuntimeEntry::Polling { .. } => Ok(None),
        SynchronizationRuntimeEntry::Ready { owner_ticket, .. } => Ok(Some(*owner_ticket)),
        SynchronizationRuntimeEntry::Stopping { .. }
        | SynchronizationRuntimeEntry::Draining { .. } => Err(ScanError::new(
            "library_synchronization_stop_in_progress",
            "Library synchronization is stopping before first-import change capture",
        )),
    }
}

pub(crate) fn stop_production_library_synchronization(
    cancellation_fence: u64,
) -> Result<(), ScanError> {
    #[cfg(windows)]
    {
        let cancellation_fence = RuntimeStopFence::decode(cancellation_fence)?;
        stop_runtime_with_fence(runtime_registry(), cancellation_fence)
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(windows)]
fn execute_runtime_poll(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: Arc<Mutex<ProductionSynchronization>>,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    execute_runtime_operation(
        registry,
        epoch,
        owner_ticket,
        cancelled,
        runtime,
        poll_runtime,
    )
}

#[cfg(windows)]
fn execute_runtime_operation<T>(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: Arc<Mutex<ProductionSynchronization>>,
    operation: impl FnOnce(&mut ProductionSynchronization) -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    let mut runtime_guard = runtime.lock().map_err(|_| {
        ScanError::new(
            "library_synchronization_state_unavailable",
            "The library synchronization runtime is unavailable",
        )
    })?;
    let result = catch_unwind(AssertUnwindSafe(|| operation(&mut runtime_guard)));
    drop(runtime_guard);
    match result {
        Ok(result) => {
            let drain_deadline =
                finish_runtime_operation(registry, epoch, owner_ticket, &cancelled, &runtime)?;
            let Some(deadline) = drain_deadline else {
                return result;
            };
            finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime)?;
            Err(ScanError::new(
                "library_synchronization_poll_cancelled",
                "A stopped synchronization epoch discarded its late poll result",
            ))
        }
        Err(_) => {
            #[cfg(test)]
            wait_before_panicked_runtime_retention(registry);
            match retain_panicked_runtime(registry, epoch, owner_ticket, &cancelled, &runtime)? {
                PanickedRuntimeRetention::Retained(deadline) => {
                    let _ =
                        finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime);
                }
                PanickedRuntimeRetention::AlreadyDrained => {}
            }
            Err(runtime_owner_panic_error())
        }
    }
}

#[cfg(all(test, windows))]
fn wait_before_panicked_runtime_retention(registry: &SynchronizationRuntimeRegistry) {
    let barrier = registry
        .panic_retention_barrier
        .lock()
        .expect("local panic retention barrier")
        .take();
    if let Some(barrier) = barrier {
        barrier.reached.wait();
        barrier.resume.wait();
    }
}

#[cfg(windows)]
fn finish_runtime_operation(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: &Arc<AtomicBool>,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<Option<Instant>, ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    match &registry_guard.entry {
        SynchronizationRuntimeEntry::Polling {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime)
            && !cancelled.load(Ordering::Acquire) =>
        {
            registry_guard.entry = SynchronizationRuntimeEntry::Ready {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(cancelled),
                runtime: Arc::clone(runtime),
            };
            drop(registry_guard);
            registry_notify_all(registry);
            Ok(None)
        }
        SynchronizationRuntimeEntry::Draining {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            deadline,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            Ok(Some(*deadline))
        }
        _ => Err(ScanError::new(
            "library_synchronization_owner_state_invalid",
            "The synchronization runtime owner no longer matches its registry epoch",
        )),
    }
}

#[cfg(windows)]
fn retain_panicked_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: &Arc<AtomicBool>,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<PanickedRuntimeRetention, ScanError> {
    cancelled.store(true, Ordering::Release);
    let mut registry_guard = lock_runtime_registry(registry)?;
    let deadline = match &registry_guard.entry {
        SynchronizationRuntimeEntry::Polling {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            cancelled: active_cancelled,
            runtime: active_runtime,
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            active_cancelled.store(true, Ordering::Release);
            Instant::now()
                .checked_add(SYNCHRONIZATION_STOP_TIMEOUT)
                .ok_or_else(stop_timeout_error)?
        }
        SynchronizationRuntimeEntry::Draining {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            deadline,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            return Ok(PanickedRuntimeRetention::Retained(*deadline));
        }
        _ => {
            let active_epoch = match &registry_guard.entry {
                SynchronizationRuntimeEntry::Empty => None,
                SynchronizationRuntimeEntry::Starting { epoch, .. }
                | SynchronizationRuntimeEntry::Ready { epoch, .. }
                | SynchronizationRuntimeEntry::Polling { epoch, .. }
                | SynchronizationRuntimeEntry::Stopping { epoch, .. }
                | SynchronizationRuntimeEntry::Draining { epoch, .. } => Some(*epoch),
            };
            if active_epoch == Some(epoch) {
                return Err(ScanError::new(
                    "library_synchronization_owner_state_invalid",
                    "A panicked synchronization owner retained an impossible same-epoch state",
                ));
            }
            if registry_guard.retired_through_epoch >= epoch {
                return Ok(PanickedRuntimeRetention::AlreadyDrained);
            }
            return Err(ScanError::new(
                "library_synchronization_owner_state_invalid",
                "A panicked synchronization owner no longer matches its registry epoch",
            ));
        }
    };
    registry_guard.entry = SynchronizationRuntimeEntry::Draining {
        epoch,
        owner_ticket,
        cancellation_fence: None,
        deadline,
        runtime: Arc::clone(runtime),
    };
    drop(registry_guard);
    registry_notify_all(registry);
    Ok(PanickedRuntimeRetention::Retained(deadline))
}

#[cfg(windows)]
fn recover_starting_runtime_panic(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
) -> Result<(), ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    if matches!(
        registry_guard.entry,
        SynchronizationRuntimeEntry::Starting {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } | SynchronizationRuntimeEntry::Stopping {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } if active_epoch == epoch && active_owner_ticket == owner_ticket
    ) {
        registry_guard.retire_epoch(epoch);
        registry_guard.entry = SynchronizationRuntimeEntry::Empty;
        drop(registry_guard);
        registry_notify_all(registry);
    }
    Ok(())
}

#[cfg(windows)]
fn runtime_owner_panic_error() -> ScanError {
    ScanError::new(
        "library_synchronization_owner_panicked",
        "The library synchronization owner panicked and entered bounded recovery",
    )
}

#[cfg(windows)]
fn finish_draining_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    deadline: Instant,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<(), ScanError> {
    {
        let registry_guard = lock_runtime_registry(registry)?;
        if !matches!(
            &registry_guard.entry,
            SynchronizationRuntimeEntry::Draining {
                epoch: active_epoch,
                owner_ticket: active_owner_ticket,
                deadline: active_deadline,
                runtime: active_runtime,
                ..
            } if *active_epoch == epoch
                && *active_owner_ticket == owner_ticket
                && *active_deadline == deadline
                && Arc::ptr_eq(active_runtime, runtime)
        ) {
            return Ok(());
        }
    }
    finish_draining_runtime_after_precheck(registry, epoch, owner_ticket, deadline, runtime)
}

#[cfg(windows)]
fn finish_draining_runtime_after_precheck(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    deadline: Instant,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<(), ScanError> {
    let mut runtime_guard = lock_draining_runtime_until(runtime, deadline)?;
    {
        let registry_guard = lock_runtime_registry(registry)?;
        if !matches!(
            &registry_guard.entry,
            SynchronizationRuntimeEntry::Draining {
                epoch: active_epoch,
                owner_ticket: active_owner_ticket,
                deadline: active_deadline,
                runtime: active_runtime,
                ..
            } if *active_epoch == epoch
                && *active_owner_ticket == owner_ticket
                && *active_deadline == deadline
                && Arc::ptr_eq(active_runtime, runtime)
        ) {
            return Ok(());
        }
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        runtime_guard.finish_stopping_until(deadline)
    }));
    match result {
        Ok(Ok(())) => {
            let mut registry_guard = lock_runtime_registry(registry)?;
            if matches!(
                &registry_guard.entry,
                SynchronizationRuntimeEntry::Draining {
                    epoch: active_epoch,
                    owner_ticket: active_owner_ticket,
                    deadline: active_deadline,
                    runtime: active_runtime,
                    ..
                } if *active_epoch == epoch
                    && *active_owner_ticket == owner_ticket
                    && *active_deadline == deadline
                    && Arc::ptr_eq(active_runtime, runtime)
            ) {
                registry_guard.retire_epoch(epoch);
                registry_guard.entry = SynchronizationRuntimeEntry::Empty;
            }
            drop(registry_guard);
            drop(runtime_guard);
            registry_notify_all(registry);
            Ok(())
        }
        Ok(Err(error)) => {
            drop(runtime_guard);
            registry_notify_all(registry);
            Err(error)
        }
        Err(_) => {
            drop(runtime_guard);
            registry_notify_all(registry);
            Err(runtime_owner_panic_error())
        }
    }
}

#[cfg(windows)]
fn lock_draining_runtime_until<'a>(
    runtime: &'a Mutex<ProductionSynchronization>,
    deadline: Instant,
) -> Result<MutexGuard<'a, ProductionSynchronization>, ScanError> {
    loop {
        match runtime.try_lock() {
            Ok(runtime) => return Ok(runtime),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ScanError::new(
                    "library_synchronization_state_unavailable",
                    "The draining synchronization runtime is unavailable",
                ));
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or_else(stop_timeout_error)?;
                thread::sleep(remaining.min(Duration::from_millis(2)));
            }
        }
    }
}

#[cfg(windows)]
fn reserve_runtime_start_ticket(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<RuntimeStartTicket, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    registry
        .allocate_lifecycle_ordinal()
        .map(RuntimeStartTicket::from_ordinal)
}

#[cfg(windows)]
fn reserve_runtime_stop_fence(
    registry: &SynchronizationRuntimeRegistry,
    timeout: Duration,
) -> Result<RuntimeStopFence, ScanError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(stop_timeout_error)?;
    reserve_runtime_stop_fence_until(registry, deadline)
}

#[cfg(windows)]
fn reserve_runtime_stop_fence_until(
    registry: &SynchronizationRuntimeRegistry,
    deadline: Instant,
) -> Result<RuntimeStopFence, ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    let next_ordinal = registry_guard.next_lifecycle_ordinal.checked_next()?;
    registry_guard.prepare_stop_fence_capacity()?;
    let cancellation_fence = RuntimeStopFence::from_ordinal(next_ordinal);
    let covered_owner = registry_guard.entry.owner_identity();
    registry_guard.next_lifecycle_ordinal = next_ordinal;
    registry_guard
        .admitted_stop_fences
        .push_back(AdmittedStopFence {
            fence: cancellation_fence,
            covered_owner,
            use_state: AdmittedStopFenceUse::Pending,
        });
    registry_guard.cancelled_through_ordinal = cancellation_fence.ordinal();
    let entry = std::mem::replace(
        &mut registry_guard.entry,
        SynchronizationRuntimeEntry::Empty,
    );
    registry_guard.entry = match entry {
        SynchronizationRuntimeEntry::Starting {
            epoch,
            owner_ticket,
            cancelled,
        } if owner_ticket.ordinal() <= cancellation_fence.ordinal() => {
            cancelled.store(true, Ordering::Release);
            SynchronizationRuntimeEntry::Stopping {
                epoch,
                owner_ticket,
                cancellation_fence,
                deadline,
            }
        }
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket,
            cancelled,
            runtime,
        }
        | SynchronizationRuntimeEntry::Polling {
            epoch,
            owner_ticket,
            cancelled,
            runtime,
        } if owner_ticket.ordinal() <= cancellation_fence.ordinal() => {
            cancelled.store(true, Ordering::Release);
            SynchronizationRuntimeEntry::Draining {
                epoch,
                owner_ticket,
                cancellation_fence: Some(cancellation_fence),
                deadline,
                runtime,
            }
        }
        SynchronizationRuntimeEntry::Stopping {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
        } => SynchronizationRuntimeEntry::Stopping {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
        },
        SynchronizationRuntimeEntry::Draining {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
            runtime,
        } => SynchronizationRuntimeEntry::Draining {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence.or(Some(cancellation_fence)),
            deadline: first_deadline,
            runtime,
        },
        entry => entry,
    };
    drop(registry_guard);
    registry_notify_all(registry);
    Ok(cancellation_fence)
}

#[cfg(windows)]
fn validate_runtime_owner_ticket(
    registry: &SynchronizationRuntimeState,
    owner_ticket: RuntimeStartTicket,
) -> Result<(), ScanError> {
    if owner_ticket.ordinal() > registry.next_lifecycle_ordinal {
        return Err(ScanError::new(
            "library_synchronization_lifecycle_ticket_invalid",
            "The synchronization lifecycle ticket was not issued by this process",
        ));
    }
    if owner_ticket.ordinal() <= registry.cancelled_through_ordinal {
        return Err(ScanError::new(
            "library_synchronization_start_cancelled",
            "The synchronization start request was cancelled before admission",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn claim_runtime_for_start_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
) -> Result<SynchronizationRuntimeClaim, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    validate_runtime_owner_ticket(&registry, owner_ticket)?;
    match std::mem::replace(&mut registry.entry, SynchronizationRuntimeEntry::Empty) {
        SynchronizationRuntimeEntry::Empty => {
            let epoch = registry.allocate_epoch()?;
            let cancelled = Arc::new(AtomicBool::new(false));
            registry.entry = SynchronizationRuntimeEntry::Starting {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
            };
            Ok(SynchronizationRuntimeClaim::Construct { epoch, cancelled })
        }
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket: active_owner_ticket,
            cancelled,
            runtime,
        } if active_owner_ticket == owner_ticket => {
            registry.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
                runtime: Arc::clone(&runtime),
            };
            Ok(SynchronizationRuntimeClaim::Poll {
                epoch,
                cancelled,
                runtime,
            })
        }
        entry => {
            let error = runtime_busy_error(&entry);
            registry.entry = entry;
            Err(error)
        }
    }
}

#[cfg(all(test, windows))]
fn claim_runtime_for_start(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<SynchronizationRuntimeClaim, ScanError> {
    let owner_ticket = reserve_runtime_start_ticket(registry)?;
    claim_runtime_for_start_with_ticket(registry, owner_ticket)
}

#[cfg(windows)]
fn promote_constructed_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: ProductionSynchronization,
) -> Result<ConstructedRuntimeDisposition, ScanError> {
    let runtime = Arc::new(Mutex::new(runtime));
    let mut registry_guard = lock_runtime_registry(registry)?;
    match &registry_guard.entry {
        SynchronizationRuntimeEntry::Starting {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && !cancelled.load(Ordering::Acquire) =>
        {
            registry_guard.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled,
                runtime: Arc::clone(&runtime),
            };
            Ok(ConstructedRuntimeDisposition::Poll(runtime))
        }
        SynchronizationRuntimeEntry::Stopping {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            cancellation_fence,
            deadline,
        } if *active_epoch == epoch && *active_owner_ticket == owner_ticket => {
            let deadline = *deadline;
            let cancellation_fence = *cancellation_fence;
            registry_guard.entry = SynchronizationRuntimeEntry::Draining {
                epoch,
                owner_ticket,
                cancellation_fence: Some(cancellation_fence),
                deadline,
                runtime: Arc::clone(&runtime),
            };
            drop(registry_guard);
            registry_notify_all(registry);
            Ok(ConstructedRuntimeDisposition::Drain { deadline, runtime })
        }
        _ => Err(ScanError::new(
            "library_synchronization_owner_state_invalid",
            "A constructed synchronization runtime no longer matches its registry epoch",
        )),
    }
}

#[cfg(windows)]
fn claim_runtime_for_poll_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
) -> Result<RuntimePollClaim, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    match std::mem::replace(&mut registry.entry, SynchronizationRuntimeEntry::Empty) {
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket: active_owner_ticket,
            cancelled,
            runtime,
        } if active_owner_ticket == owner_ticket
            && owner_ticket.ordinal() > registry.cancelled_through_ordinal =>
        {
            registry.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
                runtime: Arc::clone(&runtime),
            };
            Ok((epoch, cancelled, runtime))
        }
        SynchronizationRuntimeEntry::Empty => {
            registry.entry = SynchronizationRuntimeEntry::Empty;
            Err(ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            ))
        }
        entry => {
            if entry.owner_ticket() != Some(owner_ticket) {
                registry.entry = entry;
                return Err(ScanError::new(
                    "library_synchronization_lifecycle_ticket_stale",
                    "The synchronization poll request does not own the active runtime",
                ));
            }
            let error = runtime_busy_error(&entry);
            registry.entry = entry;
            Err(error)
        }
    }
}

#[cfg(all(test, windows))]
fn claim_runtime_for_poll(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<RuntimePollClaim, ScanError> {
    let owner_ticket = lock_runtime_registry(registry)?
        .entry
        .owner_ticket()
        .ok_or_else(|| {
            ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            )
        })?;
    claim_runtime_for_poll_with_ticket(registry, owner_ticket)
}

#[cfg(all(test, windows))]
fn stop_runtime(registry: &SynchronizationRuntimeRegistry) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence(registry, SYNCHRONIZATION_STOP_TIMEOUT)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

#[cfg(all(test, windows))]
fn stop_runtime_with_timeout(
    registry: &SynchronizationRuntimeRegistry,
    timeout: Duration,
) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence(registry, timeout)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

#[cfg(all(test, windows))]
fn stop_runtime_until(
    registry: &SynchronizationRuntimeRegistry,
    requested_deadline: Instant,
) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence_until(registry, requested_deadline)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

#[cfg(windows)]
fn stop_runtime_with_fence(
    registry: &SynchronizationRuntimeRegistry,
    cancellation_fence: RuntimeStopFence,
) -> Result<(), ScanError> {
    lock_runtime_registry(registry)?.consume_admitted_stop_fence(cancellation_fence)?;
    loop {
        let draining = {
            let registry_guard = lock_runtime_registry(registry)?;
            if cancellation_fence.ordinal() > registry_guard.next_lifecycle_ordinal
                || cancellation_fence.ordinal() > registry_guard.cancelled_through_ordinal
            {
                return Err(invalid_stop_fence_error());
            }
            match &registry_guard.entry {
                SynchronizationRuntimeEntry::Empty => return Ok(()),
                entry
                    if entry
                        .owner_ticket()
                        .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal()) =>
                {
                    return Ok(());
                }
                SynchronizationRuntimeEntry::Draining {
                    epoch,
                    owner_ticket,
                    deadline,
                    runtime,
                    ..
                } => Some((*epoch, *owner_ticket, *deadline, Arc::clone(runtime))),
                SynchronizationRuntimeEntry::Stopping { .. } => None,
                SynchronizationRuntimeEntry::Starting { .. }
                | SynchronizationRuntimeEntry::Ready { .. }
                | SynchronizationRuntimeEntry::Polling { .. } => {
                    return Err(ScanError::new(
                        "library_synchronization_owner_state_invalid",
                        "The synchronization stop fence did not linearize its matching owner",
                    ));
                }
            }
        };
        if let Some((epoch, owner_ticket, deadline, runtime)) = draining {
            return finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime);
        }

        let registry_guard = lock_runtime_registry(registry)?;
        if matches!(registry_guard.entry, SynchronizationRuntimeEntry::Empty)
            || registry_guard
                .entry
                .owner_ticket()
                .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal())
        {
            return Ok(());
        }
        let deadline = match registry_guard.entry {
            SynchronizationRuntimeEntry::Stopping { deadline, .. }
            | SynchronizationRuntimeEntry::Draining { deadline, .. } => deadline,
            SynchronizationRuntimeEntry::Empty => return Ok(()),
            _ => {
                return Err(ScanError::new(
                    "library_synchronization_owner_state_invalid",
                    "The synchronization stop fence lost its admitted owner",
                ));
            }
        };
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(stop_timeout_error)?;
        let (registry_guard, wait) = registry
            .changed
            .wait_timeout(registry_guard, remaining)
            .map_err(|_| {
                ScanError::new(
                    "library_synchronization_state_unavailable",
                    "The library synchronization runtime state is unavailable",
                )
            })?;
        if wait.timed_out()
            && !matches!(registry_guard.entry, SynchronizationRuntimeEntry::Empty)
            && !registry_guard
                .entry
                .owner_ticket()
                .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal())
        {
            return Err(stop_timeout_error());
        }
    }
}

#[cfg(windows)]
fn stop_timeout_error() -> ScanError {
    ScanError::new(
        "library_synchronization_stop_timeout",
        "Synchronization workers did not stop within the bounded shutdown window",
    )
}

#[cfg(windows)]
fn invalid_stop_fence_error() -> ScanError {
    ScanError::new(
        "library_synchronization_lifecycle_fence_invalid",
        "The synchronization stop fence was not admitted by this process",
    )
}

#[cfg(windows)]
fn registry_notify_all(registry: &SynchronizationRuntimeRegistry) {
    registry.changed.notify_all();
}

#[cfg(windows)]
fn runtime_busy_error(entry: &SynchronizationRuntimeEntry) -> ScanError {
    let (code, message) = match entry {
        SynchronizationRuntimeEntry::Starting { .. } => (
            "library_synchronization_start_in_progress",
            "Synchronization startup is already in progress",
        ),
        SynchronizationRuntimeEntry::Polling { .. } => (
            "library_synchronization_poll_in_progress",
            "Synchronization polling is already in progress",
        ),
        SynchronizationRuntimeEntry::Stopping { .. }
        | SynchronizationRuntimeEntry::Draining { .. } => (
            "library_synchronization_stop_in_progress",
            "The prior synchronization epoch is still stopping",
        ),
        SynchronizationRuntimeEntry::Empty | SynchronizationRuntimeEntry::Ready { .. } => (
            "library_synchronization_state_raced",
            "The synchronization runtime changed while acquiring ownership",
        ),
    };
    ScanError::new(code, message)
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
    runtime.prune_recovery_inventory_sources(catalog)?;
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
    let allows_broker_activation = production_client_allows_broker_activation();
    let factory = production_persistent_change_journal();
    new_production_synchronization_with_factory(
        production_library_change_source_factory(),
        factory,
        allows_broker_activation,
    )
}

#[cfg(windows)]
fn new_production_synchronization_with_factory(
    start_source: LibraryChangeSourceStarter,
    factory: Arc<dyn PersistentChangeJournal>,
    allows_broker_activation: bool,
) -> ProductionSynchronization {
    let (persistent_change_journal_factory, persistent_change_journal, journal_opened) =
        if allows_broker_activation {
            (
                Some(Arc::clone(&factory)),
                PersistentChangeJournalConnection::LiveOnly(
                    PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
                ),
                false,
            )
        } else {
            (
                None,
                PersistentChangeJournalConnection::LiveOnly(
                    PersistentChangeJournalLiveOnlyReason::PortableDistribution,
                ),
                true,
            )
        };
    ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_production(start_source),
        catalog_session: None,
        poll_catalog: PollCatalogOwner::default(),
        persistent_change_journal_factory,
        _persistent_change_journal: persistent_change_journal,
        persistent_change_journal_opened: journal_opened,
        persistent_change_journal_caller: None,
        journal: None,
        journal_volume_cursor: None,
        journal_root_cursor: None,
        journal_next_action_is_read: true,
        live: None,
        live_root_cursor: None,
        recovery: None,
        recovery_inventory_sources: BTreeMap::new(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_retries: BTreeMap::new(),
        authoritative_root_cursor: None,
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
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
        journal_volume_cursor: None,
        journal_root_cursor: None,
        journal_next_action_is_read: true,
        live: None,
        live_root_cursor: None,
        recovery: None,
        recovery_inventory_sources: BTreeMap::new(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_retries: BTreeMap::new(),
        authoritative_root_cursor: None,
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
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
        &self,
        catalog: &SqliteCatalog,
        snapshot: &mut LibrarySynchronizationSnapshot,
    ) -> Result<(), ScanError> {
        let capabilities = catalog.load_persistent_journal_capabilities()?;
        for status in &mut snapshot.roots {
            if status.availability != crate::domain::LibraryRootAvailability::Available {
                status.continuity = PersistentJournalContinuityState::Unavailable;
                continue;
            }
            status.continuity = match self._persistent_change_journal {
                PersistentChangeJournalConnection::LiveOnly(_) => {
                    PersistentJournalContinuityState::LiveOnly
                }
                PersistentChangeJournalConnection::Connected(_) => capabilities
                    .iter()
                    .find(|capability| {
                        capability.root_id == status.root_id
                            && capability.root_generation.value() == status.root_generation
                    })
                    .map_or(
                        PersistentJournalContinuityState::BaselineRequired,
                        |capability| capability.continuity,
                    ),
            };
        }
        Ok(())
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

    #[allow(
        dead_code,
        reason = "R2c-O owns the reconnect seam before R2c-P checkpoint policy invokes it"
    )]
    fn reconnect_persistent_change_journal(
        &mut self,
    ) -> Result<(), PersistentChangeJournalOperationError> {
        let factory = self
            .persistent_change_journal_factory
            .as_ref()
            .ok_or(PersistentChangeJournalOperationError::TransportUnavailable)?;
        let previous = std::mem::replace(
            &mut self._persistent_change_journal,
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
            ),
        );
        let close_result = previous.close();
        self._persistent_change_journal = factory.connect();
        if matches!(
            self._persistent_change_journal,
            PersistentChangeJournalConnection::Connected(_)
        ) && self.persistent_change_journal_caller.is_none()
        {
            self.persistent_change_journal_caller =
                production_persistent_journal_caller_claim().ok();
        }
        self.persistent_change_journal_opened = true;
        close_result
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
            RecoveryTaskKind::CandidateDrain { .. } => LibrarySynchronizationPhase::Reconciliation,
            RecoveryTaskKind::LegacyUnownedDrain { .. } => {
                LibrarySynchronizationPhase::Reconciliation
            }
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
enum ReadyRecoveryWork {
    CandidateDrain {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
    LegacyUnownedDrain {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
    Control {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
}

#[cfg(windows)]
impl ReadyRecoveryWork {
    fn root_id(&self) -> &str {
        match self {
            Self::CandidateDrain { root_id, .. }
            | Self::LegacyUnownedDrain { root_id, .. }
            | Self::Control { root_id, .. } => root_id,
        }
    }

    const fn root_generation(&self) -> LibraryRootGeneration {
        match self {
            Self::CandidateDrain {
                root_generation, ..
            }
            | Self::LegacyUnownedDrain {
                root_generation, ..
            }
            | Self::Control {
                root_generation, ..
            } => *root_generation,
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
    let journal_capacity_debt_roots =
        runtime.journal_page_capacity_debt_roots(catalog, snapshot)?;
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
            || root.recovery_blocked
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
        if !journal_capacity_debt_roots.is_empty()
            && !journal_capacity_debt_roots
                .iter()
                .any(|(root_id, generation)| {
                    root_id == &root.root_id && *generation == root_generation
                })
        {
            continue;
        }
        if catalog.has_ready_metadata_inventory_recovery_candidates(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? {
            runtime.authoritative_root_cursor = Some(root.root_id.clone());
            return Ok(Some(ReadyRecoveryWork::CandidateDrain {
                root_id: root.root_id.clone(),
                root_generation,
            }));
        }
        if catalog.has_ready_legacy_unowned_recovery_debt(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? {
            runtime.authoritative_root_cursor = Some(root.root_id.clone());
            return Ok(Some(ReadyRecoveryWork::LegacyUnownedDrain {
                root_id: root.root_id.clone(),
                root_generation,
            }));
        }
        if !journal_capacity_debt_roots.is_empty() {
            continue;
        }
        if catalog
            .has_unresolved_metadata_inventory_recovery_candidates(&root.root_id, root_generation)?
        {
            continue;
        }
        if catalog.has_ready_metadata_inventory_recovery(
            &root.root_id,
            root_generation,
            now_unix_ms,
            runtime.runtime.queue_policy(),
        )? {
            runtime.authoritative_root_cursor = Some(root.root_id.clone());
            return Ok(Some(ReadyRecoveryWork::Control {
                root_id: root.root_id.clone(),
                root_generation,
            }));
        }
    }
    Ok(None)
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
fn runtime_registry() -> &'static SynchronizationRuntimeRegistry {
    SYNCHRONIZATION_RUNTIME.get_or_init(SynchronizationRuntimeRegistry::default)
}

#[cfg(windows)]
fn lock_runtime_registry(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<MutexGuard<'_, SynchronizationRuntimeState>, ScanError> {
    registry.state.lock().map_err(|_| {
        ScanError::new(
            "library_synchronization_state_unavailable",
            "The library synchronization runtime state is unavailable",
        )
    })
}

#[cfg(windows)]
impl ProductionSynchronization {
    fn journal_page_capacity_debt_roots(
        &self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
    ) -> Result<Vec<(String, LibraryRootGeneration)>, ScanError> {
        if !self.journal_next_action_is_read
            || !matches!(
                self._persistent_change_journal,
                PersistentChangeJournalConnection::Connected(_)
            )
            || self.persistent_change_journal_caller.is_none()
        {
            return Ok(Vec::new());
        }
        let Some(work) = self.next_journal_volume(catalog, snapshot)? else {
            return Ok(Vec::new());
        };
        let mut roots = Vec::new();
        for root in work.roots {
            if !catalog.journal_admission_has_headroom(
                &root.root_id,
                root.root_generation,
                self.runtime.queue_policy(),
            )? {
                roots.push((root.root_id, root.root_generation));
            }
        }
        Ok(roots)
    }

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
        self.start_live_work(
            root_id,
            root_generation,
            now_unix_ms,
            storage.catalog_path.clone(),
        )
    }

    fn start_live_work(
        &mut self,
        root_id: String,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        if self.live.is_some() {
            return Err(ScanError::new(
                "live_reconciliation_already_running",
                "Another P0 live worker already owns the reserved slot",
            ));
        }
        let queue_policy = self.runtime.queue_policy();
        let recovery_policy = self.runtime.recovery_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p0-live-reconciliation".to_owned())
            .spawn(move || {
                let catalog = catalog_session.open_in_lane(LibraryChangeLane::Live);
                let result = catalog.and_then(|mut catalog| {
                    let Some(root) = catalog.load_incremental_catalog_root(&worker_root_id)? else {
                        return Ok(AuthoritativeLibraryChangeReport::default());
                    };
                    if root.root_generation != root_generation || root.active_scan_id.is_none() {
                        return Ok(AuthoritativeLibraryChangeReport::default());
                    }
                    if let Some(leased) = catalog.lease_live_authoritative_library_change(
                        &worker_root_id,
                        root_generation,
                        now_unix_ms,
                        queue_policy,
                    )? {
                        #[cfg(test)]
                        pause_live_worker_after_lease(&worker_root_id);
                        process_leased_authoritative_library_change_cancellable(
                            &mut catalog,
                            &root,
                            &leased,
                            now_unix_ms,
                            queue_policy,
                            recovery_policy,
                            &worker_cancelled,
                        )
                    } else {
                        process_ready_library_changes_in_lane_cancellable(
                            &mut catalog,
                            &worker_root_id,
                            root_generation,
                            LibraryChangeLane::Live,
                            now_unix_ms,
                            queue_policy,
                            &worker_cancelled,
                        )
                        .map(|incremental| AuthoritativeLibraryChangeReport { incremental })
                    }
                });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "live_reconciliation_worker_start_failed",
                    format!("Could not start the reserved P0 worker: {error}"),
                )
            })?;
        self.live = Some(LiveTask {
            root_id,
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn poll_live(&mut self, now_unix_ms: i64) -> u32 {
        let Some(task) = self.live.as_mut() else {
            return 0;
        };
        let result = match task.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ScanError::new(
                "live_reconciliation_worker_disconnected",
                "The reserved P0 worker stopped without a result",
            ))),
        };
        let Some(result) = result else {
            return 0;
        };
        let root_id = task.root_id.clone();
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        self.live = None;
        match result {
            Ok(report) => report.incremental.applied_mutation_count,
            Err(error) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] live worker failed code={} message={}",
                    error.code,
                    one_line_message(&error.message),
                );
                self.runtime
                    .record_recovery_failure(&root_id, &error.code, now_unix_ms);
                0
            }
        }
    }

    fn poll_journal(&mut self) -> u32 {
        let Some(task) = self.journal.as_mut() else {
            return 0;
        };
        let result = match task.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ScanError::new(
                "persistent_journal_worker_disconnected",
                "The persistent journal worker stopped without a result",
            ))),
        };
        let Some(result) = result else {
            return 0;
        };
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        self.journal = None;
        match result {
            Ok(JournalTaskOutcome::Page(_report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] journal page finished roots={} failed={} observations={} checkpoints={}",
                    _report.enrolled_root_count,
                    _report.failed_root_count,
                    _report.observation_count,
                    _report.advanced_checkpoint_count,
                );
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
        if let Some((root_id, root_generation)) =
            self.next_journal_root(catalog, snapshot, now_unix_ms)?
        {
            self.journal_root_cursor = Some(root_id.clone());
            self.journal_next_action_is_read = true;
            return self.start_journal_drain(
                root_id,
                root_generation,
                now_unix_ms,
                storage.catalog_path.clone(),
            );
        }
        let (session, caller) = match (
            &self._persistent_change_journal,
            self.persistent_change_journal_caller.as_ref(),
        ) {
            (PersistentChangeJournalConnection::Connected(session), Some(caller)) => {
                (Arc::clone(session), caller.clone())
            }
            (PersistentChangeJournalConnection::LiveOnly(_), _) => {
                if let Some(work) = super::journal_baseline::select_opening_work(
                    catalog,
                    snapshot,
                    self.journal_root_cursor.as_deref(),
                )? {
                    super::journal_baseline::persist_unavailable_session(
                        catalog,
                        &work,
                        now_unix_ms,
                    )?;
                }
                return Ok(());
            }
            _ => return Ok(()),
        };

        if let Some(work) = super::journal_baseline::select_closing_work(
            catalog,
            snapshot,
            self.journal_root_cursor.as_deref(),
        )? {
            self.journal_root_cursor = Some(work.root_id().to_owned());
            return self.start_baseline_closing(
                work,
                session,
                caller,
                now_unix_ms,
                storage.catalog_path.clone(),
            );
        }
        if let Some(work) = super::journal_baseline::select_opening_work(
            catalog,
            snapshot,
            self.journal_root_cursor.as_deref(),
        )? {
            self.journal_root_cursor = Some(work.root_id().to_owned());
            return self.start_baseline_opening(
                work,
                session,
                caller,
                now_unix_ms,
                storage.catalog_path.clone(),
            );
        }

        if self.journal_next_action_is_read
            && let Some(work) = self.next_journal_volume(catalog, snapshot)?
        {
            for root in &work.roots {
                if !catalog.journal_admission_has_headroom(
                    &root.root_id,
                    root.root_generation,
                    self.runtime.queue_policy(),
                )? {
                    return Ok(());
                }
            }
            self.journal_volume_cursor = Some(work.key.clone());
            self.journal_next_action_is_read = false;
            return self.start_journal_page(
                work,
                session,
                caller,
                now_unix_ms,
                storage.catalog_path.clone(),
            );
        }
        self.journal_next_action_is_read = true;
        Ok(())
    }

    fn next_journal_volume(
        &self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
    ) -> Result<Option<JournalVolumeWork>, ScanError> {
        let capabilities = catalog.load_persistent_journal_capabilities()?;
        let roots = catalog.load_incremental_catalog_roots()?;
        let mut volumes = BTreeMap::<String, JournalVolumeWork>::new();
        for root in roots {
            let Some(status) = snapshot.roots.iter().find(|status| {
                status.root_id == root.root_id
                    && status.availability == crate::domain::LibraryRootAvailability::Available
                    && status.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
            }) else {
                continue;
            };
            let generation =
                LibraryRootGeneration::new(status.root_generation).ok_or_else(|| {
                    ScanError::new(
                        "library_root_generation_invalid",
                        "The synchronization root generation is invalid",
                    )
                })?;
            if !capabilities.iter().any(|capability| {
                capability.root_id == root.root_id
                    && capability.root_generation == generation
                    && capability.state == PersistentJournalCapabilityState::Supported
                    && matches!(
                        capability.continuity,
                        PersistentJournalContinuityState::Current
                            | PersistentJournalContinuityState::CatchingUp
                    )
            }) {
                continue;
            }
            let Some(checkpoint) =
                catalog.load_persistent_journal_checkpoint(&root.root_id, generation)?
            else {
                continue;
            };
            if !matches!(
                checkpoint.continuity,
                PersistentJournalContinuityState::Current
                    | PersistentJournalContinuityState::CatchingUp
            ) || checkpoint.failure.is_some()
            {
                continue;
            }
            let base_key = format!(
                "{}|{}|{}",
                checkpoint.volume.volume_guid,
                checkpoint.volume.volume_serial,
                checkpoint.journal_id.value(),
            );
            let prefix = format!("{base_key}|");
            let last_chunk = volumes
                .range(format!("{base_key}|")..)
                .take_while(|(key, _)| key.starts_with(&prefix))
                .last()
                .map(|(key, work)| {
                    (
                        key.rsplit('|')
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .unwrap_or(0),
                        work.roots.len(),
                    )
                });
            let chunk_index = last_chunk.map_or(0, |(index, root_count)| {
                if root_count >= 8 {
                    index.saturating_add(1)
                } else {
                    index
                }
            });
            let key = format!("{base_key}|{chunk_index}");
            let volume = volumes
                .entry(key.clone())
                .or_insert_with(|| JournalVolumeWork {
                    key,
                    checkpoints: Vec::new(),
                    roots: Vec::new(),
                });
            volume.checkpoints.push(checkpoint);
            volume.roots.push(JournalRootWork {
                root_id: root.root_id,
                root_generation: generation,
                root_path: std::path::PathBuf::from(root.root_path),
            });
        }
        let Some(key) = select_rotated_key(volumes.keys(), self.journal_volume_cursor.as_deref())
        else {
            return Ok(None);
        };
        Ok(volumes.remove(&key))
    }

    fn next_journal_root(
        &self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
    ) -> Result<Option<(String, LibraryRootGeneration)>, ScanError> {
        let mut eligible = Vec::new();
        for root in &snapshot.roots {
            if root.availability != crate::domain::LibraryRootAvailability::Available
                || root.source_health != crate::domain::LibraryChangeSourceHealth::Healthy
            {
                continue;
            }
            let generation = LibraryRootGeneration::new(root.root_generation).ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
            if catalog.has_ready_journal_path_library_change(
                &root.root_id,
                generation,
                now_unix_ms,
                self.runtime.queue_policy(),
            )? {
                eligible.push(root.root_id.clone());
            }
        }
        let Some(root_id) =
            select_rotated_key(eligible.iter(), self.journal_root_cursor.as_deref())
        else {
            return Ok(None);
        };
        let root = snapshot
            .roots
            .iter()
            .find(|root| root.root_id == root_id)
            .expect("selected journal root comes from the snapshot");
        let generation = LibraryRootGeneration::new(root.root_generation).ok_or_else(|| {
            ScanError::new(
                "library_root_generation_invalid",
                "The synchronization root generation is invalid",
            )
        })?;
        Ok(Some((root_id, generation)))
    }

    fn start_baseline_opening(
        &mut self,
        work: super::journal_baseline::JournalBaselineOpeningWork,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        let queue_policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-baseline-opening".to_owned())
            .spawn(move || {
                let result = super::journal_baseline::capture_opening_boundary(
                    work,
                    session.as_ref(),
                    &caller,
                    observed_unix_ms,
                    catalog_session.as_ref(),
                    queue_policy,
                    worker_cancelled.as_ref(),
                )
                .map(|()| JournalTaskOutcome::BaselineBoundary);
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the opening boundary worker: {error}"),
                )
            })?;
        self.journal = Some(JournalTask {
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn start_baseline_closing(
        &mut self,
        work: super::journal_baseline::JournalBaselineClosingWork,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-baseline-closing".to_owned())
            .spawn(move || {
                let result = super::journal_baseline::capture_closing_boundary(
                    work,
                    session.as_ref(),
                    &caller,
                    observed_unix_ms,
                    catalog_session.as_ref(),
                    worker_cancelled.as_ref(),
                )
                .map(|()| JournalTaskOutcome::BaselineBoundary);
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the closing boundary worker: {error}"),
                )
            })?;
        self.journal = Some(JournalTask {
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn start_journal_page(
        &mut self,
        work: JournalVolumeWork,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        let policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-journal-page".to_owned())
            .spawn(move || {
                let result = (|| {
                    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
                    let mut report = PersistentJournalEnrollmentReport::default();
                    let mut registrations = Vec::<ProductionPersistentJournalRoot>::new();
                    let mut broker_roots = Vec::new();
                    let mut checkpoints = Vec::new();
                    for (root, checkpoint) in work.roots.into_iter().zip(work.checkpoints) {
                        let registration = match describe_production_persistent_journal_root(
                            &root.root_id,
                            root.root_generation.value(),
                            &root.root_path,
                        ) {
                            Ok(registration) => registration,
                            Err(error) => {
                                let failure = persist_journal_registration_failure(
                                    &mut catalog,
                                    &root.root_id,
                                    root.root_generation,
                                    error,
                                    observed_unix_ms,
                                    policy,
                                )?;
                                report.failed_root_count =
                                    report.failed_root_count.saturating_add(1);
                                report.root_failures.push(failure);
                                continue;
                            }
                        };
                        broker_roots.push(PersistentJournalBrokerRoot {
                            authorization: registration.authorization.clone(),
                            client_root_handle: registration.client_root_handle,
                            volume_serial: registration.volume_serial,
                        });
                        registrations.push(registration);
                        checkpoints.push(checkpoint);
                    }
                    if checkpoints.is_empty() {
                        return Ok(JournalTaskOutcome::Page(report));
                    }
                    let reader = SessionBackedPersistentJournalVolumeReader::with_retained_session(
                        session,
                        caller,
                        broker_roots,
                    )?;
                    let healthy_report = catch_up_persistent_journal_volume(
                        &mut catalog,
                        &reader,
                        &checkpoints,
                        observed_unix_ms,
                        policy,
                        &worker_cancelled,
                    )?;
                    report.enrolled_root_count = report
                        .enrolled_root_count
                        .saturating_add(healthy_report.enrolled_root_count);
                    report.failed_root_count = report
                        .failed_root_count
                        .saturating_add(healthy_report.failed_root_count);
                    report.observation_count = report
                        .observation_count
                        .saturating_add(healthy_report.observation_count);
                    report.advanced_checkpoint_count = report
                        .advanced_checkpoint_count
                        .saturating_add(healthy_report.advanced_checkpoint_count);
                    report.root_failures.extend(healthy_report.root_failures);
                    drop(registrations);
                    Ok(JournalTaskOutcome::Page(report))
                })();
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the persistent journal page worker: {error}"),
                )
            })?;
        self.journal = Some(JournalTask {
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn start_journal_drain(
        &mut self,
        root_id: String,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        catalog_path: std::path::PathBuf,
    ) -> Result<(), ScanError> {
        let policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-journal-drain".to_owned())
            .spawn(move || {
                let result = catalog_session
                    .open_in_lane(LibraryChangeLane::Journal)
                    .and_then(|mut catalog| {
                        process_ready_library_changes_in_lane_cancellable(
                            &mut catalog,
                            &root_id,
                            root_generation,
                            LibraryChangeLane::Journal,
                            now_unix_ms,
                            policy,
                            &worker_cancelled,
                        )
                        .map(JournalTaskOutcome::Drain)
                    });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the persistent journal drain worker: {error}"),
                )
            })?;
        self.journal = Some(JournalTask {
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn prune_recovery_inventory_sources(
        &mut self,
        catalog: &SqliteCatalog,
    ) -> Result<(), ScanError> {
        let mut stale_change_ids = Vec::new();
        for (change_id, source) in &self.recovery_inventory_sources {
            let is_current = catalog
                .load_metadata_inventory_recovery_authority(*change_id)?
                .is_some_and(|authority| {
                    authority.retired_unix_ms.is_none() && authority.run_id == source.run_id()
                });
            if !is_current {
                stale_change_ids.push(*change_id);
            }
        }
        for change_id in stale_change_ids {
            self.recovery_inventory_sources.remove(&change_id);
        }
        Ok(())
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
        let metadata_inventory_change_id = match &task.kind {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => Some(*change_id),
            RecoveryTaskKind::BoundedAuthoritative { .. }
            | RecoveryTaskKind::CandidateDrain { .. }
            | RecoveryTaskKind::LegacyUnownedDrain { .. } => None,
        };
        #[cfg(debug_assertions)]
        let recovery_kind = match &task.kind {
            RecoveryTaskKind::BoundedAuthoritative { .. } => "bounded-authoritative",
            RecoveryTaskKind::MetadataInventory { .. } => "metadata-inventory",
            RecoveryTaskKind::CandidateDrain { .. } => "candidate-drain",
            RecoveryTaskKind::LegacyUnownedDrain { .. } => "legacy-unowned-drain",
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
                if let Some(source) = retained_source {
                    self.recovery_inventory_sources.insert(change_id, source);
                } else {
                    self.recovery_inventory_sources.remove(&change_id);
                }
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
                self.recovery_retries.remove(&root_id);
                Ok(report.incremental.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::CandidateDrain(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_retries.remove(&root_id);
                Ok(report.applied_mutation_count)
            }
            Ok(RecoveryTaskOutcome::LegacyUnownedDrain(report)) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Ame sync] recovery finished kind={recovery_kind} root={root_id} result=ok mutations={}",
                    report.applied_mutation_count,
                );
                self.runtime.acknowledge_recovery_success(&root_id);
                self.recovery_retries.remove(&root_id);
                Ok(report.applied_mutation_count)
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
        let change_id = leased.change.id;
        let task_kind = if is_inventory {
            RecoveryTaskKind::MetadataInventory {
                continuity_revision,
                change_id,
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
        let metadata_inventory_page_entries = self.metadata_inventory_page_entries;
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
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let retained_inventory_source = Arc::new(Mutex::new(if is_inventory {
            self.recovery_inventory_sources.remove(&change_id)
        } else {
            None
        }));
        let worker_retained_inventory_source = Arc::clone(&retained_inventory_source);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = spawn_recovery_worker(format!("ame-{recovery_kind}-recovery"), move || {
            let result = take_transferred_inventory_source(&worker_retained_inventory_source)
                .and_then(|retained_inventory_source| {
                    catalog_session
                        .open_in_lane(LibraryChangeLane::Recovery)
                        .and_then(|mut catalog| {
                            let Some(root) =
                                catalog.load_incremental_catalog_root(&worker_root_id)?
                            else {
                                return Ok(if is_inventory {
                                    RecoveryTaskOutcome::MetadataInventory(Box::new(
                                        MetadataInventoryRecoveryPage {
                                            report: MetadataInventoryRecoveryReport::default(),
                                            retained_source: None,
                                        },
                                    ))
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
                                    RecoveryTaskOutcome::MetadataInventory(Box::new(
                                        MetadataInventoryRecoveryPage {
                                            report: MetadataInventoryRecoveryReport {
                                                incremental: deferred.incremental,
                                                ..MetadataInventoryRecoveryReport::default()
                                            },
                                            retained_source: None,
                                        },
                                    ))
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
                                let yield_page = |_phase| thread::yield_now();
                                process_leased_metadata_inventory_change_with_retained_source(
                                &mut catalog,
                                &root,
                                &leased,
                                MetadataInventoryRecoveryExecution::new(
                                    now_unix_ms,
                                    metadata_inventory_page_entries,
                                    queue_policy,
                                    MetadataInventoryWorkerControl::with_progress_and_page_yield(
                                        &worker_cancelled,
                                        &report_progress,
                                        &yield_page,
                                    ),
                                ),
                                retained_inventory_source,
                            )
                            .map(|page| RecoveryTaskOutcome::MetadataInventory(Box::new(page)))
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
                        })
                });
            let _ = sender.send(result);
        });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                if let Some(source) = take_transferred_inventory_source(&retained_inventory_source)?
                {
                    self.recovery_inventory_sources.insert(change_id, source);
                }
                return Err(ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start {recovery_kind} recovery: {error}"),
                ));
            }
        };
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
        let policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let phase = Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation));
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p2-candidate-drain".to_owned())
            .spawn(move || {
                let result = if worker_cancelled.load(Ordering::Acquire) {
                    Ok(RecoveryTaskOutcome::CandidateDrain(
                        IncrementalLibraryChangeReport::default(),
                    ))
                } else {
                    catalog_session
                        .open_in_lane(LibraryChangeLane::Recovery)
                        .and_then(|mut catalog| {
                            process_ready_metadata_inventory_recovery_candidates_cancellable(
                                &mut catalog,
                                &worker_root_id,
                                root_generation,
                                now_unix_ms,
                                policy,
                                &worker_cancelled,
                            )
                            .map(RecoveryTaskOutcome::CandidateDrain)
                        })
                };
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start the P2 candidate drain page: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id,
            kind: RecoveryTaskKind::CandidateDrain {
                continuity_revision,
            },
            phase,
            cancelled,
            receiver,
            worker: Some(worker),
        });
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
        let policy = self.runtime.queue_policy();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let phase = Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation));
        let catalog_session = self.validated_catalog_session(&catalog_path)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p2-legacy-unowned-drain".to_owned())
            .spawn(move || {
                let result = if worker_cancelled.load(Ordering::Acquire) {
                    Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                        IncrementalLibraryChangeReport::default(),
                    ))
                } else {
                    catalog_session
                        .open_in_lane(LibraryChangeLane::Recovery)
                        .and_then(|mut catalog| {
                            let compacted = catalog.compact_legacy_unowned_recovery_controls(
                                &worker_root_id,
                                root_generation,
                                now_unix_ms,
                                policy,
                            )?;
                            let mut report = process_ready_unowned_recovery_paths_cancellable(
                                &mut catalog,
                                &worker_root_id,
                                root_generation,
                                now_unix_ms,
                                policy,
                                &worker_cancelled,
                            )?;
                            report.superseded_count = report
                                .superseded_count
                                .checked_add(compacted)
                                .ok_or_else(|| {
                                    ScanError::new(
                                        "library_change_count_overflow",
                                        "The compacted legacy recovery count overflowed",
                                    )
                                })?;
                            Ok(RecoveryTaskOutcome::LegacyUnownedDrain(report))
                        })
                };
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "authoritative_recovery_worker_start_failed",
                    format!("Could not start the legacy unowned P2 drain page: {error}"),
                )
            })?;
        self.recovery = Some(RecoveryTask {
            root_id,
            kind: RecoveryTaskKind::LegacyUnownedDrain {
                continuity_revision,
            },
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
                ..
            }
            | RecoveryTaskKind::CandidateDrain {
                continuity_revision,
            }
            | RecoveryTaskKind::LegacyUnownedDrain {
                continuity_revision,
            } => continuity_revision,
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
        assert!(!should_panic, "controlled draining owner panic");
    }

    fn request_stop(&mut self) -> Result<(), ScanError> {
        if self.is_stopping {
            return Ok(());
        }
        self.is_stopping = true;
        self.stop_requested.store(true, Ordering::Release);
        if let Some(task) = &self.live {
            task.cancelled.store(true, Ordering::Release);
        }
        if let Some(task) = &self.journal {
            task.cancelled.store(true, Ordering::Release);
        }
        if let Some(task) = &self.recovery {
            task.cancelled.store(true, Ordering::Release);
        }
        self.inventory_cleanup.request_stop();
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
        task.cancelled.store(true, Ordering::Release);
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

    fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        #[cfg(test)]
        self.maybe_panic_while_draining(DrainPanicPoint::RequestStop);
        self.request_stop()?;
        if let Some(task) = self.live.as_mut() {
            finish_runtime_task_until(
                &task.receiver,
                &mut task.worker,
                deadline,
                "live_reconciliation_stop_timeout",
                "P0 reconciliation did not stop within the bounded shutdown window",
            )?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P0Result);
            self.live = None;
        }
        if let Some(task) = self.journal.as_mut() {
            finish_runtime_task_until(
                &task.receiver,
                &mut task.worker,
                deadline,
                "persistent_journal_stop_timeout",
                "Persistent journal work did not stop within the bounded shutdown window",
            )?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P1Result);
            self.journal = None;
        }
        if let Some(task) = self.recovery.as_mut() {
            finish_runtime_task_until(
                &task.receiver,
                &mut task.worker,
                deadline,
                "authoritative_recovery_stop_timeout",
                "Authoritative recovery did not stop within the bounded shutdown window",
            )?;
            #[cfg(test)]
            self.maybe_panic_while_draining(DrainPanicPoint::P2Result);
            self.recovery = None;
        }
        self.inventory_cleanup.finish_stopping_until(deadline)?;
        self.finish_core_shutdown_until(deadline)
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

#[cfg(windows)]
fn persist_journal_registration_failure(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    error: PersistentChangeJournalOperationError,
    failed_unix_ms: i64,
    policy: crate::domain::LibraryChangeQueuePolicy,
) -> Result<PersistentJournalRootFailure, ScanError> {
    let classified = classify_persistent_journal_operation_failure(error);
    let failure = PersistentJournalRootFailure {
        root_id: root_id.to_owned(),
        root_generation,
        kind: classified.kind,
        failure: classified.failure,
        opening_boundary: classified.opening_boundary.map(|boundary| *boundary),
    };
    failure.validate()?;
    if failure.kind != PersistentJournalRootFailureKind::Cancelled {
        catalog.persist_persistent_journal_root_failure(&failure, failed_unix_ms, policy)?;
    }
    Ok(failure)
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
mod tests {
    #[cfg(windows)]
    mod priority;
    #[cfg(windows)]
    mod retained_gap;

    #[cfg(windows)]
    mod priority_journal;

    #[cfg(windows)]
    mod poll_catalog_runtime;

    #[cfg(windows)]
    mod ingress_reservation;

    #[cfg(windows)]
    mod root_availability;

    use super::*;
    #[cfg(windows)]
    use crate::domain::{LibraryChangeQueueHealth, ScanRequest};
    #[cfg(windows)]
    use crate::ports::{CatalogRepository, LibraryChangeQueue};
    #[cfg(windows)]
    use priority_journal::PriorityJournalSession;
    #[cfg(windows)]
    use rusqlite::OptionalExtension;

    #[cfg(windows)]
    #[derive(Clone, Copy)]
    struct HealthyFactory;

    #[cfg(windows)]
    struct HealthySource;

    #[cfg(windows)]
    struct RetainedSourceProbe;

    #[cfg(windows)]
    impl crate::ports::MetadataInventorySource for RetainedSourceProbe {
        fn next_page(
            &mut self,
            _max_entries: u32,
            _cancelled: &AtomicBool,
        ) -> Result<crate::domain::MetadataInventoryPage, ScanError> {
            Ok(crate::domain::MetadataInventoryPage {
                page_index: 0,
                entries: Vec::new(),
                cursor: None,
                is_complete: true,
                frontier: Vec::new(),
            })
        }
    }

    #[cfg(windows)]
    #[derive(Clone, Default)]
    struct QueuedSourceFactory {
        batches: Arc<
            Mutex<
                std::collections::BTreeMap<
                    String,
                    std::collections::VecDeque<crate::domain::LibraryChangeSourceBatch>,
                >,
            >,
        >,
    }

    #[cfg(windows)]
    struct QueuedSource {
        root_id: String,
        batches: Arc<
            Mutex<
                std::collections::BTreeMap<
                    String,
                    std::collections::VecDeque<crate::domain::LibraryChangeSourceBatch>,
                >,
            >,
        >,
    }

    #[cfg(windows)]
    struct CloseProbeSession {
        close_count: Arc<std::sync::atomic::AtomicUsize>,
        close_failures: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[cfg(windows)]
    struct BlockingCloseSession {
        close_count: Arc<std::sync::atomic::AtomicUsize>,
        close_entered: Arc<AtomicBool>,
        close_release: Arc<AtomicBool>,
        close_finished: Arc<AtomicBool>,
        panic_after_release: bool,
    }

    #[cfg(windows)]
    struct NoChangeJournalSession {
        register_count: Arc<std::sync::atomic::AtomicUsize>,
        query_count: Arc<std::sync::atomic::AtomicUsize>,
        shared_read_count: Arc<std::sync::atomic::AtomicUsize>,
        close_count: Arc<std::sync::atomic::AtomicUsize>,
        client_instances: Arc<Mutex<Vec<[u8; 16]>>>,
    }

    #[cfg(windows)]
    struct AdvancingJournalSession {
        boundaries: Mutex<std::collections::VecDeque<i64>>,
        query_count: Arc<std::sync::atomic::AtomicUsize>,
        shared_read_count: Arc<std::sync::atomic::AtomicUsize>,
        read_release: Arc<AtomicBool>,
        close_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[cfg(windows)]
    struct CompletedJournalRead {
        response: BrokerResponse,
        release: Arc<AtomicBool>,
    }

    #[cfg(windows)]
    struct CountingJournalFactory {
        connect_count: Arc<std::sync::atomic::AtomicUsize>,
        close_count: Arc<std::sync::atomic::AtomicUsize>,
        close_failures: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[cfg(windows)]
    struct BlockingJournalFactory {
        connect_entered: Arc<AtomicBool>,
        connect_release: Arc<AtomicBool>,
        connect_count: Arc<std::sync::atomic::AtomicUsize>,
        close_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[cfg(windows)]
    #[derive(Clone)]
    struct OrderingProbeFactory {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    #[cfg(windows)]
    struct OrderingProbeSource;

    #[cfg(windows)]
    impl PersistentChangeJournal for CountingJournalFactory {
        fn connect(&self) -> PersistentChangeJournalConnection {
            self.connect_count.fetch_add(1, Ordering::AcqRel);
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&self.close_count),
                close_failures: Arc::clone(&self.close_failures),
            }))
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournal for BlockingJournalFactory {
        fn connect(&self) -> PersistentChangeJournalConnection {
            self.connect_count.fetch_add(1, Ordering::AcqRel);
            self.connect_entered.store(true, Ordering::Release);
            while !self.connect_release.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&self.close_count),
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }))
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournal for OrderingProbeFactory {
        fn connect(&self) -> PersistentChangeJournalConnection {
            self.events.lock().expect("ordering events").push("journal");
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
            )
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournalSession for CloseProbeSession {
        fn register_root(
            &self,
            _request: RegisterRootRequest,
        ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn query_journal(
            &self,
            _request: QueryJournalRequest,
        ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_range(
            &self,
            _request: ReadJournalRangeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_volume(
            &self,
            _request: ReadJournalVolumeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
            self.close_count.fetch_add(1, Ordering::AcqRel);
            if self
                .close_failures
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                    remaining.checked_sub(1)
                })
                .is_ok()
            {
                return Err(PersistentChangeJournalOperationError::TransportUnavailable);
            }
            Ok(())
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournalSession for BlockingCloseSession {
        fn register_root(
            &self,
            _request: RegisterRootRequest,
        ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn query_journal(
            &self,
            _request: QueryJournalRequest,
        ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_range(
            &self,
            _request: ReadJournalRangeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_volume(
            &self,
            _request: ReadJournalVolumeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
            self.close_count.fetch_add(1, Ordering::AcqRel);
            self.close_entered.store(true, Ordering::Release);
            while !self.close_release.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            self.close_finished.store(true, Ordering::Release);
            assert!(!self.panic_after_release, "controlled journal close panic");
            Ok(())
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournalSession for NoChangeJournalSession {
        fn register_root(
            &self,
            request: RegisterRootRequest,
        ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
            self.register_count.fetch_add(1, Ordering::AcqRel);
            self.client_instances
                .lock()
                .expect("no-change client instances")
                .push(request.caller.client_instance);
            Ok(RootCapability([7; 32]))
        }

        fn query_journal(
            &self,
            request: QueryJournalRequest,
        ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            self.query_count.fetch_add(1, Ordering::AcqRel);
            Ok(BrokerResponse::Journal {
                request_id: 1,
                binding: crate::journal_broker::ResponseBinding::from_request(
                    &request.caller,
                    &request.root,
                ),
                capability: crate::journal_broker::JournalCapability::Supported,
                journal_id: Some(44),
                first_usn: Some(1),
                next_usn: Some(20),
            })
        }

        fn begin_read_range(
            &self,
            _request: ReadJournalRangeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_volume(
            &self,
            _request: ReadJournalVolumeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            self.shared_read_count.fetch_add(1, Ordering::AcqRel);
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
            self.close_count.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournalSession for AdvancingJournalSession {
        fn register_root(
            &self,
            _request: RegisterRootRequest,
        ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
            Ok(RootCapability([6; 32]))
        }

        fn query_journal(
            &self,
            request: QueryJournalRequest,
        ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            self.query_count.fetch_add(1, Ordering::AcqRel);
            let next_usn = self
                .boundaries
                .lock()
                .expect("journal boundaries")
                .pop_front()
                .unwrap_or(21);
            Ok(BrokerResponse::Journal {
                request_id: 1,
                binding: crate::journal_broker::ResponseBinding::from_request(
                    &request.caller,
                    &request.root,
                ),
                capability: JournalCapability::Supported,
                journal_id: Some(44),
                first_usn: Some(1),
                next_usn: Some(next_usn),
            })
        }

        fn begin_read_range(
            &self,
            _request: ReadJournalRangeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_volume(
            &self,
            request: ReadJournalVolumeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            self.shared_read_count.fetch_add(1, Ordering::AcqRel);
            let root = request
                .roots
                .first()
                .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
            Ok(Box::new(CompletedJournalRead {
                response: BrokerResponse::ReadVolume {
                    request_id: 1,
                    client_instance: request.caller.client_instance,
                    volume_id: root.root.volume_id.clone(),
                    journal_id: request.journal_id,
                    requested_end_usn: request.end_usn,
                    max_records: request.max_records,
                    max_evidence_bytes: request.max_evidence_bytes,
                    outcomes: vec![crate::journal_broker::SharedJournalRootOutcome {
                        binding: crate::journal_broker::ResponseBinding::from_request(
                            &request.caller,
                            &root.root,
                        ),
                        requested_start_usn: root.start_usn,
                        covered_until_usn: Some(request.end_usn),
                        is_complete: true,
                        candidates: Vec::new(),
                        failure: None,
                    }],
                    handoffs: Vec::new(),
                    pending_renames: Vec::new(),
                },
                release: Arc::clone(&self.read_release),
            }))
        }

        fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
            self.close_count.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
    }

    #[cfg(windows)]
    impl PersistentChangeJournalRead for CompletedJournalRead {
        fn cancel(
            &self,
            _caller: crate::journal_broker::CallerClaim,
        ) -> Result<(), PersistentChangeJournalOperationError> {
            self.release.store(true, Ordering::Release);
            Ok(())
        }

        fn wait(self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            while !self.release.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            Ok(self.response)
        }
    }

    #[cfg(windows)]
    fn test_live_only_connection() -> PersistentChangeJournalConnection {
        PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
        )
    }

    #[cfg(windows)]
    fn metadata_inventory_lease(change_id: LibraryChangeId) -> LeasedLibraryChange {
        let generation = LibraryRootGeneration::initial();
        let lease_generation = 1;
        let lease_expires_unix_ms = 10_000;
        LeasedLibraryChange {
            change: crate::domain::DurableLibraryChange {
                id: change_id,
                intent: crate::domain::LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                    first_observed_unix_ms: 1,
                    most_recent_observed_unix_ms: 1,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                },
                status: crate::domain::LibraryChangeQueueStatus::Leased,
                ready_unix_ms: 1,
                attempt_count: 1,
                next_retry_unix_ms: None,
                lease_generation,
                lease_expires_unix_ms: Some(lease_expires_unix_ms),
                last_failure: Some(crate::domain::LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "fixture metadata inventory authority".to_owned(),
                }),
                catalog_revision_at_enqueue: 0,
                catalog_revision_at_success: None,
                catch_up_source: None,
                catch_up_watermark: None,
                catch_up_lineage: Vec::new(),
                superseded_by_change_id: None,
            },
            lease_generation,
            lease_expires_unix_ms,
        }
    }

    #[cfg(windows)]
    fn retained_source_probe(run_id: &str) -> RetainedMetadataInventorySource {
        RetainedMetadataInventorySource::for_test(run_id, Box::new(RetainedSourceProbe))
    }

    #[cfg(windows)]
    fn runtime_with_close_probe(
        close_count: Arc<std::sync::atomic::AtomicUsize>,
    ) -> ProductionSynchronization {
        new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count,
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })),
        )
    }

    #[cfg(windows)]
    fn runtime_with_blocking_close(
        close_count: Arc<std::sync::atomic::AtomicUsize>,
        close_entered: Arc<AtomicBool>,
        close_release: Arc<AtomicBool>,
        close_finished: Arc<AtomicBool>,
    ) -> ProductionSynchronization {
        new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
                close_count,
                close_entered,
                close_release,
                close_finished,
                panic_after_release: false,
            })),
        )
    }

    #[cfg(windows)]
    fn runtime_with_drain_panic(
        point: DrainPanicPoint,
        remaining: Option<usize>,
        close_count: Arc<std::sync::atomic::AtomicUsize>,
    ) -> ProductionSynchronization {
        let mut runtime = runtime_with_close_probe(close_count);
        match point {
            DrainPanicPoint::P0Result => {
                let (sender, receiver) = mpsc::sync_channel(1);
                let worker = thread::spawn(move || {
                    let _ = sender.send(Ok(AuthoritativeLibraryChangeReport::default()));
                });
                runtime.live = Some(LiveTask {
                    root_id: "panic-p0".to_owned(),
                    cancelled: Arc::new(AtomicBool::new(false)),
                    receiver,
                    worker: Some(worker),
                });
            }
            DrainPanicPoint::P1Result => {
                let (sender, receiver) = mpsc::sync_channel(1);
                let worker = thread::spawn(move || {
                    let _ = sender.send(Ok(JournalTaskOutcome::Page(
                        PersistentJournalEnrollmentReport::default(),
                    )));
                });
                runtime.journal = Some(JournalTask {
                    cancelled: Arc::new(AtomicBool::new(false)),
                    receiver,
                    worker: Some(worker),
                });
            }
            DrainPanicPoint::P2Result => {
                let (sender, receiver) = mpsc::sync_channel(1);
                let worker = thread::spawn(move || {
                    let _ = sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                        IncrementalLibraryChangeReport::default(),
                    )));
                });
                runtime.recovery = Some(RecoveryTask {
                    root_id: "panic-p2".to_owned(),
                    kind: RecoveryTaskKind::LegacyUnownedDrain {
                        continuity_revision: 1,
                    },
                    phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
                    cancelled: Arc::new(AtomicBool::new(false)),
                    receiver,
                    worker: Some(worker),
                });
            }
            DrainPanicPoint::RequestStop
            | DrainPanicPoint::JournalCloseInstalled
            | DrainPanicPoint::JournalCloseResult => {}
        }
        runtime.inject_drain_panic(point, remaining);
        runtime
    }

    #[cfg(windows)]
    fn ready_test_registry(
        mut runtime: ProductionSynchronization,
    ) -> Arc<SynchronizationRuntimeRegistry> {
        let cancelled = Arc::new(AtomicBool::new(false));
        runtime.stop_requested = Arc::clone(&cancelled);
        Arc::new(SynchronizationRuntimeRegistry {
            state: Mutex::new(SynchronizationRuntimeState {
                next_epoch: 1,
                next_lifecycle_ordinal: LifecycleOrdinal(1),
                cancelled_through_ordinal: LifecycleOrdinal(0),
                admitted_stop_fences: VecDeque::new(),
                retired_through_epoch: 0,
                entry: SynchronizationRuntimeEntry::Ready {
                    epoch: 1,
                    owner_ticket: RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)),
                    cancelled,
                    runtime: Arc::new(Mutex::new(runtime)),
                },
            }),
            changed: Condvar::new(),
            panic_retention_barrier: Mutex::new(None),
        })
    }

    #[cfg(windows)]
    fn wait_for_test_signal(signal: &AtomicBool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !signal.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(
            signal.load(Ordering::Acquire),
            "blocking operation did not start"
        );
    }

    #[cfg(windows)]
    fn assert_start_is_blocked_while_stopping(registry: &SynchronizationRuntimeRegistry) {
        let error = match claim_runtime_for_start(registry) {
            Ok(_) => panic!("a stopping epoch must not admit another synchronization session"),
            Err(error) => error,
        };
        assert_eq!(error.code, "library_synchronization_stop_in_progress");
    }

    #[cfg(windows)]
    fn wait_for_draining_state(
        registry: &SynchronizationRuntimeRegistry,
        timeout: Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if matches!(
                registry.state.lock().expect("local registry").entry,
                SynchronizationRuntimeEntry::Draining { .. }
            ) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::yield_now();
        }
    }

    #[cfg(windows)]
    fn wait_for_retained_journal_close_worker(
        registry: &SynchronizationRuntimeRegistry,
        timeout: Duration,
    ) {
        let deadline = Instant::now() + timeout;
        loop {
            let is_finished = {
                let state = registry.state.lock().expect("local registry");
                match &state.entry {
                    SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                        .lock()
                        .expect("draining runtime")
                        .journal_close
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished),
                    _ => false,
                }
            };
            if is_finished {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the retained journal-close worker did not finish after release"
            );
            thread::yield_now();
        }
    }

    #[cfg(windows)]
    fn retained_stop_deadline(registry: &SynchronizationRuntimeRegistry) -> Option<Instant> {
        match registry.state.lock().expect("local registry").entry {
            SynchronizationRuntimeEntry::Stopping { deadline, .. }
            | SynchronizationRuntimeEntry::Draining { deadline, .. } => Some(deadline),
            _ => None,
        }
    }

    #[cfg(windows)]
    fn active_owner_ticket(registry: &SynchronizationRuntimeRegistry) -> RuntimeStartTicket {
        registry
            .state
            .lock()
            .expect("local registry")
            .entry
            .owner_ticket()
            .expect("test runtime owner ticket")
    }

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
    impl crate::ports::LibraryChangeSourceFactory for QueuedSourceFactory {
        type Source = QueuedSource;

        fn start(
            &self,
            request: &crate::ports::LibraryChangeSourceRequest,
        ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
            Ok(QueuedSource {
                root_id: request.root_id.clone(),
                batches: Arc::clone(&self.batches),
            })
        }
    }

    #[cfg(windows)]
    impl crate::ports::LibraryChangeSourceFactory for OrderingProbeFactory {
        type Source = OrderingProbeSource;

        fn start(
            &self,
            _request: &crate::ports::LibraryChangeSourceRequest,
        ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
            self.events.lock().expect("ordering events").push("watcher");
            Ok(OrderingProbeSource)
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

    #[cfg(windows)]
    impl crate::ports::LibraryChangeSource for QueuedSource {
        fn health(&self) -> crate::domain::LibraryChangeSourceHealth {
            crate::domain::LibraryChangeSourceHealth::Healthy
        }

        fn drain(
            &mut self,
            _max_observations: usize,
        ) -> Result<crate::domain::LibraryChangeSourceBatch, crate::domain::LibraryChangeSourceError>
        {
            Ok(self
                .batches
                .lock()
                .expect("queued source batches")
                .entry(self.root_id.clone())
                .or_default()
                .pop_front()
                .unwrap_or(crate::domain::LibraryChangeSourceBatch {
                    observations: Vec::new(),
                    health: crate::domain::LibraryChangeSourceHealth::Healthy,
                    dropped_observation_count: 0,
                    ignored_callback_count: 0,
                    last_issue_code: None,
                }))
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

    #[cfg(windows)]
    impl crate::ports::LibraryChangeSource for OrderingProbeSource {
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

    #[cfg(windows)]
    struct ProductionGapFixture {
        _directory: tempfile::TempDir,
        source_root: std::path::PathBuf,
        root_id: String,
        storage: crate::application::storage::StoragePaths,
        scan_rows_before: i64,
    }

    #[cfg(windows)]
    #[derive(Debug)]
    struct DurableGapRow {
        change_id: i64,
        intent_kind: String,
        scope: String,
        relative_path: String,
        origin: String,
        lane: String,
        status: String,
        first_sequence: String,
        most_recent_sequence: String,
        coalesced_observation_count: i64,
    }

    #[cfg(windows)]
    impl ProductionGapFixture {
        fn new(name: &str, source_entry_count: usize) -> Self {
            let directory = tempfile::tempdir().expect("production gap test directory");
            let source_root = directory.path().join("source");
            std::fs::create_dir_all(&source_root).expect("production gap source root");
            for index in 0..source_entry_count {
                std::fs::write(
                    source_root.join(format!("sentinel-{index:04}.txt")),
                    b"metadata-only recovery fixture",
                )
                .expect("production gap source entry");
            }
            let discovery = crate::adapters::FileDiscovery::new(&source_root.to_string_lossy())
                .expect("production gap root discovery");
            let root_path = source_root.to_string_lossy().into_owned();
            let publication_identity = discovery
                .metadata_inventory_root_identity()
                .expect("production gap root identity query")
                .expect("production gap stable root identity");
            let root_id =
                crate::application::scan_library::stable_id("library-root-v1", &root_path);
            let storage = crate::application::storage::StoragePaths {
                catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
                preview_root: directory.path().join("previews"),
                preview_budget_bytes: 64 * 1024 * 1024,
                settings_path: directory.path().join("settings").join("storage.sqlite3"),
            };
            let request = ScanRequest {
                scan_id: format!("initial-{name}"),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            };
            let mut catalog =
                SqliteCatalog::open(storage.catalog_path.clone()).expect("production gap catalog");
            catalog
                .begin_scan_with_publication_namespace(
                    &request,
                    &root_id,
                    &root_path,
                    &publication_identity,
                )
                .expect("begin production gap fixture scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&request.scan_id)
                .expect("prove production gap fixture first-import handoff");
            catalog
                .publish_scan(&request.scan_id, &root_id, 0, 0)
                .expect("publish production gap fixture scan");
            drop(catalog);
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open production gap assertion catalog");
            let scan_rows_before = connection
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
                .expect("count production gap scan rows");
            drop(connection);
            Self {
                _directory: directory,
                source_root,
                root_id,
                storage,
                scan_rows_before,
            }
        }

        fn new_with_media_baseline(name: &str) -> Self {
            let directory = tempfile::tempdir().expect("media gap test directory");
            let source_root = directory.path().join("source");
            std::fs::create_dir_all(&source_root).expect("media gap source root");
            image::RgbImage::from_pixel(2, 2, image::Rgb([20, 40, 80]))
                .save_with_format(source_root.join("unchanged.png"), image::ImageFormat::Png)
                .expect("unchanged media baseline");
            image::RgbImage::from_pixel(2, 2, image::Rgb([80, 40, 20]))
                .save_with_format(source_root.join("removed.png"), image::ImageFormat::Png)
                .expect("removed media baseline");
            let discovery = crate::adapters::FileDiscovery::new(&source_root.to_string_lossy())
                .expect("media gap root discovery");
            let root_path = discovery
                .canonical_root()
                .expect("canonical media gap root")
                .to_string_lossy()
                .into_owned();
            let root_id =
                crate::application::scan_library::stable_id("library-root-v1", &root_path);
            let storage = crate::application::storage::StoragePaths {
                catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
                preview_root: directory.path().join("previews"),
                preview_budget_bytes: 64 * 1024 * 1024,
                settings_path: directory.path().join("settings").join("storage.sqlite3"),
            };
            crate::application::scan_library::run_scan_with_storage(
                ScanRequest {
                    scan_id: format!("initial-{name}"),
                    root_path,
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                |_| true,
                storage.clone(),
            )
            .expect("publish explicit media baseline");
            let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                .expect("open media baseline catalog");
            assert!(
                catalog
                    .load_incremental_location_by_relative_path(&root_id, "unchanged.png")
                    .expect("load unchanged media baseline")
                    .is_some()
            );
            assert!(
                catalog
                    .load_incremental_location_by_relative_path(&root_id, "removed.png")
                    .expect("load removable media baseline")
                    .is_some()
            );
            drop(catalog);
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open media baseline assertion catalog");
            let scan_rows_before = connection
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
                .expect("count media baseline scan rows");
            drop(connection);
            Self {
                _directory: directory,
                source_root,
                root_id,
                storage,
                scan_rows_before,
            }
        }

        fn observer_request(&self) -> crate::ports::LibraryChangeSourceRequest {
            crate::ports::LibraryChangeSourceRequest {
                root_id: self.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                root_path: self.source_root.clone(),
                ingress_capacity: 16,
            }
        }

        fn assert_no_automatic_full_scan(&self) {
            let connection = rusqlite::Connection::open(&self.storage.catalog_path)
                .expect("open full-scan assertion catalog");
            let scan_rows_after: i64 = connection
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
                .expect("count final production gap scan rows");
            assert_eq!(scan_rows_after, self.scan_rows_before);
        }
    }

    #[cfg(windows)]
    fn fast_gap_runtime(
        factory: QueuedSourceFactory,
        connection: PersistentChangeJournalConnection,
    ) -> ProductionSynchronization {
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(factory),
            connection,
        );
        production.runtime.queue_policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            retry_initial_delay_millis: 1,
            retry_maximum_delay_millis: 8,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        production.runtime.restart_policy = crate::domain::LibraryChangeRestartPolicy {
            initial_delay_millis: 1,
            maximum_delay_millis: 8,
        };
        production
    }

    #[cfg(windows)]
    fn push_gap_batch(
        factory: &QueuedSourceFactory,
        root_id: &str,
        batch: crate::domain::LibraryChangeSourceBatch,
    ) {
        factory
            .batches
            .lock()
            .expect("queued production gap batches")
            .entry(root_id.to_owned())
            .or_default()
            .push_back(batch);
    }

    #[cfg(windows)]
    fn wait_for_durable_gap(
        production: &mut ProductionSynchronization,
        fixture: &ProductionGapFixture,
    ) -> DurableGapRow {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            poll_runtime_with_storage(production, &fixture.storage)
                .expect("poll production gap ingress");
            let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open durable gap assertion catalog");
            let row = connection
                .query_row(
                    "SELECT queue.id, queue.intent_kind, queue.scope, queue.relative_path,
                            queue.origin, lanes.lane, queue.status, queue.first_sequence,
                            queue.most_recent_sequence, queue.coalesced_observation_count
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                     WHERE queue.root_id = ?1 AND queue.root_generation = 1
                       AND queue.intent_kind = 'freshness_unknown' AND queue.scope = 'root'
                       AND queue.origin = 'live_notification' AND lanes.lane = 'p0_live'
                     ORDER BY queue.id DESC LIMIT 1",
                    [&fixture.root_id],
                    |row| {
                        Ok(DurableGapRow {
                            change_id: row.get(0)?,
                            intent_kind: row.get(1)?,
                            scope: row.get(2)?,
                            relative_path: row.get(3)?,
                            origin: row.get(4)?,
                            lane: row.get(5)?,
                            status: row.get(6)?,
                            first_sequence: row.get(7)?,
                            most_recent_sequence: row.get(8)?,
                            coalesced_observation_count: row.get(9)?,
                        })
                    },
                )
                .optional()
                .expect("load durable production gap");
            if let Some(row) = row {
                return row;
            }
            assert!(
                Instant::now() < deadline,
                "production observer did not persist its evidence gap"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[cfg(windows)]
    fn assert_p0_live_gap(row: &DurableGapRow) {
        assert_eq!(row.intent_kind, "freshness_unknown");
        assert_eq!(row.scope, "root");
        assert_eq!(row.relative_path, "");
        assert_eq!(row.origin, "live_notification");
        assert_eq!(row.lane, "p0_live");
        assert!(matches!(
            row.status.as_str(),
            "pending" | "leased" | "retry_wait" | "superseded"
        ));
        let first_sequence = row
            .first_sequence
            .parse::<u64>()
            .expect("canonical first sequence");
        let most_recent_sequence = row
            .most_recent_sequence
            .parse::<u64>()
            .expect("canonical most-recent sequence");
        assert!(first_sequence > 0);
        assert!(most_recent_sequence >= first_sequence);
        assert!(row.coalesced_observation_count > 0);
    }

    #[cfg(windows)]
    fn drive_gap_to_synchronized(
        production: &mut ProductionSynchronization,
        fixture: &ProductionGapFixture,
    ) -> LibrarySynchronizationSnapshot {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let snapshot = poll_runtime_with_storage(production, &fixture.storage)
                .expect("drive production gap recovery");
            let status = snapshot.roots.first().expect("production gap root status");
            let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
                .expect("open production gap metrics catalog");
            let metrics = catalog
                .load_library_change_root_queue_metrics(
                    &fixture.root_id,
                    LibraryRootGeneration::initial(),
                    now_unix_ms().expect("production gap metrics time"),
                    production.runtime.queue_policy(),
                )
                .expect("load production gap queue metrics");
            if status.freshness == crate::domain::CatalogFreshnessState::Synchronized
                && metrics.pending_count == 0
                && metrics.leased_count == 0
                && metrics.retry_wait_count == 0
            {
                return snapshot;
            }
            if Instant::now() >= deadline {
                let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
                    .expect("open production gap timeout diagnostics");
                let queue = connection
                    .prepare(
                        "SELECT id, origin, status, attempt_count, last_failure_code,
                                last_failure_message
                         FROM library_change_queue WHERE root_id = ?1 ORDER BY id",
                    )
                    .and_then(|mut statement| {
                        statement
                            .query_map([&fixture.root_id], |row| {
                                Ok((
                                    row.get::<_, i64>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                    row.get::<_, i64>(3)?,
                                    row.get::<_, Option<String>>(4)?,
                                    row.get::<_, Option<String>>(5)?,
                                ))
                            })?
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .expect("load production gap timeout queue");
                let inventory = connection
                    .query_row(
                        "SELECT COUNT(*),
                                COALESCE(GROUP_CONCAT(status || ':' || enumeration_complete || ':' || absence_authority), '')
                         FROM library_metadata_inventory_runs",
                        [],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                    )
                    .expect("load production gap timeout inventory");
                panic!(
                    "production gap did not converge: freshness={:?} pending={} leased={} retry_wait={} last_issue={:?} queue={queue:?} inventory={inventory:?}",
                    status.freshness,
                    metrics.pending_count,
                    metrics.leased_count,
                    metrics.retry_wait_count,
                    status.last_issue_code,
                );
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[cfg(windows)]
    fn assert_p2_gap_consumer(fixture: &ProductionGapFixture, gap_change_id: i64) {
        let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open P2 gap consumer catalog");
        let evidence: (String, Option<i64>, String, String, String, String, String) = connection
            .query_row(
                "SELECT lineage.consumer_kind, lineage.recovery_change_id,
                        gap.status, recovery.origin, recovery_lanes.lane,
                        recovery.intent_kind, authority.reason
                 FROM library_live_gap_recovery_claims AS lineage
                 JOIN library_change_queue AS gap ON gap.id = lineage.gap_change_id
                 JOIN library_change_queue AS recovery ON recovery.id = lineage.recovery_change_id
                 JOIN library_change_queue_lanes AS recovery_lanes
                   ON recovery_lanes.change_id = recovery.id
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = recovery.id
                 WHERE lineage.gap_change_id = ?1",
                [gap_change_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("load P2 live-gap lineage and consumer");
        assert_eq!(evidence.0, "metadata_inventory_control");
        assert!(evidence.1.is_some());
        assert_eq!(evidence.2, "superseded");
        assert_eq!(evidence.3, "metadata_inventory");
        assert_eq!(evidence.4, "p2_recovery");
        assert_eq!(evidence.5, "freshness_unknown");
        assert_eq!(evidence.6, "watcher_uncovered_gap");
    }

    #[cfg(windows)]
    fn assert_journal_gap_consumer(fixture: &ProductionGapFixture, gap_change_id: i64) {
        let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open journal gap consumer catalog");
        let evidence: (
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT lineage.consumer_kind, lineage.source_range_id, gap.status,
                            ranges.root_id, ranges.requested_start_usn,
                            ranges.covered_until_usn, lifecycle.lifecycle_state,
                            lineage.opening_next_usn
                     FROM library_live_gap_recovery_claims AS lineage
                     JOIN library_change_queue AS gap ON gap.id = lineage.gap_change_id
                     JOIN library_persistent_journal_source_ranges AS ranges
                       ON ranges.id = lineage.source_range_id
                     JOIN library_persistent_journal_range_lifecycle AS lifecycle
                       ON lifecycle.source_range_id = ranges.id
                     WHERE lineage.gap_change_id = ?1",
                [gap_change_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("load journal live-gap lineage and consumer");
        assert_eq!(evidence.0, "journal_source_range");
        assert!(evidence.1.is_some());
        assert_eq!(evidence.2, "superseded");
        assert_eq!(evidence.3, fixture.root_id);
        assert_eq!(evidence.4, evidence.7);
        assert!(
            evidence.5.parse::<i64>().expect("covered journal USN")
                >= evidence.4.parse::<i64>().expect("opening journal USN")
        );
        assert_eq!(evidence.6, "completed");
        let p2_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_live_gap_recovery_claims
                 WHERE gap_change_id = ?1 AND recovery_change_id IS NOT NULL",
                [gap_change_id],
                |row| row.get(0),
            )
            .expect("count unexpected P2 live-gap consumer");
        assert_eq!(p2_count, 0);
    }

    #[cfg(windows)]
    fn seed_current_gap_journal(fixture: &ProductionGapFixture) {
        let registration = describe_production_persistent_journal_root(
            &fixture.root_id,
            LibraryRootGeneration::initial().value(),
            &fixture.source_root,
        )
        .expect("describe production gap journal root");
        let root_reference = crate::domain::JournalFileReference::from_bytes(
            &registration.authorization.root_identity,
        )
        .expect("production gap root file reference");
        let volume = crate::domain::PersistentJournalVolumeIdentity {
            volume_guid: registration.authorization.volume_id.clone(),
            volume_serial: registration.volume_serial,
        };
        let journal_id =
            crate::domain::JournalIdentifier::new(44).expect("production gap journal ID");
        let usn = crate::domain::JournalUsn::new(20).expect("production gap journal USN");
        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open production gap journal catalog");
        let root = catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load production gap journal root")
            .expect("published production gap journal root");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1_000,
            })
            .expect("save production gap journal capability");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                volume,
                root_file_reference: root_reference,
                journal_id,
                next_unread_usn: usn,
                captured_exclusive_end: usn,
                covered_catalog_revision: root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1_000,
            })
            .expect("seed production gap journal checkpoint");
    }

    #[test]
    fn current_time_is_representable() {
        assert!(now_unix_ms().expect("current time") > 0);
    }

    #[cfg(windows)]
    #[test]
    fn production_native_need_rescan_live_only_preserves_p0_and_converges_through_p2() {
        let fixture = ProductionGapFixture::new_with_media_baseline("native-need-rescan");
        let unchanged_path = fixture.source_root.join("unchanged.png");
        let removed_path = fixture.source_root.join("removed.png");
        let added_path = fixture.source_root.join("added.png");
        let unchanged_before = std::fs::read(&unchanged_path).expect("read unchanged baseline");
        let unchanged_hash_before = blake3::hash(&unchanged_before);
        std::fs::remove_file(&removed_path).expect("remove baseline media");
        image::RgbImage::from_pixel(2, 2, image::Rgb([40, 120, 200]))
            .save_with_format(&added_path, image::ImageFormat::Png)
            .expect("add changed media");
        let added_before = std::fs::read(&added_path).expect("read added source");
        let added_hash_before = blake3::hash(&added_before);
        let root_path = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open P2 instrumentation catalog")
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load P2 instrumentation root")
            .expect("P2 instrumentation root")
            .root_path;
        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        crate::adapters::reset_source_content_open_instrumentation(&root_path);
        let factory = QueuedSourceFactory::default();
        let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("start native-gap production observer");
        let batch = crate::adapters::native_need_rescan_batch_for_test(&fixture.observer_request())
            .expect("map native notify need_rescan");
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(
            batch.observations[0].kind,
            crate::domain::LibraryChangeObservationKind::EvidenceGap
        );
        assert_eq!(
            batch.observations[0].origin,
            crate::domain::LibraryChangeOrigin::LiveNotification
        );
        assert_eq!(
            batch.last_issue_code.as_deref(),
            Some("change_source_rescan_required")
        );
        push_gap_batch(&factory, &fixture.root_id, batch);

        let gap = wait_for_durable_gap(&mut production, &fixture);
        assert_p0_live_gap(&gap);
        let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_p2_gap_consumer(&fixture, gap.change_id);
        let mut mutation_connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open legacy-contract mutation evidence");
        let mutation = mutation_connection
            .transaction()
            .expect("begin legacy-contract mutation");
        assert_eq!(
            mutation
                .execute(
                    "DELETE FROM asset_locations
                     WHERE root_id = ?1 AND relative_path = 'added.png'
                       AND scan_id = (
                         SELECT active_scan_id FROM library_roots WHERE id = ?1
                       )",
                    [&fixture.root_id],
                )
                .expect("mutate visible location inside rollback boundary"),
            1,
        );
        let legacy_contract: (String, String, i64, i64, i64) = mutation
            .query_row(
                "SELECT gap.status, claim.consumer_kind,
                        (SELECT COUNT(*) FROM library_change_queue
                         WHERE status IN ('pending', 'leased', 'retry_wait')),
                        (SELECT COUNT(*) FROM scan_runs),
                        (SELECT COUNT(*)
                         FROM library_roots AS root
                         JOIN asset_locations AS location
                           ON location.scan_id = root.active_scan_id
                         WHERE root.id = ?2 AND location.relative_path = 'added.png')
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
                 WHERE claim.gap_change_id = ?1",
                rusqlite::params![gap.change_id, fixture.root_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("load legacy-contract mutation evidence");
        assert_eq!(
            legacy_contract,
            (
                "superseded".to_owned(),
                "metadata_inventory_control".to_owned(),
                0,
                fixture.scan_rows_before,
                0,
            ),
            "the former empty-fixture contract stays green even when visible publication is removed"
        );
        mutation
            .rollback()
            .expect("rollback legacy-contract mutation");
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open P2 visibility catalog");
        let added = catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "added.png")
            .expect("load added P2 location")
            .expect("added P2 location must be visible");
        assert_eq!(added.relative_path, "added.png");
        assert_eq!(added.width, 2);
        assert_eq!(added.height, 2);
        assert_eq!(
            added.file_size,
            u64::try_from(added_before.len()).expect("added source size")
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, "removed.png")
                .expect("load removed P2 location")
                .is_none(),
            "P2 absence publication must remove the deleted location"
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, "unchanged.png")
                .expect("load unchanged P2 location")
                .is_some(),
            "unchanged baseline media must remain visible"
        );
        let queue = catalog
            .load_library_change_root_queue_metrics(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms().expect("visibility metrics clock"),
                production.runtime.queue_policy(),
            )
            .expect("load terminal P2 visibility queue");
        assert_eq!(queue.pending_count, 0);
        assert_eq!(queue.leased_count, 0);
        assert_eq!(queue.retry_wait_count, 0);
        drop(catalog);
        let terminal: (i64, i64, i64, i64) =
            rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open terminal P2 visibility evidence")
                .query_row(
                    "SELECT
                       (SELECT COUNT(*)
                        FROM library_roots AS root
                        JOIN asset_locations AS location
                          ON location.scan_id = root.active_scan_id
                        WHERE root.id = ?1 AND location.relative_path = 'added.png'),
                       (SELECT COUNT(*)
                        FROM library_live_gap_recovery_claims AS claim
                        WHERE claim.root_id = ?1 AND claim.consumed_unix_ms IS NULL),
                       (SELECT COUNT(*) FROM library_recovery_authorities AS authority
                        WHERE authority.root_id = ?1 AND authority.retired_unix_ms IS NULL),
                       (SELECT COUNT(*) FROM scan_runs)",
                    [&fixture.root_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load terminal P2 visibility evidence");
        assert_eq!(terminal, (1, 0, 0, fixture.scan_rows_before));
        let unchanged_after = std::fs::read(&unchanged_path).expect("reread unchanged source");
        let added_after = std::fs::read(&added_path).expect("reread added source");
        assert_eq!(unchanged_after, unchanged_before);
        assert_eq!(blake3::hash(&unchanged_after), unchanged_hash_before);
        assert_eq!(added_after, added_before);
        assert_eq!(blake3::hash(&added_after), added_hash_before);
        assert!(!removed_path.exists());
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 2);
        assert_eq!(crate::adapters::source_spool_open_count(&root_path), 1);
        assert_eq!(crate::adapters::source_content_open_count(&root_path), 1);
        fixture.assert_no_automatic_full_scan();
        production
            .stop()
            .expect("stop native-gap production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_observer_ingress_drop_synthesizes_a_durable_consumed_p0_gap() {
        let fixture = ProductionGapFixture::new("observer-ingress-drop", 0);
        let factory = QueuedSourceFactory::default();
        let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("start ingress-drop production observer");
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 3,
                ignored_callback_count: 0,
                last_issue_code: Some("change_source_ingress_overflow".to_owned()),
            },
        );

        let gap = wait_for_durable_gap(&mut production, &fixture);
        assert_p0_live_gap(&gap);
        let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

        assert_eq!(snapshot.roots[0].pending_change_count, 0);
        assert_eq!(snapshot.roots[0].retry_wait_count, 0);
        assert_p2_gap_consumer(&fixture, gap.change_id);
        fixture.assert_no_automatic_full_scan();
        production
            .stop()
            .expect("stop ingress-drop production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_capacity_deferred_gap_wakes_after_one_p2_slot_and_converges() {
        let fixture = ProductionGapFixture::new("capacity-deferred-gap", 0);
        let factory = QueuedSourceFactory::default();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_unresolved_changes: 16,
            max_lease_batch: 16,
            lease_duration_millis:
                crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
            max_attempts: 4,
            retry_initial_delay_millis: 1,
            retry_maximum_delay_millis: 8,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let p2_capacity =
            usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
                .expect("P2 capacity");
        assert_eq!(p2_capacity, 12);
        let generation = LibraryRootGeneration::initial();
        let base_unix_ms = now_unix_ms().expect("capacity fixture clock");
        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open capacity fixture catalog");
        let fillers = (0..p2_capacity)
            .map(|index| crate::domain::LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: format!("capacity-{index:02}.jpg"),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::UserRefresh,
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: u64::try_from(index + 1).expect("capacity sequence"),
                most_recent_sequence: u64::try_from(index + 1).expect("capacity sequence"),
                coalesced_observation_count: 1,
            })
            .collect::<Vec<_>>();
        catalog
            .enqueue_library_change_intents(&fillers, base_unix_ms, policy)
            .expect("fill P2 lane");
        let mut filler_leases = catalog
            .lease_path_library_changes_in_lane(
                &fixture.root_id,
                generation,
                crate::domain::LibraryChangeLane::Recovery,
                base_unix_ms,
                policy,
            )
            .expect("lease P2 capacity fixtures");
        assert_eq!(filler_leases.len(), p2_capacity);
        drop(catalog);

        let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("start capacity-gap production observer");
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 1,
                ignored_callback_count: 0,
                last_issue_code: Some("change_source_ingress_overflow".to_owned()),
            },
        );
        let gap = wait_for_durable_gap(&mut production, &fixture);
        let attempt_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("exercise capacity deferral budget");
            let evidence: (Option<String>, i64, i64, Option<i64>) =
                rusqlite::Connection::open(&fixture.storage.catalog_path)
                    .expect("open capacity deferral evidence")
                    .query_row(
                        "SELECT last_failure_code, attempt_count, lease_generation,
                                next_retry_unix_ms
                         FROM library_change_queue WHERE id = ?1",
                        [gap.change_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .expect("load capacity deferral evidence");
            if evidence.0.as_deref() == Some("live_gap_p2_capacity_deferred")
                && evidence.1 == 0
                && evidence.2 >= i64::from(policy.max_attempts + 1)
                && evidence.3.is_some()
            {
                break;
            }
            assert!(
                Instant::now() < attempt_deadline,
                "capacity wait exhausted or stopped retrying: {evidence:?}"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let released = filler_leases.pop().expect("one P2 slot owner");
        let released_unix_ms = now_unix_ms().expect("capacity release clock");
        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open capacity release catalog");
        let root = catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load capacity release root")
            .expect("capacity release root");
        assert_eq!(
            catalog
                .complete_library_change(
                    released.change.id,
                    released.lease_generation,
                    root.catalog_revision,
                    released_unix_ms,
                )
                .expect("release one P2 slot"),
            crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
        );
        drop(catalog);

        let promotion_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("wake capacity-deferred P0 gap");
            let ownership: i64 = rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open capacity ownership evidence")
                .query_row(
                    "SELECT COUNT(*)
                         FROM library_live_gap_recovery_claims AS claim
                         JOIN library_recovery_authorities AS authority
                           ON authority.change_id = claim.recovery_change_id
                         WHERE claim.gap_change_id = ?1
                           AND claim.consumer_kind = 'metadata_inventory_control'
                           AND authority.reason = 'watcher_uncovered_gap'",
                    [gap.change_id],
                    |row| row.get(0),
                )
                .expect("load capacity ownership evidence");
            if ownership == 1 {
                break;
            }
            assert!(
                Instant::now() < promotion_deadline,
                "one released P2 slot did not wake and promote the P0 gap"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open remaining capacity catalog");
        let root = catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("reload capacity root")
            .expect("capacity root");
        for filler in filler_leases {
            assert_eq!(
                catalog
                    .complete_library_change(
                        filler.change.id,
                        filler.lease_generation,
                        root.catalog_revision,
                        released_unix_ms + 1,
                    )
                    .expect("release remaining P2 fixture"),
                crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
            );
        }
        drop(catalog);

        let snapshot = drive_gap_to_synchronized(&mut production, &fixture);
        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_p2_gap_consumer(&fixture, gap.change_id);
        let active_authorities: i64 = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open terminal capacity evidence")
            .query_row(
                "SELECT COUNT(*) FROM library_recovery_authorities
                     WHERE retired_unix_ms IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("count terminal capacity authorities");
        assert_eq!(active_authorities, 0);
        fixture.assert_no_automatic_full_scan();
        production
            .stop()
            .expect("stop capacity-gap production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_precise_p0_path_stays_visible_while_a_leased_gap_returns_to_capacity_wait() {
        let fixture = ProductionGapFixture::new("leased-capacity-precise-path", 0);
        let factory = QueuedSourceFactory::default();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_unresolved_changes: 4,
            max_lease_batch: 4,
            lease_duration_millis:
                crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
            max_attempts: 4,
            retry_initial_delay_millis: 5_000,
            retry_maximum_delay_millis: 5_000,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let generation = LibraryRootGeneration::initial();
        let p2_capacity =
            usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
                .expect("P2 capacity");
        assert_eq!(p2_capacity, 2);
        let base_unix_ms = now_unix_ms().expect("leased capacity fixture clock");
        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open leased capacity fixture catalog");
        let fillers = (0..p2_capacity)
            .map(|index| crate::domain::LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: format!("capacity-{index:02}.jpg"),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::UserRefresh,
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: u64::try_from(index + 1).expect("capacity sequence"),
                most_recent_sequence: u64::try_from(index + 1).expect("capacity sequence"),
                coalesced_observation_count: 1,
            })
            .collect::<Vec<_>>();
        catalog
            .enqueue_library_change_intents(&fillers, base_unix_ms, policy)
            .expect("fill P2 lane");
        let filler_leases = catalog
            .lease_path_library_changes_in_lane(
                &fixture.root_id,
                generation,
                crate::domain::LibraryChangeLane::Recovery,
                base_unix_ms,
                policy,
            )
            .expect("lease every P2 capacity fixture");
        assert_eq!(filler_leases.len(), p2_capacity);
        drop(catalog);

        let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("start leased-capacity production observer");
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 1,
                ignored_callback_count: 0,
                last_issue_code: Some("change_source_ingress_overflow".to_owned()),
            },
        );
        let gap = wait_for_durable_gap(&mut production, &fixture);
        let initial_deferral_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("drive initial capacity deferral");
            let deferred: bool = rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open initial capacity evidence")
                .query_row(
                    "SELECT status = 'retry_wait'
                                AND last_failure_code = 'live_gap_p2_capacity_deferred'
                                AND next_retry_unix_ms IS NOT NULL
                         FROM library_change_queue WHERE id = ?1",
                    [gap.change_id],
                    |row| row.get(0),
                )
                .expect("load initial capacity evidence");
            if deferred && production.live.is_none() {
                break;
            }
            assert!(
                Instant::now() < initial_deferral_deadline,
                "P0 gap did not enter typed capacity wait"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let forced_due_unix_ms = now_unix_ms().expect("forced due clock");
        rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open exact race setup")
            .execute(
                "UPDATE library_change_queue
                 SET next_retry_unix_ms = ?1, updated_unix_ms = ?1
                 WHERE id = ?2 AND status = 'retry_wait'
                   AND last_failure_code = 'live_gap_p2_capacity_deferred'",
                rusqlite::params![forced_due_unix_ms, gap.change_id],
            )
            .expect("make the typed capacity gap due once");
        let pause = install_live_worker_after_lease_pause(&fixture.root_id);
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("schedule exact leased capacity gap");
        pause.wait_until_reached(Duration::from_secs(3));

        let leased_gap: (String, i64, Option<i64>, String) =
            rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open leased capacity evidence")
                .query_row(
                    "SELECT status, lease_generation, lease_expires_unix_ms,
                            last_failure_code
                     FROM library_change_queue WHERE id = ?1",
                    [gap.change_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load leased capacity evidence");
        assert_eq!(leased_gap.0, "leased");
        assert!(leased_gap.1 >= 2);
        assert!(leased_gap.2.is_some());
        assert_eq!(leased_gap.3, "live_gap_p2_capacity_deferred");

        let relative_path = "visible-during-capacity.png";
        image::RgbImage::from_pixel(3, 2, image::Rgb([71, 149, 211]))
            .save(fixture.source_root.join(relative_path))
            .expect("write precise P0 media fixture");
        let observed_unix_ms = now_unix_ms().expect("precise P0 observation clock");
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: vec![crate::domain::LibraryChangeObservation {
                    root_id: fixture.root_id.clone(),
                    root_generation: generation,
                    sequence: 2,
                    observed_unix_ms,
                    kind: crate::domain::LibraryChangeObservationKind::Created,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: relative_path.to_owned(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                }],
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 0,
                ignored_callback_count: 0,
                last_issue_code: None,
            },
        );
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("admit precise P0 path while gap worker is leased");
        let concurrent_shape: (String, i64, Option<i64>, i64, String, String) =
            rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open concurrent leased-gap evidence")
                .query_row(
                    "SELECT gap.status, gap.lease_generation, gap.lease_expires_unix_ms,
                            COUNT(path.id), MIN(path.scope), MIN(path.intent_kind)
                     FROM library_change_queue AS gap
                     LEFT JOIN library_change_queue AS path
                       ON path.root_id = gap.root_id
                      AND path.root_generation = gap.root_generation
                      AND path.relative_path = ?2
                      AND path.status IN ('pending', 'leased', 'retry_wait', 'completed')
                     WHERE gap.id = ?1",
                    rusqlite::params![gap.change_id, relative_path],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .expect("load concurrent leased-gap evidence");
        assert_eq!(concurrent_shape.0, "leased");
        assert_eq!(concurrent_shape.1, leased_gap.1);
        assert_eq!(concurrent_shape.2, leased_gap.2);
        assert_eq!(concurrent_shape.3, 1);
        assert_eq!(concurrent_shape.4, "path");
        assert_eq!(concurrent_shape.5, "reconcile");

        pause.release();
        let visible_deadline = Instant::now() + Duration::from_secs(4);
        let terminal_gap = loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("drive precise P0 publication beside capacity wait");
            let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
                .expect("open precise P0 visibility catalog");
            let is_visible = catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, relative_path)
                .expect("load precise P0 location")
                .is_some();
            drop(catalog);
            let gap_state: (String, Option<String>, Option<i64>, i64) =
                rusqlite::Connection::open(&fixture.storage.catalog_path)
                    .expect("open deferred gap state")
                    .query_row(
                        "SELECT gap.status, gap.last_failure_code, gap.next_retry_unix_ms,
                                (SELECT COUNT(*)
                                 FROM library_live_gap_recovery_claims AS claim
                                 WHERE claim.gap_change_id = gap.id)
                         FROM library_change_queue AS gap WHERE gap.id = ?1",
                        [gap.change_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .expect("load deferred gap state");
            if is_visible {
                break gap_state;
            }
            assert!(
                Instant::now() < visible_deadline,
                "precise P0 path was not visible before the gap retry deadline"
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(terminal_gap.0, "retry_wait");
        assert_eq!(
            terminal_gap.1.as_deref(),
            Some("live_gap_p2_capacity_deferred")
        );
        assert!(terminal_gap.2.is_some());
        assert_eq!(terminal_gap.3, 0);
        let precise_status: String = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open precise completion evidence")
            .query_row(
                "SELECT status FROM library_change_queue
                     WHERE root_id = ?1 AND relative_path = ?2",
                rusqlite::params![fixture.root_id, relative_path],
                |row| row.get(0),
            )
            .expect("load precise completion evidence");
        assert_eq!(precise_status, "completed");
        fixture.assert_no_automatic_full_scan();
        drop(filler_leases);
        drop(pause);
        production
            .stop()
            .expect("stop leased-capacity production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_offline_then_available_root_preserves_live_gap_lineage_until_consumed() {
        let fixture = ProductionGapFixture::new("offline-available", 0);
        let factory = QueuedSourceFactory::default();
        let mut production = fast_gap_runtime(factory, test_live_only_connection());
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("start availability production observer");
        let offline_root = fixture.source_root.with_file_name("source-offline");
        std::fs::rename(&fixture.source_root, &offline_root)
            .expect("make production root unavailable");
        let unavailable = poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("observe unavailable production root");
        assert!(matches!(
            unavailable.roots[0].availability,
            crate::domain::LibraryRootAvailability::Offline
                | crate::domain::LibraryRootAvailability::Missing
        ));
        std::fs::rename(&offline_root, &fixture.source_root)
            .expect("restore production root availability");

        let gap = wait_for_durable_gap(&mut production, &fixture);
        assert_p0_live_gap(&gap);
        let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_p2_gap_consumer(&fixture, gap.change_id);
        fixture.assert_no_automatic_full_scan();
        production
            .stop()
            .expect("stop availability production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_restart_recovers_an_expired_live_gap_lease_and_retains_its_consumer_lineage() {
        let fixture = ProductionGapFixture::new("restart-expired-gap", 0);
        let factory = QueuedSourceFactory::default();
        let mut first = fast_gap_runtime(factory.clone(), test_live_only_connection());
        poll_runtime_with_storage(&mut first, &fixture.storage)
            .expect("start pre-crash production observer");
        first.is_stopping = true;
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 1,
                ignored_callback_count: 0,
                last_issue_code: Some("change_source_ingress_overflow".to_owned()),
            },
        );
        let gap = wait_for_durable_gap(&mut first, &fixture);
        assert_p0_live_gap(&gap);
        let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open crash-boundary catalog");
        let updated = connection
            .execute(
                "UPDATE library_change_queue
                 SET status = 'leased', attempt_count = attempt_count + 1,
                     lease_generation = lease_generation + 1,
                     lease_expires_unix_ms = 0, next_retry_unix_ms = NULL
                 WHERE id = ?1 AND status = 'pending'",
                [gap.change_id],
            )
            .expect("persist simulated crashed live lease");
        assert_eq!(updated, 1);
        drop(connection);
        drop(first);

        let mut restarted = fast_gap_runtime(factory, test_live_only_connection());
        let snapshot = drive_gap_to_synchronized(&mut restarted, &fixture);

        assert_eq!(snapshot.roots[0].pending_change_count, 0);
        assert_eq!(snapshot.roots[0].retry_wait_count, 0);
        assert_p2_gap_consumer(&fixture, gap.change_id);
        fixture.assert_no_automatic_full_scan();
        restarted.stop().expect("stop restarted production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn production_continuous_journal_range_owns_and_consumes_live_gap_before_supersession() {
        let fixture = ProductionGapFixture::new("journal-covered-gap", 0);
        seed_current_gap_journal(&fixture);
        let factory = QueuedSourceFactory::default();
        let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(AdvancingJournalSession {
            boundaries: Mutex::new(std::collections::VecDeque::from([20, 21, 21])),
            query_count: Arc::clone(&query_count),
            shared_read_count: Arc::clone(&shared_read_count),
            read_release: Arc::new(AtomicBool::new(true)),
            close_count: Arc::clone(&close_count),
        });
        let mut production = fast_gap_runtime(
            factory.clone(),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [17; 16],
        });
        let initial_deadline = Instant::now() + Duration::from_secs(3);
        loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("establish current production journal");
            if query_count.load(Ordering::Acquire) >= 1 && production.journal.is_none() {
                break;
            }
            assert!(
                Instant::now() < initial_deadline,
                "initial current journal query did not settle"
            );
            thread::sleep(Duration::from_millis(2));
        }
        push_gap_batch(
            &factory,
            &fixture.root_id,
            crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 1,
                ignored_callback_count: 0,
                last_issue_code: Some("change_source_ingress_overflow".to_owned()),
            },
        );

        let gap = wait_for_durable_gap(&mut production, &fixture);
        assert_p0_live_gap(&gap);
        let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_journal_gap_consumer(&fixture, gap.change_id);
        assert!(shared_read_count.load(Ordering::Acquire) >= 1);
        fixture.assert_no_automatic_full_scan();
        production
            .stop()
            .expect("stop journal-gap production runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn first_import_preflight_waits_until_the_runtime_is_ready_to_poll() {
        let empty_registry = SynchronizationRuntimeRegistry::default();
        assert_eq!(
            first_import_runtime_owner_for_poll(&empty_registry).expect("inspect an empty runtime"),
            None
        );

        let starting_registry = SynchronizationRuntimeRegistry::default();
        let starting_owner = reserve_runtime_start_ticket(&starting_registry)
            .expect("reserve the starting runtime owner");
        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start_with_ticket(&starting_registry, starting_owner)
                .expect("claim the starting runtime")
        else {
            panic!("an empty runtime must enter Starting")
        };
        assert_eq!(
            first_import_runtime_owner_for_poll(&starting_registry)
                .expect("inspect a starting runtime"),
            None
        );

        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let ready_registry = ready_test_registry(runtime_with_close_probe(close_count));
        let ready_owner = active_owner_ticket(&ready_registry);
        assert_eq!(
            first_import_runtime_owner_for_poll(&ready_registry).expect("inspect a ready runtime"),
            Some(ready_owner)
        );

        let _poll_claim = claim_runtime_for_poll_with_ticket(&ready_registry, ready_owner)
            .expect("claim the ready runtime for polling");
        assert_eq!(
            first_import_runtime_owner_for_poll(&ready_registry)
                .expect("inspect a polling runtime"),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn first_import_preflight_rejects_stopping_and_draining_runtimes() {
        let stopping_registry = SynchronizationRuntimeRegistry::default();
        let stopping_owner = reserve_runtime_start_ticket(&stopping_registry)
            .expect("reserve the stopping runtime owner");
        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start_with_ticket(&stopping_registry, stopping_owner)
                .expect("claim the runtime before stopping")
        else {
            panic!("an empty runtime must enter Starting")
        };
        reserve_runtime_stop_fence(&stopping_registry, Duration::from_secs(1))
            .expect("move the starting runtime to Stopping");
        let stopping_error = first_import_runtime_owner_for_poll(&stopping_registry)
            .expect_err("Stopping must terminate first-import preflight");
        assert_eq!(
            stopping_error.code,
            "library_synchronization_stop_in_progress"
        );

        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let draining_registry = ready_test_registry(runtime_with_close_probe(close_count));
        reserve_runtime_stop_fence(&draining_registry, Duration::from_secs(1))
            .expect("move the ready runtime to Draining");
        let draining_error = first_import_runtime_owner_for_poll(&draining_registry)
            .expect_err("Draining must terminate first-import preflight");
        assert_eq!(
            draining_error.code,
            "library_synchronization_stop_in_progress"
        );
    }

    #[cfg(windows)]
    #[test]
    fn stop_before_delayed_start_admission_prevents_a_ghost_runtime() {
        let registry = Arc::new(SynchronizationRuntimeRegistry::default());
        let owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve the pre-stop owner ticket");
        let release_start = Arc::new(std::sync::Barrier::new(2));
        let constructor_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let start_registry = Arc::clone(&registry);
        let start_release = Arc::clone(&release_start);
        let calls = Arc::clone(&constructor_calls);

        let start = thread::spawn(move || {
            start_release.wait();
            start_runtime_with_ticket(
                &start_registry,
                owner_ticket,
                || -> ProductionSynchronization {
                    calls.fetch_add(1, Ordering::AcqRel);
                    panic!("a cancelled request must not reach construction")
                },
            )
        });

        stop_runtime(&registry).expect("an empty registry still accepts the stop boundary");
        release_start.wait();
        let error = start
            .join()
            .expect("delayed start thread")
            .expect_err("the pre-stop request must be rejected");

        assert_eq!(error.code, "library_synchronization_start_cancelled");
        assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
    }

    #[cfg(windows)]
    #[test]
    fn old_poll_ticket_cannot_attach_to_a_replacement_epoch() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let old_owner_ticket = active_owner_ticket(&registry);

        stop_runtime(&registry).expect("stop the original owner");
        let new_owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve replacement owner ticket");
        let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
            claim_runtime_for_start_with_ticket(&registry, new_owner_ticket)
                .expect("admit the replacement owner")
        else {
            panic!("a replacement owner must construct a new epoch")
        };
        let replacement = runtime_with_close_probe(Arc::clone(&close_count));
        let replacement = match promote_constructed_runtime(
            &registry,
            epoch,
            new_owner_ticket,
            Arc::clone(&cancelled),
            replacement,
        )
        .expect("promote the replacement owner")
        {
            ConstructedRuntimeDisposition::Poll(runtime) => runtime,
            ConstructedRuntimeDisposition::Drain { .. } => {
                panic!("a newly admitted owner must not inherit the old stop fence")
            }
        };
        execute_runtime_operation(
            &registry,
            epoch,
            new_owner_ticket,
            cancelled,
            Arc::clone(&replacement),
            |_| Ok(()),
        )
        .expect("publish the replacement owner as ready");

        let error = match claim_runtime_for_poll_with_ticket(&registry, old_owner_ticket) {
            Err(error) => error,
            Ok(_) => panic!("an old poll ticket must not claim the replacement runtime"),
        };

        assert_eq!(error.code, "library_synchronization_lifecycle_ticket_stale");
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready {
            epoch: active_epoch,
            owner_ticket,
            runtime,
            ..
        } = &state.entry
        else {
            panic!("the rejected old poll must leave the replacement ready")
        };
        assert_eq!(*active_epoch, epoch);
        assert_eq!(*owner_ticket, new_owner_ticket);
        assert!(Arc::ptr_eq(runtime, &replacement));
    }

    #[cfg(windows)]
    #[test]
    fn same_start_ticket_can_retry_but_a_different_ticket_cannot_attach() {
        let registry = SynchronizationRuntimeRegistry::default();
        let owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve the original owner ticket");
        let SynchronizationRuntimeClaim::Construct { epoch, .. } =
            claim_runtime_for_start_with_ticket(&registry, owner_ticket)
                .expect("admit the original owner")
        else {
            panic!("the original owner must construct")
        };
        recover_starting_runtime_panic(&registry, epoch, owner_ticket)
            .expect("recover the failed construction");

        let SynchronizationRuntimeClaim::Construct {
            epoch: retry_epoch, ..
        } = claim_runtime_for_start_with_ticket(&registry, owner_ticket)
            .expect("the same bounded request may retry after recovery")
        else {
            panic!("the same owner ticket must construct a replacement epoch")
        };
        assert_eq!(retry_epoch, epoch + 1);

        let different_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve a different request ticket");
        let error = match claim_runtime_for_start_with_ticket(&registry, different_ticket) {
            Err(error) => error,
            Ok(_) => panic!("a different request must not attach to the starting owner"),
        };
        assert_eq!(error.code, "library_synchronization_start_in_progress");
        assert_eq!(active_owner_ticket(&registry), owner_ticket);
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_boundary_rejects_zero_and_unissued_start_tickets() {
        for owner_ticket in [
            0,
            RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)).raw(),
        ] {
            let registry = SynchronizationRuntimeRegistry::default();
            let constructor_calls = std::sync::atomic::AtomicUsize::new(0);

            let error = RuntimeStartTicket::decode(owner_ticket)
                .and_then(|owner_ticket| {
                    start_runtime_with_ticket(
                        &registry,
                        owner_ticket,
                        || -> ProductionSynchronization {
                            constructor_calls.fetch_add(1, Ordering::AcqRel);
                            panic!("an invalid ticket must not reach construction")
                        },
                    )
                })
                .expect_err("a zero or unissued start ticket must fail closed");

            assert_eq!(
                error.code,
                "library_synchronization_lifecycle_ticket_invalid"
            );
            assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
            let state = registry.state.lock().expect("local registry");
            assert_eq!(state.next_epoch, 0);
            assert_eq!(state.next_lifecycle_ordinal, LifecycleOrdinal(0));
            assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
            assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
        }
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_boundary_rejects_zero_and_unadmitted_stop_fences() {
        let registry = SynchronizationRuntimeRegistry::default();

        for raw in [0, RuntimeStopFence::KIND] {
            let zero_error = RuntimeStopFence::decode(raw)
                .and_then(|fence| stop_runtime_with_fence(&registry, fence))
                .expect_err("a zero-ordinal stop fence must fail closed");
            assert_eq!(
                zero_error.code,
                "library_synchronization_lifecycle_fence_invalid"
            );
        }

        let unadmitted_start =
            reserve_runtime_start_ticket(&registry).expect("reserve a non-fence ticket");
        let unadmitted_raw = RuntimeStopFence::from_ordinal(unadmitted_start.ordinal()).raw();
        let unadmitted_error = RuntimeStopFence::decode(unadmitted_raw)
            .and_then(|fence| stop_runtime_with_fence(&registry, fence))
            .expect_err("a same-kind but unissued stop fence must fail closed");
        assert_eq!(
            unadmitted_error.code,
            "library_synchronization_lifecycle_fence_invalid"
        );

        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.next_epoch, 0);
        assert_eq!(state.next_lifecycle_ordinal, unadmitted_start.ordinal());
        assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
        assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
    }

    #[cfg(windows)]
    #[test]
    fn stop_fence_capacity_failure_preserves_all_unconsumed_admission_state() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let fences = (0..MAX_ADMITTED_STOP_FENCES)
            .map(|_| {
                reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
                    .expect("reserve one bounded unconsumed stop fence")
            })
            .collect::<Vec<_>>();
        let first_fence = fences[0];
        let (next_ordinal, cancelled_through, ledger, owner, deadline, runtime) = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Draining {
                cancellation_fence,
                deadline,
                runtime,
                ..
            } = &state.entry
            else {
                panic!("the first fence must linearize the owner into draining")
            };
            assert_eq!(*cancellation_fence, Some(first_fence));
            (
                state.next_lifecycle_ordinal,
                state.cancelled_through_ordinal,
                state.admitted_stop_fences.clone(),
                state.entry.owner_identity(),
                *deadline,
                Arc::clone(runtime),
            )
        };

        let error = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect_err("a full unconsumed ledger must reject another stop fence");

        assert_eq!(
            error.code,
            "library_synchronization_lifecycle_fence_capacity_exhausted"
        );
        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.next_lifecycle_ordinal, next_ordinal);
        assert_eq!(state.cancelled_through_ordinal, cancelled_through);
        assert_eq!(state.admitted_stop_fences, ledger);
        assert_eq!(state.entry.owner_identity(), owner);
        let SynchronizationRuntimeEntry::Draining {
            cancellation_fence,
            deadline: active_deadline,
            runtime: active_runtime,
            ..
        } = &state.entry
        else {
            panic!("capacity rejection must preserve the draining owner")
        };
        assert_eq!(*cancellation_fence, Some(first_fence));
        assert_eq!(*active_deadline, deadline);
        assert!(Arc::ptr_eq(active_runtime, &runtime));
        assert!(
            state
                .admitted_stop_fences
                .iter()
                .all(|entry| entry.use_state == AdmittedStopFenceUse::Pending)
        );
        drop(state);

        stop_runtime_with_fence(&registry, first_fence)
            .expect("an existing admitted fence remains usable after capacity rejection");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn consumed_stop_fence_history_is_reclaimed_only_under_capacity_pressure() {
        let registry = SynchronizationRuntimeRegistry::default();
        let fences = (0..MAX_ADMITTED_STOP_FENCES)
            .map(|_| {
                reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
                    .expect("fill the bounded stop-fence ledger")
            })
            .collect::<Vec<_>>();
        let consumed = fences[0];
        stop_runtime_with_fence(&registry, consumed).expect("consume the oldest empty fence");
        {
            let state = registry.state.lock().expect("local registry");
            assert_eq!(state.admitted_stop_fences.len(), MAX_ADMITTED_STOP_FENCES);
            assert_eq!(
                state
                    .admitted_stop_fences
                    .front()
                    .map(|entry| entry.use_state),
                Some(AdmittedStopFenceUse::Consumed)
            );
        }

        let replacement = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("capacity pressure may reclaim consumed empty history");
        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.admitted_stop_fences.len(), MAX_ADMITTED_STOP_FENCES);
        assert!(
            !state
                .admitted_stop_fences
                .iter()
                .any(|entry| entry.fence == consumed)
        );
        assert_eq!(
            state.admitted_stop_fences.back().map(|entry| entry.fence),
            Some(replacement)
        );
        drop(state);

        let error = stop_runtime_with_fence(&registry, consumed)
            .expect_err("reclaimed history no longer proves exact issuance");
        assert_eq!(
            error.code,
            "library_synchronization_lifecycle_fence_invalid"
        );
    }

    #[cfg(windows)]
    #[test]
    fn consumed_fence_covering_a_draining_owner_survives_capacity_pressure() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let runtime = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let runtime_guard = runtime.lock().expect("hold the native owner");
        let deadline = Instant::now() + Duration::from_millis(20);
        let first_fence = reserve_runtime_stop_fence_until(&registry, deadline)
            .expect("reserve the first owner-covering fence");
        thread::sleep(Duration::from_millis(30));
        let timeout = stop_runtime_with_fence(&registry, first_fence)
            .expect_err("the held owner must exhaust the first deadline");
        assert_eq!(timeout.code, "library_synchronization_stop_timeout");

        for _ in 1..MAX_ADMITTED_STOP_FENCES {
            reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
                .expect("fill remaining ledger slots without dropping the active fence");
        }
        let capacity_error = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect_err("the consumed active-owner fence is not reclaimable");
        assert_eq!(
            capacity_error.code,
            "library_synchronization_lifecycle_fence_capacity_exhausted"
        );
        assert!(
            registry
                .state
                .lock()
                .expect("local registry")
                .admitted_stop_fences
                .iter()
                .any(|entry| {
                    entry.fence == first_fence && entry.use_state == AdmittedStopFenceUse::Consumed
                })
        );

        drop(runtime_guard);
        let reap_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match stop_runtime_with_fence(&registry, first_fence) {
                Ok(()) => break,
                Err(error)
                    if error.code == "library_synchronization_stop_timeout"
                        && Instant::now() < reap_deadline =>
                {
                    thread::yield_now();
                }
                Err(error) => panic!("the retained fence did not reap its owner: {error:?}"),
            }
        }
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn repeated_and_concurrent_calls_share_one_admitted_stop_fence() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("reserve one shared stop fence");
        let first_registry = Arc::clone(&registry);
        let second_registry = Arc::clone(&registry);

        let first = thread::spawn(move || stop_runtime_with_fence(&first_registry, fence));
        let second = thread::spawn(move || stop_runtime_with_fence(&second_registry, fence));
        first
            .join()
            .expect("join first stop caller")
            .expect("first stop caller succeeds");
        second
            .join()
            .expect("join second stop caller")
            .expect("second stop caller converges");
        stop_runtime_with_fence(&registry, fence).expect("a repeated stop remains idempotent");

        assert_eq!(close_count.load(Ordering::Acquire), 1);
        let state = registry.state.lock().expect("local registry");
        assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
        assert_eq!(
            state
                .admitted_stop_fences
                .iter()
                .find(|entry| entry.fence == fence)
                .map(|entry| entry.use_state),
            Some(AdmittedStopFenceUse::Consumed)
        );
    }

    #[cfg(windows)]
    #[test]
    fn empty_stop_fence_is_a_delayed_noop_for_a_newer_owner() {
        let registry = SynchronizationRuntimeRegistry::default();
        let old_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("reserve an empty-registry stop fence");
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve a newer start owner");
        let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
            claim_runtime_for_start_with_ticket(&registry, owner_ticket)
                .expect("admit the newer owner")
        else {
            panic!("the newer owner must construct")
        };
        let runtime = runtime_with_close_probe(Arc::clone(&close_count));
        let runtime = match promote_constructed_runtime(
            &registry,
            epoch,
            owner_ticket,
            Arc::clone(&cancelled),
            runtime,
        )
        .expect("promote the newer owner")
        {
            ConstructedRuntimeDisposition::Poll(runtime) => runtime,
            ConstructedRuntimeDisposition::Drain { .. } => {
                panic!("an old empty fence must not attach to a newer owner")
            }
        };
        execute_runtime_operation(
            &registry,
            epoch,
            owner_ticket,
            cancelled,
            Arc::clone(&runtime),
            |_| Ok(()),
        )
        .expect("publish the newer owner as ready");

        stop_runtime_with_fence(&registry, old_fence)
            .expect("the old empty fence is an idempotent no-op");
        assert_eq!(close_count.load(Ordering::Acquire), 0);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Ready { .. }
        ));

        stop_runtime(&registry).expect("a fresh fence drains the newer owner");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_boundary_rejects_a_start_ticket_after_a_later_stop_fence_is_admitted() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let start_ticket = active_owner_ticket(&registry);
        let stop_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("reserve the matching stop fence");
        let forged_fence = start_ticket.raw() | RuntimeStopFence::KIND;

        let error = RuntimeStopFence::decode(forged_fence)
            .and_then(|fence| stop_runtime_with_fence(&registry, fence))
            .expect_err("changing a start ticket kind bit must not forge an admitted stop fence");

        assert_eq!(
            error.code,
            "library_synchronization_lifecycle_fence_invalid"
        );
        assert_eq!(close_count.load(Ordering::Acquire), 0);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Draining { .. }
        ));

        stop_runtime_with_fence(&registry, stop_fence)
            .expect("the admitted stop fence must drain its matching owner");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_boundary_epoch_exhaustion_restores_empty_state() {
        let registry = SynchronizationRuntimeRegistry::default();
        let owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve the final start owner");
        {
            let mut state = registry.state.lock().expect("local registry");
            state.next_epoch = u64::MAX;
        }
        let constructor_calls = std::sync::atomic::AtomicUsize::new(0);

        let error =
            start_runtime_with_ticket(&registry, owner_ticket, || -> ProductionSynchronization {
                constructor_calls.fetch_add(1, Ordering::AcqRel);
                panic!("an exhausted epoch must not reach construction")
            })
            .expect_err("an exhausted epoch allocator must fail closed");

        assert_eq!(error.code, "library_synchronization_epoch_exhausted");
        assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.next_epoch, u64::MAX);
        assert_eq!(state.next_lifecycle_ordinal, owner_ticket.ordinal());
        assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
        assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_boundary_old_stop_fence_cannot_drain_a_newer_epoch() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let old_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("reserve the original owner fence");
        stop_runtime_with_fence(&registry, old_fence).expect("drain the original owner");
        assert_eq!(close_count.load(Ordering::Acquire), 1);

        let new_owner_ticket =
            reserve_runtime_start_ticket(&registry).expect("reserve a newer owner ticket");
        let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
            claim_runtime_for_start_with_ticket(&registry, new_owner_ticket)
                .expect("admit the newer owner")
        else {
            panic!("the newer owner must construct a new epoch")
        };
        let replacement = runtime_with_close_probe(Arc::clone(&close_count));
        let replacement = match promote_constructed_runtime(
            &registry,
            epoch,
            new_owner_ticket,
            Arc::clone(&cancelled),
            replacement,
        )
        .expect("promote the newer owner")
        {
            ConstructedRuntimeDisposition::Poll(runtime) => runtime,
            ConstructedRuntimeDisposition::Drain { .. } => {
                panic!("the newer owner must not inherit the old fence")
            }
        };
        execute_runtime_operation(
            &registry,
            epoch,
            new_owner_ticket,
            cancelled,
            Arc::clone(&replacement),
            |_| Ok(()),
        )
        .expect("publish the newer owner as ready");

        stop_runtime_with_fence(&registry, old_fence)
            .expect("the stale fence is an idempotent no-op for a newer owner");

        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            runtime,
            ..
        } = &state.entry
        else {
            panic!("the stale fence must leave the newer owner ready")
        };
        assert_eq!(*active_epoch, epoch);
        assert_eq!(*active_owner_ticket, new_owner_ticket);
        assert!(Arc::ptr_eq(runtime, &replacement));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        drop(state);

        stop_runtime(&registry).expect("drain the newer owner with a fresh fence");
        assert_eq!(close_count.load(Ordering::Acquire), 2);
    }

    #[cfg(windows)]
    #[test]
    fn lifecycle_ticket_and_stop_fence_overflow_fail_closed() {
        let registry = SynchronizationRuntimeRegistry::default();
        {
            let mut state = registry.state.lock().expect("local registry");
            state.next_lifecycle_ordinal = LifecycleOrdinal(LifecycleOrdinal::MAX);
            state.cancelled_through_ordinal = LifecycleOrdinal(LifecycleOrdinal::MAX - 1);
        }

        let start_error = reserve_runtime_start_ticket(&registry)
            .expect_err("an exhausted ticket allocator must reject start");
        let stop_error = reserve_runtime_stop_fence(&registry, Duration::from_secs(2))
            .expect_err("an exhausted ticket allocator must reject stop");

        assert_eq!(
            start_error.code,
            "library_synchronization_lifecycle_ticket_exhausted"
        );
        assert_eq!(
            stop_error.code,
            "library_synchronization_lifecycle_ticket_exhausted"
        );
        let state = registry.state.lock().expect("local registry");
        assert_eq!(
            state.next_lifecycle_ordinal,
            LifecycleOrdinal(LifecycleOrdinal::MAX)
        );
        assert_eq!(
            state.cancelled_through_ordinal,
            LifecycleOrdinal(LifecycleOrdinal::MAX - 1)
        );
        assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
    }

    #[cfg(windows)]
    #[test]
    fn delayed_stop_task_uses_the_deadline_reserved_by_the_sync_fence() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let runtime = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let runtime_guard = runtime.lock().expect("hold the native owner");
        let deadline = Instant::now() + Duration::from_millis(20);
        let cancellation_fence = reserve_runtime_stop_fence_until(&registry, deadline)
            .expect("the public stop call reserves its fence and deadline");
        thread::sleep(Duration::from_millis(30));

        let started = Instant::now();
        let error = stop_runtime_with_fence(&registry, cancellation_fence)
            .expect_err("a delayed async stop task must not receive a fresh deadline");

        assert_eq!(error.code, "library_synchronization_stop_timeout");
        assert!(started.elapsed() < Duration::from_millis(20));
        assert_eq!(retained_stop_deadline(&registry), Some(deadline));
        drop(runtime_guard);
        let reap_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match stop_runtime_with_fence(&registry, cancellation_fence) {
                Ok(()) => break,
                Err(error)
                    if error.code == "library_synchronization_stop_timeout"
                        && Instant::now() < reap_deadline =>
                {
                    thread::yield_now();
                }
                Err(error) => panic!("the retained fence did not reap its owner: {error:?}"),
            }
        }
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn panicked_constructor_is_contained_without_clearing_a_later_epoch() {
        let registry = SynchronizationRuntimeRegistry::default();

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            start_runtime_with(&registry, || -> ProductionSynchronization {
                panic!("secret constructor panic payload")
            })
        }));

        let error = outcome
            .expect("the registry boundary must contain the constructor panic")
            .expect_err("a contained constructor panic remains a structured failure");
        assert_eq!(error.code, "library_synchronization_owner_panicked");
        assert_eq!(
            error.message,
            "The library synchronization owner panicked and entered bounded recovery"
        );
        assert!(!error.message.contains("secret constructor panic payload"));
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));

        let SynchronizationRuntimeClaim::Construct {
            epoch: next_epoch, ..
        } = claim_runtime_for_start(&registry).expect("constructor recovery admits a later epoch")
        else {
            panic!("constructor recovery must leave an empty registry")
        };
        assert_eq!(next_epoch, 2);
        recover_starting_runtime_panic(
            &registry,
            1,
            RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)),
        )
        .expect("ignore stale constructor recovery");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Starting { epoch: 2, .. }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn panicked_constructor_releases_a_concurrent_stopper_with_the_same_deadline() {
        let registry = Arc::new(SynchronizationRuntimeRegistry::default());
        let constructor_entered = Arc::new(AtomicBool::new(false));
        let constructor_release = Arc::new(AtomicBool::new(false));
        let owner_registry = Arc::clone(&registry);
        let owner_entered = Arc::clone(&constructor_entered);
        let owner_release = Arc::clone(&constructor_release);
        let owner = thread::spawn(move || {
            start_runtime_with(&owner_registry, move || -> ProductionSynchronization {
                owner_entered.store(true, Ordering::Release);
                while !owner_release.load(Ordering::Acquire) {
                    thread::yield_now();
                }
                panic!("secret concurrent constructor panic payload")
            })
        });
        wait_for_test_signal(&constructor_entered);

        let stop_deadline = Instant::now() + Duration::from_millis(500);
        let stop_registry = Arc::clone(&registry);
        let stopper = thread::spawn(move || stop_runtime_until(&stop_registry, stop_deadline));
        let transition_deadline = Instant::now() + Duration::from_millis(100);
        loop {
            if matches!(
                registry.state.lock().expect("local registry").entry,
                SynchronizationRuntimeEntry::Stopping { .. }
            ) {
                break;
            }
            assert!(
                Instant::now() < transition_deadline,
                "concurrent stop did not claim the constructor epoch"
            );
            thread::yield_now();
        }
        assert_eq!(retained_stop_deadline(&registry), Some(stop_deadline));

        constructor_release.store(true, Ordering::Release);
        let owner_error = owner
            .join()
            .expect("join panicked-constructor owner boundary")
            .expect_err("constructor panic remains a structured failure");
        assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
        assert!(!owner_error.message.contains("secret concurrent"));
        stopper
            .join()
            .expect("join concurrent stopper")
            .expect("constructor recovery notifies the concurrent stopper");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));

        let SynchronizationRuntimeClaim::Construct { epoch, .. } =
            claim_runtime_for_start(&registry).expect("constructor recovery admits a new epoch")
        else {
            panic!("constructor recovery must leave the registry empty")
        };
        assert_eq!(epoch, 2);
    }

    #[cfg(windows)]
    #[test]
    fn panicked_poll_owner_is_contained_and_releases_the_registry_epoch() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim panic poll owner");
        let owner_ticket = active_owner_ticket(&registry);

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_runtime_operation(
                &registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                |_| -> Result<(), ScanError> { panic!("controlled poll owner panic") },
            )
        }));

        let error = outcome
            .expect("the registry boundary must contain the poll owner panic")
            .expect_err("a contained owner panic remains a structured failure");
        assert_eq!(error.code, "library_synchronization_owner_panicked");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        assert_eq!(close_count.load(Ordering::Acquire), 1);

        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start(&registry).expect("panic cleanup permits the next epoch")
        else {
            panic!("panic cleanup must leave an empty registry")
        };
    }

    #[cfg(windows)]
    #[test]
    fn panicked_poll_racing_stop_before_deadline_drains_the_same_owner_once() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let runtime_identity = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim racing panic owner");
        let owner_ticket = active_owner_ticket(&registry);
        let operation_entered = Arc::new(AtomicBool::new(false));
        let operation_release = Arc::new(AtomicBool::new(false));
        let owner_registry = Arc::clone(&registry);
        let owner_entered = Arc::clone(&operation_entered);
        let owner_release = Arc::clone(&operation_release);
        let owner = thread::spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                move |_| -> Result<(), ScanError> {
                    owner_entered.store(true, Ordering::Release);
                    while !owner_release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    panic!("controlled racing poll panic")
                },
            )
        });
        wait_for_test_signal(&operation_entered);

        let stop_deadline = Instant::now() + Duration::from_millis(500);
        let stop_registry = Arc::clone(&registry);
        let stopper = thread::spawn(move || stop_runtime_until(&stop_registry, stop_deadline));
        assert!(wait_for_draining_state(
            &registry,
            Duration::from_millis(100)
        ));
        {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Draining {
                deadline, runtime, ..
            } = &state.entry
            else {
                panic!("concurrent stop must publish Draining before cleanup")
            };
            assert_eq!(*deadline, stop_deadline);
            assert!(Arc::ptr_eq(runtime, &runtime_identity));
        }

        operation_release.store(true, Ordering::Release);
        let owner_error = owner
            .join()
            .expect("join racing poll owner boundary")
            .expect_err("the racing poll panic remains structured");
        assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
        stopper
            .join()
            .expect("join racing stopper")
            .expect("one drainer completes and the stale drainer exits cleanly");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(!runtime_identity.is_poisoned());
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
    }

    #[cfg(windows)]
    #[test]
    fn panicked_poll_reports_stable_error_after_stop_drains_and_restarts_epoch() {
        let old_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&old_close_count)));
        let reached = Arc::new(std::sync::Barrier::new(2));
        let resume = Arc::new(std::sync::Barrier::new(2));
        *registry
            .panic_retention_barrier
            .lock()
            .expect("install panic retention barrier") = Some(PanicRetentionBarrier {
            reached: Arc::clone(&reached),
            resume: Arc::clone(&resume),
        });
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim panicked poll owner");
        let owner_ticket = active_owner_ticket(&registry);
        let owner_registry = Arc::clone(&registry);
        let owner = thread::spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                |_| -> Result<(), ScanError> { panic!("secret raced panic payload") },
            )
        });

        reached.wait();
        stop_runtime_until(&registry, Instant::now() + Duration::from_millis(500))
            .expect("concurrent stop drains the old owner before panic reconciliation");
        assert_eq!(old_close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));

        let SynchronizationRuntimeClaim::Construct {
            epoch: new_epoch,
            cancelled: new_cancelled,
        } = claim_runtime_for_start(&registry).expect("admit replacement epoch")
        else {
            panic!("a drained owner must admit replacement construction")
        };
        let new_owner_ticket = active_owner_ticket(&registry);
        assert_eq!(new_epoch, epoch + 1);
        let new_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut replacement = runtime_with_close_probe(Arc::clone(&new_close_count));
        replacement.stop_requested = Arc::clone(&new_cancelled);
        let ConstructedRuntimeDisposition::Poll(new_runtime) = promote_constructed_runtime(
            &registry,
            new_epoch,
            new_owner_ticket,
            Arc::clone(&new_cancelled),
            replacement,
        )
        .expect("install replacement runtime") else {
            panic!("a fresh replacement epoch must enter polling")
        };

        resume.wait();
        let owner_error = owner
            .join()
            .expect("join panicked old owner")
            .expect_err("the old owner panic remains a structured failure");
        assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
        assert_eq!(
            owner_error.message,
            "The library synchronization owner panicked and entered bounded recovery"
        );
        assert!(!owner_error.message.contains("secret raced panic payload"));
        {
            let state = registry.state.lock().expect("replacement registry");
            let SynchronizationRuntimeEntry::Polling {
                epoch: active_epoch,
                cancelled,
                runtime,
                ..
            } = &state.entry
            else {
                panic!("old panic reconciliation must preserve the replacement epoch")
            };
            assert_eq!(*active_epoch, new_epoch);
            assert!(!cancelled.load(Ordering::Acquire));
            assert!(Arc::ptr_eq(runtime, &new_runtime));
        }
        assert_eq!(new_close_count.load(Ordering::Acquire), 0);

        stop_runtime_until(&registry, Instant::now() + Duration::from_millis(500))
            .expect("clean replacement epoch");
        assert_eq!(new_close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn panicked_poll_cannot_install_an_external_runtime_into_stopping() {
        let registered_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(
            &registered_close_count,
        )));
        let deadline = Instant::now() + Duration::from_millis(500);
        let owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1));
        let cancellation_fence = RuntimeStopFence::from_ordinal(LifecycleOrdinal(2));
        {
            let mut state = registry.state.lock().expect("local registry");
            state.next_lifecycle_ordinal = cancellation_fence.ordinal();
            state.cancelled_through_ordinal = cancellation_fence.ordinal();
            state.entry = SynchronizationRuntimeEntry::Stopping {
                epoch: 1,
                owner_ticket,
                cancellation_fence,
                deadline,
            };
        }
        let external_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let external_runtime = Arc::new(Mutex::new(runtime_with_close_probe(Arc::clone(
            &external_close_count,
        ))));
        let cancelled = Arc::new(AtomicBool::new(false));

        let error = match retain_panicked_runtime(
            &registry,
            1,
            owner_ticket,
            &cancelled,
            &external_runtime,
        ) {
            Ok(_) => {
                panic!("runtime-less Stopping cannot adopt a poll owner's external runtime")
            }
            Err(error) => error,
        };

        assert_eq!(error.code, "library_synchronization_owner_state_invalid");
        assert!(cancelled.load(Ordering::Acquire));
        {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Stopping {
                epoch,
                deadline: active_deadline,
                ..
            } = state.entry
            else {
                panic!("failed reconciliation must preserve runtime-less Stopping")
            };
            assert_eq!(epoch, 1);
            assert_eq!(active_deadline, deadline);
        }
        assert_eq!(registered_close_count.load(Ordering::Acquire), 0);
        assert_eq!(external_close_count.load(Ordering::Acquire), 0);
    }

    #[cfg(windows)]
    #[test]
    fn panicked_poll_retains_all_workers_and_the_expired_public_stop_deadline() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let mut production = runtime_with_blocking_close(
            Arc::clone(&close_count),
            Arc::clone(&close_entered),
            Arc::clone(&close_release),
            Arc::clone(&close_finished),
        );

        let p0_cancelled = Arc::new(AtomicBool::new(false));
        let p0_release = Arc::new(AtomicBool::new(false));
        let p0_finished = Arc::new(AtomicBool::new(false));
        let (p0_sender, p0_receiver) = mpsc::sync_channel(1);
        let p0_worker_cancelled = Arc::clone(&p0_cancelled);
        let p0_worker_release = Arc::clone(&p0_release);
        let p0_worker_finished = Arc::clone(&p0_finished);
        let p0_worker = thread::spawn(move || {
            while !p0_worker_cancelled.load(Ordering::Acquire)
                || !p0_worker_release.load(Ordering::Acquire)
            {
                thread::yield_now();
            }
            p0_worker_finished.store(true, Ordering::Release);
            let _ = p0_sender.send(Ok(AuthoritativeLibraryChangeReport::default()));
        });
        production.live = Some(LiveTask {
            root_id: "panic-p0".to_owned(),
            cancelled: p0_cancelled,
            receiver: p0_receiver,
            worker: Some(p0_worker),
        });

        let p1_cancelled = Arc::new(AtomicBool::new(false));
        let p1_release = Arc::new(AtomicBool::new(false));
        let p1_finished = Arc::new(AtomicBool::new(false));
        let (p1_sender, p1_receiver) = mpsc::sync_channel(1);
        let p1_worker_cancelled = Arc::clone(&p1_cancelled);
        let p1_worker_release = Arc::clone(&p1_release);
        let p1_worker_finished = Arc::clone(&p1_finished);
        let p1_worker = thread::spawn(move || {
            while !p1_worker_cancelled.load(Ordering::Acquire)
                || !p1_worker_release.load(Ordering::Acquire)
            {
                thread::yield_now();
            }
            p1_worker_finished.store(true, Ordering::Release);
            let _ = p1_sender.send(Ok(JournalTaskOutcome::Page(
                PersistentJournalEnrollmentReport::default(),
            )));
        });
        production.journal = Some(JournalTask {
            cancelled: p1_cancelled,
            receiver: p1_receiver,
            worker: Some(p1_worker),
        });

        let p2_cancelled = Arc::new(AtomicBool::new(false));
        let p2_release = Arc::new(AtomicBool::new(false));
        let p2_finished = Arc::new(AtomicBool::new(false));
        let (p2_sender, p2_receiver) = mpsc::sync_channel(1);
        let p2_worker_cancelled = Arc::clone(&p2_cancelled);
        let p2_worker_release = Arc::clone(&p2_release);
        let p2_worker_finished = Arc::clone(&p2_finished);
        let p2_worker = thread::spawn(move || {
            while !p2_worker_cancelled.load(Ordering::Acquire)
                || !p2_worker_release.load(Ordering::Acquire)
            {
                thread::yield_now();
            }
            p2_worker_finished.store(true, Ordering::Release);
            let _ = p2_sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                IncrementalLibraryChangeReport::default(),
            )));
        });
        production.recovery = Some(RecoveryTask {
            root_id: "panic-p2".to_owned(),
            kind: RecoveryTaskKind::LegacyUnownedDrain {
                continuity_revision: 1,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: p2_cancelled,
            receiver: p2_receiver,
            worker: Some(p2_worker),
        });

        let registry = ready_test_registry(production);
        let runtime_identity = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim panicked poll owner");
        let owner_ticket = active_owner_ticket(&registry);
        assert!(Arc::ptr_eq(&runtime_identity, &runtime));
        let operation_entered = Arc::new(AtomicBool::new(false));
        let operation_release = Arc::new(AtomicBool::new(false));
        let owner_registry = Arc::clone(&registry);
        let owner_entered = Arc::clone(&operation_entered);
        let owner_release = Arc::clone(&operation_release);
        let owner = thread::spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                move |_| -> Result<(), ScanError> {
                    owner_entered.store(true, Ordering::Release);
                    while !owner_release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    panic!("secret poll panic after worker creation")
                },
            )
        });
        wait_for_test_signal(&operation_entered);

        let public_deadline = Instant::now() + Duration::from_millis(15);
        let stop_error = stop_runtime_until(&registry, public_deadline)
            .expect_err("the public deadline expires while the poll guard owns the runtime");
        assert_eq!(stop_error.code, "library_synchronization_stop_timeout");
        {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Draining {
                epoch: draining_epoch,
                deadline,
                runtime,
                ..
            } = &state.entry
            else {
                panic!("stop must retain the complete polling runtime in Draining")
            };
            assert_eq!(*draining_epoch, epoch);
            assert_eq!(*deadline, public_deadline);
            assert!(Arc::ptr_eq(runtime, &runtime_identity));
        }

        operation_release.store(true, Ordering::Release);
        let owner_error = owner
            .join()
            .expect("join panicked poll owner boundary")
            .expect_err("a panicked poll owner remains a structured failure");
        assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
        assert!(!owner_error.message.contains("secret poll panic"));
        assert!(!runtime_identity.is_poisoned());
        {
            let runtime = runtime_identity.lock().expect("retained runtime");
            assert!(
                runtime
                    .live
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some()
            );
            assert!(
                runtime
                    .journal
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some()
            );
            assert!(
                runtime
                    .recovery
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some()
            );
        }
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        assert_start_is_blocked_while_stopping(&registry);

        p0_release.store(true, Ordering::Release);
        p1_release.store(true, Ordering::Release);
        p2_release.store(true, Ordering::Release);
        wait_for_test_signal(&p0_finished);
        wait_for_test_signal(&p1_finished);
        wait_for_test_signal(&p2_finished);
        let lane_worker_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let all_finished = {
                let runtime = runtime_identity.lock().expect("retained lane runtime");
                runtime
                    .live
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some_and(JoinHandle::is_finished)
                    && runtime
                        .journal
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished)
                    && runtime
                        .recovery
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished)
            };
            if all_finished {
                break;
            }
            assert!(
                Instant::now() < lane_worker_deadline,
                "retained lane workers did not finish"
            );
            thread::yield_now();
        }
        let close_error = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
            .expect_err("the expired first deadline must remain authoritative at journal close");
        assert_eq!(close_error.code, "library_synchronization_stop_timeout");
        wait_for_test_signal(&close_entered);
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        {
            let runtime = runtime_identity.lock().expect("retained close runtime");
            assert!(runtime.live.is_none());
            assert!(runtime.journal.is_none());
            assert!(runtime.recovery.is_none());
            assert!(runtime.journal_close.is_some());
        }

        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let close_worker_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let finished = runtime_identity
                .lock()
                .expect("retained close runtime")
                .journal_close
                .as_ref()
                .and_then(|task| task.worker.as_ref())
                .is_some_and(JoinHandle::is_finished);
            if finished {
                break;
            }
            assert!(
                Instant::now() < close_worker_deadline,
                "retained close worker did not finish"
            );
            thread::yield_now();
        }
        stop_runtime_until(&registry, public_deadline + Duration::from_secs(2))
            .expect("retry reaps every retained owner under the original deadline");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct {
            epoch: next_epoch, ..
        } = claim_runtime_for_start(&registry).expect("full panic drain admits the next epoch")
        else {
            panic!("the next epoch must wait for the full drain")
        };
        assert_eq!(next_epoch, 2);
    }

    #[cfg(windows)]
    #[test]
    fn one_shot_drain_panics_retain_the_same_owner_and_retry_every_cleanup_boundary() {
        for point in [
            DrainPanicPoint::RequestStop,
            DrainPanicPoint::P0Result,
            DrainPanicPoint::P1Result,
            DrainPanicPoint::P2Result,
            DrainPanicPoint::JournalCloseInstalled,
            DrainPanicPoint::JournalCloseResult,
        ] {
            let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let registry = ready_test_registry(runtime_with_drain_panic(
                point,
                Some(1),
                Arc::clone(&close_count),
            ));
            let runtime_identity = {
                let state = registry.state.lock().expect("local registry");
                let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                    panic!("test runtime must begin ready")
                };
                Arc::clone(runtime)
            };
            let first_deadline = Instant::now() + Duration::from_millis(500);

            let error = stop_runtime_until(&registry, first_deadline)
                .expect_err("the injected drain panic must remain structured");
            assert_eq!(error.code, "library_synchronization_owner_panicked");
            assert_eq!(
                error.message,
                "The library synchronization owner panicked and entered bounded recovery"
            );
            assert!(!runtime_identity.is_poisoned());
            {
                let state = registry.state.lock().expect("local registry");
                let SynchronizationRuntimeEntry::Draining {
                    epoch,
                    deadline,
                    runtime,
                    ..
                } = &state.entry
                else {
                    panic!("a drain panic must fail closed in Draining")
                };
                assert_eq!(*epoch, 1, "unexpected epoch for {point:?}");
                assert_eq!(
                    *deadline, first_deadline,
                    "refreshed deadline for {point:?}"
                );
                assert!(Arc::ptr_eq(runtime, &runtime_identity));
            }
            assert_start_is_blocked_while_stopping(&registry);

            stop_runtime_until(&registry, first_deadline + Duration::from_secs(1))
                .unwrap_or_else(|error| panic!("one-shot {point:?} did not retry: {error:?}"));
            assert_eq!(close_count.load(Ordering::Acquire), 1, "{point:?}");
            assert!(matches!(
                registry.state.lock().expect("local registry").entry,
                SynchronizationRuntimeEntry::Empty
            ));
            let SynchronizationRuntimeClaim::Construct { epoch, .. } =
                claim_runtime_for_start(&registry)
                    .unwrap_or_else(|error| panic!("{point:?} did not permit restart: {error:?}"))
            else {
                panic!("one-shot {point:?} must admit a fresh construction epoch")
            };
            assert_eq!(epoch, 2);
        }
    }

    #[cfg(windows)]
    #[test]
    fn persistent_drain_panic_fails_closed_without_reclosing_or_admitting_aba() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_drain_panic(
            DrainPanicPoint::JournalCloseResult,
            None,
            Arc::clone(&close_count),
        ));
        let runtime_identity = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let first_deadline = Instant::now() + Duration::from_millis(500);

        for requested_deadline in [first_deadline, first_deadline + Duration::from_secs(1)] {
            let error = stop_runtime_until(&registry, requested_deadline)
                .expect_err("a persistent drain panic must remain fail closed");
            assert_eq!(error.code, "library_synchronization_owner_panicked");
            assert!(!runtime_identity.is_poisoned());
            assert_eq!(retained_stop_deadline(&registry), Some(first_deadline));
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Draining { runtime, .. } = &state.entry else {
                panic!("persistent panic must never publish Empty")
            };
            assert!(Arc::ptr_eq(runtime, &runtime_identity));
            assert_eq!(close_count.load(Ordering::Acquire), 1);
        }
        assert_start_is_blocked_while_stopping(&registry);
        {
            let mut runtime = runtime_identity
                .lock()
                .expect("unpoisoned persistent owner");
            assert!(runtime.core_stopped);
            assert!(!runtime.journal_closed);
            let close = runtime
                .journal_close
                .as_ref()
                .expect("the close result remains owned for retry");
            assert!(close.worker.is_none());
            assert!(close.outcome.is_some());
            runtime.clear_drain_panic();
        }

        stop_runtime_until(&registry, first_deadline + Duration::from_secs(2))
            .expect("clearing the persistent injection allows the same owner to finish");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct { epoch, .. } =
            claim_runtime_for_start(&registry)
                .expect("complete persistent recovery admits restart")
        else {
            panic!("persistent recovery must not admit an ABA epoch before cleanup")
        };
        assert_eq!(epoch, 2);
    }

    #[cfg(windows)]
    #[test]
    fn stale_drainer_rechecks_registry_after_runtime_lock_before_cleanup() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let old_runtime = Arc::new(Mutex::new(runtime_with_close_probe(Arc::clone(
            &close_count,
        ))));
        let deadline = Instant::now() + Duration::from_millis(500);
        let old_owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1));
        let old_stop_fence = RuntimeStopFence::from_ordinal(LifecycleOrdinal(2));
        let registry = Arc::new(SynchronizationRuntimeRegistry {
            state: Mutex::new(SynchronizationRuntimeState {
                next_epoch: 1,
                next_lifecycle_ordinal: old_stop_fence.ordinal(),
                cancelled_through_ordinal: old_stop_fence.ordinal(),
                admitted_stop_fences: VecDeque::new(),
                retired_through_epoch: 0,
                entry: SynchronizationRuntimeEntry::Draining {
                    epoch: 1,
                    owner_ticket: old_owner_ticket,
                    cancellation_fence: Some(old_stop_fence),
                    deadline,
                    runtime: Arc::clone(&old_runtime),
                },
            }),
            changed: Condvar::new(),
            panic_retention_barrier: Mutex::new(None),
        });
        let old_guard = old_runtime.lock().expect("hold old runtime owner");
        let stale_registry = Arc::clone(&registry);
        let stale_runtime = Arc::clone(&old_runtime);
        let stale = thread::spawn(move || {
            finish_draining_runtime_after_precheck(
                &stale_registry,
                1,
                old_owner_ticket,
                deadline,
                &stale_runtime,
            )
        });

        let mut replacement =
            runtime_with_close_probe(Arc::new(std::sync::atomic::AtomicUsize::new(0)));
        let replacement_cancelled = Arc::new(AtomicBool::new(false));
        replacement.stop_requested = Arc::clone(&replacement_cancelled);
        let replacement_runtime = Arc::new(Mutex::new(replacement));
        let replacement_owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(3));
        {
            let mut state = registry.state.lock().expect("local registry");
            state.next_epoch = 2;
            state.next_lifecycle_ordinal = replacement_owner_ticket.ordinal();
            state.entry = SynchronizationRuntimeEntry::Ready {
                epoch: 2,
                owner_ticket: replacement_owner_ticket,
                cancelled: replacement_cancelled,
                runtime: Arc::clone(&replacement_runtime),
            };
        }
        drop(old_guard);
        stale
            .join()
            .expect("join stale drainer")
            .expect("a stale drainer exits without touching another epoch");

        assert_eq!(close_count.load(Ordering::Acquire), 0);
        {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { epoch, runtime, .. } = &state.entry else {
                panic!("stale drain must preserve the replacement epoch")
            };
            assert_eq!(*epoch, 2);
            assert!(Arc::ptr_eq(runtime, &replacement_runtime));
        }
        old_runtime
            .lock()
            .expect("old runtime remains unpoisoned")
            .stop()
            .expect("clean up the isolated old runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn production_stop_closes_the_retained_persistent_journal_session() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&close_count),
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })),
        );

        production.stop().expect("stop production synchronization");

        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn blocked_journal_close_uses_the_remaining_epoch_deadline_and_reaps_once() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let mut runtime = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
                close_count: Arc::clone(&close_count),
                close_entered: Arc::clone(&close_entered),
                close_release: Arc::clone(&close_release),
                close_finished: Arc::clone(&close_finished),
                panic_after_release: false,
            })),
        );
        let live_cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&live_cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            while !worker_cancelled.load(Ordering::Acquire) {
                thread::yield_now();
            }
            thread::sleep(Duration::from_millis(20));
            let _ = sender.send(Ok(AuthoritativeLibraryChangeReport::default()));
        });
        runtime.live = Some(LiveTask {
            root_id: "deadline-root".to_owned(),
            cancelled: live_cancelled,
            receiver,
            worker: Some(worker),
        });
        let registry = ready_test_registry(runtime);

        let started = Instant::now();
        let error = stop_runtime_with_timeout(&registry, Duration::from_millis(100))
            .expect_err("a blocked journal close must exhaust the original epoch deadline");
        assert_eq!(error.code, "library_synchronization_stop_timeout");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(close_entered.load(Ordering::Acquire));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Draining { .. }
        ));
        assert_start_is_blocked_while_stopping(&registry);

        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let is_finished = {
                let state = registry.state.lock().expect("local registry");
                match &state.entry {
                    SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                        .lock()
                        .expect("draining runtime")
                        .journal_close
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished),
                    _ => false,
                }
            };
            if is_finished {
                break;
            }
            assert!(
                Instant::now() < worker_finished_deadline,
                "the retained journal-close worker did not finish after release"
            );
            thread::yield_now();
        }

        stop_runtime(&registry).expect("retry reaps the same completed journal-close worker");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start(&registry).expect("a reaped close admits the next epoch")
        else {
            panic!("an empty registry must admit construction");
        };
    }

    #[cfg(windows)]
    #[test]
    fn failed_journal_close_is_cached_and_never_invoked_again() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut runtime = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&close_count),
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
            })),
        );
        let first = runtime
            .finish_stopping_until(Instant::now() + Duration::from_secs(1))
            .expect_err("the controlled close failure must remain visible");
        assert_eq!(first.code, "persistent_change_journal_close_failed");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(runtime.journal_close.is_some());

        let retry = runtime
            .finish_stopping_until(Instant::now() + Duration::from_secs(1))
            .expect_err("retry must project the retained close outcome");
        assert_eq!(retry.code, first.code);
        assert_eq!(retry.message, first.message);
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(runtime.journal_close.is_some());
        assert!(!runtime.journal_closed);
    }

    #[cfg(windows)]
    #[test]
    fn panicked_journal_close_is_joined_once_and_remains_fail_closed() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let mut runtime = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
                close_count: Arc::clone(&close_count),
                close_entered: Arc::clone(&close_entered),
                close_release: Arc::clone(&close_release),
                close_finished: Arc::clone(&close_finished),
                panic_after_release: true,
            })),
        );

        let first = runtime
            .finish_stopping_until(Instant::now() + Duration::from_millis(25))
            .expect_err("the blocked close must retain its worker");
        assert_eq!(first.code, "library_synchronization_stop_timeout");
        assert!(close_entered.load(Ordering::Acquire));
        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
        while !runtime
            .journal_close
            .as_ref()
            .and_then(|task| task.worker.as_ref())
            .is_some_and(JoinHandle::is_finished)
        {
            assert!(
                Instant::now() < worker_finished_deadline,
                "the panicked close worker did not terminate"
            );
            thread::yield_now();
        }

        let panic = runtime
            .finish_stopping_until(Instant::now() + Duration::from_secs(1))
            .expect_err("a panicked close must fail closed");
        assert_eq!(
            panic.code,
            "persistent_change_journal_close_worker_panicked"
        );
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        let retry = runtime
            .finish_stopping_until(Instant::now() + Duration::from_secs(1))
            .expect_err("a retry must retain the panic outcome");
        assert_eq!(retry.code, panic.code);
        assert_eq!(retry.message, panic.message);
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn one_stop_joins_delayed_p0_p1_p2_then_allows_immediate_restart() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut runtime = runtime_with_close_probe(Arc::clone(&close_count));
        let p0_cancelled = Arc::new(AtomicBool::new(false));
        let p1_cancelled = Arc::new(AtomicBool::new(false));
        let p2_cancelled = Arc::new(AtomicBool::new(false));
        let p0_finished = Arc::new(AtomicBool::new(false));
        let p1_finished = Arc::new(AtomicBool::new(false));
        let p2_finished = Arc::new(AtomicBool::new(false));

        let (p0_sender, p0_receiver) = mpsc::sync_channel(1);
        let p0_worker_cancelled = Arc::clone(&p0_cancelled);
        let p0_worker_finished = Arc::clone(&p0_finished);
        let p0_worker = thread::spawn(move || {
            while !p0_worker_cancelled.load(Ordering::Acquire) {
                thread::yield_now();
            }
            thread::sleep(Duration::from_millis(30));
            p0_worker_finished.store(true, Ordering::Release);
            let _ = p0_sender.send(Ok(AuthoritativeLibraryChangeReport::default()));
        });
        runtime.live = Some(LiveTask {
            root_id: "root-p0".to_owned(),
            cancelled: Arc::clone(&p0_cancelled),
            receiver: p0_receiver,
            worker: Some(p0_worker),
        });

        let (p1_sender, p1_receiver) = mpsc::sync_channel(1);
        let p1_worker_cancelled = Arc::clone(&p1_cancelled);
        let p1_worker_finished = Arc::clone(&p1_finished);
        let p1_worker = thread::spawn(move || {
            while !p1_worker_cancelled.load(Ordering::Acquire) {
                thread::yield_now();
            }
            thread::sleep(Duration::from_millis(30));
            p1_worker_finished.store(true, Ordering::Release);
            let _ = p1_sender.send(Ok(JournalTaskOutcome::Page(
                PersistentJournalEnrollmentReport::default(),
            )));
        });
        runtime.journal = Some(JournalTask {
            cancelled: Arc::clone(&p1_cancelled),
            receiver: p1_receiver,
            worker: Some(p1_worker),
        });

        let (p2_sender, p2_receiver) = mpsc::sync_channel(1);
        let p2_worker_cancelled = Arc::clone(&p2_cancelled);
        let p2_worker_finished = Arc::clone(&p2_finished);
        let p2_worker = thread::spawn(move || {
            while !p2_worker_cancelled.load(Ordering::Acquire) {
                thread::yield_now();
            }
            thread::sleep(Duration::from_millis(30));
            p2_worker_finished.store(true, Ordering::Release);
            let _ = p2_sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                IncrementalLibraryChangeReport::default(),
            )));
        });
        runtime.recovery = Some(RecoveryTask {
            root_id: "root-p2".to_owned(),
            kind: RecoveryTaskKind::LegacyUnownedDrain {
                continuity_revision: 1,
            },
            phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            cancelled: Arc::clone(&p2_cancelled),
            receiver: p2_receiver,
            worker: Some(p2_worker),
        });
        let registry = ready_test_registry(runtime);

        stop_runtime(&registry).expect("one stop joins every delayed worker");
        assert!(p0_finished.load(Ordering::Acquire));
        assert!(p1_finished.load(Ordering::Acquire));
        assert!(p2_finished.load(Ordering::Acquire));
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        stop_runtime(&registry).expect("stop remains idempotent");
        assert_eq!(close_count.load(Ordering::Acquire), 1);

        let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
            claim_runtime_for_start(&registry).expect("restart immediately after successful stop")
        else {
            panic!("empty registry must admit a fresh construction epoch");
        };
        let owner_ticket = active_owner_ticket(&registry);
        let mut restarted = runtime_with_close_probe(Arc::clone(&close_count));
        restarted.stop_requested = Arc::clone(&cancelled);
        let restarted = match promote_constructed_runtime(
            &registry,
            epoch,
            owner_ticket,
            Arc::clone(&cancelled),
            restarted,
        )
        .expect("promote restarted runtime")
        {
            ConstructedRuntimeDisposition::Poll(runtime) => runtime,
            ConstructedRuntimeDisposition::Drain { .. } => {
                panic!("an active construction epoch must promote for polling")
            }
        };
        execute_runtime_operation(&registry, epoch, owner_ticket, cancelled, restarted, |_| {
            Ok(())
        })
        .expect("install restarted runtime");
        stop_runtime(&registry).expect("stop restarted runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 2);
    }

    #[cfg(windows)]
    fn assert_stop_is_responsive_during_blocked_poll_operation(operation_name: &'static str) {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim local poll owner");
        let owner_ticket = active_owner_ticket(&registry);
        let operation_entered = Arc::new(AtomicBool::new(false));
        let operation_release = Arc::new(AtomicBool::new(false));
        let owner_registry = Arc::clone(&registry);
        let owner_entered = Arc::clone(&operation_entered);
        let owner_release = Arc::clone(&operation_release);
        let owner = thread::Builder::new()
            .name(format!("ame-test-blocked-{operation_name}"))
            .spawn(move || {
                execute_runtime_operation(
                    &owner_registry,
                    epoch,
                    owner_ticket,
                    cancelled,
                    runtime,
                    move |_| {
                        owner_entered.store(true, Ordering::Release);
                        while !owner_release.load(Ordering::Acquire) {
                            std::thread::yield_now();
                        }
                        Ok(())
                    },
                )
            })
            .expect("start blocked poll owner");
        wait_for_test_signal(&operation_entered);

        let stop_registry = Arc::clone(&registry);
        let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
        let stopper = thread::spawn(move || {
            let _ = stop_sender.send(stop_runtime(&stop_registry));
        });
        assert!(
            matches!(
                stop_receiver.recv_timeout(Duration::from_millis(50)),
                Err(RecvTimeoutError::Timeout)
            ),
            "stop must not report success while {operation_name} still owns the runtime"
        );
        assert_start_is_blocked_while_stopping(&registry);

        operation_release.store(true, Ordering::Release);
        stop_receiver
            .recv_timeout(Duration::from_millis(500))
            .expect("single stop must finish after the poll owner releases")
            .expect("single stop completes the late runtime drain");
        stopper.join().expect("join bounded stop caller");
        let error = owner
            .join()
            .expect("join late poll owner")
            .expect_err("a stopped epoch must discard the late operation result");
        assert_eq!(error.code, "library_synchronization_poll_cancelled");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        stop_runtime(&registry).expect("idempotent stop after completed drain");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn stop_and_restart_remain_responsive_during_blocked_filesystem_and_writer_work() {
        assert_stop_is_responsive_during_blocked_poll_operation("filesystem");
        assert_stop_is_responsive_during_blocked_poll_operation("sqlite-writer");
    }

    #[cfg(windows)]
    #[test]
    fn late_poll_after_public_stop_deadline_retains_original_epoch_drain() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let runtime = runtime_with_blocking_close(
            Arc::clone(&close_count),
            Arc::clone(&close_entered),
            Arc::clone(&close_release),
            Arc::clone(&close_finished),
        );
        let registry = ready_test_registry(runtime);
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim late poll owner");
        let owner_ticket = active_owner_ticket(&registry);
        let operation_entered = Arc::new(AtomicBool::new(false));
        let operation_release = Arc::new(AtomicBool::new(false));
        let owner_registry = Arc::clone(&registry);
        let owner_entered = Arc::clone(&operation_entered);
        let owner_release = Arc::clone(&operation_release);
        let owner = thread::spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                move |_| {
                    owner_entered.store(true, Ordering::Release);
                    while !owner_release.load(Ordering::Acquire) {
                        thread::yield_now();
                    }
                    Ok(())
                },
            )
        });
        wait_for_test_signal(&operation_entered);

        let public_deadline = Instant::now() + Duration::from_millis(25);
        let stopped = stop_runtime_until(&registry, public_deadline)
            .expect_err("the public stop deadline must expire while poll owns the runtime");
        assert_eq!(stopped.code, "library_synchronization_stop_timeout");
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        let repeated = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
            .expect_err("a repeated stop must not refresh the first public deadline");
        assert_eq!(repeated.code, "library_synchronization_stop_timeout");
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        assert_start_is_blocked_while_stopping(&registry);

        operation_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_entered);
        let retained_before_release = wait_for_draining_state(&registry, Duration::from_millis(50));
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let late_result = owner.join().expect("join late poll owner");

        assert!(
            retained_before_release,
            "a late poll runtime must enter Draining under the already exhausted public deadline"
        );
        assert_eq!(
            late_result
                .expect_err("the stopped late poll cannot publish")
                .code,
            "library_synchronization_stop_timeout"
        );
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert_start_is_blocked_while_stopping(&registry);
        wait_for_retained_journal_close_worker(&registry, Duration::from_secs(2));

        stop_runtime(&registry).expect("retry reaps the original completed close task");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start(&registry).expect("completed drain permits restart")
        else {
            panic!("an empty registry must admit the next epoch");
        };
    }

    #[cfg(windows)]
    #[test]
    fn late_start_after_public_stop_deadline_retains_original_epoch_drain() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(SynchronizationRuntimeRegistry::default());
        let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
            claim_runtime_for_start(&registry).expect("claim construction epoch")
        else {
            panic!("an empty registry must admit construction");
        };
        let owner_ticket = active_owner_ticket(&registry);

        let public_deadline = Instant::now() + Duration::from_millis(25);
        let stopped = stop_runtime_until(&registry, public_deadline)
            .expect_err("the public stop deadline must expire while construction is outside");
        assert_eq!(stopped.code, "library_synchronization_stop_timeout");
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        let repeated = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
            .expect_err("a repeated stop must not refresh the first public deadline");
        assert_eq!(repeated.code, "library_synchronization_stop_timeout");
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        assert!(cancelled.load(Ordering::Acquire));
        let runtime = runtime_with_blocking_close(
            Arc::clone(&close_count),
            Arc::clone(&close_entered),
            Arc::clone(&close_release),
            Arc::clone(&close_finished),
        );
        let (retained_deadline, runtime) = match promote_constructed_runtime(
            &registry,
            epoch,
            owner_ticket,
            Arc::clone(&cancelled),
            runtime,
        )
        .expect("late construction retains its stopping owner")
        {
            ConstructedRuntimeDisposition::Drain { deadline, runtime } => (deadline, runtime),
            ConstructedRuntimeDisposition::Poll(_) => {
                panic!("a stopped construction epoch must drain rather than poll")
            }
        };
        assert_eq!(retained_deadline, public_deadline);
        let owner_registry = Arc::clone(&registry);
        let owner = thread::spawn(move || {
            finish_draining_runtime(
                &owner_registry,
                epoch,
                owner_ticket,
                retained_deadline,
                &runtime,
            )
        });
        wait_for_test_signal(&close_entered);
        let retained_before_release = wait_for_draining_state(&registry, Duration::from_millis(50));
        assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let late_result = owner.join().expect("join late construction owner");

        assert!(
            retained_before_release,
            "a late constructed runtime must enter Draining under the exhausted public deadline"
        );
        assert_eq!(
            late_result
                .expect_err("late construction drain must retain the timeout")
                .code,
            "library_synchronization_stop_timeout"
        );
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert_start_is_blocked_while_stopping(&registry);
        wait_for_retained_journal_close_worker(&registry, Duration::from_secs(2));

        stop_runtime(&registry).expect("retry reaps the original late-construction close task");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct { .. } =
            claim_runtime_for_start(&registry).expect("completed drain permits restart")
        else {
            panic!("an empty registry must admit the next epoch");
        };
    }

    #[cfg(windows)]
    #[test]
    fn blocked_broker_connect_cannot_hold_the_registry_or_install_a_late_session() {
        let connect_entered = Arc::new(AtomicBool::new(false));
        let connect_release = Arc::new(AtomicBool::new(false));
        let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let runtime = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            Arc::new(BlockingJournalFactory {
                connect_entered: Arc::clone(&connect_entered),
                connect_release: Arc::clone(&connect_release),
                connect_count: Arc::clone(&connect_count),
                close_count: Arc::clone(&close_count),
            }),
            true,
        );
        let registry = ready_test_registry(runtime);
        let (epoch, cancelled, runtime) =
            claim_runtime_for_poll(&registry).expect("claim connect poll owner");
        let owner_ticket = active_owner_ticket(&registry);
        let owner_registry = Arc::clone(&registry);
        let owner = thread::spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                |runtime| {
                    runtime.open_persistent_change_journal_after_watcher();
                    Ok(())
                },
            )
        });
        wait_for_test_signal(&connect_entered);

        let stop_registry = Arc::clone(&registry);
        let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
        let stopper = thread::spawn(move || {
            let _ = stop_sender.send(stop_runtime(&stop_registry));
        });
        assert!(matches!(
            stop_receiver.recv_timeout(Duration::from_millis(50)),
            Err(RecvTimeoutError::Timeout)
        ));
        assert_start_is_blocked_while_stopping(&registry);
        assert_eq!(connect_count.load(Ordering::Acquire), 1);

        connect_release.store(true, Ordering::Release);
        stop_receiver
            .recv_timeout(Duration::from_millis(500))
            .expect("single stop must finish after broker connect releases")
            .expect("single stop rejects and drains the late session");
        stopper.join().expect("join connect stop caller");
        let error = owner
            .join()
            .expect("join blocked connect owner")
            .expect_err("late broker connection must not install");
        assert_eq!(error.code, "library_synchronization_poll_cancelled");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        assert_eq!(connect_count.load(Ordering::Acquire), 1);
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn timed_out_p1_then_journal_close_share_one_epoch_deadline_and_reap_once() {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_entered = Arc::new(AtomicBool::new(false));
        let close_release = Arc::new(AtomicBool::new(false));
        let close_finished = Arc::new(AtomicBool::new(false));
        let mut runtime = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
                close_count: Arc::clone(&close_count),
                close_entered: Arc::clone(&close_entered),
                close_release: Arc::clone(&close_release),
                close_finished: Arc::clone(&close_finished),
                panic_after_release: false,
            })),
        );
        let worker_cancelled = Arc::new(AtomicBool::new(false));
        let worker_release = Arc::new(AtomicBool::new(false));
        let completed_sibling = Arc::new(AtomicBool::new(false));
        let release = Arc::clone(&worker_release);
        let sibling = Arc::clone(&completed_sibling);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            while !release.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            sibling.store(true, Ordering::Release);
            let _ = sender.send(Ok(JournalTaskOutcome::Page(
                PersistentJournalEnrollmentReport {
                    enrolled_root_count: 1,
                    ..PersistentJournalEnrollmentReport::default()
                },
            )));
        });
        runtime.journal = Some(JournalTask {
            cancelled: Arc::clone(&worker_cancelled),
            receiver,
            worker: Some(worker),
        });
        let registry = ready_test_registry(runtime);

        let error = stop_runtime_with_timeout(&registry, Duration::from_millis(25))
            .expect_err("uncooperative worker must produce a structured timeout");
        assert_eq!(error.code, "persistent_journal_stop_timeout");
        assert!(worker_cancelled.load(Ordering::Acquire));
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Draining { .. }
        ));
        assert_start_is_blocked_while_stopping(&registry);
        assert_eq!(close_count.load(Ordering::Acquire), 0);

        worker_release.store(true, Ordering::Release);
        let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let is_finished = {
                let state = registry.state.lock().expect("local registry");
                match &state.entry {
                    SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                        .lock()
                        .expect("draining runtime")
                        .journal
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished),
                    _ => false,
                }
            };
            if is_finished {
                break;
            }
            assert!(
                Instant::now() < worker_finished_deadline,
                "the retained P1 worker did not finish after release"
            );
            thread::yield_now();
        }

        let exhausted = stop_runtime(&registry)
            .expect_err("journal close must not receive a fresh deadline after P1 exhaustion");
        assert_eq!(exhausted.code, "library_synchronization_stop_timeout");
        wait_for_test_signal(&close_entered);
        assert!(close_entered.load(Ordering::Acquire));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Draining { .. }
        ));

        close_release.store(true, Ordering::Release);
        wait_for_test_signal(&close_finished);
        let close_worker_finished_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let is_finished = {
                let state = registry.state.lock().expect("local registry");
                match &state.entry {
                    SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                        .lock()
                        .expect("draining runtime")
                        .journal_close
                        .as_ref()
                        .and_then(|task| task.worker.as_ref())
                        .is_some_and(JoinHandle::is_finished),
                    _ => false,
                }
            };
            if is_finished {
                break;
            }
            assert!(
                Instant::now() < close_worker_finished_deadline,
                "the retained journal-close worker did not finish after release"
            );
            thread::yield_now();
        }

        stop_runtime(&registry).expect("retry joins the same completed close task");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        assert!(completed_sibling.load(Ordering::Acquire));
    }

    #[cfg(windows)]
    #[test]
    fn production_journal_factory_reconnects_after_closing_the_previous_session() {
        let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let factory = Arc::new(CountingJournalFactory {
            connect_count: Arc::clone(&connect_count),
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
        let mut production = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            factory,
            true,
        );
        production.open_persistent_change_journal_after_watcher();

        production
            .reconnect_persistent_change_journal()
            .expect("reconnect journal session");

        assert_eq!(connect_count.load(Ordering::Acquire), 2);
        assert_eq!(close_count.load(Ordering::Acquire), 1);
        production.stop().expect("stop reconnected runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 2);
    }

    #[cfg(windows)]
    #[test]
    fn reconnect_isolates_a_close_failure_and_installs_a_fresh_session() {
        let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let factory = Arc::new(CountingJournalFactory {
            connect_count: Arc::clone(&connect_count),
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        });
        let mut production = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            factory,
            true,
        );
        production.open_persistent_change_journal_after_watcher();

        assert_eq!(
            production.reconnect_persistent_change_journal(),
            Err(PersistentChangeJournalOperationError::TransportUnavailable)
        );
        assert_eq!(connect_count.load(Ordering::Acquire), 2);
        assert_eq!(close_count.load(Ordering::Acquire), 1);

        production
            .stop()
            .expect("stop fresh session after close failure");
        assert_eq!(close_count.load(Ordering::Acquire), 2);
    }

    #[cfg(windows)]
    #[test]
    fn portable_production_never_invokes_the_journal_factory() {
        let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let factory = Arc::new(CountingJournalFactory {
            connect_count: Arc::clone(&connect_count),
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
        let mut production = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            factory,
            false,
        );

        assert_eq!(connect_count.load(Ordering::Acquire), 0);
        assert!(matches!(
            production._persistent_change_journal,
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::PortableDistribution
            )
        ));
        assert_eq!(
            production.reconnect_persistent_change_journal(),
            Err(PersistentChangeJournalOperationError::TransportUnavailable)
        );
        assert_eq!(connect_count.load(Ordering::Acquire), 0);
        production.stop().expect("stop portable runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 0);
    }

    #[cfg(windows)]
    #[test]
    fn installed_production_defers_the_journal_factory_until_after_watcher_start() {
        let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let factory = Arc::new(CountingJournalFactory {
            connect_count: Arc::clone(&connect_count),
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
        let mut production = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            factory,
            true,
        );

        assert_eq!(connect_count.load(Ordering::Acquire), 0);
        production.open_persistent_change_journal_after_watcher();
        assert_eq!(connect_count.load(Ordering::Acquire), 1);
        production.stop().expect("stop installed runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn production_poll_starts_the_watcher_before_opening_the_journal_boundary() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "initial-ordering-scan".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove ordering fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        drop(catalog);

        let events = Arc::new(Mutex::new(Vec::new()));
        let probe = OrderingProbeFactory {
            events: Arc::clone(&events),
        };
        let mut production = new_production_synchronization_with_factory(
            crate::ports::erase_library_change_source_factory(probe.clone()),
            Arc::new(probe),
            true,
        );

        let snapshot =
            poll_runtime_with_storage(&mut production, &storage).expect("poll synchronization");

        assert_eq!(
            *events.lock().expect("ordering evidence"),
            ["watcher", "journal"]
        );
        let status = snapshot.roots.first().expect("root synchronization status");
        assert_eq!(
            status.continuity,
            PersistentJournalContinuityState::LiveOnly
        );
        assert_eq!(
            status.source_health,
            crate::domain::LibraryChangeSourceHealth::Healthy
        );
        assert_eq!(
            status.freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        production.stop().expect("stop synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn production_snapshot_projects_an_explicit_v30_claim_as_manual_recovery() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "explicit-projection-initial".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove projection fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open explicit projection fixture");
        connection
            .execute(
                "INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, attempt_count, next_retry_unix_ms,
                   last_failure_code, last_failure_message,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES (
                   ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
                   41, 41, '1', '1', 1, 'retry_wait', 41, 0, NULL,
                   'live_gap_v30_explicit_recovery_required',
                   'The ambiguous historical gap requires an explicit library update',
                   1, 41, 41
                 )",
                [&root_id],
            )
            .expect("insert explicit projection gap");
        let gap_change_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO library_live_gap_recovery_claims(
                   gap_change_id, root_id, root_generation, consumer_kind,
                   created_unix_ms
                 ) VALUES (?1, ?2, 1, 'explicit_recovery_required', 41)",
                rusqlite::params![gap_change_id, root_id],
            )
            .expect("insert explicit projection claim");
        drop(connection);

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::PortableDistribution,
            ),
        );
        let snapshot =
            poll_runtime_with_storage(&mut production, &storage).expect("poll explicit projection");
        let status = snapshot
            .roots
            .iter()
            .find(|status| status.root_id == root_id)
            .expect("explicit projection root");

        assert_eq!(
            status.freshness,
            crate::domain::CatalogFreshnessState::NeedsReconciliation
        );
        assert_eq!(status.phase, LibrarySynchronizationPhase::Blocked);
        assert_eq!(status.queue_health, LibraryChangeQueueHealth::Degraded);
        assert!(status.recovery_blocked);
        assert_eq!(
            status.last_issue_code.as_deref(),
            Some("live_gap_v30_explicit_recovery_required")
        );
        assert!(production.recovery.is_none());
        production.stop().expect("stop explicit projection runtime");
    }

    #[cfg(windows)]
    #[test]
    fn local_p1_registration_failure_is_durable_and_root_isolated() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
        for (root_id, marker) in [("bad-root", 1_u8), ("healthy-root", 2_u8)] {
            let root_path = directory.path().join(root_id);
            std::fs::create_dir_all(&root_path).expect("create root");
            let root_path = root_path.to_string_lossy().into_owned();
            let request = ScanRequest {
                scan_id: format!("scan-{root_id}"),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            };
            catalog
                .begin_scan(&request, root_id, &root_path)
                .expect("begin root scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&request.scan_id)
                .expect("prove P1 fixture first-import handoff");
            catalog
                .publish_scan(&request.scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    state: PersistentJournalCapabilityState::Supported,
                    continuity: PersistentJournalContinuityState::Current,
                    failure: None,
                    updated_unix_ms: 1,
                })
                .expect("seed current capability");
            catalog
                .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    volume: PersistentJournalVolumeIdentity {
                        volume_guid: "registration-volume".to_owned(),
                        volume_serial: 7,
                    },
                    root_file_reference: JournalFileReference::V2([marker; 8]),
                    journal_id: JournalIdentifier::new(44).expect("journal ID"),
                    next_unread_usn: JournalUsn::new(20).expect("next USN"),
                    captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
                    covered_catalog_revision: 1,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    continuity: PersistentJournalContinuityState::Current,
                    failure: None,
                    updated_unix_ms: 1,
                })
                .expect("seed current checkpoint");
        }

        let failure = persist_journal_registration_failure(
            &mut catalog,
            "bad-root",
            generation,
            PersistentChangeJournalOperationError::InvalidRequest,
            1_000,
            policy,
        )
        .expect("persist typed registration failure");

        assert_eq!(
            failure.kind,
            PersistentJournalRootFailureKind::NonRecoverable
        );
        let bad = catalog
            .load_persistent_journal_checkpoint("bad-root", generation)
            .expect("load bad checkpoint")
            .expect("bad checkpoint");
        let healthy = catalog
            .load_persistent_journal_checkpoint("healthy-root", generation)
            .expect("load healthy checkpoint")
            .expect("healthy checkpoint");
        assert_eq!(
            bad.continuity,
            PersistentJournalContinuityState::RecoveryRequired
        );
        assert_eq!(
            healthy.continuity,
            PersistentJournalContinuityState::Current
        );
        assert_eq!(bad.next_unread_usn.value(), 20);
        let metrics = catalog
            .load_library_change_root_queue_metrics("bad-root", generation, 1_000, policy)
            .expect("load bad-root queue metrics");
        assert_eq!(metrics.pending_count, 0);
        assert_eq!(metrics.leased_count, 0);
        assert_eq!(metrics.retry_wait_count, 0);
    }

    #[cfg(windows)]
    #[test]
    fn journal_volume_selection_excludes_a_recovery_required_sibling() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let generation = LibraryRootGeneration::initial();
        let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
        let mut checkpoints = Vec::new();
        for (root_id, marker) in [("p1-root", 1_u8), ("p2-root", 2_u8)] {
            let request = ScanRequest {
                scan_id: format!("scan-{root_id}"),
                root_path: format!("C:/{root_id}"),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            };
            catalog
                .begin_scan(&request, root_id, &request.root_path)
                .expect("begin root scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&request.scan_id)
                .expect("prove lane fixture first-import handoff");
            catalog
                .publish_scan(&request.scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    state: PersistentJournalCapabilityState::Supported,
                    continuity: PersistentJournalContinuityState::Current,
                    failure: None,
                    updated_unix_ms: 1,
                })
                .expect("seed supported root");
            let checkpoint = PersistentJournalCheckpoint {
                root_id: root_id.to_owned(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "volume-shared".to_owned(),
                    volume_serial: 7,
                },
                root_file_reference: JournalFileReference::V2([marker; 8]),
                journal_id: JournalIdentifier::new(44).expect("journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("next USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
                covered_catalog_revision: 1,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1,
            };
            catalog
                .seed_persistent_journal_checkpoint_for_test(&checkpoint)
                .expect("seed current checkpoint");
            checkpoints.push(checkpoint);
        }
        let recovery_checkpoint = &checkpoints[1];
        catalog
            .persist_persistent_journal_root_failure(
                &crate::domain::PersistentJournalRootFailure {
                    root_id: recovery_checkpoint.root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                    failure: PersistentJournalFailure {
                        code: "fixture-containment".to_owned(),
                        message: "Fixture containment failure".to_owned(),
                    },
                    opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                        volume: recovery_checkpoint.volume.clone(),
                        root_file_reference: recovery_checkpoint.root_file_reference.clone(),
                        journal_id: recovery_checkpoint.journal_id,
                        next_usn: recovery_checkpoint.next_unread_usn,
                        protocol_version: recovery_checkpoint.protocol_version,
                        contract_version: recovery_checkpoint.contract_version,
                    }),
                },
                2,
                crate::domain::LibraryChangeQueuePolicy {
                    debounce_millis: 0,
                    ..crate::domain::LibraryChangeQueuePolicy::default()
                },
            )
            .expect("admit recovery-required sibling")
            .expect("containment recovery is allowlisted");
        let snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 0,
            applied_mutation_count: 0,
            roots: ["p1-root", "p2-root"]
                .into_iter()
                .map(|root_id| crate::domain::LibraryRootSynchronizationStatus {
                    root_id: root_id.to_owned(),
                    root_generation: generation.value(),
                    availability: crate::domain::LibraryRootAvailability::Available,
                    freshness: crate::domain::CatalogFreshnessState::Synchronized,
                    freshness_cause: crate::domain::CatalogFreshnessCause::NoPendingChanges,
                    continuity: if root_id == "p1-root" {
                        PersistentJournalContinuityState::Current
                    } else {
                        PersistentJournalContinuityState::RecoveryRequired
                    },
                    phase: LibrarySynchronizationPhase::Synchronized,
                    source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                    queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                    pending_change_count: 0,
                    retry_wait_count: 0,
                    freshness_unknown_count: 0,
                    recovery_blocked: false,
                    last_issue_code: None,
                })
                .collect(),
        };
        let production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );

        let work = production
            .next_journal_volume(&catalog, &snapshot)
            .expect("select current P1 volume")
            .expect("current P1 work");

        assert_eq!(work.roots.len(), 1);
        assert_eq!(work.roots[0].root_id, "p1-root");
        assert_eq!(work.checkpoints.len(), 1);
        assert_eq!(work.checkpoints[0].root_id, "p1-root");
    }

    #[cfg(windows)]
    #[test]
    fn one_runtime_validates_the_catalog_once_across_one_hundred_polls() {
        let directory = tempfile::tempdir().expect("runtime catalog directory");
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        crate::adapters::reset_full_schema_validation_count(&storage.catalog_path);
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );

        for _ in 0..100 {
            poll_runtime_with_storage(&mut production, &storage).expect("production poll");
        }

        assert_eq!(
            crate::adapters::full_schema_validation_count(&storage.catalog_path),
            1,
            "the production epoch must run O(N) schema validation only once"
        );
        assert_eq!(production.poll_catalog.connection_counts(), (1, 0));
        production.stop().expect("stop production runtime");
        assert_eq!(production.poll_catalog.connection_counts(), (1, 1));
    }

    #[cfg(windows)]
    #[test]
    fn one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        std::fs::write(source_root.join("sentinel.txt"), b"must not be enumerated")
            .expect("source-root enumeration sentinel");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "initial-p1-scan".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let registration = describe_production_persistent_journal_root(
            &root_id,
            LibraryRootGeneration::initial().value(),
            &source_root,
        )
        .expect("describe disposable P1 root");
        let root_reference = crate::domain::JournalFileReference::from_bytes(
            &registration.authorization.root_identity,
        )
        .expect("root file reference");
        let volume = crate::domain::PersistentJournalVolumeIdentity {
            volume_guid: registration.authorization.volume_id.clone(),
            volume_serial: registration.volume_serial,
        };
        let journal_id = crate::domain::JournalIdentifier::new(44).expect("journal ID");
        let usn = crate::domain::JournalUsn::new(20).expect("USN");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove journal fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        let root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load root")
            .expect("published root");
        catalog
            .save_persistent_journal_capability(&crate::domain::PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: crate::domain::PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1_000,
            })
            .expect("save supported capability");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                volume,
                root_file_reference: root_reference,
                journal_id,
                next_unread_usn: usn,
                captured_exclusive_end: usn,
                covered_catalog_revision: root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: crate::domain::PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1_000,
            })
            .expect("seed established checkpoint");
        drop(catalog);

        let scan_rows_before = {
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open scan-row assertion catalog");
            connection
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("count initial scan rows")
        };
        let register_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let client_instances = Arc::new(Mutex::new(Vec::new()));
        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        crate::adapters::reset_source_root_entry_enumeration_count(&root_path);
        crate::adapters::reset_configured_root_open_instrumentation(&root_path);
        crate::adapters::reset_source_content_open_instrumentation(&root_path);
        crate::adapters::reset_observer_root_handle_open_count(&root_path);
        crate::adapters::reset_root_availability_metadata_probe_instrumentation(&root_path);
        let mut availability_poll_count = 0_u64;

        for startup_index in 0..100_usize {
            let session: Arc<dyn PersistentChangeJournalSession> =
                Arc::new(NoChangeJournalSession {
                    register_count: Arc::clone(&register_count),
                    query_count: Arc::clone(&query_count),
                    shared_read_count: Arc::clone(&shared_read_count),
                    close_count: Arc::clone(&close_count),
                    client_instances: Arc::clone(&client_instances),
                });
            let mut production = new_production_synchronization_with_connection(
                crate::adapters::production_library_change_source_factory(),
                PersistentChangeJournalConnection::Connected(session),
            );
            let client_seed = u8::try_from(startup_index + 1).expect("startup client instance");
            production.persistent_change_journal_caller =
                Some(crate::journal_broker::CallerClaim {
                    process_id: std::process::id(),
                    session_id: 1,
                    client_instance: [client_seed; 16],
                });

            let expected_queries = startup_index + 1;
            availability_poll_count = availability_poll_count.saturating_add(1);
            let first_snapshot = poll_runtime_with_storage(&mut production, &storage)
                .expect("start production observer and no-change query");
            let first_status = first_snapshot
                .roots
                .first()
                .expect("root synchronization status");
            assert_eq!(
                first_status.continuity,
                PersistentJournalContinuityState::Current
            );
            assert_eq!(
                first_status.source_health,
                crate::domain::LibraryChangeSourceHealth::Healthy
            );
            let query_deadline = std::time::Instant::now() + Duration::from_secs(5);
            while std::time::Instant::now() < query_deadline {
                if query_count.load(Ordering::Acquire) == expected_queries {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(query_count.load(Ordering::Acquire), expected_queries);
            let harvest_deadline = std::time::Instant::now() + Duration::from_secs(5);
            let harvested = loop {
                availability_poll_count = availability_poll_count.saturating_add(1);
                let snapshot = poll_runtime_with_storage(&mut production, &storage)
                    .expect("harvest completed no-change query");
                if production.journal.is_none() {
                    break snapshot;
                }
                assert!(
                    std::time::Instant::now() < harvest_deadline,
                    "no-change query was not harvested by the production coordinator"
                );
                std::thread::sleep(Duration::from_millis(5));
            };
            let harvested_status = harvested
                .roots
                .first()
                .expect("harvested root synchronization status");
            assert_eq!(
                harvested_status.continuity,
                PersistentJournalContinuityState::Current
            );
            assert_eq!(register_count.load(Ordering::Acquire), expected_queries);
            assert_eq!(query_count.load(Ordering::Acquire), expected_queries);
            assert_eq!(shared_read_count.load(Ordering::Acquire), 0);
            assert_eq!(
                crate::adapters::observer_root_handle_open_count(&root_path),
                u64::try_from(expected_queries).expect("watcher root handles")
            );
            assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
            assert_eq!(crate::adapters::source_spool_open_count(&root_path), 0);
            assert_eq!(
                crate::adapters::source_root_entry_enumeration_count(&root_path),
                0
            );
            assert_eq!(crate::adapters::source_content_open_count(&root_path), 0);
            assert_eq!(
                crate::adapters::root_availability_metadata_probe_count(&root_path),
                availability_poll_count,
                "every production poll must perform exactly one metadata-only root availability probe"
            );
            assert_eq!(
                crate::adapters::configured_root_open_count(&root_path, true),
                0
            );
            assert_eq!(
                crate::adapters::configured_root_open_count(&root_path, false),
                0
            );
            production.stop().expect("stop no-change P1 scheduler");
            assert_eq!(close_count.load(Ordering::Acquire), expected_queries);
        }

        assert_eq!(register_count.load(Ordering::Acquire), 100);
        assert_eq!(query_count.load(Ordering::Acquire), 100);
        assert_eq!(shared_read_count.load(Ordering::Acquire), 0);
        assert_eq!(close_count.load(Ordering::Acquire), 100);
        let client_instances = client_instances.lock().expect("no-change client instances");
        assert_eq!(client_instances.len(), 100);
        assert!(
            client_instances.iter().all(|instance| *instance != [0; 16]),
            "every production startup must use a nonzero instance identity"
        );
        let unique_instances = client_instances
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique_instances.len(), 100);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open inventory assertion catalog");
        let inventory_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                [],
                |row| row.get(0),
            )
            .expect("inventory count");
        assert_eq!(inventory_count, 0);
        let scan_rows_after = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count final scan rows");
        assert_eq!(scan_rows_after, scan_rows_before);
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        assert_eq!(crate::adapters::source_spool_open_count(&root_path), 0);
        assert_eq!(
            crate::adapters::source_root_entry_enumeration_count(&root_path),
            0,
            "a continuous no-change startup must enumerate zero source-root entries"
        );
        assert_eq!(
            crate::adapters::configured_root_open_count(&root_path, true),
            0,
            "a continuous no-change startup must not construct a discovery root handle"
        );
        assert_eq!(
            crate::adapters::configured_root_open_count(&root_path, false),
            0,
            "a continuous no-change startup must not construct a publication guard"
        );
        assert_eq!(
            crate::adapters::source_content_open_count(&root_path),
            0,
            "a continuous no-change startup must open zero media content"
        );
        let availability_metadata_probes =
            crate::adapters::root_availability_metadata_probe_count(&root_path);
        assert_eq!(availability_metadata_probes, availability_poll_count);
        assert!(
            (200..=300).contains(&availability_metadata_probes),
            "100 no-change startups must remain within a bounded metadata-only availability-probe envelope; observed {availability_metadata_probes}"
        );
        eprintln!(
            "R2c-R no-change startups=100 production_polls={} production_observer_starts=100 watcher_root_handles={} nonzero_unique_client_instances={} journal_registers={} journal_queries={} journal_reads={} journal_closes={} source_enumeration_spool_opens=0 source_entry_reads=0 source_root_entries=0 inventory_runs=0 full_scan_rows_added=0 media_content_opens=0 availability_metadata_probes={} discovery_root_handles=0 publication_guard_root_handles=0",
            availability_poll_count,
            crate::adapters::observer_root_handle_open_count(&root_path),
            unique_instances.len(),
            register_count.load(Ordering::Acquire),
            query_count.load(Ordering::Acquire),
            shared_read_count.load(Ordering::Acquire),
            close_count.load(Ordering::Acquire),
            availability_metadata_probes,
        );
        drop(connection);
    }

    #[cfg(windows)]
    #[test]
    fn production_baseline_brackets_inventory_and_publishes_current_atomically() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "initial-production-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove baseline fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("require an existing-root production baseline");
        drop(catalog);

        let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(NoChangeJournalSession {
            register_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            query_count: Arc::clone(&query_count),
            shared_read_count: Arc::clone(&shared_read_count),
            close_count: Arc::clone(&close_count),
            client_instances: Arc::new(Mutex::new(Vec::new())),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [8; 16],
        });

        let mut saw_active_baseline = false;
        let mut completed = false;
        for _ in 0..300 {
            poll_runtime_with_storage(&mut production, &storage).expect("poll baseline lifecycle");
            let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                .expect("inspect baseline catalog");
            saw_active_baseline |= !catalog
                .load_persistent_journal_baselines()
                .expect("load active baselines")
                .is_empty();
            let checkpoint = catalog
                .load_persistent_journal_checkpoint(&root_id, LibraryRootGeneration::initial())
                .expect("load baseline checkpoint");
            drop(catalog);
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open baseline assertion catalog");
            let completed_baselines: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_baselines
                     WHERE root_id = ?1 AND phase = 'completed'",
                    [&root_id],
                    |row| row.get(0),
                )
                .expect("count completed baselines");
            let retired_authorities: i64 = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM library_persistent_journal_baselines AS baseline
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = baseline.change_id
                     WHERE baseline.root_id = ?1
                       AND authority.retired_unix_ms IS NOT NULL",
                    [&root_id],
                    |row| row.get(0),
                )
                .expect("count retired baseline authorities");
            completed = completed_baselines == 1
                && retired_authorities == 1
                && checkpoint.is_some_and(|checkpoint| {
                    checkpoint.continuity == PersistentJournalContinuityState::Current
                });
            if completed {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(saw_active_baseline);
        assert!(completed);
        assert!(query_count.load(Ordering::Acquire) >= 2);
        assert_eq!(shared_read_count.load(Ordering::Acquire), 0);
        production.stop().expect("stop production baseline runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn production_baseline_replays_changes_during_inventory_before_publishing_current() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "initial-production-advancing-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove advancing-baseline fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("require an advancing existing-root production baseline");
        drop(catalog);

        let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let read_release = Arc::new(AtomicBool::new(false));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(AdvancingJournalSession {
            boundaries: Mutex::new(std::collections::VecDeque::from([20, 21])),
            query_count: Arc::clone(&query_count),
            shared_read_count: Arc::clone(&shared_read_count),
            read_release: Arc::clone(&read_release),
            close_count: Arc::clone(&close_count),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [5; 16],
        });

        let mut saw_catching_up = false;
        let mut completed = false;
        for _ in 0..300 {
            poll_runtime_with_storage(&mut production, &storage)
                .expect("poll advancing baseline lifecycle");
            let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                .expect("inspect baseline catalog");
            let checkpoint = catalog
                .load_persistent_journal_checkpoint(&root_id, LibraryRootGeneration::initial())
                .expect("load advancing checkpoint");
            saw_catching_up |= checkpoint.as_ref().is_some_and(|checkpoint| {
                checkpoint.next_unread_usn.value() >= 21
                    && checkpoint.continuity == PersistentJournalContinuityState::CatchingUp
            });
            drop(catalog);
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open advancing assertion catalog");
            let evidence: (i64, i64, i64) = connection
                .query_row(
                    "SELECT
                       (SELECT COUNT(*)
                        FROM library_persistent_journal_baselines
                        WHERE root_id = ?1 AND phase = 'completed'),
                       (SELECT COUNT(*)
                        FROM library_persistent_journal_source_ranges AS ranges
                        JOIN library_persistent_journal_range_lifecycle AS lifecycle
                          ON lifecycle.source_range_id = ranges.id
                        WHERE ranges.root_id = ?1
                          AND ranges.requested_start_usn = '20'
                          AND ranges.covered_until_usn = '21'
                          AND lifecycle.lifecycle_state = 'completed'),
                       (SELECT COUNT(*)
                        FROM library_persistent_journal_baselines AS baseline
                        JOIN library_recovery_authorities AS authority
                          ON authority.change_id = baseline.change_id
                        WHERE baseline.root_id = ?1
                          AND authority.retired_unix_ms IS NOT NULL)",
                    [&root_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("load advancing baseline evidence");
            if shared_read_count.load(Ordering::Acquire) == 1
                && !read_release.load(Ordering::Acquire)
            {
                saw_catching_up |= checkpoint.as_ref().is_some_and(|checkpoint| {
                    checkpoint.next_unread_usn.value() == 20
                        && checkpoint.captured_exclusive_end.value() == 21
                        && checkpoint.continuity == PersistentJournalContinuityState::CatchingUp
                });
                assert_eq!(evidence.0, 0);
                read_release.store(true, Ordering::Release);
            }
            completed = evidence == (1, 1, 1)
                && checkpoint.is_some_and(|checkpoint| {
                    checkpoint.continuity == PersistentJournalContinuityState::Current
                        && checkpoint.next_unread_usn.value() == 21
                });
            if completed {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        read_release.store(true, Ordering::Release);
        assert!(saw_catching_up);
        assert!(completed);
        assert!(query_count.load(Ordering::Acquire) >= 3);
        assert_eq!(shared_read_count.load(Ordering::Acquire), 1);
        production
            .stop()
            .expect("stop advancing production baseline runtime");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn broker_absence_and_mismatch_remain_live_only_without_inventory() {
        for reason in [
            PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
            PersistentChangeJournalLiveOnlyReason::ProtocolMismatch,
        ] {
            let directory = tempfile::tempdir().expect("test directory");
            let source_root = directory.path().join("source");
            std::fs::create_dir_all(&source_root).expect("source root");
            let root_path = source_root.to_string_lossy().into_owned();
            let root_id =
                crate::application::scan_library::stable_id("library-root-v1", &root_path);
            let storage = crate::application::storage::StoragePaths {
                catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
                preview_root: directory.path().join("previews"),
                preview_budget_bytes: 64 * 1024 * 1024,
                settings_path: directory.path().join("settings").join("storage.sqlite3"),
            };
            let request = ScanRequest {
                scan_id: format!("initial-{reason:?}"),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            };
            let mut catalog =
                SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
            catalog
                .begin_scan(&request, &root_id, &root_path)
                .expect("begin initial scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&request.scan_id)
                .expect("prove LiveOnly fixture first-import handoff");
            catalog
                .publish_scan(&request.scan_id, &root_id, 0, 0)
                .expect("publish initial scan");
            drop(catalog);

            let mut production = new_production_synchronization_with_connection(
                crate::ports::erase_library_change_source_factory(HealthyFactory),
                PersistentChangeJournalConnection::LiveOnly(reason),
            );
            let snapshot = poll_runtime_with_storage(&mut production, &storage)
                .expect("poll LiveOnly production synchronization");

            assert!(matches!(
                &production._persistent_change_journal,
                PersistentChangeJournalConnection::LiveOnly(actual) if *actual == reason
            ));
            assert!(production.recovery.is_none());
            assert_eq!(snapshot.roots.len(), 1);
            assert_eq!(
                snapshot.roots[0].source_health,
                crate::domain::LibraryChangeSourceHealth::Healthy
            );
            assert_eq!(
                snapshot.roots[0].freshness,
                crate::domain::CatalogFreshnessState::Synchronized
            );
            assert_eq!(snapshot.roots[0].pending_change_count, 0);
            assert_eq!(snapshot.roots[0].freshness_unknown_count, 0);

            let catalog =
                SqliteCatalog::open(storage.catalog_path.clone()).expect("reopen catalog");
            let metrics = catalog
                .load_library_change_root_queue_metrics(
                    &root_id,
                    crate::domain::LibraryRootGeneration::initial(),
                    now_unix_ms().expect("metrics time"),
                    crate::domain::LibraryChangeQueuePolicy::default(),
                )
                .expect("load queue metrics");
            assert_eq!(metrics.pending_count, 0);
            assert_eq!(metrics.leased_count, 0);
            assert_eq!(metrics.retry_wait_count, 0);
            drop(catalog);
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open catalog for inventory assertion");
            let inventory_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                    [],
                    |row| row.get(0),
                )
                .expect("count metadata inventory runs");
            assert_eq!(inventory_count, 0);

            production.stop().expect("stop production synchronization");
        }
    }

    #[cfg(windows)]
    #[test]
    fn stale_live_only_first_import_capability_is_refreshed_once() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "stale-live-only-first-import".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let generation = LibraryRootGeneration::initial();
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin first import");
        let (_, started_unix_ms) = catalog
            .pristine_first_import_scan(&root_id, generation)
            .expect("load pristine first import")
            .expect("pristine first import");
        let stale_unix_ms = started_unix_ms
            .checked_sub(1)
            .expect("fixture scan starts after the Unix epoch");
        catalog
            .save_persistent_journal_capability(
                &super::super::journal_baseline::live_only_capability(
                    &root_id,
                    generation,
                    stale_unix_ms,
                ),
            )
            .expect("seed stale LiveOnly capability");
        assert!(
            !catalog
                .first_import_change_capture_is_ready(&request.scan_id, &root_id, generation)
                .expect("inspect stale first-import handoff")
        );
        drop(catalog);
        crate::application::catalog_session::open_catalog(
            &storage.catalog_path,
            LibraryChangeLane::Recovery,
        )
        .expect("prime protected catalog session");
        let _capture = crate::application::scan_library::hold_first_import_capture(
            &request.scan_id,
            &storage.catalog_path,
            &root_id,
            generation,
        )
        .expect("executing first import");

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        let snapshot = poll_runtime_with_storage(&mut production, &storage)
            .expect("refresh stale first-import handoff");
        assert_eq!(
            snapshot.roots[0].source_health,
            crate::domain::LibraryChangeSourceHealth::Healthy
        );

        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect refreshed catalog");
        assert!(
            catalog
                .first_import_change_capture_is_ready(&request.scan_id, &root_id, generation)
                .expect("inspect refreshed first-import handoff")
        );
        let refreshed_unix_ms = catalog
            .load_persistent_journal_capabilities()
            .expect("load refreshed capability")
            .into_iter()
            .find(|capability| {
                capability.root_id == root_id && capability.root_generation == generation
            })
            .expect("refreshed LiveOnly capability")
            .updated_unix_ms;
        assert!(refreshed_unix_ms >= started_unix_ms);
        drop(catalog);

        poll_runtime_with_storage(&mut production, &storage)
            .expect("poll after first-import handoff became current");
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect stable catalog");
        let stable_unix_ms = catalog
            .load_persistent_journal_capabilities()
            .expect("load stable capability")
            .into_iter()
            .find(|capability| {
                capability.root_id == root_id && capability.root_generation == generation
            })
            .expect("stable LiveOnly capability")
            .updated_unix_ms;
        assert_eq!(stable_unix_ms, refreshed_unix_ms);
        production
            .stop()
            .expect("stop stale LiveOnly production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn failed_first_import_retries_with_fresh_generation_and_capture_owner() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let failed_request = ScanRequest {
            scan_id: "failed-first-import".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let retry_request = ScanRequest {
            scan_id: "retried-first-import".to_owned(),
            ..failed_request.clone()
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&failed_request, &root_id, &root_path)
            .expect("begin failed first import");
        let (_, failed_started_unix_ms) = catalog
            .pristine_first_import_scan(&root_id, generation)
            .expect("load failed first import")
            .expect("failed first import is pristine");
        let stale_unix_ms = failed_started_unix_ms
            .checked_sub(1)
            .expect("fixture scan starts after the Unix epoch");
        catalog
            .save_persistent_journal_capability(
                &super::super::journal_baseline::live_only_capability(
                    &root_id,
                    generation,
                    stale_unix_ms,
                ),
            )
            .expect("seed failed import capability");
        catalog
            .abandon_scan(&failed_request.scan_id, "failed", 0)
            .expect("abandon first import");
        catalog
            .begin_scan(&retry_request, &root_id, &root_path)
            .expect("begin explicit retry in a fresh generation");
        let root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load retry root")
            .expect("retry root");
        let generation = generation.next().expect("fresh generation");
        assert_eq!(root.root_generation, generation);
        assert!(
            !catalog
                .first_import_change_capture_is_ready(&retry_request.scan_id, &root_id, generation,)
                .expect("inspect stale retry handoff")
        );
        drop(catalog);
        crate::application::catalog_session::open_catalog(
            &storage.catalog_path,
            LibraryChangeLane::Recovery,
        )
        .expect("prime protected retry catalog session");
        let _capture = crate::application::scan_library::hold_first_import_capture(
            &retry_request.scan_id,
            &storage.catalog_path,
            &root_id,
            generation,
        )
        .expect("executing explicit retry");

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        poll_runtime_with_storage(&mut production, &storage)
            .expect("refresh explicit retry handoff");
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect retry catalog");
        assert!(
            catalog
                .first_import_change_capture_is_ready(&retry_request.scan_id, &root_id, generation,)
                .expect("inspect refreshed retry handoff")
        );
        production
            .stop()
            .expect("stop failed-import retry production runtime");
    }

    #[cfg(windows)]
    #[test]
    fn startup_retires_legacy_full_scan_into_bounded_inventory_without_new_scan() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let root_path = source_root.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let initial = ScanRequest {
            scan_id: "initial-scan".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let legacy = ScanRequest {
            scan_id: "legacy-automatic-full-scan".to_owned(),
            ..initial.clone()
        };
        let generation = crate::domain::LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&initial, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
            .expect("prove automatic-recovery fixture first-import handoff");
        catalog
            .publish_scan(&initial.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
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
            .expect("enqueue legacy recovery evidence");
        catalog
            .begin_authoritative_scan(&legacy, &root_id, &root_path)
            .expect("begin legacy automatic full scan");
        drop(catalog);

        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(
                crate::ports::erase_library_change_source_factory(HealthyFactory),
            ),
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
        };
        poll_runtime_with_storage(&mut production, &storage)
            .expect("start production synchronization");
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(25));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("advance production synchronization");
        }

        assert!(production.legacy_automatic_scans_retired);

        production.stop().expect("stop production synchronization");
        let catalog = SqliteCatalog::open(storage.catalog_path).expect("reopen catalog");
        let active_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load active root")
            .expect("active root");

        assert!(
            catalog
                .load_authoritative_recoverable_scan_after(None)
                .expect("legacy recoverable scan")
                .is_none()
        );
        assert_eq!(
            active_root.active_scan_id.as_deref(),
            Some(initial.scan_id.as_str())
        );
        let connection = rusqlite::Connection::open(catalog.catalog_path())
            .expect("open catalog for inventory assertion");
        let recovery_evidence: (i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_metadata_inventory_runs),
                   (SELECT COUNT(*) FROM scan_runs),
                   (SELECT COUNT(*) FROM library_change_queue_lanes
                    WHERE lane = 'p2_recovery')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("count bounded recovery and full-scan rows");
        assert_eq!(recovery_evidence, (0, 2, 1));
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
                continuity: PersistentJournalContinuityState::CatchingUp,
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
                continuity: PersistentJournalContinuityState::RecoveryRequired,
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
                continuity: PersistentJournalContinuityState::RecoveryRequired,
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
            .prove_live_only_first_import_handoff_for_test("scan-a")
            .expect("prove continuity revision fixture first-import handoff");
        catalog
            .publish_scan("scan-a", root_id, 0, 0)
            .expect("publish root scan");
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(
                crate::ports::erase_library_change_source_factory(HealthyFactory),
            ),
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
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
                change_id: LibraryChangeId::new(1).expect("test recovery change ID"),
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
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
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
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
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
        let mut baseline_change_ids = BTreeMap::new();
        for (index, (root_id, root_path)) in [("root-a", "C:\\RootA"), ("root-b", "C:\\RootB")]
            .into_iter()
            .enumerate()
        {
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
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove recovery cursor fixture first-import handoff");
            catalog
                .publish_scan(&scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    state: PersistentJournalCapabilityState::Supported,
                    continuity: PersistentJournalContinuityState::BaselineRequired,
                    failure: None,
                    updated_unix_ms: 1_000,
                })
                .expect("persist supported baseline capability");
            let identity_byte = u8::try_from(index + 1).expect("fixture identity byte");
            let baseline = catalog
                .begin_persistent_journal_baseline(
                    &PersistentJournalBaselineStartRequest {
                        run_id: format!("round-robin-{root_id}"),
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        authority_reason: LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                        volume: PersistentJournalVolumeIdentity {
                            volume_guid: format!("round-robin-volume-{index}"),
                            volume_serial: u64::try_from(index + 1).expect("fixture volume serial"),
                        },
                        root_file_reference: JournalFileReference::V2([identity_byte; 8]),
                        journal_id: JournalIdentifier::new(44).expect("fixture journal ID"),
                        opening_next_usn: JournalUsn::new(20).expect("fixture opening USN"),
                        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                        authorized_unix_ms: 1_000,
                    },
                    policy,
                )
                .expect("begin ready baseline recovery fixture");
            baseline_change_ids.insert(root_id, baseline.change_id);
        }
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 1_500,
                    most_recent_observed_unix_ms: 1_500,
                    first_sequence: 2,
                    most_recent_sequence: 2,
                    coalesced_observation_count: 1,
                }],
                1_500,
                policy,
            )
            .expect("enqueue competing root-a P0 gap");
        let competing = catalog
            .lease_live_authoritative_library_change("root-a", generation, 1_500, policy)
            .expect("lease competing root-a P0 gap")
            .expect("competing root-a P0 gap");
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                competing.change.id,
                competing.lease_generation,
                &crate::domain::LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "Fixture gap requires a distinct P2 owner".to_owned(),
                },
                1_500,
                policy,
            )
            .expect("promote competing root-a P2 owner");
        let root_a_baseline = baseline_change_ids["root-a"];
        rusqlite::Connection::open(catalog.catalog_path())
            .expect("open root-a affinity evidence")
            .execute(
                "UPDATE library_change_queue SET ready_unix_ms = 3000 WHERE id = ?1",
                [i64::try_from(root_a_baseline.value()).expect("root-a baseline ID")],
            )
            .expect("delay exact root-a baseline owner");
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
                    continuity: PersistentJournalContinuityState::RecoveryRequired,
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
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
        };

        let first = ready_recovery_work(&mut production, &catalog, &snapshot, 2_000)
            .expect("skip unavailable exact root-a owner")
            .expect("root-b remains ready");
        assert_eq!(first.root_id(), "root-b");
        rusqlite::Connection::open(catalog.catalog_path())
            .expect("open root-a affinity release")
            .execute(
                "UPDATE library_change_queue SET ready_unix_ms = 2000 WHERE id = ?1",
                [i64::try_from(root_a_baseline.value()).expect("root-a baseline ID")],
            )
            .expect("release exact root-a baseline owner");
        let second = ready_recovery_work(&mut production, &catalog, &snapshot, 2_000)
            .expect("select released exact root-a owner")
            .expect("root-a becomes ready");
        assert_eq!(second.root_id(), "root-a");
        let leased_root_a = catalog
            .lease_metadata_inventory_recovery("root-a", generation, 2_000, policy)
            .expect("lease exact root-a baseline owner")
            .expect("exact root-a baseline owner");
        assert_eq!(leased_root_a.change.id, root_a_baseline);
        let wrapped = ready_recovery_work(&mut production, &catalog, &snapshot, 2_000)
            .expect("wrapped ready root")
            .expect("wrapped root");

        assert_eq!(wrapped.root_id(), "root-b");
    }

    #[cfg(windows)]
    #[test]
    fn production_does_not_mint_recovery_authority_for_an_ordinary_consistency_audit() {
        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("ordinary audit root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical ordinary audit root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("ordinary audit root identity query")
            .expect("ordinary audit root stable identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let generation = crate::domain::LibraryRootGeneration::initial();
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "ordinary-audit-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("ordinary-audit-scan")
            .expect("prove audit fixture first-import handoff");
        catalog
            .publish_scan("ordinary-audit-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: 1,
                    most_recent_observed_unix_ms: 1,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                1,
                policy,
            )
            .expect("enqueue ordinary audit");
        drop(catalog);

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage).expect("poll production runtime");

        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
        ));
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("finish blocking the unowned ordinary audit");
            if production.recovery.is_none() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the unowned ordinary audit did not reach its durable blocked state"
            );
        }
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open authority evidence catalog");
        let authority_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_recovery_authorities",
                [],
                |row| row.get(0),
            )
            .expect("count recovery authorities");
        assert_eq!(authority_count, 0, "worker must never mint authority");
        let blocked: (String, i64, Option<i64>, Option<String>) = connection
            .query_row(
                "SELECT status, attempt_count, next_retry_unix_ms, last_failure_code
                 FROM library_change_queue WHERE root_id = ?1",
                [&root_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load blocked ordinary audit");
        assert_eq!(
            blocked,
            (
                "retry_wait".to_owned(),
                i64::from(policy.max_attempts),
                None,
                Some("legacy_recovery_authority_missing".to_owned())
            )
        );
        poll_runtime_with_storage(&mut production, &storage)
            .expect("blocked ordinary audit remains idempotent");
        assert!(
            production.recovery.is_none(),
            "a blocked unowned audit must not restart the worker"
        );
        production.stop().expect("stop production synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn production_coordinator_completes_4096_real_files_across_the_default_page() {
        const TOTAL_FILES: usize = METADATA_INVENTORY_WORK_PAGE_ENTRIES as usize + 1;
        const TEST_DEADLINE: Duration = Duration::from_secs(300);

        let started = std::time::Instant::now();
        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let template = directory.path().join("template.png");
        image::RgbImage::from_pixel(1, 1, image::Rgb([40, 80, 160]))
            .save(&template)
            .expect("PNG template");
        let png = std::fs::read(&template).expect("read PNG template");
        for index in 0..TOTAL_FILES {
            std::fs::write(source.join(format!("image-{index:05}.png")), &png)
                .expect("write controlled PNG");
        }

        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("controlled root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical controlled root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("controlled root identity query")
            .expect("controlled stable root identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            lease_duration_millis:
                crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let generation = LibraryRootGeneration::initial();
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "production-4096-initial".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 64,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin empty initial catalog");
        catalog
            .prove_live_only_first_import_handoff_for_test("production-4096-initial")
            .expect("prove 4096-file fixture first-import handoff");
        catalog
            .publish_scan("production-4096-initial", &root_id, 0, 0)
            .expect("publish empty initial catalog");

        let registration =
            describe_production_persistent_journal_root(&root_id, generation.value(), &source)
                .expect("describe controlled root");
        let root_reference =
            JournalFileReference::from_bytes(&registration.authorization.root_identity)
                .expect("controlled root reference");
        let volume = PersistentJournalVolumeIdentity {
            volume_guid: registration.authorization.volume_id.clone(),
            volume_serial: registration.volume_serial,
        };
        let journal_id = JournalIdentifier::new(44).expect("journal ID");
        let opening_next_usn = JournalUsn::new(20).expect("opening USN");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("persist supported journal capability");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load controlled root")
            .expect("published controlled root");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.clone(),
                root_generation: generation,
                volume: volume.clone(),
                root_file_reference: root_reference.clone(),
                journal_id,
                next_unread_usn: opening_next_usn,
                captured_exclusive_end: opening_next_usn,
                covered_catalog_revision: current_root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("persist opening checkpoint");
        let recovery_control = catalog
            .persist_persistent_journal_root_failure(
                &crate::domain::PersistentJournalRootFailure {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                    failure: PersistentJournalFailure {
                        code: "production-4096-containment".to_owned(),
                        message: "Controlled recovery fixture".to_owned(),
                    },
                    opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                        volume: volume.clone(),
                        root_file_reference: root_reference,
                        journal_id,
                        next_usn: opening_next_usn,
                        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    }),
                },
                base_unix_ms + 1,
                policy,
            )
            .expect("persist typed containment failure")
            .expect("containment failure admits recovery");
        drop(catalog);

        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(AdvancingJournalSession {
            boundaries: Mutex::new(std::collections::VecDeque::from([
                opening_next_usn.value(),
                opening_next_usn.value(),
            ])),
            query_count: Arc::clone(&query_count),
            shared_read_count: Arc::clone(&shared_read_count),
            read_release: Arc::new(AtomicBool::new(true)),
            close_count: Arc::clone(&close_count),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [6; 16],
        });
        production.runtime.queue_policy = policy;
        assert_eq!(
            production.metadata_inventory_page_entries,
            METADATA_INVENTORY_WORK_PAGE_ENTRIES
        );

        let evidence_connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open retained evidence connection");
        let mut saw_bounded_raw_capture = false;
        let mut saw_default_source_page = false;
        let mut saw_backpressured_drain = false;
        let mut saw_refill_beyond_first_admission = false;
        let mut completed = false;
        let mut last_evidence = None;
        while started.elapsed() < TEST_DEADLINE {
            poll_runtime_with_storage(&mut production, &storage)
                .expect("drive production recovery coordinator");
            let source_reads = crate::adapters::source_entry_read_count(&root_path);
            saw_bounded_raw_capture |= source_reads > 0 && source_reads < TOTAL_FILES as u64;

            let evidence: (
                i64,
                i64,
                String,
                String,
                i64,
                String,
                String,
                String,
                i64,
                i64,
            ) = evidence_connection
                .query_row(
                    "SELECT
                           COALESCE(run.staged_entry_count, 0),
                           COALESCE(run.candidate_count, 0),
                           COALESCE(run.status, 'missing'),
                           control.status,
                           authority.retired_unix_ms IS NOT NULL,
                           baseline.phase,
                           COALESCE(checkpoint.continuity_state, 'missing'),
                           root_state.continuity_state,
                           (SELECT COUNT(*)
                            FROM library_metadata_inventory_candidate_owners AS owner
                            WHERE owner.run_id = authority.run_id),
                           (SELECT COUNT(*)
                            FROM library_metadata_inventory_candidate_owners AS owner
                            JOIN library_change_queue AS owned ON owned.id = owner.change_id
                            WHERE owner.run_id = authority.run_id
                              AND owned.status = 'completed')
                         FROM library_recovery_authorities AS authority
                         JOIN library_change_queue AS control ON control.id = authority.change_id
                         JOIN library_persistent_journal_baselines AS baseline
                           ON baseline.change_id = authority.change_id
                         JOIN library_persistent_journal_root_state AS root_state
                           ON root_state.root_id = authority.root_id
                          AND root_state.root_generation = authority.root_generation
                         LEFT JOIN library_metadata_inventory_runs AS run
                           ON run.id = authority.run_id
                         LEFT JOIN library_persistent_journal_checkpoints AS checkpoint
                           ON checkpoint.root_id = authority.root_id
                          AND checkpoint.root_generation = authority.root_generation
                         WHERE authority.change_id = ?1",
                    [i64::try_from(recovery_control.value()).expect("control ID")],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                            row.get(9)?,
                        ))
                    },
                )
                .expect("load production-chain evidence");
            saw_default_source_page |=
                evidence.0 == i64::from(METADATA_INVENTORY_WORK_PAGE_ENTRIES);
            saw_backpressured_drain |= evidence.1
                == i64::from(policy.lane_capacity(LibraryChangeLane::Recovery))
                && evidence.9 > 0;
            saw_refill_beyond_first_admission |=
                evidence.1 > i64::from(policy.lane_capacity(LibraryChangeLane::Recovery));
            if evidence
                == (
                    i64::try_from(TOTAL_FILES).expect("staged total"),
                    i64::try_from(TOTAL_FILES).expect("candidate total"),
                    "completed".to_owned(),
                    "completed".to_owned(),
                    1,
                    "completed".to_owned(),
                    "current".to_owned(),
                    "current".to_owned(),
                    i64::try_from(TOTAL_FILES).expect("owner total"),
                    i64::try_from(TOTAL_FILES).expect("terminal total"),
                )
            {
                completed = true;
                break;
            }
            last_evidence = Some(evidence);
            std::thread::sleep(Duration::from_millis(2));
        }

        assert!(
            completed,
            "the production chain must finish every owner and retire authority; last={last_evidence:?}"
        );
        assert!(
            saw_bounded_raw_capture,
            "the real directory must yield before consuming all 4096 source entries"
        );
        assert!(
            saw_default_source_page,
            "the production source must durably publish its 4095-entry default page"
        );
        assert!(
            saw_backpressured_drain,
            "the first P2 admission must drain before candidate refill"
        );
        assert!(
            saw_refill_beyond_first_admission,
            "production refill must cross the first P2 admission window"
        );
        assert_eq!(
            crate::adapters::source_entry_read_count(&root_path),
            u64::try_from(TOTAL_FILES).expect("source reads")
        );
        assert_eq!(crate::adapters::source_spool_open_count(&root_path), 1);
        assert!(crate::adapters::source_peak_staged_window(&root_path) <= 128);
        let spool_state: (i64, i64) = evidence_connection
            .query_row(
                "SELECT COUNT(*), COUNT(*) FILTER (WHERE state = 'retired')
                 FROM library_metadata_inventory_spools",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load atomic retirement and background cleanup evidence");
        assert!(
            matches!(spool_state, (0, 0) | (1, 1)),
            "completed recovery permits only an absent or retired spool, never executable storage: {spool_state:?}"
        );
        assert!(
            query_count.load(Ordering::Acquire) >= 1,
            "production must capture the closing journal boundary"
        );
        assert_eq!(
            shared_read_count.load(Ordering::Acquire),
            1,
            "closing must perform exactly one bounded replay read even when the window is empty"
        );
        production.stop().expect("stop production synchronization");
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())
            .unwrap_or_else(|error| {
                let evidence: String = evidence_connection.query_row(
                    "SELECT json_object('baseline_completed', baseline.completed_unix_ms,
                        'closing', baseline.closing_next_usn, 'next', checkpoint.next_unread_usn,
                        'end', checkpoint.captured_exclusive_end,
                        'root_state', root.continuity_state, 'checkpoint_state', checkpoint.continuity_state,
                        'range_start', ranges.requested_start_usn, 'range_end', ranges.requested_end_usn,
                        'range_covered', ranges.covered_until_usn, 'range_state', ranges.status,
                        'range_enrolled', ranges.enrolled_unix_ms, 'range_lifecycle', lifecycle.lifecycle_state)
                     FROM library_persistent_journal_baselines AS baseline
                     JOIN library_persistent_journal_root_state AS root USING(root_id, root_generation)
                     JOIN library_persistent_journal_checkpoints AS checkpoint USING(root_id, root_generation)
                     LEFT JOIN library_persistent_journal_source_ranges AS ranges
                       ON ranges.root_id = baseline.root_id AND ranges.covered_until_usn = checkpoint.next_unread_usn
                     LEFT JOIN library_persistent_journal_range_lifecycle AS lifecycle ON lifecycle.source_range_id = ranges.id
                     ORDER BY ranges.enrolled_unix_ms DESC LIMIT 1", [], |row| row.get(0),
                ).expect("bounded baseline failure evidence");
                panic!("FULL reopen after production retirement: {error:?}; {evidence}")
            });
        let run_id: String = evidence_connection
            .query_row(
                "SELECT run_id FROM library_recovery_authorities WHERE change_id = ?1",
                [i64::try_from(recovery_control.value()).expect("control ID")],
                |row| row.get(0),
            )
            .expect("completed recovery run identity");
        let run = catalog
            .load_metadata_inventory_run(&run_id)
            .expect("load completed run")
            .expect("retention preserves the completed summary");
        assert_eq!(
            catalog
                .stage_metadata_inventory_page(
                    &run_id,
                    &crate::domain::MetadataInventoryPage {
                        page_index: run.next_page_index,
                        entries: Vec::new(),
                        cursor: None,
                        is_complete: true,
                        frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                            "",
                            Some(publication_identity),
                        )],
                    },
                    now_unix_ms().expect("terminal page attempt clock"),
                )
                .expect_err("completed recovery cannot accept another source page")
                .code,
            "metadata_inventory_run_not_running",
        );
        let remaining_entries: i64 = evidence_connection.query_row(
            "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1)
                  + (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1)",
            [&run_id], |row| row.get(0),
        ).expect("persisted raw and logical debt after all workers stop");
        let mut reclaimed_entries = 0_i64;
        let mut cleanup_complete = false;
        for _ in 0..128 {
            assert!(
                started.elapsed() < TEST_DEADLINE,
                "cleanup shares the original test deadline"
            );
            let report = catalog
                .cleanup_terminal_metadata_inventories(0, 128, 1, Default::default())
                .expect("one real bounded cleanup batch");
            assert!(report.removed_entry_count <= 128);
            assert_eq!(
                report.removed_run_count, 0,
                "existing terminal retention stays intact"
            );
            reclaimed_entries += i64::from(report.removed_entry_count);
            if !report.has_more {
                cleanup_complete = true;
                break;
            }
        }
        assert!(
            cleanup_complete,
            "the 4096-file debt must finish within 128 bounded batches"
        );
        assert_eq!(reclaimed_entries, remaining_entries);
        let retained: (i64, i64, i64, i64, i64) = evidence_connection.query_row(
            "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
                    (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories WHERE run_id = ?1),
                    (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1),
                    (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1),
                    (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = ?1)",
            [&run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).expect("physical retirement and retained candidate lineage");
        assert_eq!(
            retained,
            (0, 0, 0, 0, i64::try_from(TOTAL_FILES).expect("owner total"))
        );
        assert_eq!(
            crate::adapters::source_entry_read_count(&root_path),
            u64::try_from(TOTAL_FILES).expect("unchanged source reads"),
        );
        SqliteCatalog::open(storage.catalog_path.clone())
            .expect("FULL reopen after all bounded raw and logical cleanup batches");
    }

    #[cfg(windows)]
    fn ordinary_user_test_directory() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::current_dir().expect("current directory"))
            .expect("ordinary-user disposable test directory")
    }

    #[cfg(windows)]
    fn assert_production_p0_catalog_commit_guard(
        scope: crate::domain::LibraryChangeScope,
        relative_path: &str,
        guard_ancestor: bool,
        with_running_scan: bool,
    ) {
        let directory = ordinary_user_test_directory();
        let namespace = directory.path().join("namespace");
        let source = namespace.join("source");
        std::fs::create_dir_all(source.join("album")).expect("source namespace");
        let changed_relative_path = if scope == crate::domain::LibraryChangeScope::Root {
            "created.png"
        } else {
            "album/created.png"
        };
        image::RgbImage::from_pixel(2, 2, image::Rgb([40, 80, 160]))
            .save_with_format(source.join(changed_relative_path), image::ImageFormat::Png)
            .expect("authoritative source fixture");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("authoritative root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical authoritative root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("authoritative root identity query")
            .expect("authoritative stable root identity");
        drop(discovery);
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "p0-commit-window-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin proven initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("p0-commit-window-scan")
            .expect("prove P0 commit-window fixture first-import handoff");
        catalog
            .publish_scan("p0-commit-window-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        let initial_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load initial root")
            .expect("published initial root");
        if with_running_scan {
            catalog
                .begin_scan_with_publication_namespace(
                    &ScanRequest {
                        scan_id: "p0-concurrent-full-scan".to_owned(),
                        root_path: root_path.clone(),
                        max_items: None,
                        max_entries: None,
                        preview_edge: 512,
                    },
                    &root_id,
                    &root_path,
                    &publication_identity,
                )
                .expect("begin concurrent full scan");
        }
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope,
                    relative_path: relative_path.to_owned(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms,
                policy,
            )
            .expect("enqueue P0 authoritative work");
        drop(catalog);

        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        let guarded_path = if guard_ancestor {
            namespace.clone()
        } else {
            source.clone()
        };
        let moved_path = guarded_path.with_file_name(if guard_ancestor {
            "held-p0-ancestor"
        } else {
            "held-p0-root"
        });
        let rename_error = Arc::new(std::sync::atomic::AtomicI32::new(i32::MIN));
        let enumerated_entries = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let hook_rename_error = Arc::clone(&rename_error);
        let hook_enumerated_entries = Arc::clone(&enumerated_entries);
        let hook_root_path = root_path.clone();
        let hook_guarded_path = guarded_path.clone();
        let hook_moved_path = moved_path.clone();
        let _commit_hook =
            crate::adapters::set_before_catalog_delta_commit_hook(&root_id, move || {
                hook_enumerated_entries.store(
                    crate::adapters::source_entry_read_count(&hook_root_path),
                    Ordering::Release,
                );
                let error = std::fs::rename(&hook_guarded_path, &hook_moved_path)
                    .expect_err("the production publication guard must reject the rename");
                hook_rename_error.store(error.raw_os_error().unwrap_or(-1), Ordering::Release);
                Err(ScanError::new(
                    "root_publication_namespace_guard_unsupported",
                    "The commit-window guard capability became unavailable",
                ))
            });

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage)
            .expect("schedule production P0 authoritative work");
        assert!(
            production.live.is_some(),
            "the production P0 worker must start"
        );

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("poll production P0 authoritative work");
            let evidence: (String, Option<String>, Option<String>, Option<i64>) =
                rusqlite::Connection::open(&storage.catalog_path)
                    .expect("open P0 commit-window evidence")
                    .query_row(
                        "SELECT status, last_failure_code, last_failure_message,
                                catalog_revision_at_success
                         FROM library_change_queue
                         WHERE root_id = ?1 AND root_generation = ?2
                           AND origin = 'live_notification'",
                        rusqlite::params![
                            root_id,
                            i64::try_from(generation.value()).expect("root generation")
                        ],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .expect("load P0 commit-window evidence");
            if evidence.0 == "retry_wait" {
                assert_eq!(
                    evidence.1.as_deref(),
                    Some("root_publication_namespace_guard_unsupported")
                );
                assert_eq!(evidence.3, None);
                assert_ne!(
                    rename_error.load(Ordering::Acquire),
                    i32::MIN,
                    "commit hook did not fire; failure was {:?}",
                    evidence.2
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the P0 commit-window failure did not become durably retryable"
            );
        }
        let rename_error = rename_error.load(Ordering::Acquire);
        assert_eq!(rename_error, 32, "the held guard must deny delete sharing");
        if scope == crate::domain::LibraryChangeScope::Path {
            assert_eq!(
                enumerated_entries.load(Ordering::Acquire),
                0,
                "an exact P0 path must not enumerate its containing directory"
            );
        } else {
            assert!(enumerated_entries.load(Ordering::Acquire) > 0);
            assert!(crate::adapters::source_spool_open_count(&root_path) > 0);
        }
        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("evidence catalog");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("reload guarded root")
            .expect("guarded root remains published");
        assert_eq!(current_root.catalog_revision, initial_root.catalog_revision);
        assert_eq!(current_root.active_scan_id, initial_root.active_scan_id);
        assert_eq!(current_root.has_running_scan, with_running_scan);
        assert_eq!(
            current_root.last_consistency_audit_unix_ms,
            initial_root.last_consistency_audit_unix_ms
        );
        assert_eq!(
            current_root.publication_root_identity.as_ref(),
            Some(&publication_identity)
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&root_id, changed_relative_path)
                .expect("load rolled-back location")
                .is_none()
        );
        drop(catalog);
        production.stop().expect("stop production synchronization");
        std::fs::rename(&guarded_path, &moved_path)
            .expect("guarded namespace rename after production worker exit");
        std::fs::rename(&moved_path, &guarded_path).expect("restore guarded namespace");
    }

    #[cfg(windows)]
    #[test]
    fn production_p0_root_holds_guard_through_catalog_rollback_on_commit_window_replacement() {
        assert_production_p0_catalog_commit_guard(
            crate::domain::LibraryChangeScope::Root,
            "",
            false,
            false,
        );
    }

    #[cfg(windows)]
    #[test]
    fn production_p0_subtree_holds_ancestor_guard_through_catalog_rollback() {
        assert_production_p0_catalog_commit_guard(
            crate::domain::LibraryChangeScope::Subtree,
            "album",
            true,
            false,
        );
    }

    #[cfg(windows)]
    #[test]
    fn production_p0_live_worker_crosses_running_full_scan_gate() {
        assert_production_p0_catalog_commit_guard(
            crate::domain::LibraryChangeScope::Path,
            "album/created.png",
            false,
            true,
        );
    }

    #[cfg(windows)]
    #[test]
    fn production_bounded_p2_holds_guard_through_catalog_rollback_without_retiring_authority() {
        let directory = ordinary_user_test_directory();
        let namespace = directory.path().join("namespace");
        let source = namespace.join("source");
        std::fs::create_dir_all(&source).expect("source namespace");
        image::RgbImage::from_pixel(2, 2, image::Rgb([80, 40, 160]))
            .save_with_format(source.join("created.png"), image::ImageFormat::Png)
            .expect("bounded P2 source fixture");
        let replacement = namespace.join("replacement");
        std::fs::create_dir(&replacement).expect("replacement root fixture");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("bounded P2 root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical bounded P2 root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("bounded P2 root identity query")
            .expect("bounded P2 stable root identity");
        drop(discovery);
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "bounded-p2-commit-window-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin proven initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("bounded-p2-commit-window-scan")
            .expect("prove bounded P2 fixture first-import handoff");
        catalog
            .publish_scan("bounded-p2-commit-window-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        let initial_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load bounded P2 root")
            .expect("published bounded P2 root");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed bounded P2 baseline capability");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &PersistentJournalBaselineStartRequest {
                    run_id: "bounded-p2-commit-window".to_owned(),
                    root_id: root_id.clone(),
                    root_generation: generation,
                    authority_reason: LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                    volume: PersistentJournalVolumeIdentity {
                        volume_guid: "bounded-p2-volume".to_owned(),
                        volume_serial: 7,
                    },
                    root_file_reference: JournalFileReference::V2([7; 8]),
                    journal_id: JournalIdentifier::new(44).expect("fixture journal ID"),
                    opening_next_usn: JournalUsn::new(20).expect("fixture opening USN"),
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    authorized_unix_ms: base_unix_ms,
                },
                policy,
            )
            .expect("begin bounded P2 control window");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::UserRefresh,
                    first_observed_unix_ms: base_unix_ms + 1,
                    most_recent_observed_unix_ms: base_unix_ms + 1,
                    first_sequence: 2,
                    most_recent_sequence: 2,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms + 1,
                policy,
            )
            .expect("merge explicit refresh into bounded P2 control");
        drop(catalog);
        let fixture_connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open bounded P2 fixture evidence");
        let fixture_control: (String, String, Option<i64>) = fixture_connection
            .query_row(
                "SELECT queue.origin, queue.status, authority.retired_unix_ms
                 FROM library_change_queue AS queue
                 JOIN library_recovery_authorities AS authority ON authority.change_id = queue.id
                 WHERE queue.id = ?1",
                [i64::try_from(baseline.change_id.value()).expect("baseline change ID")],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load bounded P2 control");
        assert_eq!(
            fixture_control,
            ("user_refresh".to_owned(), "pending".to_owned(), None)
        );
        drop(fixture_connection);

        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        let displaced = namespace.join("displaced-source");
        let rename_error = Arc::new(std::sync::atomic::AtomicI32::new(i32::MIN));
        let enumerated_entries = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let hook_rename_error = Arc::clone(&rename_error);
        let hook_enumerated_entries = Arc::clone(&enumerated_entries);
        let hook_root_path = root_path.clone();
        let hook_source = source.clone();
        let hook_displaced = displaced.clone();
        let _commit_hook =
            crate::adapters::set_before_catalog_delta_commit_hook(&root_id, move || {
                hook_enumerated_entries.store(
                    crate::adapters::source_entry_read_count(&hook_root_path),
                    Ordering::Release,
                );
                let error = std::fs::rename(&hook_source, &hook_displaced)
                    .expect_err("the production P2 guard must prevent root displacement");
                hook_rename_error.store(error.raw_os_error().unwrap_or(-1), Ordering::Release);
                Err(ScanError::new(
                    "root_publication_namespace_guard_unsupported",
                    "The P2 commit-window guard capability became unavailable",
                ))
            });

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage)
            .expect("schedule production bounded P2 work");
        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::BoundedAuthoritative { .. })
        ));

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("poll production bounded P2 work");
            let evidence: (String, Option<String>, Option<i64>, String, Option<i64>) =
                rusqlite::Connection::open(&storage.catalog_path)
                    .expect("open bounded P2 commit-window evidence")
                    .query_row(
                        "SELECT queue.status, queue.last_failure_code,
                                queue.catalog_revision_at_success, baseline.phase,
                                authority.retired_unix_ms
                         FROM library_change_queue AS queue
                         JOIN library_persistent_journal_baselines AS baseline
                           ON baseline.change_id = queue.id
                         JOIN library_recovery_authorities AS authority
                           ON authority.change_id = queue.id
                         WHERE queue.id = ?1",
                        [i64::try_from(baseline.change_id.value()).expect("baseline change ID")],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        },
                    )
                    .expect("load bounded P2 commit-window evidence");
            if evidence.0 == "retry_wait" {
                assert_eq!(
                    evidence,
                    (
                        "retry_wait".to_owned(),
                        Some("root_publication_namespace_guard_unsupported".to_owned()),
                        None,
                        "inventory".to_owned(),
                        None,
                    )
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the bounded P2 commit-window failure did not become durably retryable"
            );
        }
        let rename_error = rename_error.load(Ordering::Acquire);
        assert_eq!(
            rename_error, 32,
            "the held P2 guard must deny delete sharing"
        );
        assert!(enumerated_entries.load(Ordering::Acquire) > 0);
        assert!(crate::adapters::source_spool_open_count(&root_path) > 0);
        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("evidence catalog");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("reload bounded P2 root")
            .expect("bounded P2 root remains published");
        assert_eq!(current_root.catalog_revision, initial_root.catalog_revision);
        assert_eq!(current_root.active_scan_id, initial_root.active_scan_id);
        assert_eq!(
            current_root.publication_root_identity.as_ref(),
            Some(&publication_identity)
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&root_id, "created.png")
                .expect("load rolled-back bounded P2 location")
                .is_none()
        );
        drop(catalog);
        production.stop().expect("stop production synchronization");
        std::fs::rename(&source, &displaced).expect("root displacement after P2 worker exit");
        std::fs::rename(&replacement, &source).expect("install replacement after guard release");
        std::fs::rename(&source, &replacement).expect("restore replacement fixture");
        std::fs::rename(&displaced, &source).expect("restore source root");
    }

    #[cfg(windows)]
    #[test]
    fn p0_root_and_subtree_use_the_reserved_worker_while_p2_is_full_active_and_retrying() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempfile::tempdir_in(current_directory).expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(source.join("album")).expect("source subtree");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("priority fixture root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical priority fixture root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("priority fixture root identity query")
            .expect("priority fixture stable root identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_unresolved_changes: 16,
            max_lease_batch: 1,
            lease_duration_millis:
                crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let p2_capacity =
            usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
                .expect("P2 capacity");
        assert_eq!(p2_capacity, 12);
        let p2_candidate_capacity = p2_capacity
            .checked_sub(1)
            .expect("the P2 control lease occupies one recovery slot");
        let generation = crate::domain::LibraryRootGeneration::initial();
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let run_id = "p0-reserved-worker-recovery";
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "p0-reserved-worker-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin proven initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("p0-reserved-worker-scan")
            .expect("prove reserved-worker fixture first-import handoff");
        catalog
            .publish_scan("p0-reserved-worker-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms,
                policy,
            )
            .expect("enqueue recovery control");
        let authority = catalog
            .lease_authoritative_library_change(&root_id, generation, base_unix_ms, policy)
            .expect("lease recovery authority")
            .expect("recovery control");
        catalog
            .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
                change_id: authority.change.id,
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
                opening_boundary: None,
                authorized_unix_ms: base_unix_ms,
                retired_unix_ms: None,
            })
            .expect("persist recovery authority");
        catalog
            .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                epoch: 1,
                scope: crate::domain::MetadataInventoryScope::Root,
                started_unix_ms: base_unix_ms,
            })
            .expect("begin recovery inventory");
        let p2_paths = (0..p2_candidate_capacity)
            .map(|index| format!("p2-{index:02}.jpg"))
            .collect::<Vec<_>>();
        catalog
            .stage_metadata_inventory_page(
                run_id,
                &crate::domain::MetadataInventoryPage {
                    page_index: 1,
                    entries: p2_paths
                        .iter()
                        .map(|relative_path| crate::domain::MetadataInventoryEntry {
                            relative_path: relative_path.clone(),
                            kind: crate::domain::MetadataInventoryEntryKind::File,
                            file_size: Some(1),
                            modified_unix_ms: base_unix_ms,
                            file_identity: None,
                            source_revision: None,
                            placeholder_state:
                                crate::domain::MetadataInventoryPlaceholderState::Available,
                            is_reparse_point: false,
                        })
                        .collect(),
                    cursor: p2_paths.last().cloned(),
                    is_complete: true,
                    frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                        "", None,
                    )],
                },
                base_unix_ms + 1,
            )
            .expect("stage full P2 slice");
        let p2_intents = p2_paths
            .iter()
            .enumerate()
            .map(
                |(index, relative_path)| crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: relative_path.clone(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                    most_recent_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                    coalesced_observation_count: 1,
                },
            )
            .collect::<Vec<_>>();
        let p2_updates = p2_paths
            .iter()
            .map(
                |relative_path| crate::domain::MetadataInventoryComparisonUpdate {
                    relative_path: relative_path.clone(),
                    status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                    candidate_previous_relative_path: None,
                },
            )
            .collect::<Vec<_>>();
        catalog
            .publish_metadata_inventory_comparison_candidates(
                &authority,
                run_id,
                &p2_intents,
                &p2_updates,
                base_unix_ms + 2,
                policy,
            )
            .expect("fill the P2 lane")
            .expect("authority remains current");
        let retry_candidate = catalog
            .lease_metadata_inventory_recovery_candidates(
                &root_id,
                generation,
                base_unix_ms + 3,
                policy,
            )
            .expect("lease retry fixture")
            .pop()
            .expect("owned retry candidate");
        catalog
            .retry_library_change(
                retry_candidate.change.id,
                retry_candidate.lease_generation,
                &crate::domain::LibraryChangeFailure {
                    code: "p2_fixture_retry".to_owned(),
                    message: "The candidate remains retryable while P0 enters".to_owned(),
                },
                base_unix_ms + 4,
                policy,
            )
            .expect("persist P2 retry state");
        drop(catalog);

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage)
            .expect("start production P2 candidate drain");
        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::CandidateDrain { .. })
        ));

        for (index, (scope, relative_path)) in [
            (crate::domain::LibraryChangeScope::Subtree, "album"),
            (crate::domain::LibraryChangeScope::Root, ""),
        ]
        .into_iter()
        .enumerate()
        {
            let sequence = u64::try_from(index + 1).expect("P0 sequence");
            let live_unix_ms = base_unix_ms + 10 + i64::from(index as u32);
            let mut catalog =
                SqliteCatalog::open(storage.catalog_path.clone()).expect("P0 enqueue catalog");
            catalog
                .enqueue_library_change_intents(
                    &[crate::domain::LibraryChangeIntent {
                        root_id: root_id.clone(),
                        root_generation: generation,
                        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                        scope,
                        relative_path: relative_path.to_owned(),
                        previous_relative_path: None,
                        origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                        first_observed_unix_ms: live_unix_ms,
                        most_recent_observed_unix_ms: live_unix_ms,
                        first_sequence: sequence,
                        most_recent_sequence: sequence,
                        coalesced_observation_count: 1,
                    }],
                    live_unix_ms,
                    policy,
                )
                .expect("P0 must enter through the reserved capacity");
            let snapshot = production
                .runtime
                .poll_without_authoritative_recovery(&mut catalog, live_unix_ms, |path| {
                    inspect_root_availability(path).availability
                })
                .expect("project P0 scheduling state");
            production
                .schedule_next_live_work(&catalog, &snapshot, live_unix_ms, &storage)
                .expect("schedule reserved P0 worker");
            assert!(
                production.live.is_some(),
                "P0 must own its independent worker"
            );
            if index == 0 {
                assert!(
                    production.recovery.is_some(),
                    "P0 scheduling must not consume the retained P2 worker slot"
                );
            }
            drop(catalog);

            let mut p0_completed = false;
            for _ in 0..200 {
                std::thread::sleep(Duration::from_millis(5));
                poll_runtime_with_storage(&mut production, &storage)
                    .expect("publish P0 authoritative work");
                let connection = rusqlite::Connection::open(&storage.catalog_path)
                    .expect("open P0 completion catalog");
                let completed_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*)
                         FROM library_change_queue AS queue
                         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                         WHERE queue.root_id = ?1 AND lane.lane = 'p0_live'
                           AND queue.status = 'completed'",
                        [&root_id],
                        |row| row.get(0),
                    )
                    .expect("count completed P0 work");
                if completed_count >= i64::try_from(index + 1).expect("P0 completion count") {
                    p0_completed = true;
                    break;
                }
            }
            assert!(
                p0_completed,
                "P0 root/subtree must publish while P2 remains active"
            );
        }

        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open priority evidence catalog");
        let retry_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_metadata_inventory_candidate_owners AS owner
                 JOIN library_change_queue AS queue ON queue.id = owner.change_id
                 WHERE owner.run_id = ?1 AND queue.status = 'retry_wait'",
                [run_id],
                |row| row.get(0),
            )
            .expect("count retained P2 retries");
        assert_eq!(retry_count, 1);
        production.stop().expect("stop production synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn production_candidate_drain_rotates_between_roots_before_revisiting_busy_work() {
        let directory = tempfile::tempdir().expect("test directory");
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_unresolved_changes: 16,
            max_lease_batch: 1,
            lease_duration_millis:
                crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let generation = crate::domain::LibraryRootGeneration::initial();
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        for (root_index, (root_id, candidate_count)) in [("root-a", 2_usize), ("root-b", 1_usize)]
            .into_iter()
            .enumerate()
        {
            let source = directory.path().join(root_id);
            std::fs::create_dir_all(&source).expect("source root");
            let root_path = source.to_string_lossy().into_owned();
            let scan_id = format!("candidate-round-robin-scan-{root_id}");
            let run_id = format!("candidate-round-robin-run-{root_id}");
            catalog
                .begin_scan(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
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
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove candidate-drain fixture first-import handoff");
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
                        origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                        first_observed_unix_ms: base_unix_ms,
                        most_recent_observed_unix_ms: base_unix_ms,
                        first_sequence: 1,
                        most_recent_sequence: 1,
                        coalesced_observation_count: 1,
                    }],
                    base_unix_ms,
                    policy,
                )
                .expect("enqueue recovery control");
            let authority = catalog
                .lease_authoritative_library_change(root_id, generation, base_unix_ms, policy)
                .expect("lease recovery authority")
                .expect("recovery control");
            catalog
                .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
                    change_id: authority.change.id,
                    run_id: run_id.clone(),
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
                    opening_boundary: None,
                    authorized_unix_ms: base_unix_ms,
                    retired_unix_ms: None,
                })
                .expect("persist recovery authority");
            let run = catalog
                .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
                    run_id: run_id.clone(),
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    epoch: 1,
                    scope: crate::domain::MetadataInventoryScope::Root,
                    started_unix_ms: base_unix_ms,
                })
                .expect("begin recovery inventory");
            let root_identity = crate::adapters::FileDiscovery::new(&root_path)
                .expect("pin candidate round-robin root")
                .metadata_inventory_root_identity()
                .expect("read candidate round-robin root identity")
                .expect("stable candidate round-robin root identity");
            catalog
                .initialize_metadata_inventory_spool(
                    &run,
                    &authority,
                    &root_identity,
                    None,
                    None,
                    base_unix_ms + 1,
                )
                .expect("persist candidate round-robin root proof");
            let paths = (0..candidate_count)
                .map(|index| format!("missing-{root_index}-{index}.jpg"))
                .collect::<Vec<_>>();
            catalog
                .stage_metadata_inventory_page(
                    &run_id,
                    &crate::domain::MetadataInventoryPage {
                        page_index: 1,
                        entries: paths
                            .iter()
                            .map(|relative_path| crate::domain::MetadataInventoryEntry {
                                relative_path: relative_path.clone(),
                                kind: crate::domain::MetadataInventoryEntryKind::File,
                                file_size: Some(1),
                                modified_unix_ms: base_unix_ms,
                                file_identity: None,
                                source_revision: None,
                                placeholder_state:
                                    crate::domain::MetadataInventoryPlaceholderState::Available,
                                is_reparse_point: false,
                            })
                            .collect(),
                        cursor: paths.last().cloned(),
                        is_complete: true,
                        frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                            "", None,
                        )],
                    },
                    base_unix_ms + 1,
                )
                .expect("stage recovery candidates");
            let intents = paths
                .iter()
                .enumerate()
                .map(
                    |(index, relative_path)| crate::domain::LibraryChangeIntent {
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                        scope: crate::domain::LibraryChangeScope::Path,
                        relative_path: relative_path.clone(),
                        previous_relative_path: None,
                        origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                        first_observed_unix_ms: base_unix_ms,
                        most_recent_observed_unix_ms: base_unix_ms,
                        first_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                        most_recent_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                        coalesced_observation_count: 1,
                    },
                )
                .collect::<Vec<_>>();
            let updates = paths
                .iter()
                .map(
                    |relative_path| crate::domain::MetadataInventoryComparisonUpdate {
                        relative_path: relative_path.clone(),
                        status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                        candidate_previous_relative_path: None,
                    },
                )
                .collect::<Vec<_>>();
            catalog
                .publish_metadata_inventory_comparison_candidates(
                    &authority,
                    &run_id,
                    &intents,
                    &updates,
                    base_unix_ms + 2,
                    policy,
                )
                .expect("publish owned recovery candidates")
                .expect("authority remains current");
        }
        drop(catalog);

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage)
            .expect("schedule first candidate root");
        assert_eq!(
            production
                .recovery
                .as_ref()
                .map(|task| task.root_id.as_str()),
            Some("root-a")
        );
        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::CandidateDrain { .. })
        ));

        let mut rotated = false;
        for _ in 0..200 {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("rotate production candidate root");
            if production
                .recovery
                .as_ref()
                .is_some_and(|task| task.root_id == "root-b")
            {
                rotated = true;
                break;
            }
        }
        assert!(
            rotated,
            "root-b must run before root-a's second owned candidate"
        );
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open round-robin evidence catalog");
        let root_a_evidence: (i64, i64) = connection
            .query_row(
                "SELECT
                   SUM(queue.status = 'completed'),
                   SUM(queue.status IN ('pending', 'leased', 'retry_wait'))
                 FROM library_metadata_inventory_candidate_owners AS owner
                 JOIN library_change_queue AS queue ON queue.id = owner.change_id
                 WHERE owner.run_id = 'candidate-round-robin-run-root-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load busy root evidence");
        assert_eq!(root_a_evidence, (1, 1));
        production.stop().expect("stop production synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn higher_lane_ready_or_active_blocks_the_next_lower_page_then_p2_resumes() {
        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let root_path = source.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_unresolved_changes: 16,
            max_lease_batch: 1,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let run_id = "lane-boundary-recovery";
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: "lane-boundary-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
            )
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("lane-boundary-scan")
            .expect("prove lane-boundary fixture first-import handoff");
        catalog
            .publish_scan("lane-boundary-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::LiveOnly,
                continuity: PersistentJournalContinuityState::LiveOnly,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("keep the fixture out of baseline acquisition");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms,
                policy,
            )
            .expect("enqueue recovery control");
        let authority = catalog
            .lease_authoritative_library_change(&root_id, generation, base_unix_ms, policy)
            .expect("lease recovery authority")
            .expect("recovery control");
        catalog
            .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
                change_id: authority.change.id,
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
                opening_boundary: None,
                authorized_unix_ms: base_unix_ms,
                retired_unix_ms: None,
            })
            .expect("persist recovery authority");
        catalog
            .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                epoch: 1,
                scope: crate::domain::MetadataInventoryScope::Root,
                started_unix_ms: base_unix_ms,
            })
            .expect("begin recovery inventory");
        catalog
            .stage_metadata_inventory_page(
                run_id,
                &crate::domain::MetadataInventoryPage {
                    page_index: 1,
                    entries: vec![crate::domain::MetadataInventoryEntry {
                        relative_path: "p2-missing.jpg".to_owned(),
                        kind: crate::domain::MetadataInventoryEntryKind::File,
                        file_size: Some(1),
                        modified_unix_ms: base_unix_ms,
                        file_identity: None,
                        source_revision: None,
                        placeholder_state:
                            crate::domain::MetadataInventoryPlaceholderState::Available,
                        is_reparse_point: false,
                    }],
                    cursor: Some("p2-missing.jpg".to_owned()),
                    is_complete: true,
                    frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                        "", None,
                    )],
                },
                base_unix_ms + 1,
            )
            .expect("stage recovery candidate");
        let p2_intent = crate::domain::LibraryChangeIntent {
            root_id: root_id.clone(),
            root_generation: generation,
            kind: crate::domain::LibraryChangeIntentKind::Reconcile,
            scope: crate::domain::LibraryChangeScope::Path,
            relative_path: "p2-missing.jpg".to_owned(),
            previous_relative_path: None,
            origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
            first_observed_unix_ms: base_unix_ms,
            most_recent_observed_unix_ms: base_unix_ms,
            first_sequence: 2,
            most_recent_sequence: 2,
            coalesced_observation_count: 1,
        };
        catalog
            .publish_metadata_inventory_comparison_candidates(
                &authority,
                run_id,
                std::slice::from_ref(&p2_intent),
                &[crate::domain::MetadataInventoryComparisonUpdate {
                    relative_path: p2_intent.relative_path.clone(),
                    status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                    candidate_previous_relative_path: None,
                }],
                base_unix_ms + 2,
                policy,
            )
            .expect("publish recovery candidate")
            .expect("authority remains current");
        let mut p1_intent = p2_intent.clone();
        p1_intent.relative_path = "p1-missing.jpg".to_owned();
        p1_intent.origin = crate::domain::LibraryChangeOrigin::StartupCatchUp;
        p1_intent.first_sequence = 3;
        p1_intent.most_recent_sequence = 3;
        let mut p0_intent = p1_intent.clone();
        p0_intent.relative_path = "p0-missing.jpg".to_owned();
        p0_intent.origin = crate::domain::LibraryChangeOrigin::LiveNotification;
        p0_intent.first_sequence = 4;
        p0_intent.most_recent_sequence = 4;
        catalog
            .enqueue_library_change_intents(&[p1_intent, p0_intent], base_unix_ms + 3, policy)
            .expect("enqueue higher lanes");
        let snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 0,
            applied_mutation_count: 0,
            roots: vec![crate::domain::LibraryRootSynchronizationStatus {
                root_id: root_id.clone(),
                root_generation: generation.value(),
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::Updating,
                freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
                continuity: PersistentJournalContinuityState::Current,
                phase: LibrarySynchronizationPhase::Reconciliation,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                pending_change_count: 3,
                retry_wait_count: 0,
                freshness_unknown_count: 0,
                recovery_blocked: false,
                last_issue_code: None,
            }],
        };
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;
        production.journal_next_action_is_read = false;
        assert!(
            ready_recovery_work(&mut production, &catalog, &snapshot, base_unix_ms + 3)
                .expect("inspect P2 readiness")
                .is_none(),
            "ready P0/P1 work must block the next P2 page"
        );
        drop(catalog);

        poll_runtime_with_storage(&mut production, &storage).expect("start P0 work");
        assert!(production.live.is_some(), "P0 must start first");
        assert!(
            production.journal.is_none(),
            "P1 cannot start beside ready P0"
        );
        assert!(
            production.recovery.is_none(),
            "P2 cannot start beside ready P0"
        );

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut saw_p1 = false;
        let mut saw_p2_after_p1 = false;
        while std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage).expect("drive lane ordering");
            if production.journal.is_some() {
                saw_p1 = true;
                assert!(
                    production.recovery.is_none(),
                    "P1 active work must block the next P2 page"
                );
            }
            if saw_p1
                && matches!(
                    production.recovery.as_ref().map(|task| &task.kind),
                    Some(RecoveryTaskKind::CandidateDrain { .. })
                )
            {
                saw_p2_after_p1 = true;
                break;
            }
        }
        assert!(saw_p1, "P1 must start after P0 completes");
        assert!(
            saw_p2_after_p1,
            "P2 must resume without starvation after higher lanes finish"
        );
        let owner_count: i64 = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open owner evidence")
            .query_row(
                "SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                 WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .expect("count retained owner");
        assert_eq!(
            owner_count, 1,
            "priority yielding must not lose P2 ownership"
        );
        production.stop().expect("stop production synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn legacy_p2_capacity_debt_allows_only_one_bounded_drain_before_p1_admission() {
        const OWNED_CANDIDATES: usize = 64;
        const LEGACY_UNOWNED_P2: usize = 3_520;

        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let root_path = source.to_string_lossy().into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_lease_batch: u32::try_from(OWNED_CANDIDATES).expect("lease batch"),
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        assert_eq!(policy.lane_capacity(LibraryChangeLane::Journal), 3_584);
        assert_eq!(policy.lane_capacity(LibraryChangeLane::Recovery), 3_072);
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let run_id = "legacy-capacity-debt";
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: "legacy-capacity-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
            )
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("legacy-capacity-scan")
            .expect("prove legacy-capacity fixture first-import handoff");
        catalog
            .publish_scan("legacy-capacity-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        let registration =
            describe_production_persistent_journal_root(&root_id, generation.value(), &source)
                .expect("describe journal root");
        let root_reference =
            JournalFileReference::from_bytes(&registration.authorization.root_identity)
                .expect("root reference");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed current capability");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load root")
            .expect("published root");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.clone(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: registration.authorization.volume_id.clone(),
                    volume_serial: registration.volume_serial,
                },
                root_file_reference: root_reference,
                journal_id: JournalIdentifier::new(44).expect("journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("next USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
                covered_catalog_revision: current_root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed current checkpoint");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                    scope: crate::domain::LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms,
                policy,
            )
            .expect("enqueue recovery control");
        let authority = catalog
            .lease_authoritative_library_change(&root_id, generation, base_unix_ms, policy)
            .expect("lease recovery authority")
            .expect("recovery control");
        catalog
            .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
                change_id: authority.change.id,
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
                opening_boundary: None,
                authorized_unix_ms: base_unix_ms,
                retired_unix_ms: None,
            })
            .expect("persist recovery authority");
        let run = catalog
            .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
                run_id: run_id.to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                epoch: 1,
                scope: crate::domain::MetadataInventoryScope::Root,
                started_unix_ms: base_unix_ms,
            })
            .expect("begin recovery inventory");
        let root_identity = crate::adapters::FileDiscovery::new(&root_path)
            .expect("pin recovery root")
            .metadata_inventory_root_identity()
            .expect("read recovery root identity")
            .expect("stable recovery root identity");
        catalog
            .initialize_metadata_inventory_spool(
                &run,
                &authority,
                &root_identity,
                None,
                None,
                base_unix_ms + 1,
            )
            .expect("persist v28 candidate root proof");
        let paths = (0..OWNED_CANDIDATES)
            .map(|index| format!("owned-{index:04}.jpg"))
            .collect::<Vec<_>>();
        catalog
            .stage_metadata_inventory_page(
                run_id,
                &crate::domain::MetadataInventoryPage {
                    page_index: 1,
                    entries: paths
                        .iter()
                        .map(|relative_path| crate::domain::MetadataInventoryEntry {
                            relative_path: relative_path.clone(),
                            kind: crate::domain::MetadataInventoryEntryKind::File,
                            file_size: Some(1),
                            modified_unix_ms: base_unix_ms,
                            file_identity: None,
                            source_revision: None,
                            placeholder_state:
                                crate::domain::MetadataInventoryPlaceholderState::Available,
                            is_reparse_point: false,
                        })
                        .collect(),
                    cursor: paths.last().cloned(),
                    is_complete: true,
                    frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                        "", None,
                    )],
                },
                base_unix_ms + 1,
            )
            .expect("stage owned candidates");
        let intents = paths
            .iter()
            .enumerate()
            .map(
                |(index, relative_path)| crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: relative_path.clone(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: u64::try_from(index + 2).expect("candidate sequence"),
                    most_recent_sequence: u64::try_from(index + 2).expect("candidate sequence"),
                    coalesced_observation_count: 1,
                },
            )
            .collect::<Vec<_>>();
        let updates = paths
            .iter()
            .map(
                |relative_path| crate::domain::MetadataInventoryComparisonUpdate {
                    relative_path: relative_path.clone(),
                    status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                    candidate_previous_relative_path: None,
                },
            )
            .collect::<Vec<_>>();
        catalog
            .publish_metadata_inventory_comparison_candidates(
                &authority,
                run_id,
                &intents,
                &updates,
                base_unix_ms + 2,
                policy,
            )
            .expect("publish owned candidates")
            .expect("authority remains current");
        drop(catalog);
        let legacy_connection =
            rusqlite::Connection::open(&storage.catalog_path).expect("open migrated debt fixture");
        legacy_connection
            .execute(
                "WITH RECURSIVE sequence(value) AS (
                   SELECT 0
                   UNION ALL
                   SELECT value + 1 FROM sequence WHERE value + 1 < ?1
                 )
                 INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 )
                 SELECT ?2, 1, 'reconcile', 'path', printf('legacy-%04d.jpg', value),
                        NULL, 'metadata_inventory', ?3, ?3,
                        printf('%d', value + 10000), printf('%d', value + 10000),
                        1, 'pending', ?3, 0, ?3, ?3
                 FROM sequence",
                rusqlite::params![
                    i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
                    root_id,
                    base_unix_ms,
                ],
            )
            .expect("seed migrated v26 P2 capacity debt");
        let (raw_lower_count, quota_lower_count): (i64, i64) = legacy_connection
            .query_row(
                "SELECT COUNT(*),
                        SUM(CASE WHEN queue.id <> ?1 THEN 1 ELSE 0 END)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.status IN ('pending', 'leased', 'retry_wait')
                   AND lane.lane <> 'p0_live'",
                [i64::try_from(authority.change.id.value()).expect("authority ID")],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("count legacy debt");
        assert_eq!((raw_lower_count, quota_lower_count), (3_585, 3_584));
        drop(legacy_connection);

        let read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(PriorityJournalSession {
            p1_root_id: root_id.clone(),
            first_usn: 20,
            published_next_usn: Arc::new(std::sync::atomic::AtomicI64::new(21)),
            candidate_count: 1,
            shared_read_count: Arc::clone(&read_count),
            close_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [5; 16],
        });
        production.runtime.queue_policy = policy;

        poll_runtime_with_storage(&mut production, &storage)
            .expect("schedule debt-repayment candidate drain");
        assert_eq!(read_count.load(Ordering::Acquire), 0);
        assert!(
            production.journal.is_none(),
            "P1 page must wait for headroom"
        );
        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::CandidateDrain { .. })
        ));

        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut p1_started = false;
        while std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage).expect("drain debt then admit P1");
            if production.journal.is_some() {
                p1_started = true;
                break;
            }
        }
        assert!(p1_started, "one bounded P2 drain must release P1 admission");
        assert!(
            production.recovery.as_ref().is_none_or(|task| {
                matches!(task.kind, RecoveryTaskKind::CandidateDrain { .. })
            }),
            "P1 may overlap only the already-started bounded debt drain"
        );
        let read_deadline = std::time::Instant::now() + Duration::from_secs(5);
        while read_count.load(Ordering::Acquire) == 0 && std::time::Instant::now() < read_deadline {
            std::thread::yield_now();
        }
        let evidence: (i64, i64, i64, i64) =
            rusqlite::Connection::open(&storage.catalog_path)
                .expect("open debt evidence")
                .query_row(
                    "SELECT
                       SUM(CASE WHEN owner.run_id = ?1 AND queue.status = 'completed' THEN 1 ELSE 0 END),
                       SUM(CASE WHEN owner.run_id = ?1 AND queue.status <> 'completed' THEN 1 ELSE 0 END),
                       (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                        WHERE run_id = ?1),
                       (SELECT COUNT(*) FROM library_change_queue
                        WHERE relative_path LIKE 'legacy-%' AND status IN ('pending', 'leased', 'retry_wait'))
                     FROM library_metadata_inventory_candidate_owners AS owner
                     JOIN library_change_queue AS queue ON queue.id = owner.change_id",
                    [run_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load debt repayment evidence");
        assert_eq!(evidence, (64, 0, 64, 3_520));
        assert_eq!(read_count.load(Ordering::Acquire), 1);
        production.stop().expect("stop production synchronization");
    }

    #[cfg(windows)]
    #[test]
    fn exactly_3584_legacy_unowned_p2_rows_release_p1_then_continue_without_loss() {
        const LEGACY_UNOWNED_P2: usize = 3_584;
        const LEASE_BATCH: usize = 64;

        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("capacity root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical capacity root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("capacity root identity query")
            .expect("capacity root stable identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_lease_batch: u32::try_from(LEASE_BATCH).expect("lease batch"),
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "zero-owner-capacity-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("zero-owner-capacity-scan")
            .expect("prove zero-owner fixture first-import handoff");
        catalog
            .publish_scan("zero-owner-capacity-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        let registration =
            describe_production_persistent_journal_root(&root_id, generation.value(), &source)
                .expect("describe journal root");
        let root_reference =
            JournalFileReference::from_bytes(&registration.authorization.root_identity)
                .expect("root reference");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed current capability");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load root")
            .expect("published root");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.clone(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: registration.authorization.volume_id,
                    volume_serial: registration.volume_serial,
                },
                root_file_reference: root_reference,
                journal_id: JournalIdentifier::new(44).expect("journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("next USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
                covered_catalog_revision: current_root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed current checkpoint");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: "p0-preempts-debt.jpg".to_owned(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: base_unix_ms,
                    most_recent_observed_unix_ms: base_unix_ms,
                    first_sequence: 50_000,
                    most_recent_sequence: 50_000,
                    coalesced_observation_count: 1,
                }],
                base_unix_ms,
                policy,
            )
            .expect("P0 reserve remains available");
        drop(catalog);
        let legacy_connection =
            rusqlite::Connection::open(&storage.catalog_path).expect("open capacity debt fixture");
        legacy_connection
            .execute(
                "WITH RECURSIVE sequence(value) AS (
                   SELECT 0
                   UNION ALL
                   SELECT value + 1 FROM sequence WHERE value + 1 < ?1
                 )
                 INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 )
                 SELECT ?2, 1, 'reconcile', 'path', printf('legacy-%04d.jpg', value),
                        NULL, 'metadata_inventory', ?3, ?3,
                        printf('%d', value + 10000), printf('%d', value + 10000),
                        1, 'pending', ?3, 0, ?3, ?3
                 FROM sequence",
                rusqlite::params![
                    i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
                    root_id,
                    base_unix_ms,
                ],
            )
            .expect("seed exact zero-owner capacity debt");
        let debt_shape: (i64, i64, i64) = legacy_connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_change_queue
                    WHERE relative_path LIKE 'legacy-%'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
                   (SELECT COUNT(*) FROM library_recovery_authorities)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("zero-owner debt shape");
        assert_eq!(
            debt_shape,
            (
                i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
                0,
                0
            )
        );
        drop(legacy_connection);

        let read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(PriorityJournalSession {
            p1_root_id: root_id.clone(),
            first_usn: 20,
            published_next_usn: Arc::new(std::sync::atomic::AtomicI64::new(21)),
            candidate_count: 1,
            shared_read_count: Arc::clone(&read_count),
            close_count: Arc::clone(&close_count),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(session),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [6; 16],
        });
        production.runtime.queue_policy = policy;

        poll_runtime_with_storage(&mut production, &storage).expect("P0 preempts capacity debt");
        assert!(
            production.live.is_some(),
            "P0 must use its reserved admission"
        );
        assert!(production.journal.is_none());
        assert!(production.recovery.is_none());

        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut saw_legacy_drain = false;
        let mut saw_p1 = false;
        while std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("drain debt and admit journal work");
            if matches!(
                production.recovery.as_ref().map(|task| &task.kind),
                Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
            ) && !saw_legacy_drain
            {
                saw_legacy_drain = true;
                assert_eq!(
                    read_count.load(Ordering::Acquire),
                    0,
                    "P1 cannot read until one bounded P2 page releases headroom"
                );
            }
            if read_count.load(Ordering::Acquire) > 0 {
                saw_p1 = true;
                break;
            }
        }
        assert!(
            saw_legacy_drain,
            "legacy debt must use the bounded escape path"
        );
        assert!(saw_p1, "one bounded legacy page must release P1 admission");
        assert_eq!(read_count.load(Ordering::Acquire), 1);

        let continue_deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut completed_legacy = 0_i64;
        while std::time::Instant::now() < continue_deadline {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("resume legacy P2 after journal work");
            completed_legacy = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open continuation evidence")
                .query_row(
                    "SELECT COUNT(*) FROM library_change_queue
                     WHERE relative_path LIKE 'legacy-%' AND status = 'completed'",
                    [],
                    |row| row.get(0),
                )
                .expect("count completed legacy debt");
            if completed_legacy >= i64::try_from(LEASE_BATCH * 2).expect("continued batch count") {
                break;
            }
        }
        assert!(
            completed_legacy >= i64::try_from(LEASE_BATCH * 2).expect("continued batch count"),
            "P2 must continue after the admitted P1 page"
        );
        let evidence: (i64, i64, i64, i64, i64) = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open final debt evidence")
            .query_row(
                "SELECT
                       SUM(status = 'completed'),
                       SUM(status IN ('pending', 'leased', 'retry_wait')),
                       COUNT(*),
                       (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
                       (SELECT COUNT(*) FROM library_recovery_authorities)
                     FROM library_change_queue
                     WHERE relative_path LIKE 'legacy-%'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("final debt evidence");
        assert_eq!(evidence.0 + evidence.1, evidence.2);
        assert_eq!(
            evidence.2,
            i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count")
        );
        assert_eq!((evidence.3, evidence.4), (0, 0));
        production.stop().expect("stop production synchronization");
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[cfg(windows)]
    #[test]
    fn exactly_one_legacy_unowned_nonpath_row_runs_once_blocks_durably_and_refresh_supersedes() {
        use crate::ports::PersistentJournalRepository;

        let directory = tempfile::tempdir().expect("test directory");
        let source = directory.path().join("source");
        std::fs::create_dir_all(&source).expect("source root");
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("legacy root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical legacy root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("legacy root identity query")
            .expect("legacy root stable identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let base_unix_ms = now_unix_ms().expect("fixture clock");
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "single-legacy-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin proven initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test("single-legacy-scan")
            .expect("prove single-legacy fixture first-import handoff");
        catalog
            .publish_scan("single-legacy-scan", &root_id, 0, 0)
            .expect("publish initial scan");
        let registration =
            describe_production_persistent_journal_root(&root_id, generation.value(), &source)
                .expect("describe single legacy journal root");
        let checkpoint_volume = PersistentJournalVolumeIdentity {
            volume_guid: registration.authorization.volume_id.clone(),
            volume_serial: registration.volume_serial,
        };
        let checkpoint_reference =
            JournalFileReference::from_bytes(&registration.authorization.root_identity)
                .expect("single legacy root reference");
        let checkpoint_journal_id = JournalIdentifier::new(44).expect("checkpoint journal ID");
        let checkpoint_next_usn = JournalUsn::new(20).expect("checkpoint next USN");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.clone(),
                root_generation: generation,
                protocol_version: 5,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                failure: None,
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed recovery-required journal capability");
        let current_root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("load single legacy root")
            .expect("published single legacy root");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.clone(),
                root_generation: generation,
                volume: checkpoint_volume.clone(),
                root_file_reference: checkpoint_reference.clone(),
                journal_id: checkpoint_journal_id,
                next_unread_usn: checkpoint_next_usn,
                captured_exclusive_end: checkpoint_next_usn,
                covered_catalog_revision: current_root.catalog_revision,
                protocol_version: 5,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                failure: Some(PersistentJournalFailure {
                    code: "legacy-recovery-required".to_owned(),
                    message: "Legacy recovery fixture requires an explicit refresh".to_owned(),
                }),
                updated_unix_ms: base_unix_ms,
            })
            .expect("seed recovery-required checkpoint");
        drop(registration);
        drop(catalog);
        let legacy_connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open single legacy seed connection");
        legacy_connection
            .execute(
                "INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (?1, 1, 'reconcile', 'subtree', 'legacy-subtree',
                           'metadata_inventory', ?2, ?2, '1', '1', 1,
                           'pending', ?2, 0, ?2, ?2)",
                rusqlite::params![root_id, base_unix_ms],
            )
            .expect("seed exactly one legacy nonpath row");
        let legacy_change_id = legacy_connection.last_insert_rowid();
        drop(legacy_connection);
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("reopen seeded catalog");
        assert!(
            catalog
                .has_ready_legacy_unowned_recovery_debt(&root_id, generation, base_unix_ms, policy,)
                .expect("single legacy debt readiness")
        );
        drop(catalog);

        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&close_count),
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })),
        );
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [10; 16],
        });
        production.runtime.queue_policy = policy;
        poll_runtime_with_storage(&mut production, &storage)
            .expect("production schedules the single legacy survivor");
        assert!(matches!(
            production.recovery.as_ref().map(|task| &task.kind),
            Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
        ));

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let blocked_snapshot = loop {
            std::thread::sleep(Duration::from_millis(5));
            let snapshot = poll_runtime_with_storage(&mut production, &storage)
                .expect("finish the single legacy worker");
            let evidence: (String, i64, Option<i64>, Option<String>, i64) =
                rusqlite::Connection::open(&storage.catalog_path)
                    .expect("open single legacy evidence")
                    .query_row(
                        "SELECT status, attempt_count, next_retry_unix_ms,
                                last_failure_code, updated_unix_ms
                         FROM library_change_queue WHERE id = ?1",
                        [legacy_change_id],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        },
                    )
                    .expect("load single legacy evidence");
            if evidence.0 == "retry_wait"
                && evidence.3.as_deref() == Some("legacy_recovery_authority_missing")
            {
                break (snapshot, evidence);
            }
            assert!(
                std::time::Instant::now() < deadline,
                "single legacy production worker did not block durably"
            );
        };
        assert_eq!(blocked_snapshot.1.1, i64::from(policy.max_attempts));
        assert_eq!(blocked_snapshot.1.2, None);
        let blocked_updated_unix_ms = blocked_snapshot.1.4;
        let stable_snapshot = poll_runtime_with_storage(&mut production, &storage)
            .expect("project blocked single legacy survivor");
        let root_status = stable_snapshot
            .roots
            .iter()
            .find(|root| root.root_id == root_id)
            .expect("single legacy root status");
        assert_eq!(
            root_status.continuity,
            PersistentJournalContinuityState::RecoveryRequired
        );
        assert!(root_status.recovery_blocked);
        assert_eq!(
            root_status.last_issue_code.as_deref(),
            Some("legacy_recovery_authority_missing")
        );
        assert!(production.recovery.is_none());
        production.stop().expect("stop first production epoch");

        let mut reopened = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
                close_count: Arc::clone(&close_count),
                close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })),
        );
        reopened.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [11; 16],
        });
        reopened.runtime.queue_policy = policy;
        for _ in 0..3 {
            let snapshot = poll_runtime_with_storage(&mut reopened, &storage)
                .expect("reopen keeps the legacy survivor blocked");
            assert!(reopened.recovery.is_none());
            assert_eq!(
                snapshot
                    .roots
                    .iter()
                    .find(|root| root.root_id == root_id)
                    .expect("reopened root status")
                    .continuity,
                PersistentJournalContinuityState::RecoveryRequired
            );
        }
        let reopened_updated_unix_ms: i64 = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open reopened legacy evidence")
            .query_row(
                "SELECT updated_unix_ms FROM library_change_queue WHERE id = ?1",
                [legacy_change_id],
                |row| row.get(0),
            )
            .expect("load reopened legacy evidence");
        assert_eq!(reopened_updated_unix_ms, blocked_updated_unix_ms);
        reopened.stop().expect("stop reopened production epoch");

        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())
            .expect("open explicit refresh catalog");
        let refresh = catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: root_id.clone(),
                    root_generation: generation,
                    kind: PersistentJournalRootFailureKind::ContainmentFailure,
                    failure: PersistentJournalFailure {
                        code: "explicit-refresh".to_owned(),
                        message: "Explicit refresh replaces blocked legacy recovery".to_owned(),
                    },
                    opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                        volume: checkpoint_volume,
                        root_file_reference: checkpoint_reference,
                        journal_id: checkpoint_journal_id,
                        next_usn: checkpoint_next_usn,
                        protocol_version: 5,
                        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    }),
                },
                base_unix_ms.saturating_add(100),
                policy,
            )
            .expect("persist explicit refresh")
            .expect("explicit refresh supersedes blocked legacy survivor");
        drop(catalog);
        let replacement: (String, Option<i64>, String) =
            rusqlite::Connection::open(&storage.catalog_path)
                .expect("open explicit refresh evidence")
                .query_row(
                    "SELECT legacy.status, legacy.superseded_by_change_id, replacement.status
                 FROM library_change_queue AS legacy
                 JOIN library_change_queue AS replacement ON replacement.id = ?2
                 WHERE legacy.id = ?1",
                    rusqlite::params![
                        legacy_change_id,
                        i64::try_from(refresh.value()).expect("refresh change ID"),
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("load explicit refresh replacement");
        assert_eq!(
            replacement,
            (
                "superseded".to_owned(),
                Some(i64::try_from(refresh.value()).expect("refresh change ID")),
                "pending".to_owned(),
            )
        );
    }

    #[cfg(windows)]
    #[test]
    fn legacy_unowned_drain_pages_rotate_fairly_across_roots() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_lease_batch: 1,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let mut roots = Vec::new();
        for (root_index, root_id) in ["legacy-root-a", "legacy-root-b"].into_iter().enumerate() {
            let root_path = directory.path().join(root_id);
            std::fs::create_dir_all(&root_path).expect("source root");
            let root_path = root_path.to_string_lossy().into_owned();
            let scan_id = format!("scan-{root_id}");
            catalog
                .begin_scan(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
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
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove legacy-drain fixture first-import handoff");
            catalog
                .publish_scan(&scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .enqueue_library_change_intents(
                    &[crate::domain::LibraryChangeIntent {
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                        scope: crate::domain::LibraryChangeScope::Path,
                        relative_path: format!("legacy-{root_index}.jpg"),
                        previous_relative_path: None,
                        origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                        first_observed_unix_ms: 100,
                        most_recent_observed_unix_ms: 100,
                        first_sequence: u64::try_from(root_index + 1).expect("sequence"),
                        most_recent_sequence: u64::try_from(root_index + 1).expect("sequence"),
                        coalesced_observation_count: 1,
                    }],
                    100,
                    policy,
                )
                .expect("seed zero-owner recovery debt");
            roots.push(crate::domain::LibraryRootSynchronizationStatus {
                root_id: root_id.to_owned(),
                root_generation: generation.value(),
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::Updating,
                freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                phase: LibrarySynchronizationPhase::Reconciliation,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                pending_change_count: 1,
                retry_wait_count: 0,
                freshness_unknown_count: 0,
                recovery_blocked: false,
                last_issue_code: None,
            });
        }
        let snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 0,
            applied_mutation_count: 0,
            roots,
        };
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;

        let first = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
            .expect("select first legacy page")
            .expect("first root has legacy debt");
        let second = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
            .expect("select second legacy page")
            .expect("second root has legacy debt");

        assert!(matches!(
            first,
            ReadyRecoveryWork::LegacyUnownedDrain { .. }
        ));
        assert!(matches!(
            second,
            ReadyRecoveryWork::LegacyUnownedDrain { .. }
        ));
        assert_ne!(first.root_id(), second.root_id());
    }

    #[cfg(windows)]
    #[test]
    fn recovery_blocked_root_is_excluded_from_p2_while_other_roots_keep_rotating() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
        let generation = LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_lease_batch: 1,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let mut roots = Vec::new();
        for (index, (root_id, recovery_blocked)) in [("blocked-root", true), ("ready-root", false)]
            .into_iter()
            .enumerate()
        {
            let root_path = directory.path().join(root_id);
            std::fs::create_dir_all(&root_path).expect("source root");
            let root_path = root_path.to_string_lossy().into_owned();
            let scan_id = format!("scan-{root_id}");
            catalog
                .begin_scan(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
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
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove recovery-blocked fixture first-import handoff");
            catalog
                .publish_scan(&scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .enqueue_library_change_intents(
                    &[crate::domain::LibraryChangeIntent {
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                        scope: crate::domain::LibraryChangeScope::Path,
                        relative_path: format!("legacy-{index}.jpg"),
                        previous_relative_path: None,
                        origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                        first_observed_unix_ms: 100,
                        most_recent_observed_unix_ms: 100,
                        first_sequence: u64::try_from(index + 1).expect("sequence"),
                        most_recent_sequence: u64::try_from(index + 1).expect("sequence"),
                        coalesced_observation_count: 1,
                    }],
                    100,
                    policy,
                )
                .expect("seed legacy unowned P2 debt");
            assert!(
                catalog
                    .has_ready_legacy_unowned_recovery_debt(root_id, generation, 100, policy,)
                    .expect("legacy debt readiness")
            );
            roots.push(crate::domain::LibraryRootSynchronizationStatus {
                root_id: root_id.to_owned(),
                root_generation: generation.value(),
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: if recovery_blocked {
                    crate::domain::CatalogFreshnessState::NeedsReconciliation
                } else {
                    crate::domain::CatalogFreshnessState::Updating
                },
                freshness_cause: if recovery_blocked {
                    crate::domain::CatalogFreshnessCause::EvidenceGap
                } else {
                    crate::domain::CatalogFreshnessCause::PendingChanges
                },
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                phase: if recovery_blocked {
                    LibrarySynchronizationPhase::Blocked
                } else {
                    LibrarySynchronizationPhase::Reconciliation
                },
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                pending_change_count: 1,
                retry_wait_count: 0,
                freshness_unknown_count: if recovery_blocked { 1 } else { 0 },
                recovery_blocked,
                last_issue_code: recovery_blocked
                    .then(|| "live_gap_v30_explicit_recovery_required".to_owned()),
            });
        }
        let snapshot = LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 0,
            applied_mutation_count: 0,
            roots,
        };
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.runtime.queue_policy = policy;

        let blocked_only = LibrarySynchronizationSnapshot {
            roots: vec![snapshot.roots[0].clone()],
            ..snapshot.clone()
        };
        assert!(
            ready_recovery_work(&mut production, &catalog, &blocked_only, 100)
                .expect("inspect blocked root")
                .is_none(),
            "a recovery-blocked root must not receive candidate, legacy, or control P2 work"
        );

        let first = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
            .expect("rotate past blocked root")
            .expect("ready root remains eligible");
        assert!(matches!(
            &first,
            ReadyRecoveryWork::LegacyUnownedDrain { .. }
        ));
        assert_eq!(first.root_id(), "ready-root");
        let second = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
            .expect("keep rotating past blocked root")
            .expect("ready root remains eligible on the next rotation");
        assert_eq!(second.root_id(), "ready-root");

        production
            .start_legacy_unowned_recovery_drain(
                first.root_id().to_owned(),
                generation,
                0,
                100,
                catalog.catalog_path().to_path_buf(),
            )
            .expect("start selected ready-root P2 work");
        let deadline = Instant::now() + Duration::from_secs(5);
        while production.recovery.is_some() {
            std::thread::sleep(Duration::from_millis(5));
            production
                .poll_recovery(100)
                .expect("finish selected ready-root P2 work");
            assert!(
                Instant::now() < deadline,
                "selected ready-root P2 work did not finish"
            );
        }

        let attempt_counts: (i64, i64) = rusqlite::Connection::open(catalog.catalog_path())
            .expect("open blocked attempt evidence")
            .query_row(
                "SELECT
                   MAX(CASE WHEN root_id = 'blocked-root' THEN attempt_count END),
                   MAX(CASE WHEN root_id = 'ready-root' THEN attempt_count END)
                 FROM library_change_queue",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load mixed-root attempt counts");
        assert_eq!(
            attempt_counts,
            (0, 1),
            "automatic P2 dispatch must lease the ready root without retrying blocked legacy debt"
        );

        let mut unblocked_snapshot = snapshot;
        unblocked_snapshot.roots[0].recovery_blocked = false;
        unblocked_snapshot.roots[0].freshness = crate::domain::CatalogFreshnessState::Updating;
        unblocked_snapshot.roots[0].freshness_cause =
            crate::domain::CatalogFreshnessCause::PendingChanges;
        unblocked_snapshot.roots[0].phase = LibrarySynchronizationPhase::Reconciliation;
        unblocked_snapshot.roots[0].freshness_unknown_count = 0;
        unblocked_snapshot.roots[0].last_issue_code = None;
        let released = ready_recovery_work(&mut production, &catalog, &unblocked_snapshot, 100)
            .expect("inspect released root")
            .expect("released root regains P2 eligibility");
        assert!(matches!(
            &released,
            ReadyRecoveryWork::LegacyUnownedDrain { .. }
        ));
        assert_eq!(released.root_id(), "blocked-root");
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_catalog_validation_failure_preserves_retained_change_owner() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        production.catalog_session = Some(Arc::new(
            SqliteCatalogSession::validate(catalog_path.clone())
                .expect("validated catalog session"),
        ));
        let change_id = LibraryChangeId::new(41).expect("fixture change ID");
        production
            .recovery_inventory_sources
            .insert(change_id, retained_source_probe("retained-validation"));

        let error = production
            .start_automatic_recovery(
                "root-a".to_owned(),
                LibraryRootGeneration::initial(),
                1,
                metadata_inventory_lease(change_id),
                1,
                directory.path().join("different-catalog.sqlite3"),
            )
            .expect_err("catalog switch must fail before source handoff");

        assert_eq!(error.code, "library_synchronization_catalog_changed");
        assert_eq!(
            production
                .recovery_inventory_sources
                .get(&change_id)
                .map(RetainedMetadataInventorySource::run_id),
            Some("retained-validation")
        );
        assert!(production.recovery.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_worker_spawn_failure_restores_retained_change_owner() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        let change_id = LibraryChangeId::new(42).expect("fixture change ID");
        production
            .recovery_inventory_sources
            .insert(change_id, retained_source_probe("retained-spawn"));
        fail_next_recovery_worker_spawn();

        let error = production
            .start_automatic_recovery(
                "root-a".to_owned(),
                LibraryRootGeneration::initial(),
                1,
                metadata_inventory_lease(change_id),
                1,
                catalog_path,
            )
            .expect_err("injected spawn failure must restore source ownership");

        assert_eq!(error.code, "authoritative_recovery_worker_start_failed");
        assert_eq!(
            production
                .recovery_inventory_sources
                .get(&change_id)
                .map(RetainedMetadataInventorySource::run_id),
            Some("retained-spawn")
        );
        assert!(production.recovery.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_new_change_does_not_take_another_retained_owner() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        let retained_change_id = LibraryChangeId::new(43).expect("retained change ID");
        let active_change_id = LibraryChangeId::new(44).expect("active change ID");
        production.recovery_inventory_sources.insert(
            retained_change_id,
            retained_source_probe("retained-other-authority"),
        );

        production
            .start_automatic_recovery(
                "root-a".to_owned(),
                LibraryRootGeneration::initial(),
                1,
                metadata_inventory_lease(active_change_id),
                1,
                catalog_path,
            )
            .expect("start independent recovery owner");
        let deadline = Instant::now() + Duration::from_secs(5);
        while production.recovery.is_some() {
            production
                .poll_recovery(2)
                .expect("collect independent recovery result");
            assert!(
                Instant::now() < deadline,
                "independent recovery did not finish"
            );
            thread::yield_now();
        }

        assert_eq!(
            production
                .recovery_inventory_sources
                .get(&retained_change_id)
                .map(RetainedMetadataInventorySource::run_id),
            Some("retained-other-authority")
        );
        assert!(
            !production
                .recovery_inventory_sources
                .contains_key(&active_change_id)
        );
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_source_pruning_discards_missing_authority() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
        let catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            test_live_only_connection(),
        );
        let change_id = LibraryChangeId::new(45).expect("stale change ID");
        production
            .recovery_inventory_sources
            .insert(change_id, retained_source_probe("stale-authority"));

        production
            .prune_recovery_inventory_sources(&catalog)
            .expect("prune stale source owner");

        assert!(production.recovery_inventory_sources.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn p2_page_completion_rotates_to_another_root_and_retains_each_frontier() {
        let directory = tempfile::tempdir().expect("test directory");
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        let generation = crate::domain::LibraryRootGeneration::initial();
        let policy = crate::domain::LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..crate::domain::LibraryChangeQueuePolicy::default()
        };
        let mut root_paths = BTreeMap::new();
        for root_id in ["root-a", "root-b"] {
            let source = directory.path().join(root_id);
            std::fs::create_dir_all(&source).expect("source root");
            for index in 0..257 {
                std::fs::write(source.join(format!("image-{index}.jpg")), b"metadata")
                    .expect("metadata fixture");
            }
            let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
                .expect("controlled root discovery");
            let root_path = discovery
                .canonical_root()
                .expect("canonical controlled root")
                .to_string_lossy()
                .into_owned();
            let publication_identity = discovery
                .metadata_inventory_root_identity()
                .expect("controlled root identity query")
                .expect("controlled stable root identity");
            root_paths.insert(root_id, root_path.clone());
            let scan_id = format!("scan-{root_id}");
            catalog
                .begin_scan_with_publication_namespace(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
                        root_path: root_path.clone(),
                        max_items: None,
                        max_entries: None,
                        preview_edge: 512,
                    },
                    root_id,
                    &root_path,
                    &publication_identity,
                )
                .expect("begin root scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove P2 page fixture first-import handoff");
            catalog
                .publish_scan(&scan_id, root_id, 0, 0)
                .expect("publish root scan");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    protocol_version: 1,
                    contract_version: 1,
                    state: PersistentJournalCapabilityState::Supported,
                    continuity: PersistentJournalContinuityState::Current,
                    failure: None,
                    updated_unix_ms: 1,
                })
                .expect("seed current journal capability");
            let root = catalog
                .load_incremental_catalog_root(root_id)
                .expect("load current root")
                .expect("published root");
            let root_marker = if root_id == "root-a" { 1 } else { 2 };
            let checkpoint = PersistentJournalCheckpoint {
                root_id: root_id.to_owned(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: format!("volume-{root_id}"),
                    volume_serial: root_marker,
                },
                root_file_reference: JournalFileReference::V2([root_marker as u8; 8]),
                journal_id: JournalIdentifier::new(9 + root_marker).expect("journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("opening USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("opening end USN"),
                covered_catalog_revision: root.catalog_revision,
                protocol_version: 1,
                contract_version: 1,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1,
            };
            catalog
                .seed_persistent_journal_checkpoint_for_test(&checkpoint)
                .expect("seed current journal checkpoint");
            catalog
                .persist_persistent_journal_root_failure(
                    &crate::domain::PersistentJournalRootFailure {
                        root_id: root_id.to_owned(),
                        root_generation: generation,
                        kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                        failure: PersistentJournalFailure {
                            code: format!("fixture-containment-{root_id}"),
                            message: format!("Fixture containment failure for {root_id}"),
                        },
                        opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                            volume: checkpoint.volume.clone(),
                            root_file_reference: checkpoint.root_file_reference.clone(),
                            journal_id: checkpoint.journal_id,
                            next_usn: checkpoint.next_unread_usn,
                            protocol_version: checkpoint.protocol_version,
                            contract_version: checkpoint.contract_version,
                        }),
                    },
                    2,
                    policy,
                )
                .expect("admit typed containment recovery")
                .expect("allowlisted containment recovery");
        }
        drop(catalog);
        let root_a_path = root_paths.remove("root-a").expect("canonical root-a path");
        let root_b_path = root_paths.remove("root-b").expect("canonical root-b path");
        crate::adapters::reset_source_enumeration_instrumentation(&root_a_path);
        crate::adapters::reset_source_enumeration_instrumentation(&root_b_path);
        let root_a_gate = crate::adapters::gate_source_enumeration(&root_a_path);
        let root_b_gate = crate::adapters::gate_source_enumeration(&root_b_path);
        let mut production = ProductionSynchronization {
            runtime: LibrarySynchronizationRuntime::new_erased(
                crate::ports::erase_library_change_source_factory(HealthyFactory),
            ),
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
            recovery: None,
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
        };

        poll_runtime_with_storage(&mut production, &storage).expect("schedule first P2 page");
        let first_root = production
            .recovery
            .as_ref()
            .expect("first recovery task")
            .root_id
            .clone();
        let first_active_change_id = match &production
            .recovery
            .as_ref()
            .expect("first recovery task")
            .kind
        {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
            _ => panic!("first root must own metadata inventory recovery"),
        };
        let first_gate = if first_root == "root-a" {
            &root_a_gate
        } else {
            &root_b_gate
        };
        let first_path = if first_root == "root-a" {
            &root_a_path
        } else {
            &root_b_path
        };
        assert!(
            first_gate.wait_until_blocked(Duration::from_secs(5)),
            "first root did not block at the real source-entry boundary"
        );
        first_gate.allow_entries(128);
        let mut second_root = None;
        for _ in 0..1_000 {
            poll_runtime_with_storage(&mut production, &storage).expect("rotate P2 page");
            if let Some(task) = production.recovery.as_ref()
                && task.root_id != first_root
            {
                second_root = Some(task.root_id.clone());
                break;
            }
            std::thread::yield_now();
        }
        let second_root = second_root.expect("second root receives the next P2 page");
        assert_ne!(first_root, second_root);
        let second_active_change_id = match &production
            .recovery
            .as_ref()
            .expect("second recovery task")
            .kind
        {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
            _ => panic!("second root must own metadata inventory recovery"),
        };
        assert_eq!(production.recovery_inventory_sources.len(), 1);
        let first_retained_change_id = *production
            .recovery_inventory_sources
            .keys()
            .next()
            .expect("first root retained change owner");
        assert_eq!(first_retained_change_id, first_active_change_id);
        let first_retained_run_id = production
            .recovery_inventory_sources
            .get(&first_retained_change_id)
            .expect("first root retained source")
            .run_id()
            .to_owned();
        let first_retained_open_count = crate::adapters::source_spool_open_count(first_path);
        assert!(
            first_retained_open_count >= 1,
            "the first root must open a source before retaining its frontier"
        );
        let second_gate = if second_root == "root-a" {
            &root_a_gate
        } else {
            &root_b_gate
        };
        let second_path = if second_root == "root-a" {
            &root_a_path
        } else {
            &root_b_path
        };
        assert!(
            second_gate.wait_until_blocked(Duration::from_secs(5)),
            "second root did not block at the real source-entry boundary"
        );
        assert_eq!(crate::adapters::source_entry_read_count(first_path), 128);
        assert_eq!(crate::adapters::source_entry_read_count(second_path), 0);
        second_gate.allow_entries(128);

        let mut third_root = None;
        for _ in 0..1_000 {
            poll_runtime_with_storage(&mut production, &storage).expect("return to first P2 root");
            if let Some(task) = production.recovery.as_ref()
                && task.root_id == first_root
            {
                third_root = Some(task.root_id.clone());
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(third_root.as_deref(), Some(first_root.as_str()));
        let third_active_change_id = match &production
            .recovery
            .as_ref()
            .expect("returned first-root recovery")
            .kind
        {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
            _ => panic!("returned first root must own metadata inventory recovery"),
        };
        assert_eq!(third_active_change_id, first_retained_change_id);
        assert_eq!(production.recovery_inventory_sources.len(), 1);
        let second_retained_change_id = *production
            .recovery_inventory_sources
            .keys()
            .next()
            .expect("second root retained change owner");
        assert_eq!(second_retained_change_id, second_active_change_id);
        let second_retained_open_count = crate::adapters::source_spool_open_count(second_path);
        assert!(
            second_retained_open_count >= 1,
            "the second root must open a source before retaining its frontier"
        );
        let evidence_connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open two-root spool evidence");
        let recovery_identity = |change_id: LibraryChangeId| {
            evidence_connection
                .query_row(
                    "SELECT queue.id, queue.root_id, queue.status, queue.attempt_count,
                            queue.last_failure_code, authority.run_id
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = queue.id
                     WHERE queue.id = ?1 AND lane.lane = 'p2_recovery'
                       AND authority.retired_unix_ms IS NULL
                     LIMIT 1",
                    [i64::try_from(change_id.value()).expect("SQLite change ID")],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    },
                )
                .expect("load retained recovery identity")
        };
        let first_retained_identity = recovery_identity(first_retained_change_id);
        let second_retained_identity = recovery_identity(second_retained_change_id);
        let second_retained_run_id = production
            .recovery_inventory_sources
            .get(&second_retained_change_id)
            .expect("second root retained source")
            .run_id()
            .to_owned();
        assert_eq!(first_retained_identity.1, first_root);
        assert_eq!(first_retained_run_id, first_retained_identity.5);
        assert_eq!(second_retained_identity.1, second_root);
        assert_eq!(second_retained_run_id, second_retained_identity.5);
        assert!(
            first_gate.wait_until_blocked(Duration::from_secs(5)),
            "first root did not retain its live iterator for the third raw batch"
        );
        assert_eq!(crate::adapters::source_entry_read_count(first_path), 128);
        assert_eq!(crate::adapters::source_entry_read_count(second_path), 128);
        first_gate.allow_entries(128);
        let third_deadline = std::time::Instant::now() + Duration::from_secs(5);
        while crate::adapters::source_entry_read_count(first_path) != 256 {
            assert!(
                std::time::Instant::now() < third_deadline,
                "first root did not consume its second bounded raw batch"
            );
            std::thread::yield_now();
        }
        let commit_deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            poll_runtime_with_storage(&mut production, &storage)
                .expect("commit the first root's second raw batch");
            if production
                .recovery
                .as_ref()
                .is_some_and(|task| task.root_id == second_root)
                && second_gate.wait_until_blocked(Duration::from_millis(10))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < commit_deadline,
                "third raw batch did not commit before the next root rotation"
            );
            std::thread::yield_now();
        }

        let active_second_change_id = match &production
            .recovery
            .as_ref()
            .expect("overlapping second-root recovery")
            .kind
        {
            RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
            _ => panic!("second root must own metadata inventory recovery"),
        };
        assert_eq!(active_second_change_id, second_retained_change_id);
        let active_second_identity = recovery_identity(active_second_change_id);
        assert_eq!(active_second_identity.5, second_retained_run_id);
        let second_released_identity = recovery_identity(second_retained_change_id);
        assert_eq!(second_released_identity.0, second_retained_identity.0);
        assert_eq!(second_released_identity.5, second_retained_identity.5);
        assert!(
            production
                .recovery_inventory_sources
                .contains_key(&first_retained_change_id)
        );
        assert!(
            !production
                .recovery_inventory_sources
                .contains_key(&second_retained_change_id)
        );
        let watcher_overlap_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let watcher_authority_count: i64 = evidence_connection
                .query_row(
                    "SELECT COUNT(*) FROM library_recovery_authorities
                     WHERE root_id = ?1 AND root_generation = 1
                       AND reason = 'watcher_uncovered_gap'
                       AND retired_unix_ms IS NULL",
                    [second_root.as_str()],
                    |row| row.get(0),
                )
                .expect("load second-root watcher authority count");
            if watcher_authority_count == 1 {
                break;
            }
            assert!(
                Instant::now() < watcher_overlap_deadline,
                "the second root did not persist its independent watcher-gap authority"
            );
            poll_runtime_with_storage(&mut production, &storage)
                .expect("advance the independent second-root watcher gap");
            assert!(
                production.recovery.as_ref().is_some_and(|task| {
                    task.root_id == second_root
                        && matches!(
                            &task.kind,
                            RecoveryTaskKind::MetadataInventory { change_id, .. }
                                if *change_id == second_retained_change_id
                        )
                }),
                "watcher-gap admission displaced the gated second-root P2 owner"
            );
            std::thread::yield_now();
        }
        let second_recovery_authorities = {
            let mut statement = evidence_connection
                .prepare(
                    "SELECT queue.id, queue.status, queue.attempt_count,
                            queue.last_failure_code, authority.run_id, authority.reason,
                            baseline.phase
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = queue.id
                     LEFT JOIN library_persistent_journal_baselines AS baseline
                       ON baseline.change_id = queue.id
                     WHERE queue.root_id = ?1 AND lane.lane = 'p2_recovery'
                       AND authority.retired_unix_ms IS NULL
                     ORDER BY queue.id",
                )
                .expect("prepare recovery authority evidence");
            statement
                .query_map([second_root.as_str()], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                })
                .expect("query recovery authority evidence")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect recovery authority evidence")
        };
        assert_eq!(
            second_recovery_authorities.len(),
            2,
            "unexpected second-root recovery authorities: {second_recovery_authorities:?}"
        );
        assert!(second_recovery_authorities.iter().any(|authority| {
            authority.0 == i64::try_from(second_retained_change_id.value()).expect("retained ID")
                && authority.4 == second_retained_run_id
        }));
        assert!(
            second_recovery_authorities
                .iter()
                .any(|authority| authority.5 == "watcher_uncovered_gap")
        );
        assert_eq!(
            crate::adapters::source_spool_open_count(first_path),
            first_retained_open_count,
            "rotation reopened the retained first-authority source"
        );
        assert_eq!(
            crate::adapters::source_spool_open_count(second_path),
            second_retained_open_count,
            "rotation reopened the retained second-authority source"
        );
        assert!(crate::adapters::source_peak_staged_window(first_path) <= 128);
        assert!(crate::adapters::source_peak_staged_window(second_path) <= 128);
        for (run_id, expected_count) in [
            (&first_retained_run_id, 256_i64),
            (&second_retained_run_id, 128_i64),
        ] {
            let evidence: (String, String, i64, i64) = evidence_connection
                .query_row(
                    "SELECT spool.state, directory.state, directory.source_entry_count,
                            (SELECT COUNT(*)
                             FROM library_metadata_inventory_spool_entries AS entry
                             WHERE entry.run_id = spool.run_id)
                     FROM library_metadata_inventory_spools AS spool
                     JOIN library_metadata_inventory_spool_directories AS directory
                       ON directory.run_id = spool.run_id
                      WHERE spool.run_id = ?1 AND directory.relative_directory = ''",
                    [run_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load two-root spool frontier");
            assert_eq!(
                evidence,
                (
                    "enumerating".to_owned(),
                    "enumerating".to_owned(),
                    expected_count,
                    expected_count,
                )
            );
        }
        second_gate.release();
        production.stop().expect("stop production synchronization");
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
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
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
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: false,
            stop_requested: Arc::new(AtomicBool::new(false)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
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
            catalog_session: None,
            poll_catalog: PollCatalogOwner::default(),
            persistent_change_journal_factory: None,
            _persistent_change_journal: test_live_only_connection(),
            persistent_change_journal_opened: true,
            persistent_change_journal_caller: None,
            journal: None,
            journal_volume_cursor: None,
            journal_root_cursor: None,
            journal_next_action_is_read: true,
            live: None,
            live_root_cursor: None,
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
            recovery_inventory_sources: BTreeMap::new(),
            metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
            recovery_retries: BTreeMap::new(),
            authoritative_root_cursor: None,
            legacy_automatic_scans_retired: false,
            inventory_cleanup: InventoryCleanupOwner::default(),
            is_stopping: true,
            stop_requested: Arc::new(AtomicBool::new(true)),
            core_stopped: false,
            journal_closed: false,
            journal_close: None,
            drain_panic: None,
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

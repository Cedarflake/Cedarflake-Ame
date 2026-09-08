use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

mod preview_health;
pub use preview_health::{PreviewHealthObservation, PreviewHealthOutcome, PreviewHealthTarget};
mod scan_publication_control;
pub(crate) use scan_publication_control::ScanPublicationControl;
mod inventory_cleanup_control;
pub use inventory_cleanup_control::InventoryCleanupControl;

use crate::domain::{
    AssetLocationView, CatalogCursor, CatalogDeltaBatch, CatalogDeltaPublication, CatalogSnapshot,
    CatalogSpaceUsage, DiscoveredFile, ExpectedFileState, FileIdentityEvidence,
    GalleryLayoutManifestChunk, GalleryLayoutManifestCursor, GalleryQuery, GalleryTimeAnchor,
    GalleryTimeline, IncrementalCatalogRoot, LibraryChangeCatchUpEvidence,
    LibraryChangeSourceBatch, LibraryChangeSourceError, LibraryChangeSourceHealth,
    LibraryChangeSourceStopReport, LibraryFolderCursor, LibraryFolderPage,
    LibraryRecoveryAuthority, LibraryRootGeneration, MediaInspection, MetadataInspection,
    MetadataInventoryCleanupReport, MetadataInventoryComparisonUpdate, MetadataInventoryEntry,
    MetadataInventoryPage, MetadataInventoryRun, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, MetadataInventoryScope, MetadataInventoryStartRequest,
    PersistentJournalBaseline, PersistentJournalBaselineClosingBoundary,
    PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCheckpoint, PersistentJournalEnrollmentReport, PersistentJournalPendingRename,
    PersistentJournalReadFailure, PersistentJournalRootFailure, PersistentJournalRootReadOutcome,
    PersistentJournalVolumeBatch, PersistentJournalVolumeIdentity, PreviewArtifact,
    PreviewMaterialization, PreviewReclamationCandidate, PreviewRequest, ScanCheckpoint, ScanError,
    ScanIssue, ScanRequest, StorageConfiguration, TerminalMediaEvidence,
};

type CatalogMaintenanceInterrupt = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub(crate) struct CatalogMaintenanceControl {
    user_cancelled: Arc<AtomicBool>,
    preempted: Arc<AtomicBool>,
    interrupt: Arc<Mutex<Option<CatalogMaintenanceInterrupt>>>,
}

impl CatalogMaintenanceControl {
    pub(crate) fn new(user_cancelled: Arc<AtomicBool>) -> Self {
        Self {
            user_cancelled,
            preempted: Arc::new(AtomicBool::new(false)),
            interrupt: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn is_interrupted(&self) -> bool {
        self.user_cancelled.load(Ordering::Acquire) || self.preempted.load(Ordering::Acquire)
    }

    pub(crate) fn is_user_cancelled(&self) -> bool {
        self.user_cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn cancel(&self) {
        self.user_cancelled.store(true, Ordering::Release);
        self.interrupt();
    }

    pub(crate) fn preempt(&self) {
        self.preempted.store(true, Ordering::Release);
        self.interrupt();
    }

    pub(crate) fn install_interrupt(
        &self,
        interrupt: CatalogMaintenanceInterrupt,
    ) -> Result<(), crate::domain::ScanError> {
        let mut installed = self.interrupt.lock().map_err(|_| {
            crate::domain::ScanError::new(
                "catalog_reclamation_control_unavailable",
                "The catalog reclamation cancellation control is unavailable",
            )
        })?;
        *installed = Some(interrupt);
        if self.is_interrupted()
            && let Some(interrupt) = installed.as_ref()
        {
            interrupt();
        }
        Ok(())
    }

    pub(crate) fn clear_interrupt(&self) {
        if let Ok(mut interrupt) = self.interrupt.lock() {
            *interrupt = None;
        }
    }

    fn interrupt(&self) {
        let interrupt = self
            .interrupt
            .lock()
            .ok()
            .and_then(|installed| installed.clone());
        if let Some(interrupt) = interrupt {
            interrupt();
        }
    }
}

pub(crate) enum CatalogMaintenanceAttempt<T> {
    Completed(T),
    Busy,
    Interrupted,
}

pub(crate) trait RetainedScanRepository {
    fn cancel_retained_scan(&mut self, scan_id: &str) -> Result<(), ScanError>;
}

pub(crate) trait CatalogSpaceRepository {
    fn inspect_catalog_space(
        &self,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError>;
    fn try_convert_to_incremental(
        &self,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError>;
    fn try_reclaim_incremental(
        &self,
        max_pages: u32,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError>;
    fn try_checkpoint_wal(
        &self,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<()>, ScanError>;
}
use crate::domain::{
    LeasedLibraryChange, LibraryChangeCapacityDeferral, LibraryChangeEnqueueReport,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeLane,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeQueueMetrics, LibraryChangeQueuePolicy,
};
#[cfg(test)]
use crate::domain::{
    LibraryChangeCatchUpBatch, LibraryChangeCatchUpCheckpoint, LibraryChangeCatchUpLimits,
    LibraryChangeCatchUpQueueBatch, RecoverableScan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeSourceRequest {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub root_path: PathBuf,
    pub ingress_capacity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPublicationAuthority {
    pub root_generation: LibraryRootGeneration,
    pub root_identity: Option<FileIdentityEvidence>,
}

pub trait LibraryChangeSource: Send + 'static {
    fn health(&self) -> LibraryChangeSourceHealth;
    fn drain(
        &mut self,
        max_observations: usize,
    ) -> Result<LibraryChangeSourceBatch, LibraryChangeSourceError>;
    fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError>;
}

#[cfg(test)]
pub trait LibraryChangeSourceFactory: Clone + Send + Sync + 'static {
    type Source: LibraryChangeSource;

    fn start(
        &self,
        request: &LibraryChangeSourceRequest,
    ) -> Result<Self::Source, LibraryChangeSourceError>;
}

pub(crate) type BoxedLibraryChangeSource = Box<dyn LibraryChangeSource>;
pub(crate) type LibraryChangeSourceStarter = Arc<
    dyn Fn(
            &LibraryChangeSourceRequest,
        ) -> Result<BoxedLibraryChangeSource, LibraryChangeSourceError>
        + Send
        + Sync,
>;

#[cfg(test)]
pub(crate) fn erase_library_change_source_factory<Factory>(
    factory: Factory,
) -> LibraryChangeSourceStarter
where
    Factory: LibraryChangeSourceFactory,
{
    Arc::new(move |request| {
        factory
            .start(request)
            .map(|source| Box::new(source) as BoxedLibraryChangeSource)
    })
}

pub trait LibraryChangeQueue {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError>;
    fn enqueue_metadata_inventory_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LibraryChangeEnqueueReport>, ScanError> {
        let _ = (authority, intents, enqueued_unix_ms, policy);
        Err(ScanError::new(
            "metadata_inventory_candidate_enqueue_unsupported",
            "This change queue does not support inventory authority protection",
        ))
    }
    #[cfg(test)]
    fn enqueue_library_change_intents_with_catch_up(
        &mut self,
        intents: &[LibraryChangeIntent],
        evidence: &LibraryChangeCatchUpEvidence,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError>;
    #[cfg(test)]
    fn enqueue_library_change_catch_up_batches(
        &mut self,
        batches: &[LibraryChangeCatchUpQueueBatch],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LibraryChangeEnqueueReport>, ScanError>;
    fn lease_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError>;
    fn lease_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError>;
    fn lease_path_library_changes_in_lane(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        lane: LibraryChangeLane,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        let _ = lane;
        self.lease_path_library_changes(root_id, root_generation, now_unix_ms, policy)
    }
    fn lease_metadata_inventory_recovery_candidates(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_path_library_changes_in_lane(
            root_id,
            root_generation,
            LibraryChangeLane::Recovery,
            now_unix_ms,
            policy,
        )
    }
    fn load_metadata_inventory_candidate_root_identity(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        let _ = (root_id, root_generation);
        Ok(None)
    }
    fn lease_unowned_recovery_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        let _ = (root_id, root_generation, now_unix_ms, policy);
        Err(ScanError::new(
            "unowned_recovery_path_lease_unsupported",
            "This change queue does not support legacy unowned recovery debt",
        ))
    }
    fn lease_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError>;
    fn complete_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError>;
    fn retry_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        failed_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError>;
    fn promote_live_watcher_gap_to_metadata_inventory(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        promoted_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError>;
    fn defer_library_change_for_capacity(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferral: LibraryChangeCapacityDeferral,
        deferred_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let _ = (
            change_id,
            lease_generation,
            deferral,
            deferred_unix_ms,
            policy,
        );
        Err(ScanError::new(
            "change_queue_capacity_deferral_unsupported",
            "This change queue does not support durable capacity deferral",
        ))
    }
    fn defer_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferred_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let outcomes = self.defer_library_changes(
            &[crate::domain::LibraryChangeLeaseIdentity {
                change_id,
                lease_generation,
            }],
            deferred_unix_ms,
        )?;
        match outcomes.as_slice() {
            [outcome] => Ok(*outcome),
            _ => Err(ScanError::new(
                "change_queue_deferral_outcome_invalid",
                "The queue must return exactly one outcome for each deferred lease",
            )),
        }
    }
    /// Atomically returns a bounded batch, with exactly one outcome per input in input order.
    fn defer_library_changes(
        &mut self,
        leases: &[crate::domain::LibraryChangeLeaseIdentity],
        deferred_unix_ms: i64,
    ) -> Result<Vec<LibraryChangeLeaseUpdateOutcome>, ScanError>;
    fn load_library_change_queue_metrics(
        &self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError>;
    fn load_library_change_root_queue_metrics(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError>;
    fn cleanup_terminal_library_changes(
        &mut self,
        terminal_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError>;
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "R2c-P admits repository methods before R2c-Q schedules production replay"
    )
)]
pub trait PersistentJournalRepository {
    fn begin_persistent_journal_baseline(
        &mut self,
        request: &PersistentJournalBaselineStartRequest,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalBaseline, ScanError> {
        let _ = (request, policy);
        Err(ScanError::new(
            "persistent_journal_baseline_unsupported",
            "This repository does not support one-time journal baselines",
        ))
    }
    fn load_persistent_journal_baselines(
        &self,
    ) -> Result<Vec<PersistentJournalBaseline>, ScanError> {
        Ok(Vec::new())
    }
    fn capture_persistent_journal_baseline_closing_boundary(
        &mut self,
        boundary: &PersistentJournalBaselineClosingBoundary,
    ) -> Result<PersistentJournalBaseline, ScanError> {
        let _ = boundary;
        Err(ScanError::new(
            "persistent_journal_baseline_unsupported",
            "This repository does not support one-time journal baselines",
        ))
    }
    fn persistent_journal_baseline_closing_is_covered(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<bool, ScanError> {
        let _ = change_id;
        Ok(false)
    }
    fn finalize_ready_first_import_journal_baseline(
        &mut self,
        completed_unix_ms: i64,
    ) -> Result<bool, ScanError> {
        let _ = completed_unix_ms;
        Ok(false)
    }
    fn load_persistent_journal_capabilities(
        &self,
    ) -> Result<Vec<PersistentJournalCapability>, ScanError>;
    fn load_persistent_journal_checkpoint(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<PersistentJournalCheckpoint>, ScanError>;
    fn save_persistent_journal_capability(
        &mut self,
        capability: &PersistentJournalCapability,
    ) -> Result<(), ScanError>;
    fn persist_persistent_journal_root_failure(
        &mut self,
        failure: &PersistentJournalRootFailure,
        failed_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LibraryChangeId>, ScanError> {
        let _ = (failure, failed_unix_ms, policy);
        Err(ScanError::new(
            "persistent_journal_failure_persistence_unsupported",
            "This repository does not support persistent journal root failures",
        ))
    }
    fn attach_persistent_journal_recovery_opening_boundary(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        boundary: &crate::domain::LibraryRecoveryOpeningBoundary,
        captured_unix_ms: i64,
    ) -> Result<Option<LibraryChangeId>, ScanError> {
        let _ = (root_id, root_generation, boundary, captured_unix_ms);
        Ok(None)
    }
    fn load_persistent_journal_pending_renames(
        &self,
        volume: &PersistentJournalVolumeIdentity,
        journal_id: crate::domain::JournalIdentifier,
    ) -> Result<Vec<PersistentJournalPendingRename>, ScanError>;
    fn publish_persistent_journal_volume_batch(
        &mut self,
        batch: &PersistentJournalVolumeBatch,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError>;
    #[cfg(test)]
    fn enroll_persistent_journal_batch(
        &mut self,
        batch: &crate::domain::PersistentJournalEnrollmentBatch,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError>;
    #[cfg(test)]
    fn advance_persistent_journal_checkpoint(
        &mut self,
        source_range_id: &str,
        checkpoint: &PersistentJournalCheckpoint,
    ) -> Result<bool, ScanError>;
}

pub trait PersistentJournalVolumeReader: Send + Sync + 'static {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending_renames: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure>;
}

#[cfg(test)]
pub trait LibraryChangeCatchUpRepository {
    fn load_library_change_catch_up_checkpoints(
        &self,
    ) -> Result<Vec<LibraryChangeCatchUpCheckpoint>, ScanError>;
    fn save_library_change_catch_up_checkpoint(
        &mut self,
        checkpoint: &LibraryChangeCatchUpCheckpoint,
    ) -> Result<(), ScanError>;
    fn cleanup_obsolete_library_change_catch_up_checkpoints(
        &mut self,
        retained_volume_ids: &[String],
        updated_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError>;
}

#[cfg(test)]
pub trait LibraryChangeCatchUpSource: Send + Sync + 'static {
    fn read_changes(
        &self,
        roots: &[IncrementalCatalogRoot],
        checkpoints: &[LibraryChangeCatchUpCheckpoint],
        observed_unix_ms: i64,
        limits: LibraryChangeCatchUpLimits,
        cancelled: &AtomicBool,
    ) -> Result<LibraryChangeCatchUpBatch, ScanError>;
}

pub trait IncrementalCatalogRepository {
    fn load_incremental_catalog_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError>;
    fn load_incremental_catalog_root(
        &self,
        root_id: &str,
    ) -> Result<Option<IncrementalCatalogRoot>, ScanError>;
    fn load_incremental_location_by_relative_path(
        &self,
        root_id: &str,
        relative_path: &str,
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn load_incremental_locations_by_relative_paths(
        &self,
        root_id: &str,
        relative_paths: &[String],
    ) -> Result<Vec<AssetLocationView>, ScanError> {
        let mut locations = Vec::with_capacity(relative_paths.len());
        for relative_path in relative_paths {
            if let Some(location) =
                self.load_incremental_location_by_relative_path(root_id, relative_path)?
            {
                locations.push(location);
            }
        }
        Ok(locations)
    }
    fn load_incremental_location_by_file_identity(
        &self,
        identity: &FileIdentityEvidence,
        catch_up_lineage: &[LibraryChangeCatchUpEvidence],
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn load_incremental_locations_in_subtree(
        &self,
        root_id: &str,
        relative_subtree: &str,
        limit: u32,
    ) -> Result<Vec<AssetLocationView>, ScanError>;
    fn load_terminal_media_evidence_by_relative_paths(
        &self,
        root_id: &str,
        relative_paths: &[String],
    ) -> Result<Vec<TerminalMediaEvidence>, ScanError>;
    fn publish_catalog_delta(
        &mut self,
        batch: &CatalogDeltaBatch,
        completed_unix_ms: i64,
    ) -> Result<CatalogDeltaPublication, ScanError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataInventorySourcePreparation {
    Ready,
    Yielded,
}

pub trait MetadataInventorySource: Send {
    fn rebind_recovery_lease(&mut self, _authority: &LeasedLibraryChange) -> Result<(), ScanError> {
        Err(ScanError::new(
            "metadata_inventory_source_rebind_unsupported",
            "This inventory source cannot continue under a different recovery lease",
        ))
    }

    fn prepare_next_page(
        &mut self,
        _max_source_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventorySourcePreparation, ScanError> {
        Ok(MetadataInventorySourcePreparation::Ready)
    }

    fn next_page(
        &mut self,
        max_entries: u32,
        cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError>;
}

pub struct MetadataInventoryAbsencePublicationRequest<'a> {
    pub run_id: &'a str,
    pub expected_cursor: Option<&'a str>,
    pub next_cursor: &'a str,
    pub intents: &'a [LibraryChangeIntent],
    pub updated_unix_ms: i64,
    pub policy: LibraryChangeQueuePolicy,
}

pub trait MetadataInventoryRepository {
    fn open_metadata_inventory_source(
        &self,
        root_path: &str,
        scope: &MetadataInventoryScope,
        run: &MetadataInventoryRun,
        authority: &LeasedLibraryChange,
        expected_publication_identity: Option<&FileIdentityEvidence>,
        expected_source_identity: &FileIdentityEvidence,
    ) -> Result<Box<dyn MetadataInventorySource>, ScanError>;

    fn load_metadata_inventory_root_identity(
        &self,
        run_id: &str,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        let _ = run_id;
        Ok(None)
    }

    fn authorize_metadata_inventory_recovery(
        &mut self,
        authority: &LibraryRecoveryAuthority,
    ) -> Result<LibraryRecoveryAuthority, ScanError>;
    fn load_metadata_inventory_recovery_authority(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<Option<LibraryRecoveryAuthority>, ScanError>;
    fn retire_metadata_inventory_recovery_authority(
        &mut self,
        change_id: LibraryChangeId,
        retired_unix_ms: i64,
    ) -> Result<LibraryRecoveryAuthority, ScanError>;
    fn metadata_inventory_is_waiting_for_closing_boundary(
        &self,
        run_id: &str,
    ) -> Result<bool, ScanError> {
        let _ = run_id;
        Ok(false)
    }
    fn metadata_inventory_recovery_allows_absence(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<bool, ScanError> {
        let _ = change_id;
        Ok(true)
    }
    fn finish_metadata_inventory_recovery(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<Option<LibraryChangeLeaseUpdateOutcome>, ScanError> {
        let _ = (
            change_id,
            lease_generation,
            catalog_revision_at_success,
            completed_unix_ms,
        );
        Ok(None)
    }
    fn begin_next_metadata_inventory(
        &mut self,
        request: &MetadataInventoryStartRequest,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn begin_metadata_inventory(
        &mut self,
        request: &MetadataInventoryRunRequest,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn stage_metadata_inventory_page(
        &mut self,
        run_id: &str,
        page: &MetadataInventoryPage,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn authorize_metadata_inventory_absence(
        &mut self,
        run_id: &str,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn load_pending_metadata_inventory_entries(
        &self,
        run_id: &str,
        limit: u32,
    ) -> Result<Vec<MetadataInventoryEntry>, ScanError>;
    fn load_metadata_inventory_previous_path(
        &self,
        run_id: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<String>, ScanError>;
    fn load_metadata_inventory_previous_paths(
        &self,
        run_id: &str,
        identities: &[FileIdentityEvidence],
    ) -> Result<Vec<(FileIdentityEvidence, String)>, ScanError> {
        let mut previous_paths = Vec::with_capacity(identities.len());
        for identity in identities {
            if let Some(previous_path) =
                self.load_metadata_inventory_previous_path(run_id, identity)?
            {
                previous_paths.push((identity.clone(), previous_path));
            }
        }
        Ok(previous_paths)
    }
    fn record_metadata_inventory_comparisons(
        &mut self,
        run_id: &str,
        updates: &[MetadataInventoryComparisonUpdate],
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn publish_metadata_inventory_comparison_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        run_id: &str,
        intents: &[LibraryChangeIntent],
        updates: &[MetadataInventoryComparisonUpdate],
        updated_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<(LibraryChangeEnqueueReport, MetadataInventoryRun)>, ScanError>;
    fn load_metadata_inventory_absence_candidates(
        &self,
        run_id: &str,
        after_relative_path: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError>;
    fn advance_metadata_inventory_absence_cursor(
        &mut self,
        run_id: &str,
        expected_cursor: Option<&str>,
        next_cursor: &str,
        candidate_count: u64,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn publish_metadata_inventory_absence_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        request: MetadataInventoryAbsencePublicationRequest<'_>,
    ) -> Result<Option<(LibraryChangeEnqueueReport, MetadataInventoryRun)>, ScanError>;
    fn complete_metadata_inventory(
        &mut self,
        run_id: &str,
        completed_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn terminate_metadata_inventory(
        &mut self,
        run_id: &str,
        status: MetadataInventoryRunStatus,
        issue: Option<(&str, &str)>,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError>;
    fn load_metadata_inventory_run(
        &self,
        run_id: &str,
    ) -> Result<Option<MetadataInventoryRun>, ScanError>;
    fn cleanup_terminal_metadata_inventories(
        &mut self,
        terminal_before_unix_ms: i64,
        entry_limit: u32,
        run_limit: u32,
        control: InventoryCleanupControl,
    ) -> Result<MetadataInventoryCleanupReport, ScanError>;
}

pub trait CatalogRepository {
    fn catalog_path(&self) -> &Path;
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production foreground scans use the Windows namespace-bound catalog entrypoint"
        )
    )]
    fn begin_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError>;
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production foreground resumes use the Windows namespace-bound catalog entrypoint"
        )
    )]
    fn resume_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError>;
    #[cfg(test)]
    fn begin_authoritative_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError>;
    #[cfg(test)]
    fn resume_authoritative_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError>;
    fn has_active_locations(&self) -> Result<bool, ScanError>;
    fn load_scan_location_by_file_identity(
        &self,
        scan_id: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn load_active_location(
        &self,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn load_active_location_by_asset_id(
        &self,
        asset_id: &str,
        preferred_location_id: Option<&str>,
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn stage_location(
        &mut self,
        scan_id: &str,
        root_id: &str,
        location: &AssetLocationView,
    ) -> Result<(), ScanError>;
    #[cfg(test)]
    fn update_active_preview(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
        request: Option<&PreviewRequest>,
    ) -> Result<(), ScanError>;
    fn update_active_preview_with_authority(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
        request: Option<&PreviewRequest>,
        publication_authority: Option<&PreviewPublicationAuthority>,
    ) -> Result<(), ScanError>;
    fn reset_all_previews_for_cleanup(&mut self) -> Result<u64, ScanError>;
    fn reset_previews_outside_root(&mut self, preview_root_prefix: &str) -> Result<u64, ScanError>;
    fn is_preview_artifact_path_indexed(
        &self,
        path: &str,
        artifact_key: Option<&str>,
    ) -> Result<bool, ScanError>;
    fn load_preview_recovery_artifacts(
        &self,
        preview_root_prefix: &str,
        after_artifact_key: Option<&str>,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError>;
    fn try_reconcile_preview_health(
        &mut self,
        target: PreviewHealthTarget<'_>,
        observation: PreviewHealthObservation,
    ) -> Result<PreviewHealthOutcome, ScanError>;
    fn touch_preview_artifacts(&mut self, artifacts: &[(String, String)])
    -> Result<u64, ScanError>;
    fn load_preview_reclamation_candidates(
        &self,
        protected_location_ids: &[String],
        current_algorithm_id: &str,
        current_algorithm_version: u32,
        current_orientation_contract: &str,
        current_preview_root_prefix: &str,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError>;
    fn remove_reclaimed_preview(
        &mut self,
        candidate: &PreviewReclamationCandidate,
    ) -> Result<bool, ScanError>;
    fn record_issue(&mut self, scan_id: &str, issue: &ScanIssue) -> Result<(), ScanError>;
    fn checkpoint_scan(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError>;
    #[cfg(test)]
    fn load_recoverable_scan(&self) -> Result<Option<RecoverableScan>, ScanError>;
    #[cfg(test)]
    fn load_paused_scan(&self) -> Result<Option<RecoverableScan>, ScanError>;
    #[cfg(test)]
    fn load_authoritative_recoverable_scan_after(
        &self,
        after_scan_id: Option<&str>,
    ) -> Result<Option<RecoverableScan>, ScanError>;
    fn claim_next_directory(&mut self, scan_id: &str) -> Result<Option<String>, ScanError>;
    fn is_current_directory_enumerated(
        &self,
        scan_id: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError>;
    fn stage_directory_entries(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        relative_paths: &[String],
    ) -> Result<(), ScanError>;
    fn complete_directory_enumeration(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
    ) -> Result<(), ScanError>;
    fn has_directory_entry(
        &self,
        scan_id: &str,
        relative_directory: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError>;
    fn load_directory_entry_window(
        &self,
        scan_id: &str,
        relative_directory: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError>;
    fn enqueue_directory(&mut self, scan_id: &str, relative_path: &str) -> Result<(), ScanError>;
    fn complete_directory(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError>;
    fn pause_scan(&mut self, scan_id: &str, checkpoint: &ScanCheckpoint) -> Result<(), ScanError>;
    fn count_staged_file_states(&mut self, scan_id: &str) -> Result<u64, ScanError>;
    fn load_staged_file_state_window(
        &self,
        scan_id: &str,
        after_location_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, String, ExpectedFileState)>, ScanError>;
    fn publish_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
    ) -> Result<(), ScanError>;
    fn abandon_scan(
        &mut self,
        scan_id: &str,
        status: &str,
        issue_count: u64,
    ) -> Result<(), ScanError>;
    fn load_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError>;
    fn load_snapshot_around_location(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor_location_id: &str,
    ) -> Result<CatalogSnapshot, ScanError>;
    fn load_snapshot_around_asset(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        requested_location_id: &str,
        anchor_asset_id: &str,
        fallback_ordinal: u64,
    ) -> Result<CatalogSnapshot, ScanError>;
    fn load_gallery_timeline(
        &mut self,
        query: &GalleryQuery,
        query_id: &str,
    ) -> Result<GalleryTimeline, ScanError>;
    fn load_gallery_layout_manifest_chunk(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&GalleryLayoutManifestCursor>,
    ) -> Result<GalleryLayoutManifestChunk, ScanError>;
    fn load_folder_page(
        &mut self,
        root_id: &str,
        parent_relative_path: &str,
        max_items: u32,
        after: Option<&LibraryFolderCursor>,
    ) -> Result<LibraryFolderPage, ScanError>;
    fn unregister_root(&mut self, root_id: &str) -> Result<bool, ScanError>;
}

pub trait StorageSettingsRepository {
    fn load_or_initialize(
        &mut self,
        defaults: &StorageConfiguration,
    ) -> Result<StorageConfiguration, ScanError>;
    fn save(
        &mut self,
        configuration: &StorageConfiguration,
        pending_preview_root: Option<&str>,
    ) -> Result<(), ScanError>;
    fn load_pending_preview_roots(&mut self) -> Result<Vec<String>, ScanError>;
    fn activate_preview_root(&mut self, preview_root: &str) -> Result<(), ScanError>;
    fn load_retired_preview_roots(&mut self) -> Result<Vec<String>, ScanError>;
    fn forget_retired_preview_root(&mut self, preview_root: &str) -> Result<bool, ScanError>;
}

pub trait PreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        source: &std::fs::File,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
        force_regenerate: bool,
    ) -> Result<PreviewMaterialization, ScanIssue>;
}

pub(crate) trait MediaInspector {
    fn metadata_engine_id(&self) -> &'static str;
    fn metadata_engine_version(&self) -> &'static str;
    fn inspection_engine_id(&self) -> &'static str;
    fn inspection_engine_version(&self) -> u32;
    fn inspect(&self, file: &DiscoveredFile) -> Result<MediaInspection, MediaInspectionFailure>;
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum MediaInspectionFailureKind {
    Retryable,
    Terminal,
}

#[derive(Clone, Debug)]
pub(crate) struct MediaInspectionFailure {
    pub kind: MediaInspectionFailureKind,
    pub issue: ScanIssue,
}

pub(crate) trait MetadataExtractor {
    fn engine_id(&self) -> &'static str;
    fn engine_version(&self) -> &'static str;
    fn extract(&self, raw_exif: Option<&[u8]>, source_path: &str) -> MetadataInspection;
}

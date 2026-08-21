use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicBool;

use crate::domain::{
    AssetLocationView, CatalogCursor, CatalogDeltaBatch, CatalogDeltaPublication, CatalogSnapshot,
    DiscoveredFile, ExpectedFileState, FileIdentityEvidence, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GalleryTimeAnchor, GalleryTimeline,
    IncrementalCatalogRoot, LibraryChangeCatchUpEvidence, LibraryChangeSourceBatch,
    LibraryChangeSourceError, LibraryChangeSourceHealth, LibraryChangeSourceStopReport,
    LibraryFolderCursor, LibraryFolderPage, LibraryRootGeneration, MediaInspection,
    MetadataInspection, PreviewArtifact, PreviewMaterialization, PreviewReclamationCandidate,
    RecoverableScan, ScanCheckpoint, ScanError, ScanIssue, ScanRequest, StorageConfiguration,
};
use crate::domain::{
    LeasedLibraryChange, LibraryChangeEnqueueReport, LibraryChangeFailure, LibraryChangeId,
    LibraryChangeIntent, LibraryChangeLeaseUpdateOutcome, LibraryChangeQueueMetrics,
    LibraryChangeQueuePolicy,
};
#[cfg(test)]
use crate::domain::{
    LibraryChangeCatchUpBatch, LibraryChangeCatchUpCheckpoint, LibraryChangeCatchUpLimits,
    LibraryChangeCatchUpQueueBatch,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeSourceRequest {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub root_path: PathBuf,
    pub ingress_capacity: usize,
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
    fn defer_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferred_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError>;
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
    fn publish_catalog_delta(
        &mut self,
        batch: &CatalogDeltaBatch,
        completed_unix_ms: i64,
    ) -> Result<CatalogDeltaPublication, ScanError>;
}

pub trait CatalogRepository {
    fn catalog_path(&self) -> &Path;
    fn begin_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError>;
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
    fn update_active_preview(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
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
    fn reconcile_preview_artifact_bytes(
        &mut self,
        candidate: &PreviewReclamationCandidate,
        actual_bytes: u64,
    ) -> Result<bool, ScanError>;
    fn invalidate_preview_recovery_artifact(
        &mut self,
        candidate: &PreviewReclamationCandidate,
    ) -> Result<bool, ScanError>;
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
    fn load_recoverable_scan(&self) -> Result<Option<RecoverableScan>, ScanError>;
    fn load_paused_scan(&self) -> Result<Option<RecoverableScan>, ScanError>;
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
    fn restore_pending_preview_roots(&mut self, preview_roots: &[String]) -> Result<(), ScanError>;
    fn load_retired_preview_roots(&mut self) -> Result<Vec<String>, ScanError>;
    fn forget_retired_preview_root(&mut self, preview_root: &str) -> Result<bool, ScanError>;
}

pub trait PreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
    ) -> Result<PreviewMaterialization, ScanIssue>;
}

pub(crate) trait MediaInspector {
    fn metadata_engine_id(&self) -> &'static str;
    fn metadata_engine_version(&self) -> &'static str;
    fn inspect(&self, file: &DiscoveredFile) -> Result<MediaInspection, ScanIssue>;
}

pub(crate) trait MetadataExtractor {
    fn engine_id(&self) -> &'static str;
    fn engine_version(&self) -> &'static str;
    fn extract(&self, raw_exif: Option<&[u8]>, source_path: &str) -> MetadataInspection;
}

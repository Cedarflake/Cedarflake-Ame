mod authoritative_library_changes;
mod catalog_reclamation;
mod catalog_session;
mod directory_synchronization;
mod incremental_library_changes;
#[cfg(test)]
mod library_change_catch_up;
mod library_change_observer;
mod library_change_queue;
/// flutter_rust_bridge:ignore
mod library_synchronization;
mod load_catalog;
mod metadata_inventory;
mod persistent_journal_continuity;
mod preview;
mod preview_cleanup;
#[cfg(test)]
mod preview_performance_acceptance;
mod preview_reclamation;
mod preview_recovery;
mod scan_library;
mod storage;

#[cfg(test)]
pub(crate) use authoritative_library_changes::process_ready_authoritative_library_change_cancellable;
pub(crate) use authoritative_library_changes::{
    AuthoritativeLibraryChangeReport, AuthoritativeRecoveryPolicy, defer_authoritative_change,
    process_leased_authoritative_library_change_cancellable, retry_authoritative_change,
};
#[cfg(not(test))]
pub(crate) use catalog_reclamation::schedule_catalog_reclamation_recovery;
pub use catalog_reclamation::{
    cancel_catalog_reclamation, load_catalog_reclamation, start_catalog_reclamation,
};
pub(crate) use catalog_reclamation::{
    catalog_reclamation_snapshot, preempt_catalog_reclamation, schedule_catalog_reclamation,
};
pub use directory_synchronization::{plan_library_changes, reconcile_path_evidence};
pub use incremental_library_changes::process_ready_library_changes;
pub(crate) use incremental_library_changes::process_ready_library_changes_in_lane;
pub(crate) use incremental_library_changes::process_ready_library_changes_in_lane_cancellable;
pub(crate) use incremental_library_changes::process_ready_metadata_inventory_recovery_candidates_cancellable;
pub(crate) use incremental_library_changes::process_ready_unowned_recovery_paths_cancellable;
pub(crate) use library_change_observer::LibraryChangeObserver;
pub use library_change_queue::enqueue_library_change_plan;
#[cfg(test)]
pub(crate) use library_change_queue::prepare_library_change_catch_up_plan;
pub use load_catalog::{
    load_catalog, load_catalog_around_asset, load_catalog_around_location,
    load_catalog_asset_by_id, load_catalog_at_time, load_gallery_layout_manifest_chunk,
    load_gallery_timeline, load_library_folders, unregister_library_root,
};
pub(crate) use metadata_inventory::{
    MetadataInventoryProgressPhase, MetadataInventoryRecoveryExecution,
    MetadataInventoryRecoveryPage, MetadataInventoryRecoveryReport, MetadataInventoryWorkerControl,
    RetainedMetadataInventorySource, leased_change_requires_metadata_inventory,
    process_leased_metadata_inventory_change_with_retained_source,
};
pub(crate) use persistent_journal_continuity::{
    PersistentJournalBrokerRoot, SessionBackedPersistentJournalVolumeReader,
    catch_up_persistent_journal_volume, classify_persistent_journal_operation_failure,
};
pub use preview::materialize_preview;
pub(crate) use preview_cleanup::{acquire_preview_generation, acquire_preview_reclamation};
pub use preview_cleanup::{cancel_preview_cleanup, clear_previews, clear_retired_previews};
pub use preview_recovery::{
    PreviewRecoveryPhase, PreviewRecoverySnapshot, preview_recovery_snapshot,
};
pub use scan_library::{
    cancel_scan, load_paused_scan, load_recoverable_scan, pause_scan, resume_scan, run_scan,
    suspend_scan,
};
pub(crate) use storage::{StoragePaths, storage_paths};
pub use storage::{load_storage_status, update_storage_settings};

pub(crate) fn reserve_production_library_synchronization_start_ticket()
-> Result<u64, crate::domain::ScanError> {
    library_synchronization::reserve_production_library_synchronization_start_ticket()
}

pub(crate) fn reserve_production_library_synchronization_stop_fence()
-> Result<u64, crate::domain::ScanError> {
    library_synchronization::reserve_production_library_synchronization_stop_fence()
}

pub(crate) fn start_production_library_synchronization(
    owner_ticket: u64,
) -> Result<crate::domain::LibrarySynchronizationSnapshot, crate::domain::ScanError> {
    library_synchronization::start_production_library_synchronization(owner_ticket)
}

pub(crate) fn poll_production_library_synchronization(
    owner_ticket: u64,
) -> Result<crate::domain::LibrarySynchronizationSnapshot, crate::domain::ScanError> {
    library_synchronization::poll_production_library_synchronization(owner_ticket)
}

pub(crate) fn stop_production_library_synchronization(
    cancellation_fence: u64,
) -> Result<(), crate::domain::ScanError> {
    library_synchronization::stop_production_library_synchronization(cancellation_fence)
}

#[cfg(test)]
pub(crate) static PREVIEW_LIFECYCLE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) const INVALIDATED_MEDIA_METADATA_ENGINE_ID: &str = "ame-invalidated-media-metadata";
pub(crate) const INVALIDATED_MEDIA_METADATA_ENGINE_VERSION: &str = "0";

pub(crate) fn media_metadata_is_invalidated(location: &crate::domain::AssetLocationView) -> bool {
    location.metadata_engine_id == INVALIDATED_MEDIA_METADATA_ENGINE_ID
        && location.metadata_engine_version == INVALIDATED_MEDIA_METADATA_ENGINE_VERSION
}

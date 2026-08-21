mod authoritative_library_changes;
mod directory_synchronization;
mod incremental_library_changes;
#[cfg(test)]
mod library_change_catch_up;
mod library_change_observer;
mod library_change_queue;
/// flutter_rust_bridge:ignore
mod library_synchronization;
mod load_catalog;
mod preview;
mod preview_cleanup;
#[cfg(test)]
mod preview_performance_acceptance;
mod preview_reclamation;
mod preview_recovery;
mod scan_library;
mod storage;

pub(crate) use authoritative_library_changes::{
    AuthoritativeLibraryChangeReport, AuthoritativeRecoveryPolicy,
    process_ready_authoritative_library_change_cancellable,
};
pub use directory_synchronization::{plan_library_changes, reconcile_path_evidence};
pub use incremental_library_changes::process_ready_library_changes;
pub(crate) use library_change_observer::LibraryChangeObserver;
pub use library_change_queue::enqueue_library_change_plan;
#[cfg(test)]
pub(crate) use library_change_queue::prepare_library_change_catch_up_plan;
pub use load_catalog::{
    load_catalog, load_catalog_around_asset, load_catalog_around_location,
    load_catalog_asset_by_id, load_catalog_at_time, load_gallery_layout_manifest_chunk,
    load_gallery_timeline, load_library_folders, unregister_library_root,
};
pub use preview::materialize_preview;
pub(crate) use preview_cleanup::{acquire_preview_generation, acquire_preview_reclamation};
pub use preview_cleanup::{cancel_preview_cleanup, clear_previews, clear_retired_previews};
pub use preview_recovery::{
    PreviewRecoveryPhase, PreviewRecoverySnapshot, preview_recovery_snapshot,
};
pub use scan_library::{
    cancel_scan, load_paused_scan, load_recoverable_scan, pause_scan, run_scan,
};
pub(crate) use scan_library::{resume_authoritative_scan, suspend_scan};
pub(crate) use storage::{StoragePaths, storage_paths};
pub use storage::{load_storage_status, update_storage_settings};

pub(crate) fn start_production_library_synchronization()
-> Result<crate::domain::LibrarySynchronizationSnapshot, crate::domain::ScanError> {
    library_synchronization::start_production_library_synchronization()
}

pub(crate) fn poll_production_library_synchronization()
-> Result<crate::domain::LibrarySynchronizationSnapshot, crate::domain::ScanError> {
    library_synchronization::poll_production_library_synchronization()
}

pub(crate) fn stop_production_library_synchronization() -> Result<(), crate::domain::ScanError> {
    library_synchronization::stop_production_library_synchronization()
}

#[cfg(test)]
pub(crate) static PREVIEW_LIFECYCLE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

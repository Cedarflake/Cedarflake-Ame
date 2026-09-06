mod durable_local_metadata_inventory;
mod exif_metadata;
mod image_orientation;
mod jpeg_preview;
mod local_files;
#[cfg(test)]
mod local_metadata_inventory;
mod media_inspector;
mod preview_cache;
mod sqlite_catalog;
mod storage_settings;
#[cfg(windows)]
mod windows_library_change_source;
#[cfg(all(windows, test))]
mod windows_usn_catch_up;

pub(crate) use durable_local_metadata_inventory::DurableLocalMetadataInventory;
#[cfg(all(windows, test))]
pub(crate) use local_files::force_publication_namespace_guard_failure_for_test;
#[cfg(windows)]
pub(crate) use local_files::open_viewer_source_guard;
pub use local_files::{
    FileDiscovery, FileVisitOutcome, inspect_root_availability, revalidate_file_state,
};
pub(crate) use local_files::{
    PreviewPublicationGuard, PublicationGuardedFileDiscovery, file_identity_evidence,
    open_catalog_identity_guard, open_preview_publication_guard, open_preview_source,
    revalidate_open_preview_source, user_visible_path,
};
#[cfg(test)]
pub(crate) use local_files::{
    canonical_source_root_path, configured_root_open_count, gate_source_enumeration,
    reset_configured_root_open_instrumentation,
    reset_root_availability_metadata_probe_instrumentation,
    reset_source_content_open_instrumentation, reset_source_enumeration_instrumentation,
    root_availability_metadata_probe_count, source_content_open_count,
    source_directory_open_count as source_spool_open_count, source_entry_read_count,
    source_peak_staged_window,
};
#[cfg(test)]
pub(crate) use local_metadata_inventory::{
    LocalMetadataInventory, reset_source_root_entry_enumeration_count, source_directory_open_count,
    source_root_entry_enumeration_count,
};
pub(crate) use media_inspector::LocalMediaInspector;
pub use preview_cache::LocalPreviewStore;
pub(crate) use preview_cache::{
    PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, current_preview_artifact_key, is_ame_preview_cache_entry,
    is_current_preview_artifact, is_managed_preview_cleanup_entry,
};
#[cfg(test)]
pub(crate) use preview_cache::{
    fail_next_atomic_replace_for_test, replace_file_atomically_for_test, seed_legacy_jpeg_preview,
};
pub use sqlite_catalog::SqliteCatalog;
pub(crate) use sqlite_catalog::{
    RejectedInputValidationRoster, SqliteCatalogReadExecutor, SqliteCatalogSession,
    SqliteCatalogSpaceMaintenance, StagedValidationOutcome, StagedValidationRoster,
    ValidatedStagingProof, load_retained_scan_issue_window, retained_scan_has_unclassified_issues,
};
#[cfg(test)]
pub(crate) use sqlite_catalog::{
    WatcherRecoveryObservation, full_schema_validation_count,
    remove_persistent_journal_v22_contract_for_test, reset_full_schema_validation_count,
    set_before_catalog_delta_commit_hook, set_before_metadata_inventory_spool_commit_hook,
};
pub use storage_settings::SqliteStorageSettings;
#[cfg(all(windows, test))]
pub(crate) use windows_library_change_source::{
    native_need_rescan_batch_for_test, observer_root_handle_open_count,
    reset_observer_root_handle_open_count,
};
pub(crate) fn production_library_change_source_factory() -> crate::ports::LibraryChangeSourceStarter
{
    std::sync::Arc::new(|request| {
        #[cfg(windows)]
        {
            windows_library_change_source::start_windows_library_change_source(request)
                .map(|source| Box::new(source) as crate::ports::BoxedLibraryChangeSource)
        }
        #[cfg(not(windows))]
        {
            let _ = request;
            Err(crate::domain::LibraryChangeSourceError::new(
                "library_change_source_unsupported",
                "Continuous library synchronization is currently supported only on Windows",
            ))
        }
    })
}

#[cfg(all(windows, test))]
pub(crate) fn production_library_change_catch_up_source()
-> impl crate::ports::LibraryChangeCatchUpSource {
    windows_usn_catch_up::WindowsUsnCatchUpSource::production()
}

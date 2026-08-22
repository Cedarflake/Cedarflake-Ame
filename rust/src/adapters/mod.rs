mod exif_metadata;
mod image_orientation;
mod jpeg_preview;
mod local_files;
mod local_metadata_inventory;
mod media_inspector;
mod preview_cache;
mod sqlite_catalog;
mod storage_settings;
#[cfg(windows)]
mod windows_library_change_source;
#[cfg(all(windows, test))]
mod windows_usn_catch_up;

pub(crate) use local_files::user_visible_path;
pub use local_files::{
    FileDiscovery, FileVisitOutcome, inspect_root_availability, revalidate_file_state,
};
pub use local_metadata_inventory::LocalMetadataInventory;
pub(crate) use media_inspector::LocalMediaInspector;
pub use preview_cache::LocalPreviewStore;
pub(crate) use preview_cache::{
    PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, current_preview_artifact_key, is_ame_preview_cache_entry,
    is_current_preview_artifact, is_managed_preview_cleanup_entry,
};
pub use sqlite_catalog::SqliteCatalog;
pub use storage_settings::SqliteStorageSettings;
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

mod load_catalog;
mod preview;
mod preview_cleanup;
mod preview_reclamation;
mod preview_recovery;
mod scan_library;
mod storage;

pub use load_catalog::{
    load_catalog, load_catalog_at_time, load_gallery_layout_manifest_chunk, load_gallery_timeline,
    load_library_folders, unregister_library_root,
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
pub(crate) use storage::{StoragePaths, storage_paths};
pub use storage::{load_storage_status, update_storage_settings};

#[cfg(test)]
pub(crate) static PREVIEW_LIFECYCLE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

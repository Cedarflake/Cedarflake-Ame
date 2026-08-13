mod exif_metadata;
mod image_orientation;
mod jpeg_preview;
mod local_files;
mod media_inspector;
mod preview_cache;
mod sqlite_catalog;
mod storage_settings;

pub(crate) use local_files::user_visible_path;
pub use local_files::{
    FileDiscovery, FileVisitOutcome, inspect_root_availability, revalidate_file_state,
};
pub(crate) use media_inspector::LocalMediaInspector;
pub use preview_cache::LocalPreviewStore;
pub(crate) use preview_cache::{
    PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, current_preview_artifact_key, is_ame_preview_cache_entry,
    is_current_preview_artifact, is_managed_preview_cleanup_entry,
};
pub use sqlite_catalog::SqliteCatalog;
pub use storage_settings::SqliteStorageSettings;

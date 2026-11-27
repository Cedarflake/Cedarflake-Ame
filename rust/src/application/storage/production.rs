use std::path::{Path, PathBuf};

use crate::domain::{ScanError, StorageConfiguration};
use crate::ports::StorageSettingsRepository;
use crate::ports::preview_cache_storage::PreviewCacheStorage;

pub(in crate::application) fn settings(
    path: &Path,
) -> Result<impl StorageSettingsRepository + use<>, ScanError> {
    crate::adapters::SqliteStorageSettings::open(path.to_path_buf())
}

pub(in crate::application) fn preview_cache() -> impl PreviewCacheStorage {
    crate::adapters::LocalPreviewCacheStorage
}

pub(super) fn default_storage(
    preview_budget_bytes: u64,
) -> Result<(PathBuf, StorageConfiguration), ScanError> {
    let directories = crate::adapters::application_data::directories()?;
    let settings_path = directories.config.join("storage").join("settings.sqlite3");
    let configuration = StorageConfiguration {
        catalog_path: directories
            .data
            .join("catalog")
            .join("ame.sqlite3")
            .to_string_lossy()
            .into_owned(),
        preview_root: preview_cache()
            .versioned_root(&directories.cache.join("previews"))
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes,
    };
    Ok((settings_path, configuration))
}

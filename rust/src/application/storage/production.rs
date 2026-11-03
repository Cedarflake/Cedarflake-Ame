use std::path::{Path, PathBuf};

use directories::ProjectDirs;

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
    let Some(project_dirs) = ProjectDirs::from("com", "Cedarflake", "Ame") else {
        return Err(ScanError::new(
            "application_storage_unavailable",
            "The operating system did not provide application storage directories",
        ));
    };
    let settings_path = project_dirs
        .config_dir()
        .join("storage")
        .join("settings.sqlite3");
    let configuration = StorageConfiguration {
        catalog_path: project_dirs
            .data_local_dir()
            .join("catalog")
            .join("ame.sqlite3")
            .to_string_lossy()
            .into_owned(),
        preview_root: preview_cache()
            .versioned_root(&project_dirs.cache_dir().join("previews"))
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes,
    };
    Ok((settings_path, configuration))
}

use std::fs;
use std::path::Path;

use crate::adapters::{PREVIEW_CACHE_VERSION, SqliteStorageSettings};
use crate::domain::{ScanError, StorageConfiguration, StorageSettingsUpdate};
use crate::ports::StorageSettingsRepository;

use super::catalog_admission::reserve_catalog_transition;
use super::{
    StoragePaths, active_catalog_has_roots, load_configured_storage,
    preserve_active_path_if_equivalent, resolved_paths_same, validate_absolute_directory,
    validate_budget, validate_configuration_paths,
};

pub(super) fn save_configuration(
    active: &StoragePaths,
    update: StorageSettingsUpdate,
) -> Result<StorageConfiguration, ScanError> {
    validate_budget(update.preview_budget_bytes)?;
    let _permit = reserve_catalog_transition(active)?;
    let mut configured = load_configured_storage(active)?;
    if let Some(catalog_directory) = update.catalog_directory {
        let directory = validate_absolute_directory(&catalog_directory, "catalog")?;
        configured.catalog_path = preserve_active_path_if_equivalent(
            directory.join("ame.sqlite3"),
            &active.catalog_path,
        )?
        .to_string_lossy()
        .into_owned();
    }
    if let Some(preview_cache_directory) = update.preview_cache_directory {
        let directory = validate_absolute_directory(&preview_cache_directory, "preview cache")?;
        configured.preview_root = preserve_active_path_if_equivalent(
            directory.join(PREVIEW_CACHE_VERSION),
            &active.preview_root,
        )?
        .to_string_lossy()
        .into_owned();
    }
    configured.preview_budget_bytes = update.preview_budget_bytes;
    if !resolved_paths_same(Path::new(&configured.catalog_path), &active.catalog_path)?
        && active_catalog_has_roots(&active.catalog_path)?
    {
        return Err(ScanError::new(
            "catalog_relocation_requires_migration",
            "图库已包含文件夹，当前不能更改数据库位置。需要先完成受支持的数据库迁移。",
        ));
    }
    validate_configuration_paths(&active.catalog_path, &configured)?;

    if let Some(parent) = Path::new(&configured.catalog_path).parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ScanError::new(
                "configured_catalog_directory_unavailable",
                format!("Could not prepare the configured catalog directory: {error}"),
            )
        })?;
    }
    fs::create_dir_all(&configured.preview_root).map_err(|error| {
        ScanError::new(
            "configured_preview_directory_unavailable",
            format!("Could not prepare the configured preview directory: {error}"),
        )
    })?;
    let mut settings = SqliteStorageSettings::open(active.settings_path.clone())?;
    let active_preview_root = active.preview_root.to_string_lossy();
    let pending_preview_root =
        (!resolved_paths_same(Path::new(&configured.preview_root), &active.preview_root)?)
            .then_some(active_preview_root.as_ref());
    settings.save(&configured, pending_preview_root)?;
    Ok(configured)
}

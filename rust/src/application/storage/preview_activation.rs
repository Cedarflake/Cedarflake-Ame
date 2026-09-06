use std::path::{Path, PathBuf};

use crate::adapters::{LocalPreviewStore, SqliteStorageSettings};
use crate::domain::{LibraryChangeLane, ScanError, StorageConfiguration};
use crate::ports::{CatalogRepository, StorageSettingsRepository};

use super::validate_preview_root_outside_sources;

pub(super) fn activate_configured_preview_root(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
) -> Result<PathBuf, ScanError> {
    activate_configured_preview_root_with(
        settings_path,
        catalog_path,
        configured,
        initialize_preview_root,
    )
}

pub(super) fn activate_configured_preview_root_with(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
    mut initialize: impl FnMut(&Path, u64) -> Result<(), ScanError>,
) -> Result<PathBuf, ScanError> {
    let target = PathBuf::from(&configured.preview_root);
    let mut settings = SqliteStorageSettings::open(settings_path.to_path_buf())?;
    let pending_roots = settings.load_pending_preview_roots()?;
    if let Err(validation_error) = validate_preview_root_outside_sources(catalog_path, &target) {
        return initialize_previous_preview_root(
            catalog_path,
            &pending_roots,
            configured.preview_budget_bytes,
            &mut initialize,
        )
        .ok_or(validation_error);
    }
    if let Err(target_error) = initialize(&target, configured.preview_budget_bytes) {
        return initialize_previous_preview_root(
            catalog_path,
            &pending_roots,
            configured.preview_budget_bytes,
            &mut initialize,
        )
        .ok_or(target_error);
    }

    if !pending_roots.is_empty() && catalog_path.exists() {
        let reset_result =
            super::super::catalog_session::open_catalog(catalog_path, LibraryChangeLane::Recovery)
                .and_then(|mut catalog| {
                    catalog
                        .reset_previews_outside_root(&preview_root_prefix(&target))
                        .map(|_| ())
                });
        if let Err(reset_error) = reset_result {
            return initialize_previous_preview_root(
                catalog_path,
                &pending_roots,
                configured.preview_budget_bytes,
                &mut initialize,
            )
            .ok_or(reset_error);
        }
    }
    // Pending ownership is the crash-recovery obligation across the two databases.
    // Retire it only after the idempotent catalog reset has committed.
    if let Err(activation_error) = settings.activate_preview_root(&configured.preview_root) {
        return initialize_previous_preview_root(
            catalog_path,
            &pending_roots,
            configured.preview_budget_bytes,
            &mut initialize,
        )
        .ok_or(activation_error);
    }
    Ok(target)
}

fn initialize_previous_preview_root(
    catalog_path: &Path,
    pending_roots: &[String],
    preview_budget_bytes: u64,
    initialize: &mut impl FnMut(&Path, u64) -> Result<(), ScanError>,
) -> Option<PathBuf> {
    pending_roots.iter().find_map(|pending_root| {
        let previous = PathBuf::from(pending_root);
        validate_preview_root_outside_sources(catalog_path, &previous).ok()?;
        initialize(&previous, preview_budget_bytes)
            .is_ok()
            .then_some(previous)
    })
}

fn initialize_preview_root(path: &Path, budget_bytes: u64) -> Result<(), ScanError> {
    LocalPreviewStore::new(path.to_path_buf(), budget_bytes)
        .map(|_| ())
        .map_err(|issue| ScanError::new(issue.code, issue.message))
}

fn preview_root_prefix(path: &Path) -> String {
    let mut prefix = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned();
    prefix.push(std::path::MAIN_SEPARATOR);
    prefix
}

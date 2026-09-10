use std::path::{Path, PathBuf};

use crate::adapters::{PreviewRootPreparation, SqliteStorageSettings, prepare_preview_root};
use crate::domain::{LibraryChangeLane, ScanError, StorageConfiguration};
use crate::ports::{CatalogRepository, StorageSettingsRepository};

use super::validate_preview_root_outside_sources;

pub(super) fn activate_configured_preview_root(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
) -> Result<PathBuf, ScanError> {
    activate_configured_preview_root_with_preparation(
        settings_path,
        catalog_path,
        configured,
        |path, _| prepare_preview_root(path),
    )
}

#[cfg(test)]
pub(super) fn activate_configured_preview_root_with(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
    mut initialize: impl FnMut(&Path, u64) -> Result<(), ScanError>,
) -> Result<PathBuf, ScanError> {
    activate_configured_preview_root_with_preparation(
        settings_path,
        catalog_path,
        configured,
        |path, budget| initialize(path, budget).map(|()| PreviewRootPreparation::Ready),
    )
}

pub(super) fn activate_configured_preview_root_with_preparation(
    settings_path: &Path,
    catalog_path: &Path,
    configured: &StorageConfiguration,
    mut initialize: impl FnMut(&Path, u64) -> Result<PreviewRootPreparation, ScanError>,
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
        .and_then(PreviousRootSelection::ready)
        .ok_or(validation_error);
    }
    match initialize(&target, configured.preview_budget_bytes) {
        Ok(PreviewRootPreparation::Ready) => {}
        Ok(PreviewRootPreparation::NamespaceUnavailable) => {
            // Read-only loading is not successful migration: keep pending ownership and all previews.
            return Ok(initialize_previous_preview_root(
                catalog_path,
                &pending_roots,
                configured.preview_budget_bytes,
                &mut initialize,
            )
            .map(PreviousRootSelection::path)
            .unwrap_or(target));
        }
        Err(target_error) => {
            return initialize_previous_preview_root(
                catalog_path,
                &pending_roots,
                configured.preview_budget_bytes,
                &mut initialize,
            )
            .and_then(PreviousRootSelection::ready)
            .ok_or(target_error);
        }
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
            .and_then(PreviousRootSelection::ready)
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
        .and_then(PreviousRootSelection::ready)
        .ok_or(activation_error);
    }
    Ok(target)
}

enum PreviousRootSelection {
    Ready(PathBuf),
    ReadOnly(PathBuf),
}

impl PreviousRootSelection {
    fn ready(self) -> Option<PathBuf> {
        match self {
            Self::Ready(path) => Some(path),
            Self::ReadOnly(_) => None,
        }
    }

    fn path(self) -> PathBuf {
        match self {
            Self::Ready(path) | Self::ReadOnly(path) => path,
        }
    }
}

fn initialize_previous_preview_root(
    catalog_path: &Path,
    pending_roots: &[String],
    preview_budget_bytes: u64,
    initialize: &mut impl FnMut(&Path, u64) -> Result<PreviewRootPreparation, ScanError>,
) -> Option<PreviousRootSelection> {
    let mut read_only = None;
    for pending_root in pending_roots {
        let previous = PathBuf::from(pending_root);
        if validate_preview_root_outside_sources(catalog_path, &previous).is_err() {
            continue;
        }
        match initialize(&previous, preview_budget_bytes) {
            Ok(PreviewRootPreparation::Ready) => {
                return Some(PreviousRootSelection::Ready(previous));
            }
            Ok(PreviewRootPreparation::NamespaceUnavailable) if read_only.is_none() => {
                read_only = Some(PreviousRootSelection::ReadOnly(previous));
            }
            Ok(PreviewRootPreparation::NamespaceUnavailable) | Err(_) => {}
        }
    }
    read_only
}

fn preview_root_prefix(path: &Path) -> String {
    let mut prefix = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned();
    prefix.push(std::path::MAIN_SEPARATOR);
    prefix
}

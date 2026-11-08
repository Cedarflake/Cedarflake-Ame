use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::installation::ClassificationInstallation;
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_acceleration::ClassificationAccelerationRepository;
use crate::ports::classification_installation::ClassificationInstallationRepository;
use crate::ports::classification_model_package::ClassificationModelPackage;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationForegroundState, MonotonicDispatchClock,
};
use super::classification_model_installation::production::{
    installation_repository, model_package,
};
use super::classification_runtime::{ClassificationRuntimePaths, NativeClassificationRuntime};
use super::{StoragePaths, storage_paths};

static SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

mod connection;
mod maintenance;
pub(crate) use connection::ClassificationDesktopSession;
pub(crate) use connection::current_device;
pub(crate) use connection::search;
pub(crate) use connection::semantic_status;
pub(crate) use maintenance::ClassificationDesktopMaintenance;

pub(crate) fn retry_semantic_index() -> Result<(), ScanError> {
    super::classification_runtime::recover_semantic_index(
        &storage_paths()?
            .catalog_path
            .with_file_name("semantic-features"),
    )
}

pub(crate) fn save_installation(models: String, runtime: String) -> Result<(), ScanError> {
    let installation = ClassificationInstallation {
        models: PathBuf::from(models),
        runtime: PathBuf::from(runtime),
    };
    super::classification_runtime::while_retired(|| {
        model_package().verify(&installation)?;
        installation_repository().save(&installation_path(&storage_paths()?), &installation)
    })
}

pub(crate) fn open() -> Result<Option<ClassificationDesktopSession>, ScanError> {
    open_with_storage(
        &storage_paths()?,
        std::env::current_exe().map_err(|error| {
            ScanError::new("classification_worker_unavailable", error.to_string())
        })?,
    )
}

fn open_with_storage(
    storage: &StoragePaths,
    executable: PathBuf,
) -> Result<Option<ClassificationDesktopSession>, ScanError> {
    ClassificationDesktopSession::open(storage.catalog_path.clone(), || {
        open_runtime(storage, executable)
    })
}

fn open_runtime(
    storage: &StoragePaths,
    executable: PathBuf,
) -> Result<Option<NativeClassificationRuntime>, ScanError> {
    let catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let state = catalog.classification_dispatch_state()?;
    if matches!(
        state.mode,
        ClassificationDispatchMode::Disabled | ClassificationDispatchMode::Stopped
    ) {
        return Ok(None);
    }
    drop(catalog);
    let installation = installation_repository()
        .load(&installation_path(storage))?
        .ok_or_else(|| {
            ScanError::new(
                "classification_installation_missing",
                "Choose the local classification model and runtime directories",
            )
        })?;
    let worker = executable.with_file_name("cedarflake_ame_classification_worker.exe");
    let signals = ClassificationDispatchSignals::new(MonotonicDispatchClock::default());
    // The desktop supplies its current activity before allowing the first turn.
    signals.foreground_changed(ClassificationForegroundState {
        suspended: true,
        ..Default::default()
    })?;
    let session = ClassificationRecordId::new(format!(
        "desktop-{}-{}-{}",
        std::process::id(),
        signals.unix_ms(),
        SESSION_SEQUENCE.fetch_add(1, Ordering::Relaxed),
    ))
    .map_err(|error| ScanError::new("classification_session_invalid", error.to_string()))?;
    let runtime = NativeClassificationRuntime::open(
        ClassificationRuntimePaths {
            database: storage.catalog_path.clone(),
            feature_cache_directory: storage
                .catalog_path
                .with_file_name("classification-features"),
            worker,
            models: installation.models,
            runtime: installation.runtime,
            gpu_enabled: installation_repository().gpu_enabled(&installation_path(storage))?,
        },
        session,
        signals,
    )?;
    Ok(Some(runtime))
}

pub(super) fn installation_path(storage: &StoragePaths) -> PathBuf {
    storage
        .settings_path
        .with_file_name("classification.sqlite3")
}

#[cfg(test)]
mod tests;

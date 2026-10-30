use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::adapters::classification_runtime::verify_local_installation;
use crate::adapters::{ClassificationInstallation, ClassificationInstallationStore};
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{ClassificationDispatchMode, ClassificationRecoveryRoot};
use crate::domain::{LibraryChangeLane, LibraryRootGeneration, ScanError};
use crate::ports::classification_work::ClassificationWorkRepository;

use super::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationForegroundState, MonotonicDispatchClock,
};
use super::classification_runtime::{
    ClassificationRuntimeControl, ClassificationRuntimePaths, NativeClassificationRuntime,
};
use super::{StoragePaths, storage_paths};

static SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct ClassificationDesktopSession {
    pending: Mutex<Option<NativeClassificationRuntime>>,
    control: ClassificationRuntimeControl,
}

pub(crate) fn save_installation(models: String, runtime: String) -> Result<(), ScanError> {
    let installation = ClassificationInstallation {
        models: PathBuf::from(models),
        runtime: PathBuf::from(runtime),
    };
    verify_local_installation(&installation.models, &installation.runtime)?;
    ClassificationInstallationStore::save(&installation_path(&storage_paths()?), &installation)
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
    let installation = ClassificationInstallationStore::load(&installation_path(storage))?
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
            feature_cache: storage
                .catalog_path
                .with_file_name("classification-features.sqlite3"),
            worker,
            models: installation.models,
            runtime: installation.runtime,
        },
        session,
        signals,
    )?;
    let control = runtime.control();
    Ok(Some(ClassificationDesktopSession {
        pending: Mutex::new(Some(runtime)),
        control,
    }))
}

fn installation_path(storage: &StoragePaths) -> PathBuf {
    storage
        .settings_path
        .with_file_name("classification.sqlite3")
}

impl ClassificationDesktopSession {
    pub fn run(&self) -> Result<(), ScanError> {
        let mut runtime = self.take_pending()?.ok_or_else(|| {
            ScanError::new(
                "classification_session_started",
                "The classification session has already started or closed",
            )
        })?;
        runtime.run()
    }

    pub fn foreground_changed(
        &self,
        state: ClassificationForegroundState,
    ) -> Result<(), ScanError> {
        self.control.signals().foreground_changed(state)
    }

    pub fn browsing_activity(&self) -> Result<(), ScanError> {
        self.control.signals().browsing_activity()
    }

    pub fn catalog_changed(&self) -> Result<(), ScanError> {
        self.control.signals().catalog_changed()
    }

    pub fn root_available(&self, root_id: String, generation: u64) -> Result<(), ScanError> {
        let root = ClassificationRecoveryRoot {
            id: root_id,
            generation: LibraryRootGeneration::new(generation).ok_or_else(|| {
                ScanError::new(
                    "classification_root_invalid",
                    "The source generation must be positive",
                )
            })?,
        };
        let mut cursor = None;
        while !self.control.signals().is_closed()? {
            let page = self.control.root_available(&root, cursor.as_ref())?;
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        Ok(())
    }

    pub fn request_close(&self) -> Result<(), ScanError> {
        self.control.signals().close()
    }

    pub fn close(&self) -> Result<(), ScanError> {
        self.request_close()?;
        drop(self.take_pending()?);
        Ok(())
    }

    fn take_pending(&self) -> Result<Option<NativeClassificationRuntime>, ScanError> {
        Ok(self
            .pending
            .lock()
            .map_err(|_| {
                ScanError::new(
                    "classification_session_unavailable",
                    "The classification session ownership is unavailable",
                )
            })?
            .take())
    }
}

impl Drop for ClassificationDesktopSession {
    fn drop(&mut self) {
        let _ = self.control.signals().close();
    }
}

#[cfg(test)]
mod tests;

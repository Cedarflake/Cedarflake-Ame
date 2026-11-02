use std::path::PathBuf;

use crate::adapters::SqliteCatalog;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationDispatchStep, ClassificationDispatcher,
};

mod admitted_text;
mod control;
mod discovery;
mod executor;
mod maintenance;
mod ownership;
mod setup;
pub use control::ClassificationRuntimeControl;
use executor::NativeExecutor;
pub use maintenance::{PreparedCategoryMaintenance, commit_category_maintenance};

pub(crate) fn while_retired<T>(
    change: impl FnOnce() -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    let _ownership = ownership::acquire()?;
    change()
}

pub struct ClassificationRuntimePaths {
    pub database: PathBuf,
    pub feature_cache_directory: PathBuf,
    pub worker: PathBuf,
    pub models: PathBuf,
    pub runtime: PathBuf,
}

pub struct NativeClassificationRuntime {
    dispatcher: ClassificationDispatcher<SqliteCatalog, NativeExecutor>,
    control: ClassificationRuntimeControl,
    ownership: ownership::RuntimeOwnership<'static>,
}

impl NativeClassificationRuntime {
    pub fn open(
        paths: ClassificationRuntimePaths,
        session: ClassificationRecordId,
        signals: ClassificationDispatchSignals,
    ) -> Result<Self, ScanError> {
        let ownership = ownership::acquire()?;
        let catalog =
            super::catalog_session::open_catalog(&paths.database, LibraryChangeLane::Recovery)?;
        let state = catalog.classification_dispatch_state()?;
        let run_id = state.run_id.as_ref().ok_or_else(setup::mismatch)?;
        let setup = catalog.load_classification_setup(run_id)?.ok_or_else(|| {
            ScanError::new(
                "classification_setup_missing",
                "The saved analysis settings are incomplete; choose an analysis configuration",
            )
        })?;
        let control = ClassificationRuntimeControl::new(
            paths.database.clone(),
            session.clone(),
            signals.clone(),
        );
        let executor = NativeExecutor::new(&paths, setup);
        let dispatcher = ClassificationDispatcher::open(catalog, executor, session, signals)?;
        Ok(Self {
            dispatcher,
            control,
            ownership,
        })
    }

    pub fn control(&self) -> ClassificationRuntimeControl {
        self.control.clone()
    }
    pub fn run(&mut self) -> Result<(), ScanError> {
        self.dispatcher.run()
    }
    pub fn run_until_maintenance(&mut self) -> Result<Option<CategoryMaintenancePlan>, ScanError> {
        let mut maintenance = discovery::NativeCategoryDiscovery::new();
        self.dispatcher.run_with_maintenance(&mut maintenance)
    }
    pub fn step(&mut self) -> Result<ClassificationDispatchStep, ScanError> {
        self.dispatcher.step()
    }
}

impl Drop for NativeClassificationRuntime {
    fn drop(&mut self) {
        self.ownership.require_retirement();
        let _ = self.control.signals().close();
        if self.dispatcher.retire().is_ok() {
            self.ownership.confirm_retirement();
        }
    }
}

#[cfg(test)]
mod tests;

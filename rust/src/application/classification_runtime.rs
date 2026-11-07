use std::path::PathBuf;
use std::sync::Arc;

use crate::adapters::SqliteCatalog;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationDispatchStep, ClassificationDispatcher,
};

mod control;
mod device;
mod discovery;
mod executor;
mod indexing;
mod interactive;
mod maintenance;
mod model_transition;
mod ownership;
mod production;
mod scheduling;
mod semantic;
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

pub(crate) fn recover_semantic_index(directory: &std::path::Path) -> Result<(), ScanError> {
    while_retired(|| production::recover_semantic_features(directory))
}

pub struct ClassificationRuntimePaths {
    pub database: PathBuf,
    pub feature_cache_directory: PathBuf,
    pub worker: PathBuf,
    pub models: PathBuf,
    pub runtime: PathBuf,
    pub gpu_enabled: bool,
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
        let mut catalog =
            super::catalog_session::open_catalog(&paths.database, LibraryChangeLane::Recovery)?;
        let state = catalog.classification_dispatch_state()?;
        let run_id = state.run_id.as_ref().ok_or_else(setup::mismatch)?;
        let mut setup = catalog.load_classification_setup(run_id)?.ok_or_else(|| {
            ScanError::new(
                "classification_setup_missing",
                "The saved analysis settings are incomplete; choose an analysis configuration",
            )
        })?;
        let device = Arc::new(device::ClassificationDeviceState::default());
        let factory = production::encoder_factory(&paths, device.clone());
        if factory.recognizes_model(setup.run().model()) {
            setup = model_transition::replace_model(
                &mut catalog,
                setup,
                factory.compatibility(),
                &session,
            )?;
        }
        let control = ClassificationRuntimeControl::new(
            paths.database.clone(),
            session.clone(),
            signals.clone(),
            device,
        );
        let scheduler = production::analysis_scheduler(factory, signals.clone());
        let executor = NativeExecutor::new(
            &paths,
            setup,
            scheduler.client(super::analysis_scheduling::Priority::Background),
        );
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
        scheduling::run(self)
    }
    pub fn step(&mut self) -> Result<ClassificationDispatchStep, ScanError> {
        self.dispatcher.step()
    }
}

impl Drop for NativeClassificationRuntime {
    fn drop(&mut self) {
        self.control.search.close();
        self.ownership.require_retirement();
        let _ = self.control.signals().close();
        if self.dispatcher.resources().1.scheduler().retire().is_ok() {
            self.ownership.confirm_retirement();
        }
    }
}

#[cfg(test)]
mod tests;

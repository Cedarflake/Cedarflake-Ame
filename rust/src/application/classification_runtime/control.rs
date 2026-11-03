use std::path::PathBuf;

use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchState, ClassificationRecoveryCursor,
    ClassificationRecoveryPage, ClassificationRecoveryRoot,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_availability::ClassificationAvailabilityRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::super::catalog_session::open_catalog;
use super::{ClassificationDispatchSignals, setup};

#[derive(Clone)]
pub struct ClassificationRuntimeControl {
    database: PathBuf,
    session: ClassificationRecordId,
    signals: ClassificationDispatchSignals,
}

impl ClassificationRuntimeControl {
    pub(super) fn new(
        database: PathBuf,
        session: ClassificationRecordId,
        signals: ClassificationDispatchSignals,
    ) -> Self {
        Self {
            database,
            session,
            signals,
        }
    }

    pub fn signals(&self) -> ClassificationDispatchSignals {
        self.signals.clone()
    }

    pub fn change(
        &self,
        expected_revision: u64,
        command: &ClassificationDispatchCommand,
    ) -> Result<ClassificationDispatchState, ScanError> {
        self.signals.dispatch_changed()?;
        let mut catalog = open_catalog(&self.database, LibraryChangeLane::Recovery)?;
        let state = catalog.classification_dispatch_state()?;
        if state.session_id.as_ref() != Some(&self.session) {
            return Err(setup::mismatch());
        }
        let result = catalog.change_classification_dispatch(expected_revision, command);
        // A wake before the transaction cannot stand in for the committed intent change.
        self.signals.dispatch_changed()?;
        result
    }

    pub fn retry(&self) -> Result<u64, ScanError> {
        let mut catalog = open_catalog(&self.database, LibraryChangeLane::Recovery)?;
        let count = catalog.retry_classification_work(&self.session, self.signals.unix_ms())?;
        self.signals.dispatch_changed()?;
        Ok(count)
    }

    pub fn sources_available(&self, sources: &[OrganizationSelection]) -> Result<u64, ScanError> {
        let mut catalog = open_catalog(&self.database, LibraryChangeLane::Recovery)?;
        let resumed = catalog.refresh_classification_sources(
            &self.session,
            sources,
            self.signals.unix_ms(),
        )?;
        self.signals.catalog_changed()?;
        Ok(resumed)
    }

    pub fn root_available(
        &self,
        root: &ClassificationRecoveryRoot,
        cursor: Option<&ClassificationRecoveryCursor>,
    ) -> Result<ClassificationRecoveryPage, ScanError> {
        let mut catalog = open_catalog(&self.database, LibraryChangeLane::Recovery)?;
        let page = catalog.recover_classification_root(
            &self.session,
            root,
            cursor,
            self.signals.unix_ms(),
        )?;
        self.signals.catalog_changed()?;
        Ok(page)
    }
}

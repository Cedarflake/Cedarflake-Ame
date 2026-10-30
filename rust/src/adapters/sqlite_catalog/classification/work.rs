use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationBacklogPage, ClassificationDispatchCommand, ClassificationDispatchState,
    ClassificationWorkIssue, ClassificationWorkLease, ClassificationWorkPriority,
};
use crate::domain::classification::{ClassificationRecordId, PredictionEvidence};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::super::{SqliteCatalog, database_error};

mod admission;
#[cfg(any(all(windows, feature = "classification-worker"), test))]
mod analysis;
mod availability;
pub(in crate::adapters::sqlite_catalog) use availability::source_removed;
mod completion;
pub(super) mod control;
mod leases;
mod publications;
pub(in crate::adapters::sqlite_catalog) use publications::root_published;
mod queue;
mod sources;
mod verification;

impl ClassificationWorkRepository for SqliteCatalog {
    fn classification_dispatch_state(&self) -> Result<ClassificationDispatchState, ScanError> {
        control::load(&self.connection).map(|dispatch| dispatch.state)
    }

    fn change_classification_dispatch(
        &mut self,
        expected_revision: u64,
        command: &ClassificationDispatchCommand,
    ) -> Result<ClassificationDispatchState, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let result = control::change(&transaction, expected_revision, command)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn open_classification_session(
        &mut self,
        expected_revision: u64,
        session_id: &ClassificationRecordId,
    ) -> Result<ClassificationDispatchState, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let result = control::open_session(&transaction, expected_revision, session_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn seed_classification_backlog(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<ClassificationBacklogPage, ScanError> {
        let transaction = self.begin_write()?;
        let result = admission::seed(&transaction, session_id, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn request_classification_sources(
        &mut self,
        session_id: &ClassificationRecordId,
        selections: &[OrganizationSelection],
        priority: ClassificationWorkPriority,
        now_unix_ms: u64,
    ) -> Result<u64, ScanError> {
        let transaction = self.begin_write()?;
        let result = queue::request(&transaction, session_id, selections, priority, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn claim_classification_work(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<Option<ClassificationWorkLease>, ScanError> {
        let transaction = self.begin_write()?;
        let result = leases::claim(&transaction, session_id, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn classification_work_ready_at(
        &self,
        session_id: &ClassificationRecordId,
    ) -> Result<Option<u64>, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let next = leases::ready_at(&transaction, session_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(next)
    }

    fn finish_classification_verification(
        &mut self,
        lease: &ClassificationWorkLease,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let result = completion::verified(&transaction, lease)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn publish_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
        evidence: &PredictionEvidence,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let result = completion::publish(&transaction, lease, evidence)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn yield_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let result = completion::yield_work(&transaction, lease)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn fail_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
        issue: &ClassificationWorkIssue,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let result = completion::fail(&transaction, lease, issue)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn retry_classification_work(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<u64, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let result = completion::retry(&transaction, session_id, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_dispatch_changed",
        "Classification task settings changed; refresh before continuing",
    )
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_work_invalid",
        "Classification task records are inconsistent",
    )
}

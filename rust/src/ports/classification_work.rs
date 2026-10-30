use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationBacklogPage, ClassificationDispatchCommand, ClassificationDispatchState,
    ClassificationWorkIssue, ClassificationWorkLease, ClassificationWorkPriority,
};
use crate::domain::classification::{ClassificationRecordId, PredictionEvidence};
use crate::domain::organization::OrganizationSelection;

pub trait ClassificationWorkRepository {
    fn classification_dispatch_state(&self) -> Result<ClassificationDispatchState, ScanError>;

    fn change_classification_dispatch(
        &mut self,
        expected_revision: u64,
        command: &ClassificationDispatchCommand,
    ) -> Result<ClassificationDispatchState, ScanError>;

    fn open_classification_session(
        &mut self,
        expected_revision: u64,
        session_id: &ClassificationRecordId,
    ) -> Result<ClassificationDispatchState, ScanError>;

    fn seed_classification_backlog(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<ClassificationBacklogPage, ScanError>;

    fn request_classification_sources(
        &mut self,
        session_id: &ClassificationRecordId,
        selections: &[OrganizationSelection],
        priority: ClassificationWorkPriority,
        now_unix_ms: u64,
    ) -> Result<u64, ScanError>;

    fn claim_classification_work(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<Option<ClassificationWorkLease>, ScanError>;

    fn finish_classification_verification(
        &mut self,
        lease: &ClassificationWorkLease,
    ) -> Result<bool, ScanError>;

    fn publish_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
        evidence: &PredictionEvidence,
    ) -> Result<bool, ScanError>;

    fn yield_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
    ) -> Result<bool, ScanError>;

    fn fail_classification_work(
        &mut self,
        lease: &ClassificationWorkLease,
        issue: &ClassificationWorkIssue,
    ) -> Result<bool, ScanError>;

    fn retry_classification_work(
        &mut self,
        session_id: &ClassificationRecordId,
        now_unix_ms: u64,
    ) -> Result<u64, ScanError>;
}

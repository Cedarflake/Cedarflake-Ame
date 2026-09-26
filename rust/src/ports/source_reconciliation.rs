use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeQueuePolicy, PreviewRequest, ScanError,
};

#[derive(Debug)]
pub(crate) enum SourceReconciliationAdmission {
    Leased(Box<LeasedLibraryChange>),
    RequestSuperseded,
    ExistingPathWork,
    LeaseUnavailable,
}

pub(crate) trait SourceReconciliationRepository {
    fn admit_source_reconciliation(
        &mut self,
        request: &PreviewRequest,
        intent: &LibraryChangeIntent,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<SourceReconciliationAdmission, ScanError>;
}

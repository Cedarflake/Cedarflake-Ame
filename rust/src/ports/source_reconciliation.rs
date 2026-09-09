use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeQueuePolicy, PreviewRequest, ScanError,
};

pub(crate) trait SourceReconciliationRepository {
    fn admit_source_reconciliation(
        &mut self,
        request: &PreviewRequest,
        intent: &LibraryChangeIntent,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError>;
}

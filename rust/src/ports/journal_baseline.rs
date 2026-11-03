use crate::domain::{
    LibraryChangeQueuePolicy, PersistentJournalBaseline, PersistentJournalBaselineStartRequest,
    PersistentJournalCapability, ScanError,
};

use super::PersistentJournalRepository;

/// Acquires publication authority inside the write transaction and retains it through commit.
pub(crate) trait JournalBaselineAdmission: PersistentJournalRepository {
    fn begin_journal_baseline_with_admission<Permit>(
        &mut self,
        request: &PersistentJournalBaselineStartRequest,
        policy: LibraryChangeQueuePolicy,
        acquire: impl FnOnce() -> Result<Permit, ScanError>,
    ) -> Result<PersistentJournalBaseline, ScanError>;

    fn save_first_import_journal_capability<Permit>(
        &mut self,
        capability: &PersistentJournalCapability,
        scan_id: &str,
        acquire: impl FnOnce() -> Result<Permit, ScanError>,
    ) -> Result<(), ScanError>;
}

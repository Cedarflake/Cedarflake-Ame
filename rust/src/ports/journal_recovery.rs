use crate::domain::{LibraryRootGeneration, ScanError};

use super::PersistentJournalRepository;

/// Resumes the retained journal cursor only after its independent recovery authority retires.
pub(crate) trait JournalRecoveryRepository: PersistentJournalRepository {
    fn has_retained_recovery_inventory(
        &self,
        root_id: &str,
        generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError>;

    fn resume_journal_after_metadata_fallback(
        &mut self,
        root_id: &str,
        generation: LibraryRootGeneration,
        now: i64,
    ) -> Result<bool, ScanError>;
}

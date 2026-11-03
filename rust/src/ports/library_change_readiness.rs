use crate::domain::{LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError};

/// Read-only scheduling hints; lease admission revalidates authority in its own transaction.
pub(crate) trait LibraryChangeReadiness {
    fn journal_admission_has_headroom(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_live_path_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_journal_path_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_metadata_inventory_recovery_candidates(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_legacy_unowned_recovery_debt(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_unresolved_metadata_inventory_recovery_candidates(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError>;
    fn has_ready_metadata_inventory_recovery(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_live_authoritative_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
    fn has_ready_journal_authoritative_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError>;
}

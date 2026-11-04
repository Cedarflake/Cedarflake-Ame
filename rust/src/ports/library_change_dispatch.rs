use super::LibraryChangeQueue;
use crate::domain::{
    LeasedLibraryChange, LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError,
};

/// Lane-specific lease admission and bounded retirement of unowned legacy recovery controls.
pub(crate) trait LibraryChangeDispatch: LibraryChangeQueue {
    fn compact_legacy_unowned_recovery_controls(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        compacted_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError>;
    fn lease_metadata_inventory_recovery(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError>;
    fn lease_live_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError>;
    fn lease_journal_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError>;
}

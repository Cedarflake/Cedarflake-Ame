use crate::domain::{LibraryChangeQueuePolicy, ScanError};

/// Prepares persisted synchronization debt before the existing observation and worker admission.
pub(crate) trait SynchronizationMaintenance {
    fn enqueue_media_admission_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError>;

    fn enqueue_metadata_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError>;

    fn retire_legacy_automatic_full_scans(
        &mut self,
        retired_unix_ms: i64,
    ) -> Result<u32, ScanError>;
}

use crate::domain::{LibraryChangeQueuePolicy, ScanError};
use crate::ports::{CatalogRepository, SynchronizationMaintenance};

use super::{SqliteCatalog, database_error, sqlite_unsigned};

impl SynchronizationMaintenance for SqliteCatalog {
    fn enqueue_media_admission_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        self.admit_media_rechecks(now_unix_ms, policy)
    }

    fn enqueue_metadata_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        self.admit_metadata_rechecks(now_unix_ms, policy)
    }

    fn retire_legacy_automatic_full_scans(
        &mut self,
        retired_unix_ms: i64,
    ) -> Result<u32, ScanError> {
        let scans = {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT scans.id, scans.issue_count
                     FROM scan_runs AS scans
                     WHERE scans.status IN ('running', 'paused')
                       AND scans.scan_owner = 'authoritative_recovery'
                     ORDER BY scans.id",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(database_error)?;
            let mut scans = Vec::new();
            for row in rows {
                let (scan_id, issue_count) = row.map_err(database_error)?;
                scans.push((scan_id, sqlite_unsigned(issue_count, "scan issue count")?));
            }
            scans
        };
        let retired_scan_count = u32::try_from(scans.len()).unwrap_or(u32::MAX);
        for (scan_id, issue_count) in scans {
            self.abandon_scan(&scan_id, "superseded", issue_count)?;
        }
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'superseded', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     superseded_by_change_id = NULL, last_failure_code = NULL,
                     last_failure_message = NULL, updated_unix_ms = ?1
                 WHERE origin = 'consistency_audit'
                   AND intent_kind = 'reconcile' AND scope = 'root'
                   AND status IN ('pending', 'leased', 'retry_wait')",
                [retired_unix_ms],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(retired_scan_count)
    }
}

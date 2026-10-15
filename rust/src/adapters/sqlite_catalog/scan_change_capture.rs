use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::{MAX_SCAN_CATCH_UP_LINEAGE, SCAN_QUEUE_LEASE_MILLIS, database_error, sqlite_integer};

pub(super) struct ScanChangeCapture<'a> {
    pub(super) scan_id: &'a str,
    pub(super) root_id: &'a str,
    pub(super) root_generation: i64,
    pub(super) high_watermark: i64,
}

impl ScanChangeCapture<'_> {
    pub(super) fn bind(
        &self,
        transaction: &Transaction<'_>,
        now_unix_ms: i64,
    ) -> Result<(), ScanError> {
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'leased', next_retry_unix_ms = NULL,
                     lease_generation = lease_generation + 1,
                     lease_expires_unix_ms = ?1, updated_unix_ms = ?2,
                     authoritative_scan_id = ?3
                  WHERE root_id = ?4 AND root_generation = ?5 AND id <= ?6
                    AND status IN ('pending', 'retry_wait')
                    AND EXISTS(
                      SELECT 1 FROM library_change_queue_lanes AS lanes
                      WHERE lanes.change_id = library_change_queue.id
                        AND lanes.lane <> 'p0_live'
                    )
                    AND NOT EXISTS(
                      SELECT 1 FROM library_recovery_authorities AS authority
                      WHERE authority.change_id = library_change_queue.id
                        AND authority.retired_unix_ms IS NULL
                    )
                    AND NOT EXISTS(
                      SELECT 1 FROM library_metadata_inventory_candidate_owners AS candidate
                      WHERE candidate.change_id = library_change_queue.id
                    )
                    AND NOT EXISTS(
                      SELECT 1 FROM library_live_gap_recovery_claims AS claim
                      WHERE claim.gap_change_id = library_change_queue.id
                    )",
                params![
                    now_unix_ms.saturating_add(SCAN_QUEUE_LEASE_MILLIS),
                    now_unix_ms,
                    self.scan_id,
                    self.root_id,
                    self.root_generation,
                    self.high_watermark,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO scan_run_catch_up_lineage(
                       scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                     )
                     SELECT ?1, lineage.catch_up_source,
                            lineage.catch_up_watermark,
                            MAX(lineage.enrolled_unix_ms)
                     FROM library_change_queue AS changes
                     JOIN library_change_queue_catch_up_lineage AS lineage
                       ON lineage.change_id = changes.id
                     WHERE changes.root_id = ?2
                       AND changes.root_generation = ?3
                        AND changes.id <= ?4
                        AND changes.authoritative_scan_id = ?1
                        AND changes.status IN ('pending', 'leased', 'retry_wait')
                        AND NOT EXISTS(
                          SELECT 1 FROM library_live_gap_recovery_claims AS claim
                          WHERE claim.gap_change_id = changes.id
                        )
                     GROUP BY lineage.catch_up_source, lineage.catch_up_watermark",
                params![
                    self.scan_id,
                    self.root_id,
                    self.root_generation,
                    self.high_watermark,
                ],
            )
            .map_err(database_error)?;
        let lineage_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM scan_run_catch_up_lineage
                     WHERE scan_id = ?1",
                [&self.scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        if lineage_count > MAX_SCAN_CATCH_UP_LINEAGE {
            return Err(ScanError::new(
                "catalog_scan_catch_up_lineage_limit_exceeded",
                "The scan captured too many catch-up watermarks",
            ));
        }
        Ok(())
    }

    pub(super) fn complete(
        &self,
        transaction: &Transaction<'_>,
        published_revision: u64,
        completed_unix_ms: i64,
    ) -> Result<(), ScanError> {
        // A retained scan from an older client may already hold recovery work. Its publication
        // cannot finish that work: the inventory still owns its journal boundary and candidates.
        self.release_recovery(transaction, completed_unix_ms)?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL,
                     catalog_revision_at_success = ?1, updated_unix_ms = ?2,
                     authoritative_scan_id = NULL
                 WHERE root_id = ?3 AND root_generation = ?4
                   AND authoritative_scan_id = ?6 AND id <= ?5
                   AND status IN ('pending', 'leased', 'retry_wait')
                   AND NOT (
                     status = 'retry_wait' AND scope = 'path'
                     AND intent_kind <> 'freshness_unknown'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_recovery_authorities AS authority
                     WHERE authority.change_id = library_change_queue.id
                       AND authority.retired_unix_ms IS NULL
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_metadata_inventory_candidate_owners AS candidate
                     WHERE candidate.change_id = library_change_queue.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = library_change_queue.id
                   )",
                params![
                    sqlite_integer(published_revision, "catalog revision")?,
                    completed_unix_ms,
                    self.root_id,
                    self.root_generation,
                    self.high_watermark,
                    self.scan_id,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET authoritative_scan_id = NULL
                 WHERE authoritative_scan_id = ?1",
                [self.scan_id],
            )
            .map_err(database_error)?;

        Ok(())
    }

    fn release_recovery(
        &self,
        transaction: &Transaction<'_>,
        now_unix_ms: i64,
    ) -> Result<(), ScanError> {
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = CASE WHEN status = 'leased' THEN 'pending' ELSE status END,
                     ready_unix_ms = CASE WHEN status = 'leased' THEN ?4 ELSE ready_unix_ms END,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     updated_unix_ms = ?4
                 WHERE authoritative_scan_id = ?1 AND root_id = ?2 AND root_generation = ?3
                   AND status IN ('pending', 'leased', 'retry_wait')
                   AND (
                     EXISTS(
                       SELECT 1 FROM library_recovery_authorities AS authority
                       WHERE authority.change_id = library_change_queue.id
                         AND authority.retired_unix_ms IS NULL
                     )
                     OR EXISTS(
                       SELECT 1 FROM library_metadata_inventory_candidate_owners AS candidate
                       WHERE candidate.change_id = library_change_queue.id
                     )
                   )",
                params![
                    self.scan_id,
                    self.root_id,
                    self.root_generation,
                    now_unix_ms
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

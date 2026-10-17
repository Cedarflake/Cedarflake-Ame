use rusqlite::{Connection, TransactionBehavior, params};

use crate::domain::ScanError;

use super::super::delete_orphan_assets;
use super::current_schema::validate_current_schema_contract_with_source_revision_rows;
use super::{database_error, unix_time_ms};

pub(super) fn recover(connection: &mut Connection) -> Result<(), ScanError> {
    let has_interrupted_claim = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_live_gap_recovery_claims AS claim
               JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
               WHERE claim.consumer_kind = 'foreground_scan'
                 AND claim.consumed_unix_ms IS NULL
                 AND scans.scan_owner = 'foreground'
                 AND scans.status IN ('running', 'paused')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_interrupted_claim {
        return Ok(());
    }

    let recovered_unix_ms = unix_time_ms();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let scan_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT DISTINCT claim.foreground_scan_id
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
                 WHERE claim.consumer_kind = 'foreground_scan'
                   AND claim.consumed_unix_ms IS NULL
                   AND scans.scan_owner = 'foreground'
                   AND scans.status IN ('running', 'paused')
                 ORDER BY claim.foreground_scan_id",
            )
            .map_err(database_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    for scan_id in scan_ids {
        let claim_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM library_live_gap_recovery_claims
                 WHERE consumer_kind = 'foreground_scan'
                   AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1",
                [&scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let restored_gaps = transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     last_failure_code = 'live_gap_v30_explicit_recovery_required',
                     last_failure_message =
                       'The explicit library update was interrupted and must be started again',
                     updated_unix_ms = ?2
                 WHERE id IN (
                   SELECT gap_change_id FROM library_live_gap_recovery_claims
                   WHERE consumer_kind = 'foreground_scan'
                     AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1
                 )
                   AND status = 'leased' AND authoritative_scan_id = ?1",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        if i64::try_from(restored_gaps).ok() != Some(claim_count) {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground recovery no longer owns every explicit gap",
            ));
        }
        let restored_claims = transaction
            .execute(
                "UPDATE library_live_gap_recovery_claims
                 SET consumer_kind = 'explicit_recovery_required',
                     foreground_scan_id = NULL, consumed_unix_ms = NULL
                 WHERE consumer_kind = 'foreground_scan'
                   AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1",
                [&scan_id],
            )
            .map_err(database_error)?;
        if i64::try_from(restored_claims).ok() != Some(claim_count) {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground consumer changed before recovery",
            ));
        }
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'pending', ready_unix_ms = ?2,
                     next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                     authoritative_scan_id = NULL, updated_unix_ms = ?2
                 WHERE authoritative_scan_id = ?1 AND status = 'leased'
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = library_change_queue.id
                   )",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue SET authoritative_scan_id = NULL
                 WHERE authoritative_scan_id = ?1",
                [&scan_id],
            )
            .map_err(database_error)?;
        let terminalized = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = 'failed', completed_unix_ms = ?2,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND scan_owner = 'foreground'
                   AND status IN ('running', 'paused')",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        if terminalized != 1 {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground scan changed before recovery",
            ));
        }
        for sql in [
            "DELETE FROM scan_run_catch_up_lineage WHERE scan_id = ?1",
            "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
            "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
            "DELETE FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
            "DELETE FROM asset_locations WHERE scan_id = ?1",
        ] {
            transaction
                .execute(sql, [&scan_id])
                .map_err(database_error)?;
        }
    }
    delete_orphan_assets(&transaction)?;
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    transaction.commit().map_err(database_error)
}

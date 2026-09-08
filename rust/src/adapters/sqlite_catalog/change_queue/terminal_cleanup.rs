use std::collections::HashSet;

use rusqlite::{Transaction, params};

use crate::domain::{LibraryChangeQueuePolicy, ScanError};

use super::super::database_error;

pub(super) fn cleanup_terminal_records(
    transaction: &Transaction<'_>,
    terminal_before_unix_ms: i64,
    limit: u32,
) -> Result<u32, ScanError> {
    if limit == 0 || limit > LibraryChangeQueuePolicy::MAX_CLEANUP_BATCH {
        return Err(ScanError::new(
            "change_queue_cleanup_limit_invalid",
            "The terminal change cleanup batch exceeds its absolute bound",
        ));
    }
    let change_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT changes.id FROM library_change_queue AS changes
                 WHERE changes.status IN ('completed', 'superseded')
                   AND changes.updated_unix_ms <= ?1
                   AND NOT EXISTS (
                     SELECT 1 FROM library_metadata_inventory_candidate_owners AS owner
                     WHERE owner.change_id = changes.id
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.root_id = changes.root_id
                       AND claim.root_generation = changes.root_generation
                       AND claim.consumer_kind = 'metadata_inventory_control'
                       AND claim.recovery_change_id = changes.id
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM library_recovery_authorities AS authority
                     WHERE authority.change_id = changes.id
                       AND authority.retired_unix_ms IS NULL
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS ownership
                     WHERE ownership.change_id = changes.id
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_change_queue AS survivor
                     WHERE survivor.id = changes.superseded_by_change_id
                       AND survivor.status IN ('pending', 'leased', 'retry_wait')
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN scan_run_catch_up_lineage AS frozen
                       ON frozen.catch_up_source = lineage.catch_up_source
                      AND frozen.catch_up_watermark = lineage.catch_up_watermark
                     JOIN scan_runs AS scans ON scans.id = frozen.scan_id
                     WHERE lineage.change_id = changes.id
                       AND scans.status IN ('running', 'paused')
                   )
                 ORDER BY changes.updated_unix_ms, changes.id
                 LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![terminal_before_unix_ms, i64::from(limit)], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(database_error)?;
        let mut change_ids = Vec::new();
        for row in rows {
            change_ids.push(row.map_err(database_error)?);
        }
        change_ids
    };
    let mut evidence = HashSet::new();
    let mut deleted_changes = 0_usize;
    for change_id in change_ids {
        let mut statement = transaction
            .prepare_cached(
                "SELECT catch_up_source, catch_up_watermark
                 FROM library_change_queue_catch_up_lineage WHERE change_id = ?1",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([change_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        for row in rows {
            evidence.insert(row.map_err(database_error)?);
        }
        drop(statement);
        super::super::spool_retirement::delete_owned_spools(
            transaction,
            super::super::spool_retirement::SpoolOwner::Change(change_id),
        )?;
        deleted_changes = deleted_changes.saturating_add(
            transaction
                .execute(
                    "DELETE FROM library_change_queue WHERE id = ?1",
                    [change_id],
                )
                .map_err(database_error)?,
        );
    }
    let mut evidence = evidence.into_iter().collect::<Vec<_>>();
    evidence.sort_unstable();
    super::super::catalog_delta::cleanup_terminal_catch_up_handoffs_batch(transaction, &evidence)?;
    let deleted_changes = u32::try_from(deleted_changes).map_err(|_| {
        ScanError::new(
            "change_queue_cleanup_count_invalid",
            "The terminal change cleanup count exceeds its supported range",
        )
    })?;
    Ok(deleted_changes)
}

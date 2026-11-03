use rusqlite::params;

use crate::domain::ScanError;

use super::database_error;

pub(super) fn cleanup_terminal_catch_up_handoffs(
    transaction: &rusqlite::Transaction<'_>,
    source: &str,
    watermark: &str,
) -> Result<(), ScanError> {
    cleanup_terminal_catch_up_handoffs_batch(
        transaction,
        &[(source.to_owned(), watermark.to_owned())],
    )
}

pub(super) fn cleanup_terminal_catch_up_handoffs_batch(
    transaction: &rusqlite::Transaction<'_>,
    evidence: &[(String, String)],
) -> Result<(), ScanError> {
    let removed_owner = release_terminal_catch_up_handoffs_batch(transaction, evidence)?;
    if removed_owner {
        super::mark_unreferenced_preview_artifacts_stale(transaction)?;
        super::delete_orphan_assets(transaction)?;
    }
    Ok(())
}

pub(super) fn release_terminal_catch_up_handoffs_batch(
    transaction: &rusqlite::Transaction<'_>,
    evidence: &[(String, String)],
) -> Result<bool, ScanError> {
    let mut removed_owner = false;
    for (source, watermark) in evidence {
        let has_active_work = has_active_owner(transaction, source, watermark)?;
        if !has_active_work {
            removed_owner |= release_catch_up_handoff_evidence(transaction, source, watermark)?;
        }
    }
    Ok(removed_owner)
}

#[cfg(test)]
pub(super) fn cleanup_obsolete_catch_up_handoffs(
    transaction: &rusqlite::Transaction<'_>,
    updated_before_unix_ms: i64,
    limit: u32,
) -> Result<(), ScanError> {
    let evidence = {
        let mut statement = transaction
            .prepare(
                "SELECT handoffs.catch_up_source, handoffs.catch_up_watermark
                 FROM (
                   SELECT catch_up_source, catch_up_watermark, updated_unix_ms
                   FROM library_change_catch_up_handoffs
                   UNION ALL
                   SELECT lineage.catch_up_source, lineage.catch_up_watermark,
                          batches.updated_unix_ms
                   FROM library_change_scan_handoff_lineage AS lineage
                   JOIN library_change_scan_handoff_batches AS batches
                     ON batches.id = lineage.batch_id
                 ) AS handoffs
                 WHERE handoffs.updated_unix_ms < ?1
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                     WHERE lineage.catch_up_source = handoffs.catch_up_source
                       AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                       AND changes.status IN ('pending', 'leased', 'retry_wait')
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM scan_run_catch_up_lineage AS lineage
                     JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                     WHERE lineage.catch_up_source = handoffs.catch_up_source
                       AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                       AND scans.status IN ('running', 'paused')
                   )
                 GROUP BY handoffs.catch_up_source, handoffs.catch_up_watermark
                 ORDER BY MIN(handoffs.updated_unix_ms),
                          handoffs.catch_up_source, handoffs.catch_up_watermark
                 LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![updated_before_unix_ms, i64::from(limit)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row.map_err(database_error)?);
        }
        evidence
    };
    cleanup_terminal_catch_up_handoffs_batch(transaction, &evidence)
}

fn release_catch_up_handoff_evidence(
    transaction: &rusqlite::Transaction<'_>,
    source: &str,
    watermark: &str,
) -> Result<bool, ScanError> {
    let batch_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT batch_id FROM library_change_scan_handoff_lineage
                 WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![source, watermark], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        let mut batch_ids = Vec::new();
        for row in rows {
            batch_ids.push(row.map_err(database_error)?);
        }
        batch_ids
    };
    let removed_legacy = transaction
        .execute(
            "DELETE FROM library_change_catch_up_handoffs
             WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            params![source, watermark],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_change_scan_handoff_lineage
             WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            params![source, watermark],
        )
        .map_err(database_error)?;
    let mut removed_batches = 0_usize;
    for batch_id in batch_ids {
        removed_batches = removed_batches.saturating_add(
            transaction
                .execute(
                    "DELETE FROM library_change_scan_handoff_batches
                     WHERE id = ?1 AND NOT EXISTS (
                       SELECT 1 FROM library_change_scan_handoff_lineage
                       WHERE batch_id = ?1
                     )",
                    [batch_id],
                )
                .map_err(database_error)?,
        );
    }
    Ok(removed_legacy > 0 || removed_batches > 0)
}

pub(super) fn has_active_owner(
    connection: &rusqlite::Connection,
    source: &str,
    watermark: &str,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_change_queue_catch_up_lineage AS lineage
               JOIN library_change_queue AS changes ON changes.id = lineage.change_id
               WHERE lineage.catch_up_source = ?1 AND lineage.catch_up_watermark = ?2
                 AND changes.status IN ('pending', 'leased', 'retry_wait')
               UNION ALL
               SELECT 1
               FROM scan_run_catch_up_lineage AS lineage
               JOIN scan_runs AS scans ON scans.id = lineage.scan_id
               WHERE lineage.catch_up_source = ?1 AND lineage.catch_up_watermark = ?2
                 AND scans.status IN ('running', 'paused')
               UNION ALL
               SELECT 1
               FROM library_persistent_journal_pending_renames AS pending
               WHERE ?1 = 'persistent_journal_v1'
                 AND pending.source_range_id = ?2
               UNION ALL
               SELECT 1
               FROM library_persistent_journal_cross_root_ranges AS owners
               JOIN library_persistent_journal_cross_root_lineage AS lineage
                 ON lineage.id = owners.lineage_id
               WHERE ?1 = 'persistent_journal_v1'
                 AND owners.source_range_id = ?2
                 AND lineage.status = 'pending'
             )",
            params![source, watermark],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}

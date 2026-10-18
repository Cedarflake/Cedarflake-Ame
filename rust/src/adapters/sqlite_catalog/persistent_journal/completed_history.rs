use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::super::database_error;

pub(in crate::adapters::sqlite_catalog) fn finalize_persistent_journal_ranges(
    transaction: &Transaction<'_>,
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    super::super::migrations::journal_contract::validate_lineage_rows(transaction)?;
    super::super::migrations::journal_contract::validate_payload_children(transaction)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_range_lifecycle AS lifecycle
             SET lifecycle_state = 'completed', completed_unix_ms = ?1, updated_unix_ms = ?1
             WHERE lifecycle.lifecycle_state = 'pending'
               AND EXISTS (
                 SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                 WHERE ranges.id = lifecycle.source_range_id
                   AND ranges.status = 'checkpointed'
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_persistent_journal_pending_renames AS pending
                 WHERE pending.source_range_id = lifecycle.source_range_id
               )
               AND NOT EXISTS (
                 SELECT 1
                 FROM library_persistent_journal_queue_lineage AS ownership
                 JOIN library_change_queue AS changes ON changes.id = ownership.change_id
                 WHERE ownership.source_range_id = lifecycle.source_range_id
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
               )",
            [completed_unix_ms],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_cross_root_lineage AS lineage
             SET status = 'completed', updated_unix_ms = ?1
             WHERE lineage.status = 'pending'
               AND 2 = (
                 SELECT COUNT(DISTINCT owners.participant_role)
                 FROM library_persistent_journal_cross_root_ranges AS owners
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = owners.source_range_id
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = owners.source_range_id
                 WHERE owners.lineage_id = lineage.id
                   AND lifecycle.lifecycle_state = 'completed'
                   AND (
                     owners.participant_role = 'previous'
                     AND ranges.root_id = lineage.previous_root_id
                     AND ranges.root_generation = lineage.previous_root_generation
                     OR owners.participant_role = 'current'
                     AND ranges.root_id = lineage.current_root_id
                     AND ranges.root_generation = lineage.current_root_generation
                   )
               )",
            [completed_unix_ms],
        )
        .map_err(database_error)?;
    super::super::migrations::journal_contract::validate_lineage_rows(transaction)?;
    super::super::migrations::journal_contract::validate_payload_children(transaction)?;

    let current_roots = {
        let mut statement = transaction
            .prepare(
                "SELECT checkpoints.root_id, checkpoints.root_generation
                 FROM library_persistent_journal_checkpoints AS checkpoints
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.root_id = checkpoints.root_id
                  AND ranges.root_generation = checkpoints.root_generation
                  AND ranges.volume_guid = checkpoints.volume_guid
                  AND ranges.volume_serial = checkpoints.volume_serial
                  AND ranges.journal_id = checkpoints.journal_id
                  AND ranges.covered_until_usn = checkpoints.next_unread_usn
                  AND ranges.requested_end_usn = checkpoints.captured_exclusive_end
                  AND ranges.protocol_version = checkpoints.protocol_version
                  AND ranges.contract_version = checkpoints.contract_version
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE checkpoints.next_unread_usn = checkpoints.captured_exclusive_end
                   AND checkpoints.last_failure_code IS NULL
                   AND lifecycle.lifecycle_state = 'completed'
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_baselines AS baseline
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = baseline.change_id
                     WHERE baseline.root_id = checkpoints.root_id
                       AND baseline.root_generation = checkpoints.root_generation
                       AND baseline.phase <> 'completed'
                       AND authority.retired_unix_ms IS NULL
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_cross_root_ranges AS owners
                     JOIN library_persistent_journal_cross_root_lineage AS lineage
                       ON lineage.id = owners.lineage_id
                     WHERE owners.source_range_id = ranges.id
                       AND lineage.status <> 'completed'
                   )
                 ORDER BY checkpoints.root_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(database_error)?;
        let mut roots = Vec::new();
        for row in rows {
            roots.push(row.map_err(database_error)?);
        }
        roots
    };
    for (root_id, root_generation) in current_roots {
        let checkpoint_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'current', updated_unix_ms = ?1
                 WHERE root_id = ?2 AND root_generation = ?3
                   AND continuity_state = 'catching_up'
                   AND next_unread_usn = captured_exclusive_end
                   AND last_failure_code IS NULL",
                params![completed_unix_ms, root_id, root_generation],
            )
            .map_err(database_error)?;
        if checkpoint_updated == 0 {
            continue;
        }
        let authority_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = ?1
                 WHERE root_id = ?2 AND root_generation = ?3
                   AND capability_state = 'supported'
                   AND continuity_state = 'catching_up'
                   AND last_failure_code IS NULL
                   AND EXISTS (
                     SELECT 1 FROM library_change_root_state AS active
                     WHERE active.root_id = ?2 AND active.generation = ?3
                       AND active.is_active = 1
                   )",
                params![completed_unix_ms, root_id, root_generation],
            )
            .map_err(database_error)?;
        if authority_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_root_authority_stale",
                "The terminal journal range lost its active root authority",
            ));
        }
    }
    cleanup_completed_persistent_journal_history(transaction, completed_unix_ms, 32)?;
    super::super::migrations::journal_contract::validate_payload_children(transaction)
}

pub(super) fn cleanup_completed_persistent_journal_history(
    transaction: &Transaction<'_>,
    _completed_unix_ms: i64,
    limit: u32,
) -> Result<(), ScanError> {
    cleanup_completed_persistent_journal_lineage_clusters(transaction, limit)?;
    cleanup_completed_persistent_journal_lineage_clusters(transaction, limit)
}

fn cleanup_completed_persistent_journal_lineage_clusters(
    transaction: &Transaction<'_>,
    limit: u32,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS persistent_journal_cleanup_ranges (
               source_range_id TEXT PRIMARY KEY
             );
             DELETE FROM persistent_journal_cleanup_ranges;",
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO persistent_journal_cleanup_ranges(source_range_id)
             SELECT ranges.id
             FROM library_persistent_journal_source_ranges AS ranges
             JOIN library_persistent_journal_range_lifecycle AS lifecycle
               ON lifecycle.source_range_id = ranges.id
             LEFT JOIN library_change_root_state AS authority
               ON authority.root_id = ranges.root_id
              AND authority.generation = ranges.root_generation
             WHERE lifecycle.lifecycle_state IN ('completed', 'superseded')
               AND NOT EXISTS (
                 SELECT 1 FROM library_persistent_journal_pending_renames AS pending
                 WHERE pending.source_range_id = ranges.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_live_gap_recovery_claims AS claim
                 WHERE claim.source_range_id = ranges.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_persistent_journal_checkpoints AS checkpoints
                 WHERE checkpoints.root_id = ranges.root_id
                   AND checkpoints.root_generation = ranges.root_generation
                   AND checkpoints.volume_guid = ranges.volume_guid
                   AND checkpoints.volume_serial = ranges.volume_serial
                   AND checkpoints.journal_id = ranges.journal_id
                   AND checkpoints.next_unread_usn = ranges.covered_until_usn
               )
               AND (
                 COALESCE(authority.is_active, 0) = 0
                 AND EXISTS (
                   SELECT 1
                   FROM library_persistent_journal_source_ranges AS newer
                   JOIN library_persistent_journal_range_lifecycle AS newer_lifecycle
                     ON newer_lifecycle.source_range_id = newer.id
                   WHERE newer.root_id = ranges.root_id
                     AND newer.root_generation = ranges.root_generation
                     AND newer_lifecycle.lifecycle_state IN ('completed', 'superseded')
                     AND NOT EXISTS (
                       SELECT 1
                       FROM library_persistent_journal_pending_renames AS newer_pending
                       WHERE newer_pending.source_range_id = newer.id
                     )
                     AND (newer.enrolled_unix_ms, newer.id)
                         > (ranges.enrolled_unix_ms, ranges.id)
                 )
                 OR authority.is_active = 1
                    AND 64 <= (
                      SELECT COUNT(*)
                      FROM library_persistent_journal_source_ranges AS newer
                      JOIN library_persistent_journal_range_lifecycle AS newer_lifecycle
                        ON newer_lifecycle.source_range_id = newer.id
                      WHERE newer.root_id = ranges.root_id
                       AND newer.root_generation = ranges.root_generation
                       AND newer_lifecycle.lifecycle_state IN ('completed', 'superseded')
                       AND NOT EXISTS (
                         SELECT 1
                         FROM library_persistent_journal_pending_renames AS newer_pending
                         WHERE newer_pending.source_range_id = newer.id
                       )
                       AND (newer.enrolled_unix_ms, newer.id)
                           > (ranges.enrolled_unix_ms, ranges.id)
                    )
               )
             ORDER BY lifecycle.updated_unix_ms, ranges.id
             LIMIT ?1",
            [i64::from(limit)],
        )
        .map_err(database_error)?;
    loop {
        let removed = transaction
            .execute(
                "DELETE FROM persistent_journal_cleanup_ranges AS candidates
                 WHERE EXISTS (
                   SELECT 1
                   FROM library_persistent_journal_cross_root_ranges AS owner
                   JOIN library_persistent_journal_cross_root_ranges AS peer
                     ON peer.lineage_id = owner.lineage_id
                   JOIN library_persistent_journal_cross_root_lineage AS lineage
                     ON lineage.id = owner.lineage_id
                   WHERE owner.source_range_id = candidates.source_range_id
                     AND (
                       lineage.status NOT IN ('completed', 'superseded')
                       OR NOT EXISTS (
                         SELECT 1 FROM persistent_journal_cleanup_ranges AS peer_candidate
                         WHERE peer_candidate.source_range_id = peer.source_range_id
                       )
                     )
                 )",
                [],
            )
            .map_err(database_error)?;
        if removed == 0 {
            break;
        }
    }
    loop {
        let removed = transaction
            .execute(
                "DELETE FROM persistent_journal_cleanup_ranges AS candidates
                 WHERE EXISTS (
                   SELECT 1
                   FROM library_persistent_journal_queue_lineage AS ownership
                   JOIN library_change_queue AS changes ON changes.id = ownership.change_id
                   WHERE ownership.source_range_id = candidates.source_range_id
                     AND (
                       changes.status NOT IN ('completed', 'superseded')
                       OR EXISTS (
                         SELECT 1
                         FROM library_persistent_journal_queue_lineage AS peer
                         WHERE peer.change_id = ownership.change_id
                           AND NOT EXISTS (
                             SELECT 1 FROM persistent_journal_cleanup_ranges AS peer_candidate
                             WHERE peer_candidate.source_range_id = peer.source_range_id
                           )
                       )
                       OR EXISTS (
                         SELECT 1
                         FROM library_change_queue_catch_up_lineage AS lineage
                         JOIN scan_run_catch_up_lineage AS frozen
                           ON frozen.catch_up_source = lineage.catch_up_source
                          AND frozen.catch_up_watermark = lineage.catch_up_watermark
                         JOIN scan_runs AS scans ON scans.id = frozen.scan_id
                         WHERE lineage.change_id = changes.id
                           AND scans.status IN ('running', 'paused')
                       )
                     )
                 )",
                [],
            )
            .map_err(database_error)?;
        if removed == 0 {
            break;
        }
    }
    let queue_evidence = {
        let mut statement = transaction
            .prepare(
                "SELECT DISTINCT lineage.catch_up_source, lineage.catch_up_watermark
                 FROM library_change_queue_catch_up_lineage AS lineage
                 JOIN library_persistent_journal_queue_lineage AS ownership
                   ON ownership.change_id = lineage.change_id
                 JOIN persistent_journal_cleanup_ranges AS candidates
                   ON candidates.source_range_id = ownership.source_range_id
                 ORDER BY lineage.catch_up_source, lineage.catch_up_watermark",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row.map_err(database_error)?);
        }
        evidence
    };
    transaction
        .execute(
            "DELETE FROM library_change_queue
             WHERE id IN (
               SELECT ownership.change_id
               FROM library_persistent_journal_queue_lineage AS ownership
               JOIN persistent_journal_cleanup_ranges AS candidates
                 ON candidates.source_range_id = ownership.source_range_id
               GROUP BY ownership.change_id
               HAVING NOT EXISTS (
                 SELECT 1
                 FROM library_persistent_journal_queue_lineage AS peer
                 WHERE peer.change_id = ownership.change_id
                   AND NOT EXISTS (
                     SELECT 1 FROM persistent_journal_cleanup_ranges AS peer_candidate
                     WHERE peer_candidate.source_range_id = peer.source_range_id
                   )
               )
             )",
            [],
        )
        .map_err(database_error)?;
    super::super::catalog_delta::cleanup_terminal_catch_up_handoffs_batch(
        transaction,
        &queue_evidence,
    )?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_cross_root_lineage AS lineage
             WHERE lineage.status IN ('completed', 'superseded')
               AND EXISTS (
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges AS owner
                 JOIN persistent_journal_cleanup_ranges AS candidates
                   ON candidates.source_range_id = owner.source_range_id
                 WHERE owner.lineage_id = lineage.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges AS owner
                 WHERE owner.lineage_id = lineage.id
                   AND NOT EXISTS (
                     SELECT 1 FROM persistent_journal_cleanup_ranges AS candidates
                     WHERE candidates.source_range_id = owner.source_range_id
                   )
               )",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_source_ranges
             WHERE id IN (
               SELECT source_range_id FROM persistent_journal_cleanup_ranges
             )",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute_batch("DROP TABLE persistent_journal_cleanup_ranges")
        .map_err(database_error)
}

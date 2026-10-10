use std::collections::BTreeMap;

use rusqlite::{Transaction, params};

use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanError,
};

use super::super::change_queue::{
    PERSISTENT_JOURNAL_CATCH_UP_SOURCE, cleanup_for_enqueue, enqueue_intents_in_transaction,
};
use super::super::{database_error, sqlite_unsigned, unix_time_ms};

const CLEANUP_EVIDENCE_BATCH_SIZE: i64 = 128;

pub(in crate::adapters::sqlite_catalog) fn remove_root_persistent_journal_state(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    prepare_scope(transaction)?;
    collect_range_closure(transaction, root_id)?;
    collect_survivor_handoffs(transaction, root_id)?;
    collect_owned_changes_and_evidence(transaction, root_id)?;
    collect_owned_cleanup_scope(transaction, root_id)?;
    detach_consumers(transaction, root_id)?;
    let survivor_intents = load_survivor_intents(transaction)?;
    release_handoffs_and_ranges(transaction, root_id)?;
    enqueue_survivor_intents(transaction, survivor_intents)?;
    mark_owned_preview_artifacts_stale(transaction)?;
    delete_owned_orphan_assets(transaction)?;
    clear_scope(transaction)
}

fn prepare_scope(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_ranges (
               source_range_id TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_changes (
               change_id INTEGER PRIMARY KEY
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_evidence (
               catch_up_source TEXT NOT NULL,
               catch_up_watermark TEXT NOT NULL,
               PRIMARY KEY(catch_up_source, catch_up_watermark)
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_survivors (
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(root_id, root_generation, relative_path)
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_assets (
               asset_id TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS persistent_journal_root_unregister_preview_artifacts (
               artifact_key TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             DELETE FROM persistent_journal_root_unregister_ranges;
             DELETE FROM persistent_journal_root_unregister_changes;
             DELETE FROM persistent_journal_root_unregister_evidence;
             DELETE FROM persistent_journal_root_unregister_survivors;
             DELETE FROM persistent_journal_root_unregister_assets;
             DELETE FROM persistent_journal_root_unregister_preview_artifacts;",
        )
        .map_err(database_error)
}

fn collect_owned_cleanup_scope(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_assets(asset_id)
             SELECT handoff.asset_id
             FROM library_change_catch_up_handoffs AS handoff
             WHERE handoff.root_id = ?1 OR EXISTS (
               SELECT 1
               FROM persistent_journal_root_unregister_evidence AS evidence
               WHERE evidence.catch_up_source = handoff.catch_up_source
                 AND evidence.catch_up_watermark = handoff.catch_up_watermark
             )
             UNION
             SELECT item.asset_id
             FROM library_change_scan_handoff_items AS item
             JOIN library_change_scan_handoff_batches AS batch
               ON batch.id = item.batch_id
             WHERE batch.source_root_id = ?1 OR EXISTS (
               SELECT 1
               FROM library_change_scan_handoff_lineage AS lineage
               JOIN persistent_journal_root_unregister_evidence AS evidence
                 ON evidence.catch_up_source = lineage.catch_up_source
                AND evidence.catch_up_watermark = lineage.catch_up_watermark
               WHERE lineage.batch_id = item.batch_id
             )",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_preview_artifacts(
               artifact_key
             )
             SELECT artifact.artifact_key
             FROM preview_artifacts AS artifact
             JOIN library_change_catch_up_handoffs AS handoff
               ON handoff.preview_status = 'ready'
              AND handoff.preview_path = artifact.artifact_path
             WHERE handoff.root_id = ?1 OR EXISTS (
               SELECT 1
               FROM persistent_journal_root_unregister_evidence AS evidence
               WHERE evidence.catch_up_source = handoff.catch_up_source
                 AND evidence.catch_up_watermark = handoff.catch_up_watermark
             )
             UNION
             SELECT artifact.artifact_key
             FROM preview_artifacts AS artifact
             JOIN library_change_scan_handoff_items AS item
               ON item.preview_status = 'ready'
              AND item.preview_path = artifact.artifact_path
             JOIN library_change_scan_handoff_batches AS batch
               ON batch.id = item.batch_id
             WHERE batch.source_root_id = ?1 OR EXISTS (
               SELECT 1
               FROM library_change_scan_handoff_lineage AS lineage
               JOIN persistent_journal_root_unregister_evidence AS evidence
                 ON evidence.catch_up_source = lineage.catch_up_source
                AND evidence.catch_up_watermark = lineage.catch_up_watermark
               WHERE lineage.batch_id = item.batch_id
             )",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn collect_range_closure(transaction: &Transaction<'_>, root_id: &str) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO persistent_journal_root_unregister_ranges(source_range_id)
             SELECT id FROM library_persistent_journal_source_ranges WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    loop {
        let expanded = transaction
            .execute(
                "INSERT OR IGNORE INTO persistent_journal_root_unregister_ranges(source_range_id)
                 SELECT peer.source_range_id
                 FROM library_persistent_journal_cross_root_ranges AS owner
                 JOIN persistent_journal_root_unregister_ranges AS candidates
                   ON candidates.source_range_id = owner.source_range_id
                 JOIN library_persistent_journal_cross_root_ranges AS peer
                   ON peer.lineage_id = owner.lineage_id",
                [],
            )
            .map_err(database_error)?;
        if expanded == 0 {
            return Ok(());
        }
    }
}

fn collect_survivor_handoffs(
    transaction: &Transaction<'_>,
    removed_root_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT lineage.previous_root_id, lineage.previous_root_generation,
                    lineage.previous_relative_path
             FROM library_persistent_journal_cross_root_lineage AS lineage
             JOIN library_change_root_state AS active
               ON active.root_id = lineage.previous_root_id
              AND active.generation = lineage.previous_root_generation
              AND active.is_active = 1
             WHERE lineage.previous_root_id <> ?1
               AND EXISTS (
                 SELECT 1
                 FROM library_persistent_journal_cross_root_ranges AS owner
                 JOIN persistent_journal_root_unregister_ranges AS candidates
                   ON candidates.source_range_id = owner.source_range_id
                 WHERE owner.lineage_id = lineage.id
               )
             UNION
             SELECT lineage.current_root_id, lineage.current_root_generation,
                    lineage.current_relative_path
             FROM library_persistent_journal_cross_root_lineage AS lineage
             JOIN library_change_root_state AS active
               ON active.root_id = lineage.current_root_id
              AND active.generation = lineage.current_root_generation
              AND active.is_active = 1
             WHERE lineage.current_root_id <> ?1
               AND EXISTS (
                 SELECT 1
                 FROM library_persistent_journal_cross_root_ranges AS owner
                 JOIN persistent_journal_root_unregister_ranges AS candidates
                   ON candidates.source_range_id = owner.source_range_id
                 WHERE owner.lineage_id = lineage.id
               )",
            [removed_root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT queue.root_id, queue.root_generation,
                    queue.previous_relative_path
             FROM library_persistent_journal_queue_lineage AS ownership
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON candidates.source_range_id = ownership.source_range_id
             JOIN library_change_queue AS queue ON queue.id = ownership.change_id
             JOIN library_change_root_state AS active
               ON active.root_id = queue.root_id
              AND active.generation = queue.root_generation
              AND active.is_active = 1
             WHERE queue.root_id <> ?1
               AND queue.scope = 'path'
               AND queue.previous_relative_path IS NOT NULL
               AND queue.previous_relative_path <> ''",
            [removed_root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT queue.root_id, queue.root_generation,
                    CASE
                      WHEN queue.scope = 'path' AND queue.relative_path <> ''
                        THEN queue.relative_path
                      ELSE ''
                    END
             FROM library_persistent_journal_queue_lineage AS ownership
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON candidates.source_range_id = ownership.source_range_id
             JOIN library_change_queue AS queue ON queue.id = ownership.change_id
             JOIN library_change_root_state AS active
               ON active.root_id = queue.root_id
              AND active.generation = queue.root_generation
              AND active.is_active = 1
             WHERE queue.root_id <> ?1",
            [removed_root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT scans.root_id, scans.root_generation_at_start, ''
             FROM scan_run_catch_up_lineage AS lineage
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON lineage.catch_up_source = ?1
              AND lineage.catch_up_watermark = candidates.source_range_id
             JOIN scan_runs AS scans ON scans.id = lineage.scan_id
             JOIN library_change_root_state AS active
               ON active.root_id = scans.root_id
              AND active.generation = scans.root_generation_at_start
              AND active.is_active = 1
             WHERE scans.root_id <> ?2",
            params![PERSISTENT_JOURNAL_CATCH_UP_SOURCE, removed_root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT handoff.root_id, active.generation,
                    CASE WHEN handoff.relative_path <> ''
                      THEN handoff.relative_path ELSE '' END
             FROM library_change_catch_up_handoffs AS handoff
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON handoff.catch_up_source = ?1
              AND handoff.catch_up_watermark = candidates.source_range_id
             JOIN library_change_root_state AS active
               ON active.root_id = handoff.root_id AND active.is_active = 1
             WHERE handoff.root_id <> ?2",
            params![PERSISTENT_JOURNAL_CATCH_UP_SOURCE, removed_root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_survivors(
               root_id, root_generation, relative_path
             )
             SELECT item.root_id, active.generation,
                    CASE WHEN item.relative_path <> ''
                      THEN item.relative_path ELSE '' END
             FROM library_change_scan_handoff_items AS item
             JOIN library_change_scan_handoff_lineage AS lineage
               ON lineage.batch_id = item.batch_id
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON lineage.catch_up_source = ?1
              AND lineage.catch_up_watermark = candidates.source_range_id
             JOIN library_change_root_state AS active
               ON active.root_id = item.root_id AND active.is_active = 1
             WHERE item.root_id <> ?2",
            params![PERSISTENT_JOURNAL_CATCH_UP_SOURCE, removed_root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn collect_owned_changes_and_evidence(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_changes(change_id)
             SELECT ownership.change_id
             FROM library_persistent_journal_queue_lineage AS ownership
             JOIN persistent_journal_root_unregister_ranges AS candidates
               ON candidates.source_range_id = ownership.source_range_id
             GROUP BY ownership.change_id
             HAVING NOT EXISTS (
               SELECT 1
               FROM library_persistent_journal_queue_lineage AS peer
               WHERE peer.change_id = ownership.change_id
                 AND NOT EXISTS (
                   SELECT 1
                   FROM persistent_journal_root_unregister_ranges AS peer_candidate
                   WHERE peer_candidate.source_range_id = peer.source_range_id
                 )
             )",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_evidence(
               catch_up_source, catch_up_watermark
             )
             SELECT ?1, source_range_id
             FROM persistent_journal_root_unregister_ranges",
            [PERSISTENT_JOURNAL_CATCH_UP_SOURCE],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_evidence(
               catch_up_source, catch_up_watermark
             )
             SELECT lineage.catch_up_source, lineage.catch_up_watermark
             FROM library_change_queue_catch_up_lineage AS lineage
             JOIN persistent_journal_root_unregister_changes AS changes
               ON changes.change_id = lineage.change_id",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_evidence(
               catch_up_source, catch_up_watermark
             )
             SELECT lineage.catch_up_source, lineage.catch_up_watermark
             FROM scan_run_catch_up_lineage AS lineage
             JOIN scan_runs AS scans ON scans.id = lineage.scan_id
             WHERE scans.root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO persistent_journal_root_unregister_evidence(
               catch_up_source, catch_up_watermark
             )
             SELECT lineage.catch_up_source, lineage.catch_up_watermark
             FROM library_change_scan_handoff_lineage AS lineage
             JOIN library_change_scan_handoff_batches AS batches
               ON batches.id = lineage.batch_id
             WHERE batches.source_root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn detach_consumers(transaction: &Transaction<'_>, root_id: &str) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM library_live_gap_recovery_claims
             WHERE root_id = ?1 OR source_range_id IN (
               SELECT source_range_id FROM persistent_journal_root_unregister_ranges
             )",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_pending_renames
             WHERE previous_root_id = ?1 OR source_range_id IN (
               SELECT source_range_id FROM persistent_journal_root_unregister_ranges
             )",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_cross_root_lineage AS lineage
             WHERE lineage.previous_root_id = ?1 OR lineage.current_root_id = ?1
                OR EXISTS (
                  SELECT 1
                  FROM library_persistent_journal_cross_root_ranges AS owner
                  JOIN persistent_journal_root_unregister_ranges AS candidates
                    ON candidates.source_range_id = owner.source_range_id
                  WHERE owner.lineage_id = lineage.id
                )",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 catalog_revision_at_success = NULL, updated_unix_ms = ?1
             WHERE id IN (
               SELECT change_id FROM persistent_journal_root_unregister_changes
             )",
            [unix_time_ms()],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_queue AS changes
             SET catch_up_source = (
                   SELECT lineage.catch_up_source
                   FROM library_change_queue_catch_up_lineage AS lineage
                   WHERE lineage.change_id = changes.id
                     AND NOT (
                       lineage.catch_up_source = ?1
                       AND lineage.catch_up_watermark IN (
                         SELECT source_range_id
                         FROM persistent_journal_root_unregister_ranges
                       )
                     )
                   ORDER BY lineage.enrolled_unix_ms, lineage.catch_up_source,
                            lineage.catch_up_watermark
                   LIMIT 1
                 ),
                 catch_up_watermark = (
                   SELECT lineage.catch_up_watermark
                   FROM library_change_queue_catch_up_lineage AS lineage
                   WHERE lineage.change_id = changes.id
                     AND NOT (
                       lineage.catch_up_source = ?1
                       AND lineage.catch_up_watermark IN (
                         SELECT source_range_id
                         FROM persistent_journal_root_unregister_ranges
                       )
                     )
                   ORDER BY lineage.enrolled_unix_ms, lineage.catch_up_source,
                            lineage.catch_up_watermark
                   LIMIT 1
                 )
             WHERE id NOT IN (
                     SELECT change_id FROM persistent_journal_root_unregister_changes
                   )
               AND catch_up_source = ?1
               AND catch_up_watermark IN (
                 SELECT source_range_id FROM persistent_journal_root_unregister_ranges
               )",
            [PERSISTENT_JOURNAL_CATCH_UP_SOURCE],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_queue
             SET catch_up_source = NULL, catch_up_watermark = NULL
             WHERE id IN (
               SELECT change_id FROM persistent_journal_root_unregister_changes
             )",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_change_queue_catch_up_lineage AS lineage
             WHERE lineage.change_id IN (
                     SELECT change_id FROM persistent_journal_root_unregister_changes
                   )
                OR (
                  lineage.catch_up_source = ?1
                  AND lineage.catch_up_watermark IN (
                    SELECT source_range_id FROM persistent_journal_root_unregister_ranges
                  )
                )",
            [PERSISTENT_JOURNAL_CATCH_UP_SOURCE],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM scan_run_catch_up_lineage AS lineage
             WHERE lineage.scan_id IN (
                     SELECT id FROM scan_runs WHERE root_id = ?1
                   )
                OR (
                  lineage.catch_up_source = ?2
                  AND lineage.catch_up_watermark IN (
                    SELECT source_range_id FROM persistent_journal_root_unregister_ranges
                  )
                )",
            params![root_id, PERSISTENT_JOURNAL_CATCH_UP_SOURCE],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_survivor_intents(
    transaction: &Transaction<'_>,
) -> Result<Vec<LibraryChangeIntent>, ScanError> {
    let observed_unix_ms = unix_time_ms();
    let sequence = sqlite_unsigned(observed_unix_ms, "root unregister observation time")?.max(1);
    let mut statement = transaction
        .prepare(
            "SELECT survivor.root_id, survivor.root_generation, survivor.relative_path
             FROM persistent_journal_root_unregister_survivors AS survivor
             WHERE survivor.relative_path = '' OR NOT EXISTS (
               SELECT 1
               FROM persistent_journal_root_unregister_survivors AS fallback
               WHERE fallback.root_id = survivor.root_id
                 AND fallback.root_generation = survivor.root_generation
                 AND fallback.relative_path = ''
             )
             ORDER BY survivor.root_id, survivor.root_generation, survivor.relative_path",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(database_error)?;
    let mut intents = Vec::new();
    for row in rows {
        let (root_id, root_generation, relative_path) = row.map_err(database_error)?;
        let root_generation = LibraryRootGeneration::new(sqlite_unsigned(
            root_generation,
            "surviving root generation",
        )?)
        .ok_or_else(|| {
            ScanError::new(
                "change_queue_generation_invalid",
                "The surviving root has an invalid generation",
            )
        })?;
        let is_fallback = relative_path.is_empty();
        intents.push(LibraryChangeIntent {
            root_id,
            root_generation,
            kind: if is_fallback {
                LibraryChangeIntentKind::FreshnessUnknown
            } else {
                LibraryChangeIntentKind::Reconcile
            },
            scope: if is_fallback {
                LibraryChangeScope::Root
            } else {
                LibraryChangeScope::Path
            },
            relative_path,
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
            first_observed_unix_ms: observed_unix_ms,
            most_recent_observed_unix_ms: observed_unix_ms,
            first_sequence: sequence,
            most_recent_sequence: sequence,
            coalesced_observation_count: 1,
        });
    }
    Ok(intents)
}

fn release_handoffs_and_ranges(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    loop {
        let evidence = {
            let mut statement = transaction
                .prepare(
                    "SELECT catch_up_source, catch_up_watermark
                     FROM persistent_journal_root_unregister_evidence
                     ORDER BY catch_up_source, catch_up_watermark
                     LIMIT ?1",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map([CLEANUP_EVIDENCE_BATCH_SIZE], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(database_error)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        if evidence.is_empty() {
            break;
        }
        super::super::catalog_delta::release_terminal_catch_up_handoffs_batch(
            transaction,
            &evidence,
        )?;
        for (source, watermark) in evidence {
            transaction
                .execute(
                    "DELETE FROM persistent_journal_root_unregister_evidence
                     WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
                    params![source, watermark],
                )
                .map_err(database_error)?;
        }
    }
    transaction
        .execute(
            "DELETE FROM library_change_catch_up_handoffs WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_change_scan_handoff_batches WHERE source_root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_source_ranges
             WHERE id IN (
               SELECT source_range_id FROM persistent_journal_root_unregister_ranges
             )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn mark_owned_preview_artifacts_stale(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND artifact_key IN (
                 SELECT artifact_key
                 FROM persistent_journal_root_unregister_preview_artifacts
               )
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn delete_owned_orphan_assets(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM assets
             WHERE id IN (
               SELECT asset_id FROM persistent_journal_root_unregister_assets
             )
               AND NOT EXISTS (
                 SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn enqueue_survivor_intents(
    transaction: &Transaction<'_>,
    intents: Vec<LibraryChangeIntent>,
) -> Result<(), ScanError> {
    if intents.is_empty() {
        return Ok(());
    }
    let enqueued_unix_ms = unix_time_ms();
    let policy = LibraryChangeQueuePolicy::default();
    cleanup_for_enqueue(transaction, enqueued_unix_ms, policy)?;
    let mut by_root = BTreeMap::<(String, u64), Vec<LibraryChangeIntent>>::new();
    for intent in intents {
        by_root
            .entry((intent.root_id.clone(), intent.root_generation.value()))
            .or_default()
            .push(intent);
    }
    for intents in by_root.into_values() {
        let report =
            enqueue_intents_in_transaction(transaction, &intents, None, enqueued_unix_ms, policy)?;
        if report.stale_generation_count != 0 {
            return Err(ScanError::new(
                "change_queue_generation_stale",
                "The surviving root generation changed before its root-removal handoff was durable",
            ));
        }
    }
    Ok(())
}

fn clear_scope(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "DROP TABLE persistent_journal_root_unregister_preview_artifacts;
             DROP TABLE persistent_journal_root_unregister_assets;
             DROP TABLE persistent_journal_root_unregister_survivors;
             DROP TABLE persistent_journal_root_unregister_evidence;
             DROP TABLE persistent_journal_root_unregister_changes;
             DROP TABLE persistent_journal_root_unregister_ranges;",
        )
        .map_err(database_error)
}

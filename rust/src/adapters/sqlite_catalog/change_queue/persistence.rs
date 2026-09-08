use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::domain::{
    DurableLibraryChange, LibraryChangeCapacityDeferral, LibraryChangeCatchUpEvidence,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueueHealth,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryChangeQueueStatus,
    LibraryChangeScope, LibraryRootGeneration, ScanError,
};

use super::super::{database_error, sqlite_integer, sqlite_u32, sqlite_unsigned};

const MAX_CATCH_UP_LINEAGE_PER_CHANGE: usize = 64;

#[derive(Clone, Debug)]
pub(super) struct ActiveChange {
    pub(super) id: LibraryChangeId,
    pub(super) intent: LibraryChangeIntent,
    pub(super) status: LibraryChangeQueueStatus,
    pub(super) catalog_revision_at_enqueue: u64,
    pub(super) catch_up_evidence: Option<LibraryChangeCatchUpEvidence>,
    pub(super) last_failure_code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GenerationDisposition {
    Current { superseded_count: u32 },
    Stale,
}

pub(in crate::adapters::sqlite_catalog) fn activate_root_change_queue(
    transaction: &Transaction<'_>,
    root_id: &str,
    now_unix_ms: i64,
) -> Result<LibraryRootGeneration, ScanError> {
    let stored = transaction
        .query_row(
            "SELECT generation, is_active
             FROM library_change_root_state WHERE root_id = ?1",
            [root_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    let Some((stored_generation, is_active)) = stored else {
        let generation = LibraryRootGeneration::initial();
        transaction
            .execute(
                "INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES (?1, ?2, 1, ?3)",
                params![
                    root_id,
                    sqlite_integer(generation.value(), "root generation")?,
                    now_unix_ms,
                ],
            )
            .map_err(database_error)?;
        seed_persistent_journal_authority(transaction, root_id, generation, now_unix_ms)?;
        return Ok(generation);
    };
    let generation =
        LibraryRootGeneration::new(sqlite_unsigned(stored_generation, "root generation")?)
            .ok_or_else(|| {
                ScanError::new(
                    "change_queue_generation_invalid",
                    "The stored root generation must be nonzero",
                )
            })?;
    if is_active {
        seed_persistent_journal_authority(transaction, root_id, generation, now_unix_ms)?;
        return Ok(generation);
    }
    let next_generation = generation.next().ok_or_else(|| {
        ScanError::new(
            "change_queue_generation_overflow",
            "The root generation cannot advance beyond its supported range",
        )
    })?;
    retire_persistent_journal_authority(transaction, root_id, generation, now_unix_ms)?;
    retire_root_publication_namespace(transaction, root_id, stored_generation)?;
    transaction
        .execute(
            "UPDATE library_change_root_state
             SET generation = ?1, is_active = 1, updated_unix_ms = ?2
             WHERE root_id = ?3 AND generation = ?4 AND is_active = 0",
            params![
                sqlite_integer(next_generation.value(), "root generation")?,
                now_unix_ms,
                root_id,
                stored_generation,
            ],
        )
        .map_err(database_error)?;
    seed_persistent_journal_authority(transaction, root_id, next_generation, now_unix_ms)?;
    Ok(next_generation)
}

fn seed_persistent_journal_authority(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, last_failure_code,
               last_failure_message, updated_unix_ms
             ) VALUES (?1, ?2, 0, 1, 'unknown', 'baseline_required', NULL, NULL, ?3)
             ON CONFLICT(root_id, root_generation) DO NOTHING",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
                updated_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn establish_root_generation(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
    now_unix_ms: i64,
) -> Result<GenerationDisposition, ScanError> {
    let root_is_registered = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
            [root_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !root_is_registered {
        return Ok(GenerationDisposition::Stale);
    }
    let stored = transaction
        .query_row(
            "SELECT generation, is_active
             FROM library_change_root_state WHERE root_id = ?1",
            [root_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    let Some((stored_generation, is_active)) = stored else {
        return Err(ScanError::new(
            "change_queue_generation_missing",
            "The registered root has no durable generation authority",
        ));
    };
    let generation_value = sqlite_integer(generation.value(), "root generation")?;
    if !is_active || generation_value < stored_generation {
        return Ok(GenerationDisposition::Stale);
    }
    if generation_value == stored_generation {
        return Ok(GenerationDisposition::Current {
            superseded_count: 0,
        });
    }
    let previous_generation =
        LibraryRootGeneration::new(sqlite_unsigned(stored_generation, "root generation")?)
            .ok_or_else(|| {
                ScanError::new(
                    "change_queue_generation_invalid",
                    "The stored root generation must be nonzero",
                )
            })?;
    retire_unconsumed_live_gap_claims(transaction, root_id, previous_generation)?;
    let superseded = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND status IN ('pending', 'leased', 'retry_wait')",
            params![now_unix_ms, root_id, stored_generation],
        )
        .map_err(database_error)?;
    retire_persistent_journal_authority(transaction, root_id, previous_generation, now_unix_ms)?;
    retire_root_publication_namespace(transaction, root_id, stored_generation)?;
    transaction
        .execute(
            "UPDATE library_change_root_state
             SET generation = ?1, is_active = 1, updated_unix_ms = ?2
             WHERE root_id = ?3",
            params![generation_value, now_unix_ms, root_id],
        )
        .map_err(database_error)?;
    seed_persistent_journal_authority(transaction, root_id, generation, now_unix_ms)?;
    Ok(GenerationDisposition::Current {
        superseded_count: u32::try_from(superseded).unwrap_or(u32::MAX),
    })
}

pub(super) fn retire_unconsumed_live_gap_claims(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<(), ScanError> {
    let root_generation = sqlite_integer(root_generation.value(), "root generation")?;
    let candidates = {
        let mut statement = transaction
            .prepare(
                "SELECT claim.gap_change_id,
                        CASE WHEN (
                          claim.root_id = ?1 AND claim.root_generation = ?2
                          AND claim.consumed_unix_ms IS NULL
                          AND claim.consumer_kind = 'pending_journal'
                          AND claim.source_range_id IS NULL
                          AND claim.recovery_change_id IS NULL
                          AND claim.foreground_scan_id IS NULL
                          AND claim.opening_volume_guid IS NOT NULL
                          AND claim.opening_volume_serial IS NOT NULL
                          AND claim.opening_root_reference_version IN (2, 3)
                          AND claim.opening_root_file_reference IS NOT NULL
                          AND claim.opening_journal_id IS NOT NULL
                          AND claim.opening_next_usn IS NOT NULL
                          AND claim.protocol_version BETWEEN 1 AND 65535
                          AND claim.contract_version = 1
                          AND gap.root_id = ?1 AND gap.root_generation = ?2
                          AND gap.origin = 'live_notification'
                          AND gap.intent_kind = 'freshness_unknown'
                          AND gap.scope = 'root' AND gap.relative_path = ''
                          AND gap.previous_relative_path IS NULL
                          AND gap.status = 'retry_wait'
                          AND gap.authoritative_scan_id IS NULL
                          AND gap.last_failure_code = 'live_gap_waiting_for_journal_range'
                          AND lane.lane = 'p0_live'
                          AND EXISTS(
                            SELECT 1
                            FROM library_persistent_journal_checkpoints AS checkpoint
                            JOIN library_persistent_journal_root_state AS journal_root
                              ON journal_root.root_id = checkpoint.root_id
                             AND journal_root.root_generation = checkpoint.root_generation
                            WHERE checkpoint.root_id = claim.root_id
                              AND checkpoint.root_generation = claim.root_generation
                              AND checkpoint.volume_guid = claim.opening_volume_guid
                              AND checkpoint.volume_serial = claim.opening_volume_serial
                              AND checkpoint.root_reference_version =
                                    claim.opening_root_reference_version
                              AND checkpoint.root_file_reference =
                                    claim.opening_root_file_reference
                              AND checkpoint.journal_id = claim.opening_journal_id
                              AND checkpoint.next_unread_usn = claim.opening_next_usn
                              AND checkpoint.captured_exclusive_end = claim.opening_next_usn
                              AND checkpoint.protocol_version = claim.protocol_version
                              AND checkpoint.contract_version = claim.contract_version
                              AND checkpoint.continuity_state = 'current'
                              AND checkpoint.last_failure_code IS NULL
                              AND journal_root.capability_state = 'supported'
                              AND journal_root.continuity_state = 'current'
                          )
                        ) OR (
                          claim.root_id = ?1 AND claim.root_generation = ?2
                          AND claim.consumed_unix_ms IS NULL
                          AND claim.consumer_kind = 'explicit_recovery_required'
                          AND claim.opening_volume_guid IS NULL
                          AND claim.opening_volume_serial IS NULL
                          AND claim.opening_root_reference_version IS NULL
                          AND claim.opening_root_file_reference IS NULL
                          AND claim.opening_journal_id IS NULL
                          AND claim.opening_next_usn IS NULL
                          AND claim.protocol_version IS NULL
                          AND claim.contract_version IS NULL
                          AND claim.source_range_id IS NULL
                          AND claim.recovery_change_id IS NULL
                          AND claim.foreground_scan_id IS NULL
                          AND gap.root_id = ?1 AND gap.root_generation = ?2
                          AND gap.origin = 'startup_catch_up'
                          AND gap.intent_kind = 'freshness_unknown'
                          AND gap.scope = 'root' AND gap.relative_path = ''
                          AND gap.previous_relative_path IS NULL
                          AND gap.status = 'retry_wait'
                          AND gap.next_retry_unix_ms IS NULL
                          AND gap.authoritative_scan_id IS NULL
                          AND gap.last_failure_code =
                                'live_gap_v30_explicit_recovery_required'
                          AND lane.lane = 'p1_journal'
                        ) THEN 1 ELSE 0 END AS is_retirable
                 FROM library_live_gap_recovery_claims AS claim
                 LEFT JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
                 LEFT JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 WHERE (
                   claim.root_id = ?1 AND claim.root_generation = ?2
                   AND claim.consumed_unix_ms IS NULL
                   AND claim.consumer_kind IN (
                     'pending_journal', 'explicit_recovery_required', 'foreground_scan'
                   )
                 ) OR (
                   gap.root_id = ?1 AND gap.root_generation = ?2
                   AND gap.status IN ('pending', 'leased', 'retry_wait')
                 )
                 ORDER BY claim.gap_change_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![root_id, root_generation], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
            })
            .map_err(database_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row.map_err(database_error)?);
        }
        candidates
    };
    if candidates.iter().any(|(_, is_retirable)| !is_retirable) {
        return Err(ScanError::new(
            "live_gap_retirement_consumer_conflict",
            "The retired root generation has live-gap ownership that requires explicit abandonment",
        ));
    }
    for (gap_change_id, _) in candidates {
        let deleted = transaction
            .execute(
                "DELETE FROM library_live_gap_recovery_claims
                 WHERE gap_change_id = ?1 AND root_id = ?2 AND root_generation = ?3
                   AND consumed_unix_ms IS NULL
                   AND consumer_kind IN ('pending_journal', 'explicit_recovery_required')",
                params![gap_change_id, root_id, root_generation],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(ScanError::new(
                "live_gap_retirement_consumer_conflict",
                "The retired root generation changed while releasing live-gap ownership",
            ));
        }
    }
    Ok(())
}

fn retire_persistent_journal_authority(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET capability_state = CASE
                   WHEN protocol_version = 0 THEN 'unknown' ELSE 'live_only' END,
                 continuity_state = 'unavailable',
                 last_failure_code = 'root_generation_retired',
                 last_failure_message = 'The root generation was retired',
                 updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3",
            params![
                updated_unix_ms,
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_source_ranges
             SET status = 'superseded', checkpointed_unix_ms = NULL
             WHERE root_id = ?1 AND root_generation = ?2",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_range_lifecycle
             SET lifecycle_state = 'superseded', completed_unix_ms = NULL,
                 updated_unix_ms = ?1
             WHERE source_range_id IN (
               SELECT id FROM library_persistent_journal_source_ranges
               WHERE root_id = ?2 AND root_generation = ?3
             )",
            params![
                updated_unix_ms,
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_cross_root_lineage
             SET status = 'superseded', updated_unix_ms = ?1
             WHERE id IN (
               SELECT owners.lineage_id
               FROM library_persistent_journal_cross_root_ranges AS owners
               JOIN library_persistent_journal_source_ranges AS ranges
                 ON ranges.id = owners.source_range_id
               WHERE ranges.root_id = ?2 AND ranges.root_generation = ?3
             )",
            params![
                updated_unix_ms,
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn retire_root_publication_namespace(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM library_scan_publication_namespace_bindings
             WHERE root_id = ?1 AND root_generation = ?2",
            params![root_id, generation],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_root_publication_namespaces
             WHERE root_id = ?1 AND root_generation = ?2",
            params![root_id, generation],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn root_generation_is_current(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
) -> Result<bool, ScanError> {
    transaction
        .query_row(
            "SELECT generation = ?1 AND is_active = 1
             FROM library_change_root_state WHERE root_id = ?2",
            params![
                sqlite_integer(generation.value(), "root generation")?,
                root_id,
            ],
            |row| row.get(0),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(database_error)
}

pub(super) fn recover_expired_leases(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    selection: i64,
) -> Result<(), ScanError> {
    let expired = {
        let mut statement = transaction
            .prepare(
                "SELECT id, attempt_count, last_failure_code FROM library_change_queue
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND status = 'leased' AND lease_expires_unix_ms <= ?3
                   AND (
                     ?4 = 0
                     OR (?4 = 1 AND scope = 'path' AND intent_kind <> 'freshness_unknown')
                     OR (?4 = 2 AND (scope <> 'path' OR intent_kind = 'freshness_unknown'))
                     OR (?4 BETWEEN 3 AND 5 AND scope = 'path'
                       AND intent_kind <> 'freshness_unknown'
                       AND EXISTS(
                         SELECT 1 FROM library_change_queue_lanes AS lanes
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = CASE ?4
                             WHEN 3 THEN 'p0_live'
                             WHEN 4 THEN 'p1_journal'
                             ELSE 'p2_recovery'
                           END
                       ))
                     OR (?4 = 6
                       AND (scope <> 'path' OR intent_kind = 'freshness_unknown')
                       AND EXISTS(
                         SELECT 1
                         FROM library_change_queue_lanes AS lanes
                         JOIN library_recovery_authorities AS authority
                           ON authority.change_id = lanes.change_id
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = 'p2_recovery'
                           AND authority.root_id = library_change_queue.root_id
                           AND authority.root_generation = library_change_queue.root_generation
                           AND authority.retired_unix_ms IS NULL
                           AND authority.run_id <> ''
                       ))
                     OR (?4 = 7
                       AND scope = 'path'
                       AND intent_kind <> 'freshness_unknown'
                       AND EXISTS(
                         SELECT 1
                         FROM library_change_queue_lanes AS lanes
                         JOIN library_metadata_inventory_candidate_owners AS owner
                           ON owner.change_id = lanes.change_id
                         JOIN library_recovery_authorities AS authority
                           ON authority.run_id = owner.run_id
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = 'p2_recovery'
                           AND authority.root_id = library_change_queue.root_id
                           AND authority.root_generation = library_change_queue.root_generation
                           AND authority.retired_unix_ms IS NULL
                       ))
                     OR (?4 = 8
                       AND (scope <> 'path' OR intent_kind = 'freshness_unknown')
                       AND EXISTS(
                         SELECT 1 FROM library_change_queue_lanes AS lanes
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = 'p0_live'
                       )
                       AND NOT EXISTS(
                         SELECT 1
                         FROM library_live_gap_recovery_claims AS claim
                         WHERE claim.gap_change_id = library_change_queue.id
                           AND claim.consumer_kind = 'pending_journal'
                       ))
                     OR (?4 = 9
                       AND scope = 'path'
                       AND intent_kind <> 'freshness_unknown'
                       AND EXISTS(
                         SELECT 1 FROM library_change_queue_lanes AS lanes
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = 'p2_recovery'
                       )
                       AND NOT EXISTS(
                         SELECT 1
                         FROM library_metadata_inventory_candidate_owners AS owner
                         WHERE owner.change_id = library_change_queue.id
                       )
                       AND NOT EXISTS(
                         SELECT 1 FROM library_recovery_authorities AS authority
                         WHERE authority.change_id = library_change_queue.id
                           AND authority.retired_unix_ms IS NULL
                       ))
                   )
                 ORDER BY lease_expires_unix_ms, id
                 LIMIT ?5",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    now_unix_ms,
                    selection,
                    i64::from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES),
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row.map_err(database_error)?);
        }
        values
    };
    for (change_id, attempt_count, last_failure_code) in expired {
        let change_id = change_id_from_sqlite(change_id)?;
        if last_failure_code.as_deref()
            == Some(LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code())
            && is_typed_capacity_deferred_live_gap(transaction, change_id)?
        {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'retry_wait', attempt_count = CASE
                           WHEN attempt_count > 0 THEN attempt_count - 1 ELSE 0 END,
                         next_retry_unix_ms = ?1, lease_expires_unix_ms = NULL,
                         last_failure_code = ?2, last_failure_message = ?3,
                         updated_unix_ms = ?4
                     WHERE id = ?5 AND status = 'leased' AND lease_expires_unix_ms <= ?4",
                    params![
                        capacity_deferral_deadline(now_unix_ms, policy),
                        LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
                        LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_message(),
                        now_unix_ms,
                        sqlite_integer(change_id.value(), "change ID")?,
                    ],
                )
                .map_err(database_error)?;
            continue;
        }
        let next_retry_unix_ms = next_retry_deadline(
            now_unix_ms,
            sqlite_u32(attempt_count, "change attempt count")?,
            policy,
        );
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', next_retry_unix_ms = ?1,
                     lease_expires_unix_ms = NULL,
                     last_failure_code = 'change_lease_expired',
                     last_failure_message = 'The prior worker did not finish before its lease expired.',
                     updated_unix_ms = ?2
                 WHERE id = ?3 AND status = 'leased' AND lease_expires_unix_ms <= ?2",
                params![
                    next_retry_unix_ms,
                    now_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn enforce_retry_attempt_limit(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    let candidates = {
        let mut statement = transaction
            .prepare(
                "SELECT id
                 FROM library_change_queue
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND status = 'retry_wait' AND attempt_count >= ?3
                   AND next_retry_unix_ms IS NOT NULL
                 ORDER BY id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(change_id_from_sqlite(row.map_err(database_error)?)?);
        }
        candidates
    };
    for change_id in candidates {
        if is_typed_capacity_deferred_live_gap(transaction, change_id)? {
            continue;
        }
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET next_retry_unix_ms = NULL, updated_unix_ms = ?1
                 WHERE id = ?2 AND status = 'retry_wait' AND attempt_count >= ?3
                   AND next_retry_unix_ms IS NOT NULL",
                params![
                    now_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                    i64::from(policy.max_attempts),
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn is_typed_capacity_deferred_live_gap(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_change_queue AS queue
               JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
               WHERE queue.id = ?1
                 AND queue.status IN ('leased', 'retry_wait')
                 AND queue.origin = 'live_notification'
                 AND queue.intent_kind = 'freshness_unknown'
                 AND queue.scope = 'root'
                 AND queue.relative_path = ''
                 AND queue.previous_relative_path IS NULL
                 AND queue.last_failure_code = ?2
                 AND lane.lane = 'p0_live'
                 AND NOT EXISTS(
                   SELECT 1 FROM library_live_gap_recovery_claims AS claim
                   WHERE claim.gap_change_id = queue.id
                 )
             )",
            params![
                sqlite_integer(change_id.value(), "change ID")?,
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
            ],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog) fn classify_lease_update(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    lease_generation: u64,
    catalog_revision_at_success: Option<u64>,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    let stored = transaction
        .query_row(
            "SELECT status, lease_generation, catalog_revision_at_enqueue
             FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((status, stored_lease_generation, enqueue_revision)) = stored else {
        return Ok(LibraryChangeLeaseUpdateOutcome::Missing);
    };
    if status == "superseded" {
        return Ok(LibraryChangeLeaseUpdateOutcome::Superseded);
    }
    if status != "leased"
        || sqlite_unsigned(stored_lease_generation, "lease generation")? != lease_generation
    {
        return Ok(LibraryChangeLeaseUpdateOutcome::LeaseMismatch);
    }
    if let Some(success_revision) = catalog_revision_at_success
        && success_revision < sqlite_unsigned(enqueue_revision, "enqueue catalog revision")?
    {
        return Err(ScanError::new(
            "change_queue_publication_revision_stale",
            "A completed change cannot publish an older catalog revision than it observed",
        ));
    }
    Ok(LibraryChangeLeaseUpdateOutcome::Applied)
}

pub(super) fn next_retry_deadline(
    failed_unix_ms: i64,
    attempt_count: u32,
    policy: LibraryChangeQueuePolicy,
) -> Option<i64> {
    if attempt_count >= policy.max_attempts {
        return None;
    }
    let exponent = attempt_count.saturating_sub(1).min(63);
    let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
    let delay = policy
        .retry_initial_delay_millis
        .saturating_mul(multiplier)
        .min(policy.retry_maximum_delay_millis);
    Some(failed_unix_ms.saturating_add(i64::try_from(delay).unwrap_or(i64::MAX)))
}

pub(super) fn capacity_deferral_deadline(
    deferred_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> i64 {
    deferred_unix_ms
        .saturating_add(i64::try_from(policy.retry_initial_delay_millis).unwrap_or(i64::MAX))
}

pub(super) fn load_active_changes(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    _configured_limit: u32,
) -> Result<Vec<ActiveChange>, ScanError> {
    let mut statement = transaction
        .prepare(
            "SELECT id, intent_kind, scope, relative_path, previous_relative_path, origin,
                    first_observed_unix_ms, most_recent_observed_unix_ms,
                    first_sequence, most_recent_sequence, coalesced_observation_count,
                    status, catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
                    last_failure_code
             FROM library_change_queue
             WHERE root_id = ?1 AND root_generation = ?2
               AND status IN ('pending', 'leased', 'retry_wait')
             ORDER BY id
             LIMIT ?3",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                i64::from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES) + 1,
            ],
            read_active_change_row,
        )
        .map_err(database_error)?;
    let mut changes = Vec::new();
    for row in rows {
        changes.push(active_change_from_raw(
            root_id,
            root_generation,
            row.map_err(database_error)?,
        )?);
    }
    if changes.len()
        > usize::try_from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES).unwrap_or(usize::MAX)
    {
        return Err(ScanError::new(
            "change_queue_capacity_corrupt",
            "The durable change queue exceeds its absolute unresolved-work bound",
        ));
    }
    Ok(changes)
}

pub(super) fn load_leased_inventory_control_ids(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Vec<LibraryChangeId>, ScanError> {
    let mut statement = transaction
        .prepare_cached(
            "SELECT id
             FROM library_change_queue
             WHERE root_id = ?1 AND root_generation = ?2
               AND status = 'leased' AND authoritative_scan_id IS NULL
               AND (
                 intent_kind = 'freshness_unknown'
                 OR origin IN ('startup_catch_up', 'consistency_audit')
                 OR last_failure_code = 'metadata_inventory_required'
               )
             ORDER BY id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(change_id_from_sqlite(row.map_err(database_error)?)?);
    }
    Ok(ids)
}

struct RawActiveChange {
    id: i64,
    kind: String,
    scope: String,
    relative_path: String,
    previous_relative_path: Option<String>,
    origin: String,
    first_observed_unix_ms: i64,
    most_recent_observed_unix_ms: i64,
    first_sequence: String,
    most_recent_sequence: String,
    coalesced_observation_count: i64,
    status: String,
    catalog_revision_at_enqueue: i64,
    catch_up_source: Option<String>,
    catch_up_watermark: Option<String>,
    last_failure_code: Option<String>,
}

fn read_active_change_row(row: &Row<'_>) -> rusqlite::Result<RawActiveChange> {
    Ok(RawActiveChange {
        id: row.get(0)?,
        kind: row.get(1)?,
        scope: row.get(2)?,
        relative_path: row.get(3)?,
        previous_relative_path: row.get(4)?,
        origin: row.get(5)?,
        first_observed_unix_ms: row.get(6)?,
        most_recent_observed_unix_ms: row.get(7)?,
        first_sequence: row.get(8)?,
        most_recent_sequence: row.get(9)?,
        coalesced_observation_count: row.get(10)?,
        status: row.get(11)?,
        catalog_revision_at_enqueue: row.get(12)?,
        catch_up_source: row.get(13)?,
        catch_up_watermark: row.get(14)?,
        last_failure_code: row.get(15)?,
    })
}

fn active_change_from_raw(
    root_id: &str,
    root_generation: LibraryRootGeneration,
    raw: RawActiveChange,
) -> Result<ActiveChange, ScanError> {
    let catch_up_evidence = match (raw.catch_up_source, raw.catch_up_watermark) {
        (Some(source), Some(watermark)) => Some(LibraryChangeCatchUpEvidence { source, watermark }),
        (None, None) => None,
        _ => {
            return Err(ScanError::new(
                "change_queue_catch_up_evidence_invalid",
                "The stored catch-up evidence is incomplete",
            ));
        }
    };
    Ok(ActiveChange {
        id: change_id_from_sqlite(raw.id)?,
        intent: LibraryChangeIntent {
            root_id: root_id.to_owned(),
            root_generation,
            kind: intent_kind_from_db(&raw.kind)?,
            scope: scope_from_db(&raw.scope)?,
            relative_path: raw.relative_path,
            previous_relative_path: raw.previous_relative_path,
            origin: origin_from_db(&raw.origin)?,
            first_observed_unix_ms: raw.first_observed_unix_ms,
            most_recent_observed_unix_ms: raw.most_recent_observed_unix_ms,
            first_sequence: parse_sequence(&raw.first_sequence)?,
            most_recent_sequence: parse_sequence(&raw.most_recent_sequence)?,
            coalesced_observation_count: sqlite_u32(
                raw.coalesced_observation_count,
                "coalesced observation count",
            )?,
        },
        status: status_from_db(&raw.status)?,
        catalog_revision_at_enqueue: sqlite_unsigned(
            raw.catalog_revision_at_enqueue,
            "enqueue catalog revision",
        )?,
        catch_up_evidence,
        last_failure_code: raw.last_failure_code,
    })
}

pub(super) fn insert_change(
    transaction: &Transaction<'_>,
    intent: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    catalog_revision: u64,
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeId, ScanError> {
    let ready_unix_ms = stabilization_deadline(intent, enqueued_unix_ms, policy);
    transaction
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms,
               catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
               'pending', ?13, ?14, ?15, ?16, ?17, ?17
             )",
            params![
                intent.root_id,
                sqlite_integer(intent.root_generation.value(), "root generation")?,
                intent_kind_to_db(intent.kind),
                scope_to_db(intent.scope),
                intent.relative_path,
                intent.previous_relative_path,
                origin_to_db(intent.origin),
                intent.first_observed_unix_ms,
                intent.most_recent_observed_unix_ms,
                intent.first_sequence.to_string(),
                intent.most_recent_sequence.to_string(),
                i64::from(intent.coalesced_observation_count),
                ready_unix_ms,
                sqlite_integer(catalog_revision, "catalog revision")?,
                evidence.map(|value| value.source.as_str()),
                evidence.map(|value| value.watermark.as_str()),
                enqueued_unix_ms,
            ],
        )
        .map_err(database_error)?;
    let change_id = change_id_from_sqlite(transaction.last_insert_rowid())?;
    if let Some(evidence) = evidence {
        attach_catch_up_lineage(transaction, change_id, evidence, enqueued_unix_ms)?;
    }
    Ok(change_id)
}

pub(super) fn update_change(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    intent: &LibraryChangeIntent,
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE library_change_queue
             SET intent_kind = ?1, scope = ?2, relative_path = ?3,
                 previous_relative_path = ?4, origin = ?5,
                 first_observed_unix_ms = ?6, most_recent_observed_unix_ms = ?7,
                 first_sequence = ?8, most_recent_sequence = ?9,
                 coalesced_observation_count = ?10, status = 'pending',
                 ready_unix_ms = ?11, attempt_count = 0, next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 catch_up_source = COALESCE(?12, catch_up_source),
                 catch_up_watermark = COALESCE(?13, catch_up_watermark),
                 updated_unix_ms = ?14
             WHERE id = ?15 AND status IN ('pending', 'retry_wait')",
            params![
                intent_kind_to_db(intent.kind),
                scope_to_db(intent.scope),
                intent.relative_path,
                intent.previous_relative_path,
                origin_to_db(intent.origin),
                intent.first_observed_unix_ms,
                intent.most_recent_observed_unix_ms,
                intent.first_sequence.to_string(),
                intent.most_recent_sequence.to_string(),
                i64::from(intent.coalesced_observation_count),
                stabilization_deadline(intent, enqueued_unix_ms, policy),
                evidence.map(|value| value.source.as_str()),
                evidence.map(|value| value.watermark.as_str()),
                enqueued_unix_ms,
                sqlite_integer(change_id.value(), "change ID")?,
            ],
        )
        .map_err(database_error)?;
    if let Some(evidence) = evidence {
        attach_catch_up_lineage(transaction, change_id, evidence, enqueued_unix_ms)?;
    }
    Ok(())
}

pub(super) fn transfer_catch_up_lineage(
    transaction: &Transaction<'_>,
    source_ids: impl IntoIterator<Item = LibraryChangeId>,
    target_id: LibraryChangeId,
) -> Result<(), ScanError> {
    for source_id in source_ids {
        if source_id == target_id {
            continue;
        }
        transaction
            .execute(
                "INSERT OR IGNORE INTO library_change_queue_catch_up_lineage(
                   change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 )
                 SELECT ?1, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 FROM library_change_queue_catch_up_lineage WHERE change_id = ?2",
                params![
                    sqlite_integer(target_id.value(), "target change ID")?,
                    sqlite_integer(source_id.value(), "source change ID")?,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_persistent_journal_queue_lineage(
                   source_range_id, change_id, enrolled_unix_ms
                 )
                 SELECT source_range_id, ?1, enrolled_unix_ms
                 FROM library_persistent_journal_queue_lineage WHERE change_id = ?2
                 ON CONFLICT(source_range_id, change_id) DO UPDATE SET
                   enrolled_unix_ms = MIN(
                     library_persistent_journal_queue_lineage.enrolled_unix_ms,
                     excluded.enrolled_unix_ms
                   )",
                params![
                    sqlite_integer(target_id.value(), "target change ID")?,
                    sqlite_integer(source_id.value(), "source change ID")?,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM library_persistent_journal_queue_lineage WHERE change_id = ?1",
                [sqlite_integer(source_id.value(), "source change ID")?],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM library_change_queue_catch_up_lineage WHERE change_id = ?1",
                [sqlite_integer(source_id.value(), "source change ID")?],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET catch_up_source = NULL, catch_up_watermark = NULL
                 WHERE id = ?1",
                [sqlite_integer(source_id.value(), "source change ID")?],
            )
            .map_err(database_error)?;
    }
    validate_catch_up_lineage_bound(transaction, target_id)
}

fn attach_catch_up_lineage(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    evidence: &LibraryChangeCatchUpEvidence,
    enrolled_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                sqlite_integer(change_id.value(), "change ID")?,
                evidence.source,
                evidence.watermark,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    validate_catch_up_lineage_bound(transaction, change_id)
}

fn validate_catch_up_lineage_bound(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
) -> Result<(), ScanError> {
    let count = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue_catch_up_lineage WHERE change_id = ?1",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if usize::try_from(count).unwrap_or(usize::MAX) > MAX_CATCH_UP_LINEAGE_PER_CHANGE {
        Err(ScanError::new(
            "change_queue_catch_up_lineage_limit_exceeded",
            "One durable change accumulated too many unresolved catch-up watermarks",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn mark_superseded(
    transaction: &Transaction<'_>,
    change_ids: impl IntoIterator<Item = LibraryChangeId>,
    superseded_by: Option<LibraryChangeId>,
    now_unix_ms: i64,
) -> Result<u32, ScanError> {
    let mut superseded_count = 0_u32;
    for change_id in change_ids {
        if superseded_by == Some(change_id) {
            continue;
        }
        let updated = transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'superseded', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, superseded_by_change_id = ?1,
                     updated_unix_ms = ?2
                 WHERE id = ?3 AND status IN ('pending', 'leased', 'retry_wait')",
                params![
                    superseded_by
                        .map(|id| sqlite_integer(id.value(), "superseding change ID"))
                        .transpose()?,
                    now_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                ],
            )
            .map_err(database_error)?;
        if updated != 0
            && let Some(target_id) = superseded_by
        {
            transaction
                .execute(
                    "UPDATE library_metadata_inventory_candidate_owners AS owner
                     SET change_id = ?1
                     WHERE owner.change_id = ?2
                       AND EXISTS(
                         SELECT 1 FROM library_change_queue AS target
                         WHERE target.id = ?1 AND target.scope = 'path'
                           AND target.relative_path = owner.relative_path
                           AND target.previous_relative_path IS owner.previous_relative_path
                       )",
                    params![
                        sqlite_integer(target_id.value(), "superseding change ID")?,
                        sqlite_integer(change_id.value(), "change ID")?,
                    ],
                )
                .map_err(database_error)?;
        }
        superseded_count = superseded_count.saturating_add(u32::try_from(updated).unwrap_or(0));
    }
    Ok(superseded_count)
}

fn stabilization_deadline(
    intent: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> i64 {
    enqueued_unix_ms
        .max(intent.most_recent_observed_unix_ms)
        .saturating_add(i64::try_from(policy.debounce_millis).unwrap_or(i64::MAX))
}

struct RawDurableChange {
    active: RawActiveChange,
    ready_unix_ms: i64,
    attempt_count: i64,
    next_retry_unix_ms: Option<i64>,
    lease_generation: i64,
    lease_expires_unix_ms: Option<i64>,
    last_failure_code: Option<String>,
    last_failure_message: Option<String>,
    catalog_revision_at_success: Option<i64>,
    catch_up_source: Option<String>,
    catch_up_watermark: Option<String>,
    superseded_by_change_id: Option<i64>,
    root_id: String,
    root_generation: i64,
}

pub(super) fn load_change(
    transaction: &Transaction<'_>,
    change_id: i64,
) -> Result<DurableLibraryChange, ScanError> {
    let raw = transaction
        .query_row(
            "SELECT id, intent_kind, scope, relative_path, previous_relative_path, origin,
                    first_observed_unix_ms, most_recent_observed_unix_ms,
                    first_sequence, most_recent_sequence, coalesced_observation_count,
                    status, catalog_revision_at_enqueue, ready_unix_ms, attempt_count,
                    next_retry_unix_ms, lease_generation, lease_expires_unix_ms,
                    last_failure_code, last_failure_message, catalog_revision_at_success,
                    catch_up_source, catch_up_watermark, superseded_by_change_id,
                    root_id, root_generation
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            read_durable_change_row,
        )
        .map_err(database_error)?;
    let catch_up_lineage = load_catch_up_lineage(transaction, change_id)?;
    durable_change_from_raw(raw, catch_up_lineage)
}

fn load_catch_up_lineage(
    transaction: &Transaction<'_>,
    change_id: i64,
) -> Result<Vec<LibraryChangeCatchUpEvidence>, ScanError> {
    let mut statement = transaction
        .prepare_cached(
            "SELECT catch_up_source, catch_up_watermark
             FROM library_change_queue_catch_up_lineage
             WHERE change_id = ?1
             ORDER BY enrolled_unix_ms DESC, catch_up_source, catch_up_watermark
             LIMIT ?2",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                change_id,
                i64::try_from(MAX_CATCH_UP_LINEAGE_PER_CHANGE + 1).unwrap_or(i64::MAX),
            ],
            |row| {
                Ok(LibraryChangeCatchUpEvidence {
                    source: row.get(0)?,
                    watermark: row.get(1)?,
                })
            },
        )
        .map_err(database_error)?;
    let mut lineage = Vec::new();
    for row in rows {
        lineage.push(row.map_err(database_error)?);
    }
    if lineage.len() > MAX_CATCH_UP_LINEAGE_PER_CHANGE {
        return Err(ScanError::new(
            "change_queue_catch_up_lineage_limit_exceeded",
            "One durable change exceeds the bounded catch-up watermark lineage",
        ));
    }
    Ok(lineage)
}

fn read_durable_change_row(row: &Row<'_>) -> rusqlite::Result<RawDurableChange> {
    Ok(RawDurableChange {
        active: RawActiveChange {
            id: row.get(0)?,
            kind: row.get(1)?,
            scope: row.get(2)?,
            relative_path: row.get(3)?,
            previous_relative_path: row.get(4)?,
            origin: row.get(5)?,
            first_observed_unix_ms: row.get(6)?,
            most_recent_observed_unix_ms: row.get(7)?,
            first_sequence: row.get(8)?,
            most_recent_sequence: row.get(9)?,
            coalesced_observation_count: row.get(10)?,
            status: row.get(11)?,
            catalog_revision_at_enqueue: row.get(12)?,
            catch_up_source: row.get(21)?,
            catch_up_watermark: row.get(22)?,
            last_failure_code: row.get(18)?,
        },
        ready_unix_ms: row.get(13)?,
        attempt_count: row.get(14)?,
        next_retry_unix_ms: row.get(15)?,
        lease_generation: row.get(16)?,
        lease_expires_unix_ms: row.get(17)?,
        last_failure_code: row.get(18)?,
        last_failure_message: row.get(19)?,
        catalog_revision_at_success: row.get(20)?,
        catch_up_source: row.get(21)?,
        catch_up_watermark: row.get(22)?,
        superseded_by_change_id: row.get(23)?,
        root_id: row.get(24)?,
        root_generation: row.get(25)?,
    })
}

fn durable_change_from_raw(
    raw: RawDurableChange,
    catch_up_lineage: Vec<LibraryChangeCatchUpEvidence>,
) -> Result<DurableLibraryChange, ScanError> {
    let root_generation =
        LibraryRootGeneration::new(sqlite_unsigned(raw.root_generation, "root generation")?)
            .ok_or_else(|| {
                ScanError::new(
                    "change_queue_generation_invalid",
                    "The stored root generation must be nonzero",
                )
            })?;
    let active = active_change_from_raw(&raw.root_id, root_generation, raw.active)?;
    let last_failure = match (raw.last_failure_code, raw.last_failure_message) {
        (Some(code), Some(message)) => Some(LibraryChangeFailure { code, message }),
        (None, None) => None,
        _ => {
            return Err(ScanError::new(
                "change_queue_failure_invalid",
                "The stored change failure evidence is incomplete",
            ));
        }
    };
    let primary_evidence = match (&raw.catch_up_source, &raw.catch_up_watermark) {
        (Some(source), Some(watermark)) => Some(LibraryChangeCatchUpEvidence {
            source: source.clone(),
            watermark: watermark.clone(),
        }),
        (None, None) => None,
        _ => {
            return Err(ScanError::new(
                "change_queue_catch_up_evidence_invalid",
                "The stored catch-up evidence is incomplete",
            ));
        }
    };
    if primary_evidence
        .as_ref()
        .is_some_and(|evidence| !catch_up_lineage.contains(evidence))
        || (primary_evidence.is_none() && !catch_up_lineage.is_empty())
    {
        return Err(ScanError::new(
            "change_queue_catch_up_lineage_invalid",
            "The durable catch-up watermark lineage does not include its primary evidence",
        ));
    }
    Ok(DurableLibraryChange {
        id: active.id,
        intent: active.intent,
        status: active.status,
        ready_unix_ms: raw.ready_unix_ms,
        attempt_count: sqlite_u32(raw.attempt_count, "change attempt count")?,
        next_retry_unix_ms: raw.next_retry_unix_ms,
        lease_generation: sqlite_unsigned(raw.lease_generation, "lease generation")?,
        lease_expires_unix_ms: raw.lease_expires_unix_ms,
        last_failure,
        catalog_revision_at_enqueue: active.catalog_revision_at_enqueue,
        catalog_revision_at_success: raw
            .catalog_revision_at_success
            .map(|value| sqlite_unsigned(value, "successful catalog revision"))
            .transpose()?,
        catch_up_source: raw.catch_up_source,
        catch_up_watermark: raw.catch_up_watermark,
        catch_up_lineage,
        superseded_by_change_id: raw
            .superseded_by_change_id
            .map(change_id_from_sqlite)
            .transpose()?,
    })
}

pub(super) fn load_metrics(
    connection: &Connection,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    load_filtered_metrics(connection, None, None, now_unix_ms, policy)
}

pub(super) fn load_root_metrics(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    load_filtered_metrics(
        connection,
        Some(root_id),
        Some(root_generation),
        now_unix_ms,
        policy,
    )
}

fn load_filtered_metrics(
    connection: &Connection,
    root_id: Option<&str>,
    root_generation: Option<LibraryRootGeneration>,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    let (
        pending,
        leased,
        retry_wait,
        completed,
        superseded,
        ready,
        expired,
        exhausted,
        freshness_unknown,
        explicit_recovery_required,
        oldest_due,
        latest_exhausted_failure_code,
    ) = connection
        .query_row(
            "SELECT
               COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'leased' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'retry_wait' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'superseded' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN
                 (status = 'pending' AND ready_unix_ms <= ?1 AND attempt_count < ?2)
                 OR
                 (status = 'retry_wait' AND next_retry_unix_ms IS NOT NULL
                   AND next_retry_unix_ms <= ?1 AND attempt_count < ?2)
                 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'leased' AND lease_expires_unix_ms <= ?1
                 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'retry_wait' AND attempt_count >= ?2
                 AND NOT (
                   COALESCE(last_failure_code = ?5, 0)
                   AND origin = 'live_notification'
                   AND intent_kind = 'freshness_unknown'
                   AND scope = 'root'
                   AND relative_path = ''
                   AND previous_relative_path IS NULL
                   AND EXISTS(
                     SELECT 1 FROM library_change_queue_lanes AS capacity_lane
                     WHERE capacity_lane.change_id = library_change_queue.id
                       AND capacity_lane.lane = 'p0_live'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS capacity_claim
                     WHERE capacity_claim.gap_change_id = library_change_queue.id
                   )
                 ) THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN intent_kind = 'freshness_unknown'
                  AND status IN ('pending', 'leased', 'retry_wait') THEN 1 ELSE 0 END), 0),
               (SELECT COUNT(*)
                FROM library_live_gap_recovery_claims AS claim
                JOIN library_change_queue AS claimed_gap
                  ON claimed_gap.id = claim.gap_change_id
                WHERE claim.consumer_kind = 'explicit_recovery_required'
                  AND (?3 IS NULL OR (
                    claimed_gap.root_id = ?3
                    AND claimed_gap.root_generation = ?4
                  ))),
               MIN(CASE
                 WHEN status = 'pending' AND ready_unix_ms <= ?1 AND attempt_count < ?2
                   THEN ready_unix_ms
                 WHEN status = 'retry_wait' AND next_retry_unix_ms <= ?1
                   AND attempt_count < ?2
                   THEN next_retry_unix_ms
                 WHEN status = 'leased' AND lease_expires_unix_ms <= ?1
                   THEN lease_expires_unix_ms
                 ELSE NULL END),
               (SELECT exhausted_change.last_failure_code
                FROM library_change_queue AS exhausted_change
                WHERE (?3 IS NULL OR (
                  exhausted_change.root_id = ?3
                  AND exhausted_change.root_generation = ?4
                ))
                  AND exhausted_change.status = 'retry_wait'
                  AND exhausted_change.attempt_count >= ?2
                  AND NOT (
                    exhausted_change.last_failure_code = ?5
                    AND exhausted_change.origin = 'live_notification'
                    AND exhausted_change.intent_kind = 'freshness_unknown'
                    AND exhausted_change.scope = 'root'
                    AND exhausted_change.relative_path = ''
                    AND exhausted_change.previous_relative_path IS NULL
                    AND EXISTS(
                      SELECT 1 FROM library_change_queue_lanes AS capacity_lane
                      WHERE capacity_lane.change_id = exhausted_change.id
                        AND capacity_lane.lane = 'p0_live'
                    )
                    AND NOT EXISTS(
                      SELECT 1 FROM library_live_gap_recovery_claims AS capacity_claim
                      WHERE capacity_claim.gap_change_id = exhausted_change.id
                    )
                  )
                  AND exhausted_change.last_failure_code IS NOT NULL
                ORDER BY exhausted_change.id DESC
                LIMIT 1)
             FROM library_change_queue
             WHERE (?3 IS NULL OR (root_id = ?3 AND root_generation = ?4))",
            params![
                now_unix_ms,
                i64::from(policy.max_attempts),
                root_id,
                root_generation
                    .map(|generation| sqlite_integer(generation.value(), "root generation"))
                    .transpose()?,
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )
        .map_err(database_error)?;
    let pending_count = sqlite_unsigned(pending, "pending change count")?;
    let leased_count = sqlite_unsigned(leased, "leased change count")?;
    let retry_wait_count = sqlite_unsigned(retry_wait, "retry-wait change count")?;
    let ready_count = sqlite_unsigned(ready, "ready change count")?;
    let expired_lease_count = sqlite_unsigned(expired, "expired lease count")?;
    let exhausted_retry_count = sqlite_unsigned(exhausted, "exhausted retry count")?;
    let explicit_recovery_required_count =
        sqlite_unsigned(explicit_recovery_required, "explicit recovery claim count")?;
    let unresolved_count = pending_count
        .saturating_add(leased_count)
        .saturating_add(retry_wait_count);
    let oldest_ready_delay_millis = oldest_due
        .and_then(|due| now_unix_ms.checked_sub(due))
        .and_then(|delay| u64::try_from(delay).ok())
        .unwrap_or(0);
    let health = if unresolved_count == 0 {
        LibraryChangeQueueHealth::Idle
    } else if expired_lease_count > 0
        || exhausted_retry_count > 0
        || explicit_recovery_required_count > 0
    {
        LibraryChangeQueueHealth::Degraded
    } else if ready_count > 0 && oldest_ready_delay_millis > 0 {
        LibraryChangeQueueHealth::Delayed
    } else {
        LibraryChangeQueueHealth::Healthy
    };
    Ok(LibraryChangeQueueMetrics {
        health,
        pending_count,
        leased_count,
        retry_wait_count,
        completed_count: sqlite_unsigned(completed, "completed change count")?,
        superseded_count: sqlite_unsigned(superseded, "superseded change count")?,
        ready_count,
        expired_lease_count,
        exhausted_retry_count,
        latest_exhausted_failure_code,
        freshness_unknown_count: sqlite_unsigned(
            freshness_unknown,
            "freshness-unknown change count",
        )?,
        explicit_recovery_required_count,
        oldest_ready_delay_millis,
    })
}

fn change_id_from_sqlite(value: i64) -> Result<LibraryChangeId, ScanError> {
    LibraryChangeId::new(sqlite_unsigned(value, "change ID")?).ok_or_else(|| {
        ScanError::new(
            "change_queue_id_invalid",
            "The stored durable change ID must be nonzero",
        )
    })
}

fn parse_sequence(value: &str) -> Result<u64, ScanError> {
    value.parse::<u64>().map_err(|_| {
        ScanError::new(
            "change_queue_sequence_invalid",
            "The stored observation sequence is outside the supported range",
        )
    })
}

fn intent_kind_to_db(kind: LibraryChangeIntentKind) -> &'static str {
    match kind {
        LibraryChangeIntentKind::Reconcile => "reconcile",
        LibraryChangeIntentKind::RenameCandidate => "rename_candidate",
        LibraryChangeIntentKind::FreshnessUnknown => "freshness_unknown",
    }
}

fn intent_kind_from_db(value: &str) -> Result<LibraryChangeIntentKind, ScanError> {
    match value {
        "reconcile" => Ok(LibraryChangeIntentKind::Reconcile),
        "rename_candidate" => Ok(LibraryChangeIntentKind::RenameCandidate),
        "freshness_unknown" => Ok(LibraryChangeIntentKind::FreshnessUnknown),
        _ => Err(invalid_enum("intent kind", value)),
    }
}

fn scope_to_db(scope: LibraryChangeScope) -> &'static str {
    match scope {
        LibraryChangeScope::Path => "path",
        LibraryChangeScope::Subtree => "subtree",
        LibraryChangeScope::Root => "root",
    }
}

fn scope_from_db(value: &str) -> Result<LibraryChangeScope, ScanError> {
    match value {
        "path" => Ok(LibraryChangeScope::Path),
        "subtree" => Ok(LibraryChangeScope::Subtree),
        "root" => Ok(LibraryChangeScope::Root),
        _ => Err(invalid_enum("scope", value)),
    }
}

fn origin_to_db(origin: LibraryChangeOrigin) -> &'static str {
    match origin {
        LibraryChangeOrigin::LiveNotification => "live_notification",
        LibraryChangeOrigin::MetadataInventory => "metadata_inventory",
        LibraryChangeOrigin::StartupCatchUp => "startup_catch_up",
        LibraryChangeOrigin::UserRefresh => "user_refresh",
        LibraryChangeOrigin::ConsistencyAudit => "consistency_audit",
    }
}

fn origin_from_db(value: &str) -> Result<LibraryChangeOrigin, ScanError> {
    match value {
        "live_notification" => Ok(LibraryChangeOrigin::LiveNotification),
        "metadata_inventory" => Ok(LibraryChangeOrigin::MetadataInventory),
        "startup_catch_up" => Ok(LibraryChangeOrigin::StartupCatchUp),
        "user_refresh" => Ok(LibraryChangeOrigin::UserRefresh),
        "consistency_audit" => Ok(LibraryChangeOrigin::ConsistencyAudit),
        _ => Err(invalid_enum("origin", value)),
    }
}

fn status_from_db(value: &str) -> Result<LibraryChangeQueueStatus, ScanError> {
    match value {
        "pending" => Ok(LibraryChangeQueueStatus::Pending),
        "leased" => Ok(LibraryChangeQueueStatus::Leased),
        "retry_wait" => Ok(LibraryChangeQueueStatus::RetryWait),
        "completed" => Ok(LibraryChangeQueueStatus::Completed),
        "superseded" => Ok(LibraryChangeQueueStatus::Superseded),
        _ => Err(invalid_enum("status", value)),
    }
}

fn invalid_enum(field: &str, value: &str) -> ScanError {
    ScanError::new(
        "change_queue_value_invalid",
        format!("The stored change queue {field} is invalid: {value}"),
    )
}

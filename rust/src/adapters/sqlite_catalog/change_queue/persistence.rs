use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::domain::{
    DurableLibraryChange, LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin,
    LibraryChangeQueueHealth, LibraryChangeQueueMetrics, LibraryChangeQueuePolicy,
    LibraryChangeQueueStatus, LibraryChangeScope, LibraryRootGeneration, ScanError,
};

use super::super::{database_error, sqlite_integer, sqlite_u32, sqlite_unsigned};

#[derive(Clone, Debug)]
pub(super) struct ActiveChange {
    pub(super) id: LibraryChangeId,
    pub(super) intent: LibraryChangeIntent,
    pub(super) status: LibraryChangeQueueStatus,
    pub(super) catalog_revision_at_enqueue: u64,
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
        return Ok(generation);
    }
    let next_generation = generation.next().ok_or_else(|| {
        ScanError::new(
            "change_queue_generation_overflow",
            "The root generation cannot advance beyond its supported range",
        )
    })?;
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
    Ok(next_generation)
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
    let superseded = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = ?1
             WHERE root_id = ?2 AND status IN ('pending', 'leased', 'retry_wait')",
            params![now_unix_ms, root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_root_state
             SET generation = ?1, is_active = 1, updated_unix_ms = ?2
             WHERE root_id = ?3",
            params![generation_value, now_unix_ms, root_id],
        )
        .map_err(database_error)?;
    Ok(GenerationDisposition::Current {
        superseded_count: u32::try_from(superseded).unwrap_or(u32::MAX),
    })
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
) -> Result<(), ScanError> {
    let expired = {
        let mut statement = transaction
            .prepare(
                "SELECT id, attempt_count FROM library_change_queue
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND status = 'leased' AND lease_expires_unix_ms <= ?3
                 ORDER BY lease_expires_unix_ms, id
                 LIMIT ?4",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    now_unix_ms,
                    i64::from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES),
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(database_error)?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row.map_err(database_error)?);
        }
        values
    };
    for (change_id, attempt_count) in expired {
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
                params![next_retry_unix_ms, now_unix_ms, change_id],
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
    transaction
        .execute(
            "UPDATE library_change_queue
             SET next_retry_unix_ms = NULL, updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND status = 'retry_wait' AND attempt_count >= ?4
               AND next_retry_unix_ms IS NOT NULL",
            params![
                now_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                i64::from(policy.max_attempts),
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn classify_lease_update(
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
                    status, catalog_revision_at_enqueue
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
    })
}

fn active_change_from_raw(
    root_id: &str,
    root_generation: LibraryRootGeneration,
    raw: RawActiveChange,
) -> Result<ActiveChange, ScanError> {
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
    })
}

pub(super) fn insert_change(
    transaction: &Transaction<'_>,
    intent: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    catalog_revision: u64,
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
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
               'pending', ?13, ?14, ?15, ?15
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
                enqueued_unix_ms,
            ],
        )
        .map_err(database_error)?;
    change_id_from_sqlite(transaction.last_insert_rowid())
}

pub(super) fn update_change(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    intent: &LibraryChangeIntent,
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
                 updated_unix_ms = ?12
             WHERE id = ?13 AND status IN ('pending', 'retry_wait')",
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
                enqueued_unix_ms,
                sqlite_integer(change_id.value(), "change ID")?,
            ],
        )
        .map_err(database_error)?;
    Ok(())
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
    durable_change_from_raw(raw)
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

fn durable_change_from_raw(raw: RawDurableChange) -> Result<DurableLibraryChange, ScanError> {
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
        oldest_due,
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
                 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN intent_kind = 'freshness_unknown'
                 AND status IN ('pending', 'leased', 'retry_wait') THEN 1 ELSE 0 END), 0),
               MIN(CASE
                 WHEN status = 'pending' AND ready_unix_ms <= ?1 AND attempt_count < ?2
                   THEN ready_unix_ms
                 WHEN status = 'retry_wait' AND next_retry_unix_ms <= ?1
                   AND attempt_count < ?2
                   THEN next_retry_unix_ms
                 WHEN status = 'leased' AND lease_expires_unix_ms <= ?1
                   THEN lease_expires_unix_ms
                 ELSE NULL END)
             FROM library_change_queue",
            params![now_unix_ms, i64::from(policy.max_attempts)],
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
                    row.get::<_, Option<i64>>(9)?,
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
    let unresolved_count = pending_count
        .saturating_add(leased_count)
        .saturating_add(retry_wait_count);
    let oldest_ready_delay_millis = oldest_due
        .and_then(|due| now_unix_ms.checked_sub(due))
        .and_then(|delay| u64::try_from(delay).ok())
        .unwrap_or(0);
    let health = if unresolved_count == 0 {
        LibraryChangeQueueHealth::Idle
    } else if expired_lease_count > 0 || exhausted_retry_count > 0 {
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
        freshness_unknown_count: sqlite_unsigned(
            freshness_unknown,
            "freshness-unknown change count",
        )?,
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
        LibraryChangeOrigin::StartupCatchUp => "startup_catch_up",
        LibraryChangeOrigin::UserRefresh => "user_refresh",
        LibraryChangeOrigin::ConsistencyAudit => "consistency_audit",
    }
}

fn origin_from_db(value: &str) -> Result<LibraryChangeOrigin, ScanError> {
    match value {
        "live_notification" => Ok(LibraryChangeOrigin::LiveNotification),
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

pub(super) fn cleanup_terminal_records(
    connection: &Connection,
    terminal_before_unix_ms: i64,
    limit: u32,
) -> Result<u32, ScanError> {
    if limit == 0 || limit > LibraryChangeQueuePolicy::MAX_CLEANUP_BATCH {
        return Err(ScanError::new(
            "change_queue_cleanup_limit_invalid",
            "The terminal change cleanup batch exceeds its absolute bound",
        ));
    }
    let deleted_changes = connection
        .execute(
            "DELETE FROM library_change_queue
             WHERE id IN (
               SELECT id FROM library_change_queue
               WHERE status IN ('completed', 'superseded') AND updated_unix_ms <= ?1
               ORDER BY updated_unix_ms, id
               LIMIT ?2
             )",
            params![terminal_before_unix_ms, i64::from(limit)],
        )
        .map_err(database_error)?;
    let deleted_changes = u32::try_from(deleted_changes).map_err(|_| {
        ScanError::new(
            "change_queue_cleanup_count_invalid",
            "The terminal change cleanup count exceeds its supported range",
        )
    })?;
    Ok(deleted_changes)
}

pub(in crate::adapters::sqlite_catalog) fn retire_root_change_queue(
    transaction: &Transaction<'_>,
    root_id: &str,
    now_unix_ms: i64,
) -> Result<(), ScanError> {
    let current_generation = transaction
        .query_row(
            "SELECT generation FROM library_change_root_state WHERE root_id = ?1",
            [root_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(current_generation) = current_generation {
        transaction
            .execute(
                "UPDATE library_change_root_state
                 SET is_active = 0, updated_unix_ms = ?1 WHERE root_id = ?2",
                params![now_unix_ms, root_id],
            )
            .map_err(database_error)?;
        if current_generation <= 0 {
            return Err(ScanError::new(
                "change_queue_generation_invalid",
                "The retired root has an invalid stored generation",
            ));
        }
    } else {
        return Err(ScanError::new(
            "change_queue_generation_missing",
            "The registered root has no durable generation authority to retire",
        ));
    }
    transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = ?1
             WHERE root_id = ?2 AND status IN ('pending', 'leased', 'retry_wait')",
            params![now_unix_ms, root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

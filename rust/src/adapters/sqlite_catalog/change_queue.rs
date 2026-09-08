use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

#[cfg(test)]
use crate::domain::LibraryChangeCatchUpQueueBatch;
use crate::domain::{
    FileIdentityEvidence, LeasedLibraryChange, LibraryChangeCapacityDeferral,
    LibraryChangeCatchUpEvidence, LibraryChangeEnqueueReport, LibraryChangeFailure,
    LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueueMetrics,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthority,
    LibraryRecoveryAuthorityReason, LibraryRecoveryOpeningBoundary, LibraryRootGeneration,
    PersistentJournalEnrollmentBatch, ScanError,
};
use crate::ports::LibraryChangeQueue;

use super::{
    SqliteCatalog, database_error, load_catalog_revision, sqlite_integer, sqlite_u32,
    sqlite_unsigned,
};

mod coalescing;
mod ingress;
mod lease_deferral;
mod metrics;
mod persistence;
pub(super) mod root_retirement;
mod terminal_cleanup;

use super::metadata_inventory::insert_metadata_inventory_recovery_authority;
pub(super) use coalescing::normalize_persistent_journal_intents;
use coalescing::{
    EnqueueContext, active_lane_counts, enqueue_one, lane_admission_allows, merge_newer_evidence,
    merge_older_evidence, queue_backpressure, validate_failure, validate_intent_batch,
    validate_policy, validate_root_id,
};
use metrics::{load_metrics, load_root_metrics};
pub(super) use persistence::activate_root_change_queue;
pub(super) use persistence::classify_lease_update;
use persistence::{
    GenerationDisposition, capacity_deferral_deadline, enforce_retry_attempt_limit,
    establish_root_generation, insert_change, load_active_changes, load_change,
    load_leased_inventory_control_ids, mark_superseded, next_retry_deadline,
    recover_expired_leases, root_generation_is_current, transfer_catch_up_lineage, update_change,
};
pub(super) use root_retirement::retire_root_change_queue;
use terminal_cleanup::cleanup_terminal_records;

pub(super) const PERSISTENT_JOURNAL_CATCH_UP_SOURCE: &str = "persistent_journal_v1";

fn metadata_inventory_recovery_affinity_change_id(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<i64>, ScanError> {
    let root_generation = sqlite_integer(root_generation.value(), "root generation")?;
    let baseline_change_ids = {
        let mut statement = connection
            .prepare(
                "SELECT baseline.change_id
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = baseline.change_id
                 WHERE baseline.root_id = ?1 AND baseline.root_generation = ?2
                   AND baseline.phase <> 'completed'
                   AND authority.reason <> 'first_import_boundary'
                 ORDER BY baseline.change_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![root_id, root_generation], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(database_error)?;
        let mut change_ids = Vec::new();
        for row in rows {
            change_ids.push(row.map_err(database_error)?);
        }
        change_ids
    };
    if baseline_change_ids.len() > 1 {
        return Err(metadata_inventory_recovery_affinity_corrupt());
    }
    let baseline_change_id = baseline_change_ids
        .first()
        .copied()
        .map(|change_id| {
            validate_metadata_inventory_recovery_affinity_owner(
                connection,
                root_id,
                root_generation,
                change_id,
                None,
            )
        })
        .transpose()?;

    let active_run_ids = {
        let mut statement = connection
            .prepare(
                "SELECT id
                 FROM library_metadata_inventory_runs
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND status IN ('running', 'comparing')
                 ORDER BY id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![root_id, root_generation], |row| {
                row.get::<_, String>(0)
            })
            .map_err(database_error)?;
        let mut run_ids = Vec::new();
        for row in rows {
            run_ids.push(row.map_err(database_error)?);
        }
        run_ids
    };
    if active_run_ids.len() > 1 {
        return Err(metadata_inventory_recovery_affinity_corrupt());
    }
    let active_change_id = active_run_ids
        .first()
        .map(|run_id| {
            let authority_change_ids = {
                let mut statement = connection
                    .prepare(
                        "SELECT change_id
                         FROM library_recovery_authorities
                         WHERE run_id = ?1
                         ORDER BY change_id",
                    )
                    .map_err(database_error)?;
                let rows = statement
                    .query_map([run_id], |row| row.get::<_, i64>(0))
                    .map_err(database_error)?;
                let mut change_ids = Vec::new();
                for row in rows {
                    change_ids.push(row.map_err(database_error)?);
                }
                change_ids
            };
            if authority_change_ids.len() != 1 {
                return Err(metadata_inventory_recovery_affinity_corrupt());
            }
            validate_metadata_inventory_recovery_affinity_owner(
                connection,
                root_id,
                root_generation,
                authority_change_ids[0],
                Some(run_id.as_str()),
            )
        })
        .transpose()?;

    if baseline_change_id.is_some()
        && active_change_id.is_some()
        && baseline_change_id != active_change_id
    {
        return Err(metadata_inventory_recovery_affinity_corrupt());
    }
    Ok(baseline_change_id.or(active_change_id))
}

fn validate_metadata_inventory_recovery_affinity_owner(
    connection: &Connection,
    root_id: &str,
    root_generation: i64,
    change_id: i64,
    expected_run_id: Option<&str>,
) -> Result<i64, ScanError> {
    sqlite_unsigned(change_id, "metadata inventory affinity change ID")?;
    let owner = connection
        .query_row(
            "SELECT authority.run_id, authority.root_id, authority.root_generation,
                    authority.retired_unix_ms,
                    queue.root_id, queue.root_generation, queue.status,
                    (SELECT COUNT(*) FROM library_change_queue_lanes AS lanes
                     WHERE lanes.change_id = authority.change_id),
                    (SELECT COUNT(*) FROM library_change_queue_lanes AS lanes
                     WHERE lanes.change_id = authority.change_id
                       AND lanes.lane = 'p2_recovery'),
                    (SELECT run.status FROM library_metadata_inventory_runs AS run
                     WHERE run.id = authority.run_id)
             FROM library_recovery_authorities AS authority
             LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
             WHERE authority.change_id = ?1",
            [change_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(metadata_inventory_recovery_affinity_corrupt)?;
    let (
        authority_run_id,
        authority_root_id,
        authority_root_generation,
        retired_unix_ms,
        queue_root_id,
        queue_root_generation,
        queue_status,
        lane_count,
        recovery_lane_count,
        run_status,
    ) = owner;
    if expected_run_id.is_some_and(|expected| expected != authority_run_id)
        || authority_root_id != root_id
        || authority_root_generation != root_generation
        || retired_unix_ms.is_some()
        || queue_root_id.as_deref() != Some(root_id)
        || queue_root_generation != Some(root_generation)
        || !matches!(
            queue_status.as_deref(),
            Some("pending" | "leased" | "retry_wait")
        )
        || lane_count != 1
        || recovery_lane_count != 1
        || run_status
            .as_deref()
            .is_some_and(|status| !matches!(status, "running" | "comparing"))
    {
        return Err(metadata_inventory_recovery_affinity_corrupt());
    }
    Ok(change_id)
}

fn metadata_inventory_recovery_affinity_corrupt() -> ScanError {
    ScanError::new(
        "metadata_inventory_recovery_affinity_corrupt",
        "The catalog cannot prove the exact metadata recovery authority owner",
    )
}

pub(super) fn wake_metadata_inventory_capacity_deferrals(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    released_unix_ms: i64,
) -> Result<u32, ScanError> {
    let updated = transaction
        .execute(
            "UPDATE library_change_queue
             SET next_retry_unix_ms = ?1, updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND status = 'retry_wait'
               AND last_failure_code = ?4
               AND intent_kind = 'freshness_unknown' AND scope = 'root'
               AND relative_path = '' AND previous_relative_path IS NULL
               AND origin = 'live_notification'
               AND EXISTS(
                 SELECT 1 FROM library_change_queue_lanes AS lane
                 WHERE lane.change_id = library_change_queue.id
                   AND lane.lane = 'p0_live'
               )
               AND NOT EXISTS(
                 SELECT 1 FROM library_live_gap_recovery_claims AS claim
                 WHERE claim.gap_change_id = library_change_queue.id
               )
               AND (next_retry_unix_ms IS NULL OR next_retry_unix_ms > ?1)",
            params![
                released_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
            ],
        )
        .map_err(database_error)?;
    u32::try_from(updated).map_err(|_| {
        ScanError::new(
            "change_queue_capacity_wake_count_invalid",
            "The number of woken capacity deferrals exceeds the supported range",
        )
    })
}

pub(super) fn insert_persistent_journal_recovery_control(
    transaction: &Transaction<'_>,
    intent: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeId, ScanError> {
    validate_policy(policy)?;
    validate_intent_batch(std::slice::from_ref(intent), intent)?;
    if intent.origin != LibraryChangeOrigin::MetadataInventory
        || intent.kind != crate::domain::LibraryChangeIntentKind::FreshnessUnknown
        || intent.scope != LibraryChangeScope::Root
        || !intent.relative_path.is_empty()
        || intent.previous_relative_path.is_some()
    {
        return Err(ScanError::new(
            "persistent_journal_recovery_control_invalid",
            "A journal continuity failure requires an independent root-scoped P2 control",
        ));
    }
    let active = load_active_changes(
        transaction,
        &intent.root_id,
        intent.root_generation,
        policy.max_unresolved_changes,
    )?;
    let admitted_counts = active_lane_counts(&active).adding(LibraryChangeLane::Recovery, 1);
    if !lane_admission_allows(policy, LibraryChangeLane::Recovery, admitted_counts) {
        return Err(queue_backpressure());
    }
    let change_id = insert_change(
        transaction,
        intent,
        enqueued_unix_ms,
        load_catalog_revision(transaction)?,
        None,
        policy,
    )?;
    transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', superseded_by_change_id = ?1,
                 next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                 updated_unix_ms = ?2
             WHERE root_id = ?3 AND root_generation = ?4 AND id <> ?1
               AND intent_kind = 'freshness_unknown' AND scope = 'root'
               AND relative_path = '' AND previous_relative_path IS NULL
               AND status = 'retry_wait' AND attempt_count >= ?5
               AND last_failure_code = 'legacy_recovery_authority_missing'
               AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_candidate_owners AS owner
                 WHERE owner.change_id = library_change_queue.id
               )
               AND NOT EXISTS(
                 SELECT 1 FROM library_recovery_authorities AS authority
                 WHERE authority.change_id = library_change_queue.id
                   AND authority.retired_unix_ms IS NULL
               )",
            params![
                sqlite_integer(change_id.value(), "recovery control change ID")?,
                enqueued_unix_ms,
                intent.root_id,
                sqlite_integer(intent.root_generation.value(), "root generation")?,
                i64::from(policy.max_attempts),
            ],
        )
        .map_err(database_error)?;
    Ok(change_id)
}

fn insert_pending_live_gap_journal_claim(
    transaction: &Transaction<'_>,
    gap_change_id: LibraryChangeId,
    intent: &LibraryChangeIntent,
    boundary: &LibraryRecoveryOpeningBoundary,
    created_unix_ms: i64,
) -> Result<(), ScanError> {
    boundary.validate()?;
    transaction
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               opening_volume_guid, opening_volume_serial,
               opening_root_reference_version, opening_root_file_reference,
               opening_journal_id, opening_next_usn, protocol_version,
               contract_version, created_unix_ms
             ) VALUES (
               ?1, ?2, ?3, 'pending_journal', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
             )",
            params![
                sqlite_integer(gap_change_id.value(), "live gap change ID")?,
                intent.root_id,
                sqlite_integer(intent.root_generation.value(), "root generation")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
                created_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn insert_live_gap_metadata_recovery_claim(
    transaction: &Transaction<'_>,
    gap_change_id: LibraryChangeId,
    intent: &LibraryChangeIntent,
    recovery_change_id: LibraryChangeId,
    consumed_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               recovery_change_id, created_unix_ms, consumed_unix_ms
             ) VALUES (?1, ?2, ?3, 'metadata_inventory_control', ?4, ?5, ?5)",
            params![
                sqlite_integer(gap_change_id.value(), "live gap change ID")?,
                intent.root_id,
                sqlite_integer(intent.root_generation.value(), "root generation")?,
                sqlite_integer(recovery_change_id.value(), "recovery change ID")?,
                consumed_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn consume_live_gap_claims_with_source_range(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
    consumed_unix_ms: i64,
) -> Result<u32, ScanError> {
    let gap_change_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT claim.gap_change_id
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN library_persistent_journal_checkpoints AS checkpoint
                   ON checkpoint.root_id = claim.root_id
                  AND checkpoint.root_generation = claim.root_generation
                  AND checkpoint.volume_guid = claim.opening_volume_guid
                  AND checkpoint.volume_serial = claim.opening_volume_serial
                  AND checkpoint.root_reference_version =
                        claim.opening_root_reference_version
                  AND checkpoint.root_file_reference = claim.opening_root_file_reference
                  AND checkpoint.journal_id = claim.opening_journal_id
                  AND checkpoint.next_unread_usn = claim.opening_next_usn
                  AND checkpoint.captured_exclusive_end = claim.opening_next_usn
                  AND checkpoint.protocol_version = claim.protocol_version
                  AND checkpoint.contract_version = claim.contract_version
                  AND checkpoint.continuity_state = 'current'
                  AND checkpoint.last_failure_code IS NULL
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = checkpoint.root_id
                  AND root.root_generation = checkpoint.root_generation
                  AND root.capability_state = 'supported'
                  AND root.continuity_state = 'current'
                 WHERE claim.consumer_kind = 'pending_journal'
                   AND claim.root_id = ?1 AND claim.root_generation = ?2
                   AND claim.opening_volume_guid = ?3
                   AND claim.opening_volume_serial = ?4
                   AND claim.opening_journal_id = ?5
                   AND claim.opening_next_usn = ?6
                   AND claim.protocol_version = ?7 AND claim.contract_version = ?8
                 ORDER BY claim.gap_change_id",
            )
            .map_err(database_error)?;
        statement
            .query_map(
                params![
                    batch.range.root_id,
                    sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                    batch.range.volume.volume_guid,
                    batch.range.volume.canonical_serial(),
                    batch.range.journal_id.to_canonical_text(),
                    batch.range.requested_start_usn.to_canonical_text(),
                    i64::from(batch.range.protocol_version),
                    i64::from(batch.range.contract_version),
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    let mut consumed = 0_u32;
    for stored_change_id in gap_change_ids {
        let gap_change_id =
            LibraryChangeId::new(u64::try_from(stored_change_id).map_err(|_| {
                ScanError::new(
                    "library_change_id_invalid",
                    "The live gap change ID is invalid",
                )
            })?)
            .ok_or_else(|| {
                ScanError::new(
                    "library_change_id_invalid",
                    "The live gap change ID is invalid",
                )
            })?;
        let updated = transaction
            .execute(
                "UPDATE library_live_gap_recovery_claims
                 SET consumer_kind = 'journal_source_range', source_range_id = ?1,
                     consumed_unix_ms = ?2
                 WHERE gap_change_id = ?3 AND consumer_kind = 'pending_journal'",
                params![batch.range.batch_id, consumed_unix_ms, stored_change_id,],
            )
            .map_err(database_error)?;
        if updated != 1
            || mark_superseded(transaction, [gap_change_id], None, consumed_unix_ms)? != 1
        {
            return Err(ScanError::new(
                "live_gap_journal_consumer_conflict",
                "The P0 live gap changed before its P1 source range became durable",
            ));
        }
        consumed = consumed.saturating_add(1);
    }
    Ok(consumed)
}

pub(super) fn transfer_pending_live_gap_claims_to_recovery(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    recovery_change_id: LibraryChangeId,
    consumed_unix_ms: i64,
) -> Result<u32, ScanError> {
    let authority_is_allowlisted = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_recovery_authorities
               WHERE change_id = ?1 AND root_id = ?2 AND root_generation = ?3
                 AND retired_unix_ms IS NULL
                 AND reason IN (
                   'journal_gap', 'journal_reset', 'journal_trim',
                   'journal_reconstruction_failure', 'containment_failure',
                   'broker_after_current_failure', 'watcher_uncovered_gap'
                 )
             )",
            params![
                sqlite_integer(recovery_change_id.value(), "recovery change ID")?,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !authority_is_allowlisted {
        return Err(ScanError::new(
            "live_gap_recovery_authority_invalid",
            "A pending live gap requires an allowlisted independent P2 recovery authority",
        ));
    }
    let stored_change_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT gap_change_id
                 FROM library_live_gap_recovery_claims
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND consumer_kind = 'pending_journal'
                 ORDER BY gap_change_id",
            )
            .map_err(database_error)?;
        statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    let mut consumed = 0_u32;
    for stored_change_id in stored_change_ids {
        let gap_change_id =
            LibraryChangeId::new(u64::try_from(stored_change_id).map_err(|_| {
                ScanError::new(
                    "library_change_id_invalid",
                    "The live gap change ID is invalid",
                )
            })?)
            .ok_or_else(|| {
                ScanError::new(
                    "library_change_id_invalid",
                    "The live gap change ID is invalid",
                )
            })?;
        transfer_catch_up_lineage(transaction, [gap_change_id], recovery_change_id)?;
        let updated = transaction
            .execute(
                "UPDATE library_live_gap_recovery_claims
                 SET consumer_kind = 'metadata_inventory_control',
                     source_range_id = NULL, recovery_change_id = ?1,
                     consumed_unix_ms = ?2
                 WHERE gap_change_id = ?3 AND consumer_kind = 'pending_journal'",
                params![
                    sqlite_integer(recovery_change_id.value(), "recovery change ID")?,
                    consumed_unix_ms,
                    stored_change_id,
                ],
            )
            .map_err(database_error)?;
        if updated != 1
            || mark_superseded(
                transaction,
                [gap_change_id],
                Some(recovery_change_id),
                consumed_unix_ms,
            )? != 1
        {
            return Err(ScanError::new(
                "live_gap_recovery_consumer_conflict",
                "The P0 live gap changed before its P2 recovery ownership became durable",
            ));
        }
        consumed = consumed.saturating_add(1);
    }
    Ok(consumed)
}

impl LibraryChangeQueue for SqliteCatalog {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        enqueue_intents(self, intents, None, enqueued_unix_ms, policy)
    }

    fn enqueue_metadata_inventory_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LibraryChangeEnqueueReport>, ScanError> {
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let result = enqueue_metadata_inventory_candidates_in_transaction(
            &transaction,
            authority,
            intents,
            enqueued_unix_ms,
            policy,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(result.map(|(report, _)| report))
    }

    fn load_metadata_inventory_candidate_root_identity(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        validate_root_id(root_id)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT spool.root_identity_scheme, spool.root_identity_value
                 FROM library_metadata_inventory_candidate_owners AS owner
                 JOIN library_change_queue AS queue ON queue.id = owner.change_id
                 JOIN library_metadata_inventory_spools AS spool ON spool.run_id = owner.run_id
                 JOIN library_recovery_authorities AS authority
                   ON authority.run_id = owner.run_id
                 WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                   AND queue.status IN ('pending', 'leased', 'retry_wait')
                   AND authority.retired_unix_ms IS NULL
                 LIMIT 2",
            )
            .map_err(database_error)?;
        let identities = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| {
                    Ok(FileIdentityEvidence {
                        scheme: row.get(0)?,
                        value: row.get(1)?,
                    })
                },
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        match identities.as_slice() {
            [] => Ok(None),
            [identity] => Ok(Some(identity.clone())),
            _ => Err(ScanError::new(
                "metadata_inventory_candidate_root_identity_conflict",
                "Owned recovery candidates do not share one durable pinned-root identity",
            )),
        }
    }

    #[cfg(test)]
    fn enqueue_library_change_intents_with_catch_up(
        &mut self,
        intents: &[LibraryChangeIntent],
        evidence: &LibraryChangeCatchUpEvidence,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        validate_catch_up_evidence(evidence)?;
        enqueue_intents(self, intents, Some(evidence), enqueued_unix_ms, policy)
    }

    #[cfg(test)]
    fn enqueue_library_change_catch_up_batches(
        &mut self,
        batches: &[LibraryChangeCatchUpQueueBatch],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LibraryChangeEnqueueReport>, ScanError> {
        validate_policy(policy)?;
        for batch in batches {
            if let Some(evidence) = &batch.evidence {
                validate_catch_up_evidence(evidence)?;
            }
            validate_enqueue_batch(&batch.intents)?;
        }
        if batches.is_empty() {
            return Ok(Vec::new());
        }
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Journal)?;
        cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
        let mut reports = Vec::with_capacity(batches.len());
        for batch in batches {
            reports.push(enqueue_intents_in_transaction(
                &transaction,
                &batch.intents,
                batch.evidence.as_ref(),
                enqueued_unix_ms,
                policy,
            )?);
        }
        transaction.commit().map_err(database_error)?;
        Ok(reports)
    }

    fn lease_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::All,
        )
    }

    fn lease_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::Path,
        )
    }

    fn lease_path_library_changes_in_lane(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        lane: LibraryChangeLane,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::PathLane(lane),
        )
    }

    fn lease_metadata_inventory_recovery_candidates(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::RecoveryCandidates,
        )
    }

    fn lease_unowned_recovery_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::UnownedRecoveryPaths,
        )
    }

    fn lease_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError> {
        let mut leased = self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::Authoritative,
        )?;
        Ok(leased.pop())
    }

    fn complete_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let lane = admission_lane_for_change_ids(&self.connection, [change_id])?;
        let transaction = self.begin_write_in_lane(lane)?;
        let outcome = classify_lease_update(
            &transaction,
            change_id,
            lease_generation,
            Some(catalog_revision_at_success),
        )?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
            let completed_change = load_change(
                &transaction,
                sqlite_integer(change_id.value(), "change ID")?,
            )?;
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'completed', lease_expires_unix_ms = NULL,
                         catalog_revision_at_success = ?1, updated_unix_ms = ?2
                     WHERE id = ?3 AND status = 'leased' AND lease_generation = ?4",
                    params![
                        sqlite_integer(catalog_revision_at_success, "catalog revision")?,
                        completed_unix_ms,
                        sqlite_integer(change_id.value(), "change ID")?,
                        sqlite_integer(lease_generation, "lease generation")?,
                    ],
                )
                .map_err(database_error)?;
            if matches!(
                lane,
                LibraryChangeLane::Journal | LibraryChangeLane::Recovery
            ) {
                wake_metadata_inventory_capacity_deferrals(
                    &transaction,
                    &completed_change.intent.root_id,
                    completed_change.intent.root_generation,
                    completed_unix_ms,
                )?;
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(outcome)
    }

    fn retry_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        failed_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        validate_policy(policy)?;
        validate_failure(failure)?;
        let lane = admission_lane_for_change_ids(&self.connection, [change_id])?;
        let transaction = self.begin_write_in_lane(lane)?;
        let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
            retry_leased_change_in_transaction(
                &transaction,
                change_id,
                lease_generation,
                failure,
                failed_unix_ms,
                policy,
            )?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(outcome)
    }

    fn promote_live_watcher_gap_to_metadata_inventory(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        promoted_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        validate_policy(policy)?;
        validate_failure(failure)?;
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Live)?;
        let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
        if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
            transaction.commit().map_err(database_error)?;
            return Ok(outcome);
        }

        let current = load_change(
            &transaction,
            sqlite_integer(change_id.value(), "change ID")?,
        )?;
        if current.intent.origin != LibraryChangeOrigin::LiveNotification
            || current.intent.scope == LibraryChangeScope::Path
            || failure.code != "metadata_inventory_required"
        {
            return Err(ScanError::new(
                "change_queue_recovery_promotion_invalid",
                "Only a proven bounded P0 watcher gap can create P2 metadata recovery",
            ));
        }
        let is_root_live_gap = current.intent.kind == LibraryChangeIntentKind::FreshnessUnknown
            && current.intent.scope == LibraryChangeScope::Root
            && current.intent.relative_path.is_empty()
            && current.intent.previous_relative_path.is_none();
        if is_root_live_gap {
            match super::persistent_journal::load_current_recovery_opening_boundary(
                &transaction,
                &current.intent.root_id,
                current.intent.root_generation,
            ) {
                Ok(opening_boundary) => {
                    insert_pending_live_gap_journal_claim(
                        &transaction,
                        change_id,
                        &current.intent,
                        &opening_boundary,
                        promoted_unix_ms,
                    )?;
                    retry_leased_change_in_transaction(
                        &transaction,
                        change_id,
                        lease_generation,
                        &LibraryChangeFailure {
                            code: "live_gap_waiting_for_journal_range".to_owned(),
                            message: "The P0 live gap is durably waiting for continuous P1 journal ownership"
                                .to_owned(),
                        },
                        promoted_unix_ms,
                        policy,
                    )?;
                    transaction.commit().map_err(database_error)?;
                    return Ok(LibraryChangeLeaseUpdateOutcome::Applied);
                }
                Err(error) if error.code == "persistent_journal_recovery_boundary_unavailable" => {
                    let mut recovery_intent = current.intent.clone();
                    recovery_intent.origin = LibraryChangeOrigin::MetadataInventory;
                    let recovery_change_id = insert_persistent_journal_recovery_control(
                        &transaction,
                        &recovery_intent,
                        promoted_unix_ms,
                        policy,
                    )?;
                    transaction
                        .execute(
                            "UPDATE library_change_queue
                             SET last_failure_code = ?1, last_failure_message = ?2,
                                 updated_unix_ms = ?3
                             WHERE id = ?4 AND status = 'pending'",
                            params![
                                failure.code,
                                failure.message,
                                promoted_unix_ms,
                                sqlite_integer(recovery_change_id.value(), "recovery change ID")?,
                            ],
                        )
                        .map_err(database_error)?;
                    transfer_catch_up_lineage(&transaction, [change_id], recovery_change_id)?;
                    insert_metadata_inventory_recovery_authority(
                        &transaction,
                        &LibraryRecoveryAuthority {
                            change_id: recovery_change_id,
                            run_id: format!("watcher-gap-promotion-{}", recovery_change_id.value()),
                            root_id: recovery_intent.root_id.clone(),
                            root_generation: recovery_intent.root_generation,
                            reason: LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
                            opening_boundary: None,
                            authorized_unix_ms: promoted_unix_ms,
                            retired_unix_ms: None,
                        },
                    )?;
                    insert_live_gap_metadata_recovery_claim(
                        &transaction,
                        change_id,
                        &current.intent,
                        recovery_change_id,
                        promoted_unix_ms,
                    )?;
                    transaction
                        .execute(
                            "UPDATE library_change_queue
                             SET last_failure_code = ?1, last_failure_message = ?2,
                                 updated_unix_ms = ?3
                             WHERE id = ?4 AND status = 'leased'
                               AND lease_generation = ?5",
                            params![
                                failure.code,
                                failure.message,
                                promoted_unix_ms,
                                sqlite_integer(change_id.value(), "change ID")?,
                                sqlite_integer(lease_generation, "lease generation")?,
                            ],
                        )
                        .map_err(database_error)?;
                    if mark_superseded(
                        &transaction,
                        [change_id],
                        Some(recovery_change_id),
                        promoted_unix_ms,
                    )? != 1
                    {
                        return Err(ScanError::new(
                            "change_queue_recovery_promotion_conflict",
                            "The P0 live gap changed before P2 recovery could be published",
                        ));
                    }
                    transaction.commit().map_err(database_error)?;
                    return Ok(LibraryChangeLeaseUpdateOutcome::Applied);
                }
                Err(error) => return Err(error),
            }
        }
        let opening_boundary =
            match super::persistent_journal::load_current_recovery_opening_boundary(
                &transaction,
                &current.intent.root_id,
                current.intent.root_generation,
            ) {
                Ok(boundary) => boundary,
                Err(error) if error.code == "persistent_journal_recovery_boundary_unavailable" => {
                    super::persistent_journal::mark_persistent_journal_recovery_required(
                        &transaction,
                        &current.intent.root_id,
                        current.intent.root_generation,
                        &error.code,
                        &error.message,
                        promoted_unix_ms,
                    )?;
                    retry_leased_change_in_transaction(
                        &transaction,
                        change_id,
                        lease_generation,
                        failure,
                        promoted_unix_ms,
                        policy,
                    )?;
                    transaction.commit().map_err(database_error)?;
                    return Ok(LibraryChangeLeaseUpdateOutcome::Applied);
                }
                Err(error) => return Err(error),
            };
        let active = load_active_changes(
            &transaction,
            &current.intent.root_id,
            current.intent.root_generation,
            policy.max_unresolved_changes,
        )?;
        let remaining = active
            .iter()
            .filter(|change| change.id != change_id)
            .cloned()
            .collect::<Vec<_>>();
        let admitted_counts = active_lane_counts(&remaining).adding(LibraryChangeLane::Recovery, 1);
        if !lane_admission_allows(policy, LibraryChangeLane::Recovery, admitted_counts) {
            return Err(queue_backpressure());
        }

        let mut recovery_intent = current.intent.clone();
        recovery_intent.origin = LibraryChangeOrigin::MetadataInventory;
        validate_intent_batch(std::slice::from_ref(&recovery_intent), &recovery_intent)?;
        let recovery_change_id = insert_change(
            &transaction,
            &recovery_intent,
            promoted_unix_ms,
            current.catalog_revision_at_enqueue,
            None,
            policy,
        )?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET last_failure_code = ?1, last_failure_message = ?2,
                     updated_unix_ms = ?3
                 WHERE id = ?4 AND status = 'pending'",
                params![
                    failure.code,
                    failure.message,
                    promoted_unix_ms,
                    sqlite_integer(recovery_change_id.value(), "recovery change ID")?,
                ],
            )
            .map_err(database_error)?;
        transfer_catch_up_lineage(&transaction, [change_id], recovery_change_id)?;
        insert_metadata_inventory_recovery_authority(
            &transaction,
            &LibraryRecoveryAuthority {
                change_id: recovery_change_id,
                run_id: format!("watcher-gap-promotion-{}", recovery_change_id.value()),
                root_id: recovery_intent.root_id.clone(),
                root_generation: recovery_intent.root_generation,
                reason: LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
                opening_boundary: Some(opening_boundary.clone()),
                authorized_unix_ms: promoted_unix_ms,
                retired_unix_ms: None,
            },
        )?;
        super::persistent_journal::insert_watcher_gap_recovery_window(
            &transaction,
            recovery_change_id,
            &recovery_intent.root_id,
            recovery_intent.root_generation,
            &opening_boundary,
            failure,
            promoted_unix_ms,
        )?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET last_failure_code = ?1, last_failure_message = ?2,
                     updated_unix_ms = ?3
                 WHERE id = ?4 AND status = 'leased' AND lease_generation = ?5",
                params![
                    failure.code,
                    failure.message,
                    promoted_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                    sqlite_integer(lease_generation, "lease generation")?,
                ],
            )
            .map_err(database_error)?;
        let superseded = mark_superseded(
            &transaction,
            [change_id],
            Some(recovery_change_id),
            promoted_unix_ms,
        )?;
        if superseded != 1 {
            return Err(ScanError::new(
                "change_queue_recovery_promotion_conflict",
                "The P0 watcher gap changed before P2 recovery could be published",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(LibraryChangeLeaseUpdateOutcome::Applied)
    }

    fn defer_library_change_for_capacity(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferral: LibraryChangeCapacityDeferral,
        deferred_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        validate_policy(policy)?;
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Live)?;
        let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
        if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
            transaction.commit().map_err(database_error)?;
            return Ok(outcome);
        }
        let current = load_change(
            &transaction,
            sqlite_integer(change_id.value(), "change ID")?,
        )?;
        let is_root_live_gap = current.intent.origin == LibraryChangeOrigin::LiveNotification
            && current.intent.kind == LibraryChangeIntentKind::FreshnessUnknown
            && current.intent.scope == LibraryChangeScope::Root
            && current.intent.relative_path.is_empty()
            && current.intent.previous_relative_path.is_none();
        let has_exact_unclaimed_live_ownership = transaction
            .query_row(
                "SELECT
                   EXISTS(
                     SELECT 1 FROM library_change_queue_lanes AS lane
                     WHERE lane.change_id = ?1 AND lane.lane = 'p0_live'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = ?1
                   )",
                [sqlite_integer(change_id.value(), "change ID")?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !is_root_live_gap
            || !has_exact_unclaimed_live_ownership
            || deferral != LibraryChangeCapacityDeferral::MetadataInventoryLane
        {
            return Err(ScanError::new(
                "change_queue_capacity_deferral_invalid",
                "Only a leased P0 root live gap may wait for P2 metadata-inventory capacity",
            ));
        }
        let updated = transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', attempt_count = CASE
                       WHEN attempt_count > 0 THEN attempt_count - 1 ELSE 0 END,
                     next_retry_unix_ms = ?1, lease_expires_unix_ms = NULL,
                     last_failure_code = ?2, last_failure_message = ?3,
                     updated_unix_ms = ?4
                 WHERE id = ?5 AND status = 'leased' AND lease_generation = ?6",
                params![
                    capacity_deferral_deadline(deferred_unix_ms, policy),
                    deferral.failure_code(),
                    deferral.failure_message(),
                    deferred_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                    sqlite_integer(lease_generation, "lease generation")?,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "change_queue_capacity_deferral_raced",
                "The leased live gap changed before capacity deferral could be persisted",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(LibraryChangeLeaseUpdateOutcome::Applied)
    }

    fn defer_library_changes(
        &mut self,
        leases: &[crate::domain::LibraryChangeLeaseIdentity],
        deferred_unix_ms: i64,
    ) -> Result<Vec<LibraryChangeLeaseUpdateOutcome>, ScanError> {
        lease_deferral::defer(self, leases, deferred_unix_ms)
    }

    fn load_library_change_queue_metrics(
        &self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        validate_policy(policy)?;
        load_metrics(&self.connection, now_unix_ms, policy)
    }

    fn load_library_change_root_queue_metrics(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        load_root_metrics(
            &self.connection,
            root_id,
            root_generation,
            now_unix_ms,
            policy,
        )
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        terminal_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError> {
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let deleted = cleanup_terminal_records(&transaction, terminal_before_unix_ms, limit)?;
        transaction.commit().map_err(database_error)?;
        Ok(deleted)
    }
}

fn retry_leased_change_in_transaction(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    lease_generation: u64,
    failure: &LibraryChangeFailure,
    failed_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    validate_failure(failure)?;
    let attempt_count = transaction
        .query_row(
            "SELECT attempt_count FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let next_retry_unix_ms = next_retry_deadline(
        failed_unix_ms,
        sqlite_u32(attempt_count, "change attempt count")?,
        policy,
    );
    let updated = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'retry_wait', next_retry_unix_ms = ?1,
                 lease_expires_unix_ms = NULL, last_failure_code = ?2,
                 last_failure_message = ?3, updated_unix_ms = ?4
             WHERE id = ?5 AND status = 'leased' AND lease_generation = ?6",
            params![
                next_retry_unix_ms,
                failure.code,
                failure.message,
                failed_unix_ms,
                sqlite_integer(change_id.value(), "change ID")?,
                sqlite_integer(lease_generation, "lease generation")?,
            ],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(ScanError::new(
            "change_queue_retry_raced",
            "The leased change changed before retry state could be persisted",
        ));
    }
    Ok(())
}

pub(super) fn enqueue_metadata_inventory_candidates_in_transaction(
    transaction: &Transaction<'_>,
    authority: &LeasedLibraryChange,
    intents: &[LibraryChangeIntent],
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<Option<(LibraryChangeEnqueueReport, Vec<LibraryChangeId>)>, ScanError> {
    validate_policy(policy)?;
    validate_enqueue_batch(intents)?;
    if intents.iter().any(|intent| {
        intent.root_id != authority.change.intent.root_id
            || intent.root_generation != authority.change.intent.root_generation
    }) {
        return Err(ScanError::new(
            "metadata_inventory_candidate_authority_mismatch",
            "Metadata inventory candidates must belong to their leased authority",
        ));
    }
    if classify_lease_update(
        transaction,
        authority.change.id,
        authority.lease_generation,
        None,
    )? != LibraryChangeLeaseUpdateOutcome::Applied
    {
        return Ok(None);
    }
    if intents.is_empty() {
        return Ok(Some((LibraryChangeEnqueueReport::default(), Vec::new())));
    }
    cleanup_for_enqueue(transaction, enqueued_unix_ms, policy)?;
    let active = load_active_changes(
        transaction,
        &authority.change.intent.root_id,
        authority.change.intent.root_generation,
        policy.max_unresolved_changes,
    )?;
    let quota_active = active
        .iter()
        .filter(|change| change.id != authority.change.id)
        .cloned()
        .collect::<Vec<_>>();
    let admitted_counts =
        active_lane_counts(&quota_active).adding(LibraryChangeLane::Recovery, intents.len());
    if !lane_admission_allows(policy, LibraryChangeLane::Recovery, admitted_counts)
        || active.len().saturating_add(intents.len())
            > usize::try_from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES)
                .unwrap_or(usize::MAX)
    {
        return Err(ScanError::new(
            "metadata_inventory_backpressure",
            "The durable path queue must drain before inventory comparison continues",
        ));
    }
    let catalog_revision = load_catalog_revision(transaction)?;
    if metadata_inventory_batch_is_disjoint(&active, authority.change.id, intents) {
        let mut change_ids = Vec::with_capacity(intents.len());
        for intent in intents {
            change_ids.push(insert_change(
                transaction,
                intent,
                enqueued_unix_ms,
                catalog_revision,
                None,
                policy,
            )?);
        }
        return Ok(Some((
            LibraryChangeEnqueueReport {
                inserted_count: u32::try_from(change_ids.len()).unwrap_or(u32::MAX),
                ..LibraryChangeEnqueueReport::default()
            },
            change_ids,
        )));
    }
    let mut report = LibraryChangeEnqueueReport::default();
    let mut change_ids = Vec::with_capacity(intents.len());
    for intent in intents {
        change_ids.push(enqueue_one(
            transaction,
            intent,
            EnqueueContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                evidence: None,
                protected_change_ids: std::slice::from_ref(&authority.change.id),
                allow_scope_degradation: false,
            },
            &mut report,
        )?);
    }
    Ok(Some((report, change_ids)))
}

fn metadata_inventory_batch_is_disjoint(
    active: &[persistence::ActiveChange],
    authority_id: LibraryChangeId,
    intents: &[LibraryChangeIntent],
) -> bool {
    let mut affected_paths = BTreeSet::new();
    for intent in intents {
        if intent.origin.lane() != LibraryChangeLane::Recovery
            || intent.scope != LibraryChangeScope::Path
            || !affected_paths.insert(intent.relative_path.as_str())
            || intent
                .previous_relative_path
                .as_deref()
                .is_some_and(|path| !affected_paths.insert(path))
        {
            return false;
        }
    }
    active
        .iter()
        .filter(|change| change.id != authority_id)
        .all(|change| {
            intents
                .iter()
                .all(|intent| !coalescing::affected_paths_overlap(&change.intent, intent))
        })
}

fn enqueue_intents(
    catalog: &mut SqliteCatalog,
    intents: &[LibraryChangeIntent],
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeEnqueueReport, ScanError> {
    validate_policy(policy)?;
    validate_enqueue_batch(intents)?;
    if intents.is_empty() {
        return Ok(LibraryChangeEnqueueReport::default());
    }
    let lane = intents
        .iter()
        .map(|intent| intent.origin.lane())
        .min_by_key(|lane| super::sqlite_write_priority(*lane))
        .unwrap_or(LibraryChangeLane::Recovery);
    let transaction = catalog.begin_write_in_lane(lane)?;
    cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
    let report =
        enqueue_intents_in_transaction(&transaction, intents, evidence, enqueued_unix_ms, policy)?;
    transaction.commit().map_err(database_error)?;
    Ok(report)
}

pub(super) fn validate_enqueue_batch(intents: &[LibraryChangeIntent]) -> Result<(), ScanError> {
    let Some(first) = intents.first() else {
        return Ok(());
    };
    validate_intent_batch(intents, first)
}

pub(super) fn cleanup_for_enqueue(
    transaction: &Transaction<'_>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    let retention_millis = i64::try_from(policy.terminal_retention_millis).unwrap_or(i64::MAX);
    cleanup_terminal_records(
        transaction,
        enqueued_unix_ms.saturating_sub(retention_millis),
        policy.cleanup_batch,
    )?;
    Ok(())
}

pub(super) fn enqueue_intents_in_transaction(
    transaction: &Transaction<'_>,
    intents: &[LibraryChangeIntent],
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeEnqueueReport, ScanError> {
    let first = intents.first().ok_or_else(|| {
        ScanError::new(
            "change_queue_batch_empty",
            "A transactional change queue batch must contain at least one intent",
        )
    })?;
    let mut report = LibraryChangeEnqueueReport::default();
    match establish_root_generation(
        transaction,
        &first.root_id,
        first.root_generation,
        enqueued_unix_ms,
    )? {
        GenerationDisposition::Current { superseded_count } => {
            report.superseded_count = superseded_count;
        }
        GenerationDisposition::Stale => {
            report.stale_generation_count = u32::try_from(intents.len()).unwrap_or(u32::MAX);
            return Ok(report);
        }
    }
    let catalog_revision = load_catalog_revision(transaction)?;
    let protected_inventory_control_ids =
        load_leased_inventory_control_ids(transaction, &first.root_id, first.root_generation)?;
    for intent in intents {
        enqueue_one(
            transaction,
            intent,
            EnqueueContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                evidence,
                protected_change_ids: &protected_inventory_control_ids,
                allow_scope_degradation: true,
            },
            &mut report,
        )?;
    }
    Ok(report)
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "R2c-P enrollment uses this before R2c-Q schedules production replay"
    )
)]
pub(super) fn validate_catch_up_evidence(
    evidence: &LibraryChangeCatchUpEvidence,
) -> Result<(), ScanError> {
    if evidence.source.trim().is_empty()
        || evidence.source.len() > 128
        || evidence.watermark.trim().is_empty()
        || evidence.watermark.len() > 1_024
    {
        Err(ScanError::new(
            "library_change_catch_up_evidence_invalid",
            "Catch-up queue evidence exceeds its bounded storage contract",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn admission_lane_for_change_ids(
    connection: &Connection,
    change_ids: impl IntoIterator<Item = LibraryChangeId>,
) -> Result<LibraryChangeLane, ScanError> {
    let mut admitted = LibraryChangeLane::Recovery;
    for change_id in change_ids {
        let lane = connection
            .query_row(
                "SELECT lanes.lane
                 FROM library_change_queue_lanes AS lanes
                 JOIN library_change_queue AS queue ON queue.id = lanes.change_id
                 WHERE queue.id = ?1",
                [sqlite_integer(change_id.value(), "change ID")?],
                |row| row.get::<_, String>(0),
            )
            .map_err(database_error)?;
        let lane = parse_admission_lane(&lane)?;
        if super::sqlite_write_priority(lane) < super::sqlite_write_priority(admitted) {
            admitted = lane;
        }
    }
    Ok(admitted)
}

fn admission_lane_for_selection(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    selection: LeaseSelection,
) -> Result<LibraryChangeLane, ScanError> {
    if let LeaseSelection::PathLane(lane) = selection {
        return Ok(lane);
    }
    if let LeaseSelection::RecoveryAuthoritative = selection {
        return Ok(LibraryChangeLane::Recovery);
    }
    if matches!(
        selection,
        LeaseSelection::RecoveryCandidates | LeaseSelection::UnownedRecoveryPaths
    ) {
        return Ok(LibraryChangeLane::Recovery);
    }
    if let LeaseSelection::LiveAuthoritative = selection {
        return Ok(LibraryChangeLane::Live);
    }
    let priority = connection
        .query_row(
            "SELECT MIN(CASE lanes.lane
               WHEN 'p0_live' THEN 0 WHEN 'p1_journal' THEN 1 ELSE 2 END)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
             WHERE queue.root_id = ?1 AND queue.root_generation = ?2
               AND (
                 ?5 = 0
                 OR (?5 = 1 AND queue.scope = 'path'
                   AND queue.intent_kind <> 'freshness_unknown')
                 OR (?5 = 2 AND (queue.scope <> 'path'
                   OR queue.intent_kind = 'freshness_unknown'))
               )
               AND (
                 (queue.attempt_count < ?3 AND queue.status = 'pending'
                   AND queue.ready_unix_ms <= ?4)
                 OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                   AND queue.next_retry_unix_ms IS NOT NULL
                   AND queue.next_retry_unix_ms <= ?4)
                 OR (queue.status = 'leased'
                   AND queue.lease_expires_unix_ms IS NOT NULL
                   AND queue.lease_expires_unix_ms <= ?4)
                 OR (queue.status = 'retry_wait' AND queue.attempt_count >= ?3
                   AND queue.next_retry_unix_ms IS NOT NULL)
               )",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                i64::from(policy.max_attempts),
                now_unix_ms,
                selection.sql_value(),
            ],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(database_error)?;
    match priority {
        Some(0) => Ok(LibraryChangeLane::Live),
        Some(1) => Ok(LibraryChangeLane::Journal),
        Some(2) | None => Ok(LibraryChangeLane::Recovery),
        Some(_) => Err(ScanError::new(
            "change_queue_lane_invalid",
            "The durable change lane has an unsupported admission priority",
        )),
    }
}

fn parse_admission_lane(lane: &str) -> Result<LibraryChangeLane, ScanError> {
    match lane {
        "p0_live" => Ok(LibraryChangeLane::Live),
        "p1_journal" => Ok(LibraryChangeLane::Journal),
        "p2_recovery" => Ok(LibraryChangeLane::Recovery),
        _ => Err(ScanError::new(
            "change_queue_lane_invalid",
            "The durable change lane has an unsupported value",
        )),
    }
}

impl SqliteCatalog {
    pub(crate) fn compact_legacy_unowned_recovery_controls(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        compacted_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        if !root_generation_is_current(&transaction, root_id, root_generation)? {
            transaction.commit().map_err(database_error)?;
            return Ok(0);
        }
        let change_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT queue.id
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                       AND queue.scope <> 'path'
                       AND lane.lane = 'p2_recovery'
                       AND queue.status IN ('pending', 'retry_wait')
                       AND NOT EXISTS(
                         SELECT 1
                         FROM library_metadata_inventory_candidate_owners AS owner
                         WHERE owner.change_id = queue.id
                       )
                       AND NOT EXISTS(
                         SELECT 1 FROM library_recovery_authorities AS authority
                         WHERE authority.change_id = queue.id
                           AND authority.retired_unix_ms IS NULL
                       )
                     ORDER BY queue.first_observed_unix_ms, queue.id
                     LIMIT ?3",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map(
                    params![
                        root_id,
                        sqlite_integer(root_generation.value(), "root generation")?,
                        i64::from(policy.max_lease_batch),
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            rows.map(|row| {
                row.map_err(database_error).and_then(|id| {
                    let id = u64::try_from(id).map_err(|_| {
                        ScanError::new(
                            "library_change_id_invalid",
                            "The legacy recovery change ID is invalid",
                        )
                    })?;
                    LibraryChangeId::new(id).ok_or_else(|| {
                        ScanError::new(
                            "library_change_id_invalid",
                            "The legacy recovery change ID is invalid",
                        )
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()?
        };
        if change_ids.is_empty() {
            transaction.commit().map_err(database_error)?;
            return Ok(0);
        }
        let survivor_id = change_ids[0];
        if change_ids.len() == 1 {
            let is_blocked_survivor = transaction
                .query_row(
                    "SELECT status = 'retry_wait' AND attempt_count >= ?1
                            AND next_retry_unix_ms IS NULL
                            AND last_failure_code = 'legacy_recovery_authority_missing'
                     FROM library_change_queue WHERE id = ?2",
                    params![
                        i64::from(policy.max_attempts),
                        sqlite_integer(survivor_id.value(), "legacy recovery survivor ID")?,
                    ],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if is_blocked_survivor {
                transaction.commit().map_err(database_error)?;
                return Ok(0);
            }
        }
        let changes = change_ids
            .iter()
            .map(|change_id| {
                load_change(
                    &transaction,
                    sqlite_integer(change_id.value(), "legacy recovery change ID")?,
                )
            })
            .collect::<Result<Vec<_>, ScanError>>()?;
        let mut merged = changes[0].intent.clone();
        merged.kind = LibraryChangeIntentKind::FreshnessUnknown;
        merged.scope = LibraryChangeScope::Root;
        merged.relative_path.clear();
        merged.previous_relative_path = None;
        for change in changes.iter().skip(1) {
            if change.intent.most_recent_observed_unix_ms >= merged.most_recent_observed_unix_ms {
                merge_newer_evidence(&mut merged, &change.intent);
            } else {
                merge_older_evidence(&mut merged, &change.intent);
            }
        }
        validate_intent_batch(std::slice::from_ref(&merged), &merged)?;
        update_change(
            &transaction,
            survivor_id,
            &merged,
            None,
            compacted_unix_ms,
            policy,
        )?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', attempt_count = ?1,
                     next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                     last_failure_code = 'legacy_recovery_authority_missing',
                     last_failure_message =
                       'Legacy recovery evidence has no authority; refresh the library to replace it.',
                     updated_unix_ms = ?2
                 WHERE id = ?3 AND status IN ('pending', 'retry_wait')",
                params![
                    i64::from(policy.max_attempts),
                    compacted_unix_ms,
                    sqlite_integer(survivor_id.value(), "legacy recovery survivor ID")?,
                ],
            )
            .map_err(database_error)?;
        let superseded = mark_superseded(
            &transaction,
            change_ids.iter().skip(1).copied(),
            Some(survivor_id),
            compacted_unix_ms,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(superseded)
    }

    pub(crate) fn journal_admission_has_headroom(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        let (total, lower): (i64, i64) = self
            .connection
            .query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN lane.lane <> 'p0_live' THEN 1 ELSE 0 END), 0)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                   AND queue.status IN ('pending', 'leased', 'retry_wait')",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(database_error)?;
        Ok(total < i64::from(policy.max_unresolved_changes)
            && lower < i64::from(policy.lane_capacity(LibraryChangeLane::Journal)))
    }

    pub(crate) fn has_ready_live_path_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND lane.lane = 'p0_live'
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn has_ready_journal_path_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND lane.lane = 'p1_journal'
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn has_ready_metadata_inventory_recovery_candidates(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   JOIN library_metadata_inventory_candidate_owners AS owner
                     ON owner.change_id = queue.id
                   JOIN library_recovery_authorities AS authority
                     ON authority.run_id = owner.run_id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND lane.lane = 'p2_recovery'
                     AND authority.root_id = queue.root_id
                     AND authority.root_generation = queue.root_generation
                     AND authority.retired_unix_ms IS NULL
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn has_ready_legacy_unowned_recovery_debt(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND lane.lane = 'p2_recovery'
                     AND NOT EXISTS(
                       SELECT 1
                       FROM library_metadata_inventory_candidate_owners AS owner
                       WHERE owner.change_id = queue.id
                     )
                     AND NOT EXISTS(
                       SELECT 1 FROM library_recovery_authorities AS authority
                       WHERE authority.change_id = queue.id
                         AND authority.retired_unix_ms IS NULL
                     )
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 ) OR (
                   SELECT COUNT(*) > 1 OR COALESCE(MAX(
                     CASE
                       WHEN queue.status = 'retry_wait'
                         AND queue.attempt_count >= ?3
                         AND queue.next_retry_unix_ms IS NULL
                         AND queue.last_failure_code = 'legacy_recovery_authority_missing'
                       THEN 0
                       ELSE 1
                     END
                   ), 0) = 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.scope <> 'path'
                     AND lane.lane = 'p2_recovery'
                     AND queue.status IN ('pending', 'retry_wait')
                     AND NOT EXISTS(
                       SELECT 1
                       FROM library_metadata_inventory_candidate_owners AS owner
                       WHERE owner.change_id = queue.id
                     )
                     AND NOT EXISTS(
                       SELECT 1 FROM library_recovery_authorities AS authority
                       WHERE authority.change_id = queue.id
                         AND authority.retired_unix_ms IS NULL
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn has_unresolved_metadata_inventory_recovery_candidates(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError> {
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_metadata_inventory_candidate_owners AS owner
                   JOIN library_recovery_authorities AS authority
                     ON authority.run_id = owner.run_id
                   JOIN library_change_queue AS queue ON queue.id = owner.change_id
                   WHERE authority.root_id = ?1 AND authority.root_generation = ?2
                     AND authority.retired_unix_ms IS NULL
                     AND queue.status <> 'completed'
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn has_ready_metadata_inventory_recovery(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        let affinity_change_id = metadata_inventory_recovery_affinity_change_id(
            &self.connection,
            root_id,
            root_generation,
        )?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                   JOIN library_recovery_authorities AS authority
                     ON authority.change_id = queue.id
                    AND authority.root_id = queue.root_id
                    AND authority.root_generation = queue.root_generation
                   LEFT JOIN library_persistent_journal_baselines AS window
                     ON window.change_id = authority.change_id
                    AND window.phase <> 'completed'
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND (?5 IS NULL OR queue.id = ?5)
                     AND lanes.lane = 'p2_recovery'
                     AND authority.retired_unix_ms IS NULL
                     AND authority.reason <> 'first_import_boundary'
                     AND authority.run_id <> ''
                     AND (authority.reason = 'watcher_uncovered_gap'
                       OR window.change_id IS NOT NULL)
                     AND (queue.scope <> 'path'
                       OR queue.intent_kind = 'freshness_unknown')
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                    affinity_change_id,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn lease_metadata_inventory_recovery(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError> {
        let mut leased = self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::RecoveryAuthoritative,
        )?;
        Ok(leased.pop())
    }

    pub(crate) fn has_ready_live_authoritative_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND lane.lane = 'p0_live'
                     AND NOT EXISTS(
                       SELECT 1 FROM library_live_gap_recovery_claims AS claim
                       WHERE claim.gap_change_id = queue.id
                         AND claim.consumer_kind = 'pending_journal'
                     )
                     AND (queue.scope <> 'path'
                       OR queue.intent_kind = 'freshness_unknown')
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    #[cfg(test)]
    pub(crate) fn has_ready_authoritative_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_queue
                   WHERE root_id = ?1 AND root_generation = ?2
                     AND (scope <> 'path' OR intent_kind = 'freshness_unknown')
                     AND (
                       (attempt_count < ?3 AND status = 'pending' AND ready_unix_ms <= ?4)
                       OR (attempt_count < ?3 AND status = 'retry_wait'
                         AND next_retry_unix_ms IS NOT NULL AND next_retry_unix_ms <= ?4)
                       OR (status = 'leased' AND lease_expires_unix_ms IS NOT NULL
                         AND lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn lease_live_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError> {
        let mut leased = self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::LiveAuthoritative,
        )?;
        Ok(leased.pop())
    }

    fn lease_library_changes_matching(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
        selection: LeaseSelection,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        let selection_sql = selection.sql_value();
        let pretransaction_recovery_affinity_change_id =
            if matches!(selection, LeaseSelection::RecoveryAuthoritative) {
                metadata_inventory_recovery_affinity_change_id(
                    &self.connection,
                    root_id,
                    root_generation,
                )?
            } else {
                None
            };
        let pass_is_needed = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_root_state AS state
                   WHERE state.root_id = ?1 AND state.generation = ?2 AND state.is_active = 1
                 ) AND EXISTS(
                   SELECT 1 FROM library_change_queue AS queue
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND (
                       (queue.status = 'retry_wait' AND queue.attempt_count >= ?3
                         AND queue.next_retry_unix_ms IS NOT NULL)
                       OR (
                         (
                           ?5 = 0
                           OR (?5 = 1 AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown')
                           OR (?5 = 2 AND (queue.scope <> 'path'
                             OR queue.intent_kind = 'freshness_unknown'))
                            OR (?5 = 6
                              AND (queue.scope <> 'path'
                                OR queue.intent_kind = 'freshness_unknown')
                              AND (?6 IS NULL OR queue.id = ?6)
                              AND EXISTS(
                               SELECT 1
                               FROM library_change_queue_lanes AS lanes
                               JOIN library_recovery_authorities AS authority
                                 ON authority.change_id = lanes.change_id
                               WHERE lanes.change_id = queue.id
                                 AND lanes.lane = 'p2_recovery'
                                 AND authority.root_id = queue.root_id
                                 AND authority.root_generation = queue.root_generation
                                 AND authority.retired_unix_ms IS NULL
                                 AND authority.reason <> 'first_import_boundary'
                                 AND authority.run_id <> ''
                                 AND (
                                   authority.reason = 'watcher_uncovered_gap'
                                   OR EXISTS(
                                     SELECT 1
                                     FROM library_persistent_journal_baselines AS window
                                     WHERE window.change_id = authority.change_id
                                       AND window.phase <> 'completed'
                                   )
                                 )
                             ))
                           OR (?5 = 7
                             AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown'
                             AND EXISTS(
                               SELECT 1
                               FROM library_change_queue_lanes AS lanes
                               JOIN library_metadata_inventory_candidate_owners AS owner
                                 ON owner.change_id = lanes.change_id
                               JOIN library_recovery_authorities AS authority
                                 ON authority.run_id = owner.run_id
                               WHERE lanes.change_id = queue.id
                                 AND lanes.lane = 'p2_recovery'
                                 AND authority.root_id = queue.root_id
                                 AND authority.root_generation = queue.root_generation
                                 AND authority.retired_unix_ms IS NULL
                             ))
                           OR (?5 = 8
                             AND (queue.scope <> 'path'
                               OR queue.intent_kind = 'freshness_unknown')
                             AND EXISTS(
                               SELECT 1 FROM library_change_queue_lanes AS lanes
                               WHERE lanes.change_id = queue.id
                                 AND lanes.lane = 'p0_live'
                             )
                             AND NOT EXISTS(
                               SELECT 1
                               FROM library_live_gap_recovery_claims AS claim
                               WHERE claim.gap_change_id = queue.id
                                 AND claim.consumer_kind = 'pending_journal'
                             ))
                           OR (?5 = 9
                             AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown'
                             AND EXISTS(
                               SELECT 1 FROM library_change_queue_lanes AS lanes
                               WHERE lanes.change_id = queue.id
                                 AND lanes.lane = 'p2_recovery'
                             )
                             AND NOT EXISTS(
                               SELECT 1
                               FROM library_metadata_inventory_candidate_owners AS owner
                               WHERE owner.change_id = queue.id
                             )
                             AND NOT EXISTS(
                               SELECT 1 FROM library_recovery_authorities AS authority
                               WHERE authority.change_id = queue.id
                                 AND authority.retired_unix_ms IS NULL
                             ))
                           OR (?5 BETWEEN 3 AND 5 AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown'
                             AND EXISTS(
                               SELECT 1 FROM library_change_queue_lanes AS lanes
                               WHERE lanes.change_id = queue.id
                                 AND lanes.lane = CASE ?5
                                   WHEN 3 THEN 'p0_live'
                                   WHEN 4 THEN 'p1_journal'
                                   ELSE 'p2_recovery'
                                 END
                             ))
                         )
                         AND (
                           (queue.attempt_count < ?3 AND queue.status = 'pending'
                             AND queue.ready_unix_ms <= ?4)
                           OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                             AND queue.next_retry_unix_ms IS NOT NULL
                             AND queue.next_retry_unix_ms <= ?4)
                           OR (queue.status = 'leased'
                             AND queue.lease_expires_unix_ms IS NOT NULL
                             AND queue.lease_expires_unix_ms <= ?4)
                         )
                       )
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                    selection_sql,
                    pretransaction_recovery_affinity_change_id,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !pass_is_needed {
            return Ok(Vec::new());
        }
        let lane = admission_lane_for_selection(
            &self.connection,
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            selection,
        )?;
        let transaction = self.begin_write_in_lane(lane)?;
        if !root_generation_is_current(&transaction, root_id, root_generation)? {
            transaction.commit().map_err(database_error)?;
            return Ok(Vec::new());
        }
        recover_expired_leases(
            &transaction,
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            selection_sql,
        )?;
        enforce_retry_attempt_limit(&transaction, root_id, root_generation, now_unix_ms, policy)?;
        let recovery_affinity_change_id =
            if matches!(selection, LeaseSelection::RecoveryAuthoritative) {
                metadata_inventory_recovery_affinity_change_id(
                    &transaction,
                    root_id,
                    root_generation,
                )?
            } else {
                None
            };
        let limit = match selection {
            LeaseSelection::Authoritative
            | LeaseSelection::RecoveryAuthoritative
            | LeaseSelection::LiveAuthoritative => 1,
            LeaseSelection::All
            | LeaseSelection::Path
            | LeaseSelection::PathLane(_)
            | LeaseSelection::RecoveryCandidates
            | LeaseSelection::UnownedRecoveryPaths => i64::from(policy.max_lease_batch),
        };
        let change_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT id
                 FROM library_change_queue
                  WHERE root_id = ?1 AND root_generation = ?2
                    AND attempt_count < ?3
                    AND (?6 <> 6 OR ?7 IS NULL OR id = ?7)
                    AND (
                     ?6 = 0
                     OR (?6 = 1 AND scope = 'path' AND intent_kind <> 'freshness_unknown')
                     OR (?6 = 2 AND (scope <> 'path' OR intent_kind = 'freshness_unknown'))
                     OR (?6 = 6
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
                           AND authority.reason <> 'first_import_boundary'
                           AND authority.run_id <> ''
                           AND (
                             authority.reason = 'watcher_uncovered_gap'
                             OR EXISTS(
                               SELECT 1
                               FROM library_persistent_journal_baselines AS window
                               WHERE window.change_id = authority.change_id
                                 AND window.phase <> 'completed'
                             )
                           )
                       ))
                     OR (?6 = 7
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
                     OR (?6 = 8
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
                     OR (?6 = 9
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
                     OR (?6 BETWEEN 3 AND 5 AND scope = 'path'
                       AND intent_kind <> 'freshness_unknown'
                       AND EXISTS(
                         SELECT 1 FROM library_change_queue_lanes AS lanes
                         WHERE lanes.change_id = library_change_queue.id
                           AND lanes.lane = CASE ?6
                             WHEN 3 THEN 'p0_live'
                             WHEN 4 THEN 'p1_journal'
                             ELSE 'p2_recovery'
                           END
                       ))
                   )
                   AND (
                     (status = 'pending' AND ready_unix_ms <= ?4)
                     OR
                     (status = 'retry_wait' AND next_retry_unix_ms IS NOT NULL
                       AND next_retry_unix_ms <= ?4)
                   )
                 ORDER BY
                   CASE status WHEN 'retry_wait' THEN next_retry_unix_ms
                     ELSE ready_unix_ms END,
                   first_observed_unix_ms, id
                 LIMIT ?5",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map(
                    params![
                        root_id,
                        sqlite_integer(root_generation.value(), "root generation")?,
                        i64::from(policy.max_attempts),
                        now_unix_ms,
                        limit,
                        selection_sql,
                        recovery_affinity_change_id,
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(database_error)?);
            }
            ids
        };
        let lease_expires_unix_ms = now_unix_ms
            .saturating_add(i64::try_from(policy.lease_duration_millis).unwrap_or(i64::MAX));
        let mut leased = Vec::with_capacity(change_ids.len());
        for change_id in change_ids {
            let updated = transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'leased', attempt_count = attempt_count + 1,
                         next_retry_unix_ms = NULL,
                         lease_generation = lease_generation + 1,
                         lease_expires_unix_ms = ?1, updated_unix_ms = ?2
                     WHERE id = ?3 AND status IN ('pending', 'retry_wait')",
                    params![lease_expires_unix_ms, now_unix_ms, change_id],
                )
                .map_err(database_error)?;
            if updated == 0 {
                continue;
            }
            let change = load_change(&transaction, change_id)?;
            leased.push(LeasedLibraryChange {
                lease_generation: change.lease_generation,
                lease_expires_unix_ms,
                change,
            });
        }
        transaction.commit().map_err(database_error)?;
        Ok(leased)
    }
}

#[derive(Clone, Copy)]
enum LeaseSelection {
    All,
    Path,
    Authoritative,
    PathLane(LibraryChangeLane),
    RecoveryAuthoritative,
    RecoveryCandidates,
    LiveAuthoritative,
    UnownedRecoveryPaths,
}

impl LeaseSelection {
    const fn sql_value(self) -> i64 {
        match self {
            Self::All => 0,
            Self::Path => 1,
            Self::Authoritative => 2,
            Self::PathLane(LibraryChangeLane::Live) => 3,
            Self::PathLane(LibraryChangeLane::Journal) => 4,
            Self::PathLane(LibraryChangeLane::Recovery) => 5,
            Self::RecoveryAuthoritative => 6,
            Self::RecoveryCandidates => 7,
            Self::LiveAuthoritative => 8,
            Self::UnownedRecoveryPaths => 9,
        }
    }
}

#[cfg(test)]
mod tests;

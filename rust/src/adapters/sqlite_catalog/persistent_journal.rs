use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeCatchUpEvidence,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthority,
    LibraryRecoveryOpeningBoundary, LibraryRootGeneration, PersistentJournalBaseline,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselinePhase,
    PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalCrossRootLineage,
    PersistentJournalEnrollmentBatch, PersistentJournalEnrollmentReport, PersistentJournalFailure,
    PersistentJournalLineageState, PersistentJournalPendingRename, PersistentJournalRangeState,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind, PersistentJournalVolumeBatch,
    PersistentJournalVolumeIdentity, ScanError, persistent_journal_batch_id,
    persistent_journal_batch_payload,
};
use crate::ports::PersistentJournalRepository;

use super::change_queue::{
    PERSISTENT_JOURNAL_CATCH_UP_SOURCE, cleanup_for_enqueue,
    consume_live_gap_claims_with_source_range, enqueue_intents_in_transaction,
    insert_persistent_journal_recovery_control, normalize_persistent_journal_intents,
    transfer_pending_live_gap_claims_to_recovery, validate_catch_up_evidence,
    validate_enqueue_batch,
};
use super::metadata_inventory::insert_metadata_inventory_recovery_authority;
use super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

mod root_unregister;

pub(super) use root_unregister::remove_root_persistent_journal_state;

impl PersistentJournalRepository for SqliteCatalog {
    fn begin_persistent_journal_baseline(
        &mut self,
        request: &PersistentJournalBaselineStartRequest,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalBaseline, ScanError> {
        request.validate()?;
        if !policy.is_valid() {
            return Err(invalid_batch("The baseline queue policy is invalid"));
        }
        let transaction = self.begin_write()?;
        require_active_root_generation(&transaction, &request.root_id, request.root_generation)?;
        if let Some(existing) =
            load_baseline_for_root(&transaction, &request.root_id, request.root_generation)?
        {
            let stored_reason = transaction
                .query_row(
                    "SELECT reason FROM library_recovery_authorities WHERE change_id = ?1",
                    [sqlite_integer(
                        existing.change_id.value(),
                        "baseline change ID",
                    )?],
                    |row| row.get::<_, String>(0),
                )
                .map_err(database_error)?;
            let requested_reason = match request.authority_reason {
                crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline => {
                    "existing_root_baseline"
                }
                crate::domain::LibraryRecoveryAuthorityReason::FirstImportBoundary => {
                    "first_import_boundary"
                }
                _ => unreachable!("baseline request validation restricts authority"),
            };
            if existing.phase == PersistentJournalBaselinePhase::Completed {
                return Err(ScanError::new(
                    "persistent_journal_baseline_already_completed",
                    "The root already completed its one-time journal baseline",
                ));
            }
            if existing.run_id != request.run_id
                || existing.volume != request.volume
                || existing.root_file_reference != request.root_file_reference
                || existing.journal_id != request.journal_id
                || existing.opening_next_usn != request.opening_next_usn
                || existing.protocol_version != request.protocol_version
                || existing.contract_version != request.contract_version
                || stored_reason != requested_reason
            {
                return Err(ScanError::new(
                    "persistent_journal_baseline_conflict",
                    "The root already owns a different durable journal baseline",
                ));
            }
            transaction.commit().map_err(database_error)?;
            return Ok(existing);
        }
        if load_checkpoint(&transaction, &request.root_id, request.root_generation)?.is_some() {
            return Err(ScanError::new(
                "persistent_journal_baseline_checkpoint_exists",
                "A root with a trustworthy checkpoint cannot start a migration baseline",
            ));
        }
        let capability_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET protocol_version = ?1, contract_version = ?2,
                     capability_state = 'supported', continuity_state = 'baseline_required',
                     last_failure_code = NULL, last_failure_message = NULL,
                     updated_unix_ms = ?3
                 WHERE root_id = ?4 AND root_generation = ?5
                   AND EXISTS(
                     SELECT 1 FROM library_change_root_state AS active
                     WHERE active.root_id = ?4 AND active.generation = ?5
                       AND active.is_active = 1
                   )
                   AND (
                     (capability_state = 'unknown' AND continuity_state = 'baseline_required'
                      AND protocol_version = 0 AND last_failure_code IS NULL)
                     OR
                     (capability_state = 'supported' AND continuity_state = 'baseline_required'
                      AND protocol_version = ?1 AND contract_version = ?2
                      AND last_failure_code IS NULL)
                   )",
                params![
                    i64::from(request.protocol_version),
                    i64::from(request.contract_version),
                    request.authorized_unix_ms,
                    request.root_id,
                    sqlite_integer(request.root_generation.value(), "root generation")?,
                ],
            )
            .map_err(database_error)?;
        if capability_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_capability_invalid",
                "The root cannot atomically acquire supported baseline-required journal authority",
            ));
        }
        let root_state =
            load_root_authority_for_key(&transaction, &request.root_id, request.root_generation)?;
        if root_state.capability_state != "supported"
            || root_state.continuity_state != "baseline_required"
            || root_state.protocol_version != request.protocol_version
            || root_state.contract_version != request.contract_version
        {
            return Err(ScanError::new(
                "persistent_journal_baseline_capability_invalid",
                "The root does not own supported baseline-required journal authority",
            ));
        }
        let root_has_published_scan = transaction
            .query_row(
                "SELECT active_scan_id IS NOT NULL FROM library_roots WHERE id = ?1",
                [&request.root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        match request.authority_reason {
            crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline
                if !root_has_published_scan =>
            {
                return Err(ScanError::new(
                    "persistent_journal_baseline_authority_invalid",
                    "An existing-root baseline requires a published catalog snapshot",
                ));
            }
            crate::domain::LibraryRecoveryAuthorityReason::FirstImportBoundary => {
                let owns_pristine_first_import = transaction
                    .query_row(
                        "SELECT EXISTS(
                           SELECT 1 FROM scan_runs AS scan
                           JOIN library_roots AS root ON root.id = scan.root_id
                           WHERE scan.id = ?1 AND scan.root_id = ?2
                             AND scan.root_generation_at_start = ?3
                             AND scan.scan_owner = 'foreground'
                             AND scan.status = 'running'
                             AND scan.visited_entries = 0
                             AND scan.accepted_items = 0
                             AND root.active_scan_id IS NULL
                         )",
                        params![
                            request.run_id,
                            request.root_id,
                            sqlite_integer(request.root_generation.value(), "root generation")?,
                        ],
                        |row| row.get::<_, bool>(0),
                    )
                    .map_err(database_error)?;
                if !owns_pristine_first_import {
                    return Err(ScanError::new(
                        "persistent_journal_first_import_boundary_late",
                        "The first-import opening boundary was not persisted before enumeration",
                    ));
                }
            }
            _ => {}
        }
        let unresolved_recovery = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND lanes.lane = 'p2_recovery'
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                 )",
                params![
                    request.root_id,
                    sqlite_integer(request.root_generation.value(), "root generation")?,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if unresolved_recovery {
            return Err(ScanError::new(
                "persistent_journal_baseline_recovery_busy",
                "Existing recovery work must reach a page boundary before baseline admission",
            ));
        }
        let sequence = request
            .journal_id
            .value()
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(u64::try_from(request.opening_next_usn.value()).unwrap_or_default())
            .max(1);
        let intent = LibraryChangeIntent {
            root_id: request.root_id.clone(),
            root_generation: request.root_generation,
            kind: LibraryChangeIntentKind::FreshnessUnknown,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::ConsistencyAudit,
            first_observed_unix_ms: request.authorized_unix_ms,
            most_recent_observed_unix_ms: request.authorized_unix_ms,
            first_sequence: sequence,
            most_recent_sequence: sequence,
            coalesced_observation_count: 1,
        };
        cleanup_for_enqueue(&transaction, request.authorized_unix_ms, policy)?;
        let enqueue = enqueue_intents_in_transaction(
            &transaction,
            std::slice::from_ref(&intent),
            None,
            request.authorized_unix_ms,
            policy,
        )
        .map_err(|error| {
            map_queue_backpressure(
                error,
                "persistent_journal_baseline_admission_failed",
                "The one-time baseline could not obtain independent durable P2 admission",
            )
        })?;
        if enqueue.inserted_count != 1
            || enqueue.coalesced_count != 0
            || enqueue.capacity_degraded
            || enqueue.stale_generation_count != 0
        {
            return Err(ScanError::new(
                "persistent_journal_baseline_admission_failed",
                "The one-time baseline could not obtain independent durable P2 admission",
            ));
        }
        let change_id = transaction
            .query_row(
                "SELECT queue.id
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                 WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                   AND queue.origin = 'consistency_audit'
                   AND queue.intent_kind = 'freshness_unknown'
                   AND queue.scope = 'root' AND queue.relative_path = ''
                   AND queue.first_sequence = ?3 AND queue.most_recent_sequence = ?3
                   AND queue.created_unix_ms = ?4 AND lanes.lane = 'p2_recovery'
                   AND queue.status = 'pending'
                 ORDER BY queue.id DESC LIMIT 1",
                params![
                    request.root_id,
                    sqlite_integer(request.root_generation.value(), "root generation")?,
                    sequence.to_string(),
                    request.authorized_unix_ms,
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "baseline change ID")?)
            .ok_or_else(|| invalid_batch("The baseline change ID is invalid"))?;
        transaction
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason,
                   opening_journal_id, opening_next_usn, authorized_unix_ms, retired_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
                params![
                    sqlite_integer(change_id.value(), "baseline change ID")?,
                    request.run_id,
                    request.root_id,
                    sqlite_integer(request.root_generation.value(), "root generation")?,
                    match request.authority_reason {
                        crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline => {
                            "existing_root_baseline"
                        }
                        crate::domain::LibraryRecoveryAuthorityReason::FirstImportBoundary => {
                            "first_import_boundary"
                        }
                        _ => unreachable!("baseline request validation restricts authority"),
                    },
                    request.journal_id.to_canonical_text(),
                    request.opening_next_usn.to_canonical_text(),
                    request.authorized_unix_ms,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_persistent_journal_baselines(
                   change_id, root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   opening_next_usn, closing_next_usn, protocol_version, contract_version,
                   phase, authorized_unix_ms, updated_unix_ms, completed_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11,
                           'inventory', ?12, ?12, NULL)",
                params![
                    sqlite_integer(change_id.value(), "baseline change ID")?,
                    request.root_id,
                    sqlite_integer(request.root_generation.value(), "root generation")?,
                    request.volume.volume_guid,
                    request.volume.canonical_serial(),
                    i64::from(request.root_file_reference.record_version()),
                    request.root_file_reference.as_bytes(),
                    request.journal_id.to_canonical_text(),
                    request.opening_next_usn.to_canonical_text(),
                    i64::from(request.protocol_version),
                    i64::from(request.contract_version),
                    request.authorized_unix_ms,
                ],
            )
            .map_err(database_error)?;
        let baseline = load_baseline(&transaction, change_id)?
            .ok_or_else(|| invalid_batch("The durable one-time baseline was not stored"))?;
        transaction.commit().map_err(database_error)?;
        Ok(baseline)
    }

    fn load_persistent_journal_baselines(
        &self,
    ) -> Result<Vec<PersistentJournalBaseline>, ScanError> {
        load_active_baselines(&self.connection)
    }

    fn capture_persistent_journal_baseline_closing_boundary(
        &mut self,
        boundary: &PersistentJournalBaselineClosingBoundary,
    ) -> Result<PersistentJournalBaseline, ScanError> {
        boundary.validate()?;
        let transaction = self.begin_write()?;
        let baseline = load_baseline(&transaction, boundary.change_id)?.ok_or_else(|| {
            ScanError::new(
                "persistent_journal_baseline_missing",
                "The one-time baseline no longer owns recovery authority",
            )
        })?;
        if baseline.phase == PersistentJournalBaselinePhase::Completed
            || baseline.volume != boundary.volume
            || baseline.root_file_reference != boundary.root_file_reference
            || baseline.journal_id != boundary.journal_id
            || baseline.protocol_version != boundary.protocol_version
            || boundary.closing_next_usn < baseline.opening_next_usn
            || boundary.captured_unix_ms < baseline.updated_unix_ms
        {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_mismatch",
                "The closing journal boundary does not continuously close the opening baseline",
            ));
        }
        if let Some(closing) = baseline.closing_next_usn {
            if closing != boundary.closing_next_usn {
                return Err(ScanError::new(
                    "persistent_journal_baseline_closing_mismatch",
                    "The baseline is already bound to a different closing journal boundary",
                ));
            }
            transaction.commit().map_err(database_error)?;
            return Ok(baseline);
        }
        let authority_reason = transaction
            .query_row(
                "SELECT reason FROM library_recovery_authorities WHERE change_id = ?1",
                [sqlite_integer(
                    boundary.change_id.value(),
                    "baseline change ID",
                )?],
                |row| row.get::<_, String>(0),
            )
            .map_err(database_error)?;
        let is_recovery_window = !matches!(
            authority_reason.as_str(),
            "existing_root_baseline" | "first_import_boundary"
        );
        let replaces_checkpoint_journal = authority_reason == "journal_reset";
        let recovery_checkpoint = if is_recovery_window {
            let checkpoint =
                load_checkpoint(&transaction, &baseline.root_id, baseline.root_generation)?
                    .ok_or_else(|| {
                        ScanError::new(
                            "persistent_journal_baseline_closing_mismatch",
                            "The recovery closing boundary has no durable checkpoint",
                        )
                    })?;
            if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
                || checkpoint.volume != baseline.volume
                || checkpoint.root_file_reference != baseline.root_file_reference
                || checkpoint.protocol_version != baseline.protocol_version
                || checkpoint.contract_version != baseline.contract_version
                || checkpoint.captured_exclusive_end != checkpoint.next_unread_usn
                || (replaces_checkpoint_journal && checkpoint.journal_id == baseline.journal_id)
                || (!replaces_checkpoint_journal
                    && (checkpoint.journal_id != baseline.journal_id
                        || checkpoint.next_unread_usn > baseline.opening_next_usn))
            {
                return Err(ScanError::new(
                    "persistent_journal_baseline_closing_mismatch",
                    "The recovery closing boundary no longer extends its last trustworthy checkpoint",
                ));
            }
            Some(checkpoint)
        } else {
            None
        };
        let catalog_revision = super::load_catalog_revision(&transaction)?;
        let checkpoint_updated = if is_recovery_window {
            let checkpoint = recovery_checkpoint
                .as_ref()
                .expect("recovery windows retain a validated checkpoint");
            let replay_start = if replaces_checkpoint_journal {
                baseline.opening_next_usn
            } else {
                checkpoint.next_unread_usn
            };
            transaction
                .execute(
                    "UPDATE library_persistent_journal_checkpoints
                     SET journal_id = ?1, next_unread_usn = ?2,
                         captured_exclusive_end = ?3, covered_catalog_revision = ?4,
                         continuity_state = 'catching_up', last_failure_code = NULL,
                         last_failure_message = NULL, updated_unix_ms = ?5
                     WHERE root_id = ?6 AND root_generation = ?7
                       AND volume_guid = ?8 AND volume_serial = ?9
                       AND root_reference_version = ?10 AND root_file_reference = ?11
                       AND journal_id = ?12 AND next_unread_usn = ?13
                       AND captured_exclusive_end = ?14 AND protocol_version = ?15
                       AND contract_version = ?16
                       AND continuity_state = 'recovery_required'",
                    params![
                        baseline.journal_id.to_canonical_text(),
                        replay_start.to_canonical_text(),
                        boundary.closing_next_usn.to_canonical_text(),
                        sqlite_integer(catalog_revision, "catalog revision")?,
                        boundary.captured_unix_ms,
                        baseline.root_id,
                        sqlite_integer(baseline.root_generation.value(), "root generation")?,
                        baseline.volume.volume_guid,
                        baseline.volume.canonical_serial(),
                        i64::from(baseline.root_file_reference.record_version()),
                        baseline.root_file_reference.as_bytes(),
                        checkpoint.journal_id.to_canonical_text(),
                        checkpoint.next_unread_usn.to_canonical_text(),
                        checkpoint.captured_exclusive_end.to_canonical_text(),
                        i64::from(baseline.protocol_version),
                        i64::from(baseline.contract_version),
                    ],
                )
                .map_err(database_error)?
        } else {
            transaction
                .execute(
                    "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   last_failure_code, last_failure_message, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                           'catching_up', NULL, NULL, ?13)",
                    params![
                        baseline.root_id,
                        sqlite_integer(baseline.root_generation.value(), "root generation")?,
                        baseline.volume.volume_guid,
                        baseline.volume.canonical_serial(),
                        i64::from(baseline.root_file_reference.record_version()),
                        baseline.root_file_reference.as_bytes(),
                        baseline.journal_id.to_canonical_text(),
                        baseline.opening_next_usn.to_canonical_text(),
                        boundary.closing_next_usn.to_canonical_text(),
                        sqlite_integer(catalog_revision, "catalog revision")?,
                        i64::from(baseline.protocol_version),
                        i64::from(baseline.contract_version),
                        boundary.captured_unix_ms,
                    ],
                )
                .map_err(database_error)?
        };
        if checkpoint_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_mismatch",
                "The closing journal boundary no longer matches its opening checkpoint",
            ));
        }
        let baseline_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_baselines
                 SET closing_next_usn = ?2, phase = 'replay', updated_unix_ms = ?3
                 WHERE change_id = ?1 AND phase = 'inventory' AND closing_next_usn IS NULL",
                params![
                    sqlite_integer(boundary.change_id.value(), "baseline change ID")?,
                    boundary.closing_next_usn.to_canonical_text(),
                    boundary.captured_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if baseline_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_mismatch",
                "The recovery window changed before closing capture",
            ));
        }
        let root_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'catching_up', updated_unix_ms = ?3
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND capability_state = 'supported'
                   AND continuity_state = ?4",
                params![
                    baseline.root_id,
                    sqlite_integer(baseline.root_generation.value(), "root generation")?,
                    boundary.captured_unix_ms,
                    if is_recovery_window {
                        "recovery_required"
                    } else {
                        "baseline_required"
                    },
                ],
            )
            .map_err(database_error)?;
        if root_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_mismatch",
                "The root continuity authority changed before closing capture",
            ));
        }
        let captured = load_baseline(&transaction, boundary.change_id)?
            .ok_or_else(|| invalid_batch("The captured baseline boundary disappeared"))?;
        transaction.commit().map_err(database_error)?;
        Ok(captured)
    }

    fn persistent_journal_baseline_closing_is_covered(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<bool, ScanError> {
        baseline_closing_is_covered(&self.connection, change_id)
    }

    fn finalize_ready_first_import_journal_baseline(
        &mut self,
        completed_unix_ms: i64,
    ) -> Result<bool, ScanError> {
        if completed_unix_ms < 0 {
            return Err(invalid_batch(
                "The first-import baseline completion time is invalid",
            ));
        }
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        let candidates = {
            let mut statement = transaction
                .prepare(
                    "SELECT baseline.change_id
                     FROM library_persistent_journal_baselines AS baseline
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = baseline.change_id
                     JOIN library_change_queue AS control ON control.id = baseline.change_id
                     JOIN library_change_root_state AS root_state
                       ON root_state.root_id = baseline.root_id
                      AND root_state.generation = baseline.root_generation
                     JOIN library_roots AS root ON root.id = baseline.root_id
                     JOIN scan_runs AS scan ON scan.id = authority.run_id
                     WHERE authority.reason = 'first_import_boundary'
                       AND authority.retired_unix_ms IS NULL
                       AND baseline.phase = 'replay'
                       AND baseline.closing_next_usn IS NOT NULL
                       AND control.status IN ('pending', 'retry_wait')
                       AND root_state.is_active = 1
                       AND root.active_scan_id = scan.id
                       AND scan.root_id = baseline.root_id
                       AND scan.root_generation_at_start = baseline.root_generation
                       AND scan.scan_owner = 'foreground'
                       AND scan.status = 'completed'
                       AND NOT EXISTS (
                         SELECT 1
                         FROM library_change_queue AS pending
                         JOIN library_change_queue_lanes AS lane
                           ON lane.change_id = pending.id
                         WHERE pending.root_id = baseline.root_id
                           AND pending.root_generation = baseline.root_generation
                           AND (
                             lane.lane IN ('p0_live', 'p1_journal')
                             OR (lane.lane = 'p2_recovery'
                               AND pending.id <> baseline.change_id)
                           )
                           AND pending.status IN ('pending', 'leased', 'retry_wait')
                       )
                     ORDER BY baseline.change_id
                     LIMIT 16",
                )
                .map_err(database_error)?;
            statement
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        let mut selected = None;
        for stored_change_id in candidates {
            let change_id = LibraryChangeId::new(sqlite_unsigned(
                stored_change_id,
                "first-import baseline change ID",
            )?)
            .ok_or_else(|| invalid_batch("The first-import baseline change ID is invalid"))?;
            if baseline_closing_is_covered(&transaction, change_id)? {
                selected = Some(stored_change_id);
                break;
            }
        }
        let Some(stored_change_id) = selected else {
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        };
        let (root_id, root_generation, updated_unix_ms) = transaction
            .query_row(
                "SELECT root_id, root_generation, updated_unix_ms
                 FROM library_persistent_journal_baselines
                 WHERE change_id = ?1 AND phase = 'replay'",
                [stored_change_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(database_error)?;
        if completed_unix_ms < updated_unix_ms {
            return Err(ScanError::new(
                "persistent_journal_first_import_completion_time_invalid",
                "The first-import baseline cannot complete before its closing replay",
            ));
        }
        let catalog_revision = super::load_catalog_revision(&transaction)?;
        let completed = transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL,
                     catalog_revision_at_success = ?2, updated_unix_ms = ?3
                 WHERE id = ?1 AND status IN ('pending', 'retry_wait')",
                params![
                    stored_change_id,
                    sqlite_integer(catalog_revision, "catalog revision")?,
                    completed_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if completed != 1 {
            return Err(ScanError::new(
                "persistent_journal_first_import_completion_raced",
                "The first-import baseline control changed during completion",
            ));
        }
        let baseline_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_baselines
                 SET phase = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE change_id = ?1 AND phase = 'replay'
                   AND completed_unix_ms IS NULL",
                params![stored_change_id, completed_unix_ms],
            )
            .map_err(database_error)?;
        let checkpoint_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'current', updated_unix_ms = ?3
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND continuity_state = 'catching_up'
                   AND next_unread_usn = captured_exclusive_end
                   AND last_failure_code IS NULL",
                params![root_id, root_generation, completed_unix_ms],
            )
            .map_err(database_error)?;
        let root_state_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = ?3
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND capability_state = 'supported'
                   AND continuity_state = 'catching_up'
                   AND last_failure_code IS NULL",
                params![root_id, root_generation, completed_unix_ms],
            )
            .map_err(database_error)?;
        let authority_retired = transaction
            .execute(
                "UPDATE library_recovery_authorities
                 SET retired_unix_ms = ?2
                 WHERE change_id = ?1 AND reason = 'first_import_boundary'
                   AND retired_unix_ms IS NULL",
                params![stored_change_id, completed_unix_ms],
            )
            .map_err(database_error)?;
        if baseline_updated != 1
            || checkpoint_updated != 1
            || root_state_updated != 1
            || authority_retired != 1
        {
            return Err(ScanError::new(
                "persistent_journal_first_import_completion_raced",
                "The first-import baseline authority changed during atomic completion",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }

    fn load_persistent_journal_capabilities(
        &self,
    ) -> Result<Vec<PersistentJournalCapability>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT journal.root_id, journal.root_generation, journal.protocol_version,
                        journal.contract_version, journal.capability_state,
                        journal.continuity_state, journal.last_failure_code,
                        journal.last_failure_message, journal.updated_unix_ms
                 FROM library_persistent_journal_root_state AS journal
                 JOIN library_change_root_state AS active
                   ON active.root_id = journal.root_id
                  AND active.generation = journal.root_generation
                  AND active.is_active = 1
                 ORDER BY journal.root_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (
                root_id,
                root_generation,
                protocol_version,
                contract_version,
                state,
                continuity,
                failure_code,
                failure_message,
                updated_unix_ms,
            ) = row.map_err(database_error)?;
            let capability = PersistentJournalCapability {
                root_id,
                root_generation: parse_root_generation(root_generation)?,
                protocol_version: parse_u16(protocol_version, "journal protocol version")?,
                contract_version: parse_u16(contract_version, "journal contract version")?,
                state: parse_capability_state(&state)?,
                continuity: parse_continuity_state(&continuity)?,
                failure: parse_failure(failure_code, failure_message)?,
                updated_unix_ms,
            };
            capability.validate()?;
            Ok(capability)
        })
        .collect()
    }

    fn load_persistent_journal_checkpoint(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<PersistentJournalCheckpoint>, ScanError> {
        require_active_root_generation(&self.connection, root_id, root_generation)?;
        load_checkpoint(&self.connection, root_id, root_generation)
    }

    fn save_persistent_journal_capability(
        &mut self,
        capability: &PersistentJournalCapability,
    ) -> Result<(), ScanError> {
        capability.validate()?;
        let (failure_code, failure_message) = failure_columns(capability.failure.as_ref());
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        let updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET protocol_version = ?1, contract_version = ?2,
                     capability_state = ?3, continuity_state = ?4,
                     last_failure_code = ?5, last_failure_message = ?6,
                     updated_unix_ms = ?7
                 WHERE root_id = ?8 AND root_generation = ?9
                   AND EXISTS(
                     SELECT 1 FROM library_change_root_state AS active
                     WHERE active.root_id = ?8 AND active.generation = ?9
                       AND active.is_active = 1
                   )",
                params![
                    i64::from(capability.protocol_version),
                    i64::from(capability.contract_version),
                    capability_state_text(capability.state),
                    continuity_state_text(capability.continuity),
                    failure_code,
                    failure_message,
                    capability.updated_unix_ms,
                    capability.root_id,
                    sqlite_integer(capability.root_generation.value(), "root generation")?,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_root_authority_stale",
                "The persistent journal capability no longer owns the active root generation",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(())
    }

    fn persist_persistent_journal_root_failure(
        &mut self,
        failure: &PersistentJournalRootFailure,
        failed_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LibraryChangeId>, ScanError> {
        failure.validate()?;
        if failed_unix_ms < 0 || !policy.is_valid() {
            return Err(invalid_batch(
                "The persistent journal root failure admission input is invalid",
            ));
        }
        if failure.kind == PersistentJournalRootFailureKind::Cancelled {
            return Ok(None);
        }
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        require_active_root_generation(&transaction, &failure.root_id, failure.root_generation)?;
        let root =
            load_root_authority_for_key(&transaction, &failure.root_id, failure.root_generation)?;
        if root.capability_state != "supported" {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        }
        let checkpoint = load_checkpoint(&transaction, &failure.root_id, failure.root_generation)?
            .ok_or_else(|| {
                ScanError::new(
                    "persistent_journal_failure_checkpoint_missing",
                    "A current persistent journal root failure has no durable checkpoint",
                )
            })?;
        if checkpoint.protocol_version != root.protocol_version
            || checkpoint.contract_version != root.contract_version
        {
            return Err(ScanError::new(
                "persistent_journal_failure_authority_mismatch",
                "The root and checkpoint disagree before failure admission",
            ));
        }
        if failure.kind == PersistentJournalRootFailureKind::Transient {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        }

        mark_persistent_journal_recovery_required(
            &transaction,
            &failure.root_id,
            failure.root_generation,
            &failure.failure.code,
            &failure.failure.message,
            failed_unix_ms,
        )?;
        let Some(reason) = failure.kind.recovery_reason() else {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        };

        let existing = transaction
            .query_row(
                "SELECT authority.change_id
                 FROM library_recovery_authorities AS authority
                 JOIN library_change_queue AS queue ON queue.id = authority.change_id
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE authority.root_id = ?1 AND authority.root_generation = ?2
                   AND authority.retired_unix_ms IS NULL
                   AND authority.reason IN (
                     'journal_gap', 'journal_reset', 'journal_trim',
                     'journal_reconstruction_failure', 'containment_failure',
                     'broker_after_current_failure', 'watcher_uncovered_gap'
                   )
                   AND queue.status IN ('pending', 'leased', 'retry_wait')
                   AND lane.lane = 'p2_recovery'
                 ORDER BY authority.change_id LIMIT 1",
                params![
                    failure.root_id,
                    sqlite_integer(failure.root_generation.value(), "root generation")?,
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(change_id) = existing {
            let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "recovery change ID")?)
                .ok_or_else(|| invalid_batch("The recovery change ID is invalid"))?;
            attach_recovery_window_if_available(
                &transaction,
                change_id,
                &failure.root_id,
                failure.root_generation,
                failure.opening_boundary.as_ref(),
                failed_unix_ms,
            )?;
            transfer_pending_live_gap_claims_to_recovery(
                &transaction,
                &failure.root_id,
                failure.root_generation,
                change_id,
                failed_unix_ms,
            )?;
            transaction.commit().map_err(database_error)?;
            return Ok(Some(change_id));
        }

        let sequence = u64::try_from(checkpoint.next_unread_usn.value())
            .unwrap_or_default()
            .max(1);
        let intent = LibraryChangeIntent {
            root_id: failure.root_id.clone(),
            root_generation: failure.root_generation,
            kind: LibraryChangeIntentKind::FreshnessUnknown,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::MetadataInventory,
            first_observed_unix_ms: failed_unix_ms,
            most_recent_observed_unix_ms: failed_unix_ms,
            first_sequence: sequence,
            most_recent_sequence: sequence,
            coalesced_observation_count: 1,
        };
        let change_id = insert_persistent_journal_recovery_control(
            &transaction,
            &intent,
            failed_unix_ms,
            policy,
        )?;
        insert_metadata_inventory_recovery_authority(
            &transaction,
            &LibraryRecoveryAuthority {
                change_id,
                run_id: format!("journal-failure-{}", change_id.value()),
                root_id: failure.root_id.clone(),
                root_generation: failure.root_generation,
                reason,
                opening_boundary: failure.opening_boundary.clone(),
                authorized_unix_ms: failed_unix_ms,
                retired_unix_ms: None,
            },
        )?;
        attach_recovery_window_if_available(
            &transaction,
            change_id,
            &failure.root_id,
            failure.root_generation,
            failure.opening_boundary.as_ref(),
            failed_unix_ms,
        )?;
        transfer_pending_live_gap_claims_to_recovery(
            &transaction,
            &failure.root_id,
            failure.root_generation,
            change_id,
            failed_unix_ms,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(Some(change_id))
    }

    fn attach_persistent_journal_recovery_opening_boundary(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        boundary: &LibraryRecoveryOpeningBoundary,
        captured_unix_ms: i64,
    ) -> Result<Option<LibraryChangeId>, ScanError> {
        boundary.validate()?;
        if captured_unix_ms < 0 {
            return Err(invalid_batch(
                "The recovery opening capture time is invalid",
            ));
        }
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        require_active_root_generation(&transaction, root_id, root_generation)?;
        let root = load_root_authority_for_key(&transaction, root_id, root_generation)?;
        if root.capability_state != "supported" || root.continuity_state != "recovery_required" {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        }
        let checkpoint =
            load_checkpoint(&transaction, root_id, root_generation)?.ok_or_else(|| {
                ScanError::new(
                    "persistent_journal_recovery_checkpoint_missing",
                    "A pending journal recovery has no durable checkpoint",
                )
            })?;
        if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
            || checkpoint.volume != boundary.volume
            || checkpoint.root_file_reference != boundary.root_file_reference
            || checkpoint.protocol_version != boundary.protocol_version
            || checkpoint.contract_version != boundary.contract_version
        {
            return Err(ScanError::new(
                "persistent_journal_recovery_opening_identity_drift",
                "The recovered journal opening boundary no longer matches the root authority",
            ));
        }
        let active = transaction
            .query_row(
                "SELECT authority.change_id,
                        window.change_id IS NOT NULL,
                        COALESCE(window.volume_guid = ?3
                          AND window.volume_serial = ?4
                          AND window.root_reference_version = ?5
                          AND window.root_file_reference = ?6
                          AND window.journal_id = ?7
                          AND window.opening_next_usn = ?8
                          AND window.protocol_version = ?9
                          AND window.contract_version = ?10
                          AND window.phase <> 'completed', 0)
                 FROM library_recovery_authorities AS authority
                 JOIN library_change_queue AS queue ON queue.id = authority.change_id
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 LEFT JOIN library_persistent_journal_baselines AS window
                   ON window.change_id = authority.change_id
                 WHERE authority.root_id = ?1 AND authority.root_generation = ?2
                   AND authority.retired_unix_ms IS NULL
                   AND queue.status IN ('pending', 'leased', 'retry_wait')
                   AND lane.lane = 'p2_recovery'
                 ORDER BY authority.change_id LIMIT 1",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    boundary.volume.volume_guid,
                    boundary.volume.canonical_serial(),
                    i64::from(boundary.root_file_reference.record_version()),
                    boundary.root_file_reference.as_bytes(),
                    boundary.journal_id.to_canonical_text(),
                    boundary.next_usn.to_canonical_text(),
                    i64::from(boundary.protocol_version),
                    i64::from(boundary.contract_version),
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, bool>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((change_id, has_window, exact_window)) = active else {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        };
        let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "recovery change ID")?)
            .ok_or_else(|| invalid_batch("The recovery change ID is invalid"))?;
        if has_window && !exact_window {
            return Err(ScanError::new(
                "persistent_journal_recovery_opening_conflict",
                "The recovery authority already owns a different opening boundary",
            ));
        }
        if !has_window {
            attach_recovery_window_if_available(
                &transaction,
                change_id,
                root_id,
                root_generation,
                Some(boundary),
                captured_unix_ms,
            )?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(Some(change_id))
    }

    fn load_persistent_journal_pending_renames(
        &self,
        volume: &PersistentJournalVolumeIdentity,
        journal_id: JournalIdentifier,
    ) -> Result<Vec<PersistentJournalPendingRename>, ScanError> {
        volume.validate()?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT carry_id, source_range_id, file_reference_version, file_reference,
                        old_usn, previous_root_id, previous_root_generation,
                        previous_relative_path, is_directory, enrolled_unix_ms
                 FROM library_persistent_journal_pending_renames
                 WHERE volume_guid = ?1 AND volume_serial = ?2 AND journal_id = ?3
                 ORDER BY old_usn, carry_id
                 LIMIT 1025",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![
                    volume.volume_guid,
                    volume.canonical_serial(),
                    journal_id.to_canonical_text(),
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, bool>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let mut pending = Vec::new();
        for row in rows {
            let row = row.map_err(database_error)?;
            pending.push(PersistentJournalPendingRename {
                carry_id: row.0,
                source_range_id: row.1,
                volume: volume.clone(),
                journal_id,
                file_reference: parse_file_reference(row.2, &row.3)?,
                old_usn: JournalUsn::parse_canonical(&row.4)?,
                previous_root_id: row.5,
                previous_root_generation: parse_root_generation(row.6)?,
                previous_relative_path: row.7,
                is_directory: row.8,
                enrolled_unix_ms: row.9,
            });
        }
        if pending.len() > 1_024 {
            return Err(ScanError::new(
                "persistent_journal_pending_rename_capacity",
                "The durable pending rename carry exceeds its bounded capacity",
            ));
        }
        for carry in &pending {
            carry.validate()?;
        }
        Ok(pending)
    }

    fn publish_persistent_journal_volume_batch(
        &mut self,
        batch: &PersistentJournalVolumeBatch,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError> {
        publish_volume_batch(self, batch, enqueued_unix_ms, policy, None)
    }

    #[cfg(test)]
    fn enroll_persistent_journal_batch(
        &mut self,
        batch: &PersistentJournalEnrollmentBatch,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError> {
        batch.validate()?;
        validate_enqueue_batch(&batch.intents)?;
        if batch.range.state != PersistentJournalRangeState::Enrolled
            || batch.range.enrolled_unix_ms != enqueued_unix_ms
        {
            return Err(invalid_batch(
                "The source range is not a new durable enrollment",
            ));
        }
        let evidence = LibraryChangeCatchUpEvidence {
            source: PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
            watermark: batch.range.batch_id.clone(),
        };
        validate_catch_up_evidence(&evidence)?;
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        require_supported_root(&transaction, batch)?;
        if source_range_exists(&transaction, batch)? {
            transaction.commit().map_err(database_error)?;
            return Ok(PersistentJournalEnrollmentReport {
                enrolled_root_count: 1,
                ..PersistentJournalEnrollmentReport::default()
            });
        }
        cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
        insert_source_range(&transaction, batch)?;
        if !batch.intents.is_empty() {
            let normalized_intents = normalize_persistent_journal_intents(&batch.intents)?;
            let queue_report = enqueue_intents_in_transaction(
                &transaction,
                &normalized_intents,
                Some(&evidence),
                enqueued_unix_ms,
                policy,
            )
            .map_err(|error| {
                map_queue_backpressure(
                    error,
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete journal range",
                )
            })?;
            if queue_report.capacity_degraded || queue_report.stale_generation_count != 0 {
                return Err(ScanError::new(
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete journal range",
                ));
            }
            transaction
                .execute(
                    "INSERT INTO library_persistent_journal_queue_lineage(
                       source_range_id, change_id, enrolled_unix_ms
                     )
                     SELECT ?1, change_id, ?2
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                     WHERE lineage.catch_up_source = ?3 AND lineage.catch_up_watermark = ?1
                       AND changes.status IN ('pending', 'leased', 'retry_wait')
                     ON CONFLICT(source_range_id, change_id) DO NOTHING",
                    params![
                        batch.range.batch_id,
                        enqueued_unix_ms,
                        PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                    ],
                )
                .map_err(database_error)?;
        }
        for lineage in &batch.cross_root_lineage {
            enroll_cross_root_lineage(&transaction, lineage, enqueued_unix_ms)?;
        }
        transaction.commit().map_err(database_error)?;
        let observation_count = batch
            .intents
            .iter()
            .map(|intent| u64::from(intent.coalesced_observation_count))
            .sum();
        Ok(PersistentJournalEnrollmentReport {
            enrolled_root_count: 1,
            observation_count,
            ..PersistentJournalEnrollmentReport::default()
        })
    }

    #[cfg(test)]
    fn advance_persistent_journal_checkpoint(
        &mut self,
        source_range_id: &str,
        checkpoint: &PersistentJournalCheckpoint,
    ) -> Result<bool, ScanError> {
        checkpoint.validate()?;
        let transaction = self.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        let range = load_checkpoint_range(&transaction, source_range_id)?;
        if range.root_id != checkpoint.root_id
            || range.root_generation != checkpoint.root_generation
            || range.volume != checkpoint.volume
            || range.journal_id != checkpoint.journal_id
            || range.protocol_version != checkpoint.protocol_version
            || range.contract_version != checkpoint.contract_version
            || range.covered_until_usn != checkpoint.next_unread_usn
            || range.requested_end_usn != checkpoint.captured_exclusive_end
        {
            return Err(invalid_batch(
                "The checkpoint does not exactly close its durable source range",
            ));
        }
        require_active_root_generation(
            &transaction,
            &checkpoint.root_id,
            checkpoint.root_generation,
        )?;
        if range.state == PersistentJournalRangeState::Checkpointed {
            let stored = load_checkpoint(
                &transaction,
                &checkpoint.root_id,
                checkpoint.root_generation,
            )?
            .ok_or_else(|| invalid_batch("The checkpointed range has no durable checkpoint"))?;
            if stored != *checkpoint {
                return Err(invalid_batch(
                    "A replayed range conflicts with its durable checkpoint",
                ));
            }
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        }
        if range.state != PersistentJournalRangeState::Enrolled {
            return Err(invalid_batch(
                "A superseded source range cannot advance a checkpoint",
            ));
        }
        let current = load_checkpoint(
            &transaction,
            &checkpoint.root_id,
            checkpoint.root_generation,
        )?
        .ok_or_else(|| {
            invalid_batch(
                "The source range cannot establish an implicit persistent journal baseline",
            )
        })?;
        let authority = load_root_authority(&transaction, checkpoint)?;
        if current.volume != range.volume
            || current.root_file_reference != checkpoint.root_file_reference
            || current.journal_id != range.journal_id
            || current.next_unread_usn != range.requested_start_usn
            || current.captured_exclusive_end > range.requested_end_usn
            || current.covered_catalog_revision > checkpoint.covered_catalog_revision
            || current.protocol_version != range.protocol_version
            || current.contract_version != range.contract_version
            || checkpoint.next_unread_usn <= current.next_unread_usn
            || checkpoint.captured_exclusive_end < current.captured_exclusive_end
            || checkpoint.updated_unix_ms < current.updated_unix_ms
            || authority.protocol_version != range.protocol_version
            || authority.contract_version != range.contract_version
            || authority.capability_state != "supported"
        {
            return Err(invalid_batch(
                "The checkpoint does not continuously advance the exact durable journal state",
            ));
        }
        compare_and_swap_checkpoint(&transaction, &current, checkpoint)?;
        let updated = transaction
            .execute(
                "UPDATE library_persistent_journal_source_ranges
                 SET status = 'checkpointed', checkpointed_unix_ms = ?1
                 WHERE id = ?2 AND root_id = ?3 AND root_generation = ?4
                   AND volume_guid = ?5 AND volume_serial = ?6 AND journal_id = ?7
                   AND requested_start_usn = ?8 AND requested_end_usn = ?9
                   AND covered_until_usn = ?10 AND protocol_version = ?11
                   AND contract_version = ?12 AND status = 'enrolled'",
                params![
                    checkpoint.updated_unix_ms,
                    source_range_id,
                    range.root_id,
                    sqlite_integer(range.root_generation.value(), "root generation")?,
                    range.volume.volume_guid,
                    range.volume.canonical_serial(),
                    range.journal_id.to_canonical_text(),
                    range.requested_start_usn.to_canonical_text(),
                    range.requested_end_usn.to_canonical_text(),
                    range.covered_until_usn.to_canonical_text(),
                    i64::from(range.protocol_version),
                    i64::from(range.contract_version),
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(invalid_batch(
                "The source range checkpoint transition raced",
            ));
        }
        let authority_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET protocol_version = ?1, contract_version = ?2,
                     capability_state = 'supported', continuity_state = ?3,
                     last_failure_code = ?4, last_failure_message = ?5,
                     updated_unix_ms = ?6
                 WHERE root_id = ?7 AND root_generation = ?8
                   AND protocol_version = ?9 AND contract_version = ?10
                   AND capability_state = ?11 AND continuity_state = ?12
                   AND last_failure_code IS ?13 AND last_failure_message IS ?14
                   AND updated_unix_ms = ?15
                   AND EXISTS(
                     SELECT 1 FROM library_change_root_state AS active
                     WHERE active.root_id = ?7 AND active.generation = ?8
                       AND active.is_active = 1
                   )",
                params![
                    i64::from(checkpoint.protocol_version),
                    i64::from(checkpoint.contract_version),
                    continuity_state_text(checkpoint.continuity),
                    checkpoint
                        .failure
                        .as_ref()
                        .map(|failure| failure.code.as_str()),
                    checkpoint
                        .failure
                        .as_ref()
                        .map(|failure| failure.message.as_str()),
                    checkpoint.updated_unix_ms,
                    checkpoint.root_id,
                    sqlite_integer(checkpoint.root_generation.value(), "root generation")?,
                    i64::from(authority.protocol_version),
                    i64::from(authority.contract_version),
                    authority.capability_state,
                    authority.continuity_state,
                    authority.failure_code,
                    authority.failure_message,
                    authority.updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if authority_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_root_authority_stale",
                "The checkpoint no longer owns the active root generation",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }
}

#[cfg(test)]
impl SqliteCatalog {
    pub(crate) fn seed_persistent_journal_checkpoint_for_test(
        &mut self,
        checkpoint: &PersistentJournalCheckpoint,
    ) -> Result<(), ScanError> {
        checkpoint.validate()?;
        self.connection
            .execute(
                "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   last_failure_code, last_failure_message, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                           ?13, ?14, ?15, ?16)
                 ON CONFLICT(root_id) DO UPDATE SET
                   root_generation = excluded.root_generation,
                   volume_guid = excluded.volume_guid,
                   volume_serial = excluded.volume_serial,
                   root_reference_version = excluded.root_reference_version,
                   root_file_reference = excluded.root_file_reference,
                   journal_id = excluded.journal_id,
                   next_unread_usn = excluded.next_unread_usn,
                   captured_exclusive_end = excluded.captured_exclusive_end,
                   covered_catalog_revision = excluded.covered_catalog_revision,
                   protocol_version = excluded.protocol_version,
                   contract_version = excluded.contract_version,
                   continuity_state = excluded.continuity_state,
                   last_failure_code = excluded.last_failure_code,
                   last_failure_message = excluded.last_failure_message,
                   updated_unix_ms = excluded.updated_unix_ms",
                params![
                    checkpoint.root_id,
                    sqlite_integer(checkpoint.root_generation.value(), "root generation")?,
                    checkpoint.volume.volume_guid,
                    checkpoint.volume.canonical_serial(),
                    i64::from(checkpoint.root_file_reference.record_version()),
                    checkpoint.root_file_reference.as_bytes(),
                    checkpoint.journal_id.to_canonical_text(),
                    checkpoint.next_unread_usn.to_canonical_text(),
                    checkpoint.captured_exclusive_end.to_canonical_text(),
                    sqlite_integer(checkpoint.covered_catalog_revision, "catalog revision")?,
                    i64::from(checkpoint.protocol_version),
                    i64::from(checkpoint.contract_version),
                    continuity_state_text(checkpoint.continuity),
                    checkpoint
                        .failure
                        .as_ref()
                        .map(|failure| failure.code.as_str()),
                    checkpoint
                        .failure
                        .as_ref()
                        .map(|failure| failure.message.as_str()),
                    checkpoint.updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

fn publish_volume_batch(
    catalog: &mut SqliteCatalog,
    batch: &PersistentJournalVolumeBatch,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    fail_after_owner: Option<usize>,
) -> Result<PersistentJournalEnrollmentReport, ScanError> {
    batch.validate()?;
    if enqueued_unix_ms < 0
        || batch.pages.iter().any(|page| {
            page.enrollment.range.batch_id != persistent_journal_batch_id(&page.enrollment)
                || page.enrollment.range.state != PersistentJournalRangeState::Enrolled
                || page.enrollment.range.enrolled_unix_ms != enqueued_unix_ms
                || page.checkpoint.continuity != PersistentJournalContinuityState::CatchingUp
                || page.checkpoint.failure.is_some()
        })
    {
        return Err(invalid_batch(
            "A volume publication must contain new ranges and non-authoritative checkpoints",
        ));
    }
    for page in &batch.pages {
        validate_enqueue_batch(&page.enrollment.intents)?;
        let evidence = LibraryChangeCatchUpEvidence {
            source: PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
            watermark: page.enrollment.range.batch_id.clone(),
        };
        validate_catch_up_evidence(&evidence)?;
    }
    validate_atomic_lineage(batch)?;

    let transaction = catalog.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
    let mut existing = Vec::with_capacity(batch.pages.len());
    for page in &batch.pages {
        require_supported_root(&transaction, &page.enrollment)?;
        reject_conflicting_range_coordinates(&transaction, &page.enrollment)?;
        existing.push(source_range_exists(&transaction, &page.enrollment)?);
    }
    if existing.iter().any(|value| *value) {
        if !existing.iter().all(|value| *value) {
            return Err(invalid_batch(
                "An atomic volume replay cannot mix durable and missing root pages",
            ));
        }
        for page in &batch.pages {
            let stored = load_checkpoint(
                &transaction,
                &page.checkpoint.root_id,
                page.checkpoint.root_generation,
            )?
            .ok_or_else(|| invalid_batch("A replayed volume page has no checkpoint"))?;
            let range = load_checkpoint_range(&transaction, &page.enrollment.range.batch_id)?;
            if stored != page.checkpoint || range.state != PersistentJournalRangeState::Checkpointed
            {
                return Err(invalid_batch(
                    "A replayed volume batch conflicts with durable normalized content",
                ));
            }
        }
        transaction.commit().map_err(database_error)?;
        return Ok(PersistentJournalEnrollmentReport {
            enrolled_root_count: u32::try_from(batch.pages.len()).map_err(|_| {
                invalid_batch("The volume replay root count exceeded its supported bound")
            })?,
            ..PersistentJournalEnrollmentReport::default()
        });
    }

    for page in &batch.pages {
        validate_checkpoint_advance(&transaction, &page.enrollment.range, &page.checkpoint)?;
    }
    validate_pending_rename_mutations(&transaction, batch)?;
    cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;

    for (index, page) in batch.pages.iter().enumerate() {
        insert_source_range(&transaction, &page.enrollment)?;
        consume_live_gap_claims_with_source_range(
            &transaction,
            &page.enrollment,
            enqueued_unix_ms,
        )?;
        if !page.enrollment.intents.is_empty() {
            let evidence = LibraryChangeCatchUpEvidence {
                source: PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
                watermark: page.enrollment.range.batch_id.clone(),
            };
            let normalized_intents =
                normalize_persistent_journal_intents(&page.enrollment.intents)?;
            let queue_report = enqueue_intents_in_transaction(
                &transaction,
                &normalized_intents,
                Some(&evidence),
                enqueued_unix_ms,
                policy,
            )
            .map_err(|error| {
                map_queue_backpressure(
                    error,
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete volume batch",
                )
            })?;
            if queue_report.capacity_degraded || queue_report.stale_generation_count != 0 {
                return Err(ScanError::new(
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete volume batch",
                ));
            }
            transaction
                .execute(
                    "INSERT INTO library_persistent_journal_queue_lineage(
                       source_range_id, change_id, enrolled_unix_ms
                     )
                     SELECT ?1, change_id, ?2
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                     WHERE lineage.catch_up_source = ?3 AND lineage.catch_up_watermark = ?1
                       AND changes.status IN ('pending', 'leased', 'retry_wait')
                     ON CONFLICT(source_range_id, change_id) DO NOTHING",
                    params![
                        page.enrollment.range.batch_id,
                        enqueued_unix_ms,
                        PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                    ],
                )
                .map_err(database_error)?;
        }
        if fail_after_owner == Some(index) {
            return Err(ScanError::new(
                "persistent_journal_test_crash",
                "Injected failure before the next volume owner was published",
            ));
        }
    }

    for page in &batch.pages {
        for carry_id in &page.enrollment.consumed_pending_rename_ids {
            let deleted = transaction
                .execute(
                    "DELETE FROM library_persistent_journal_pending_renames
                     WHERE carry_id = ?1",
                    [carry_id],
                )
                .map_err(database_error)?;
            if deleted != 1 {
                return Err(invalid_batch(
                    "A consumed pending rename carry changed during publication",
                ));
            }
        }
        for carry in &page.enrollment.pending_renames {
            insert_pending_rename(&transaction, carry)?;
        }
    }

    for page in &batch.pages {
        for lineage in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            enroll_cross_root_lineage(&transaction, lineage, enqueued_unix_ms)?;
        }
    }
    for page in &batch.pages {
        apply_checkpoint_advance(&transaction, &page.enrollment.range, &page.checkpoint)?;
    }
    finalize_persistent_journal_ranges(&transaction, enqueued_unix_ms)?;
    transaction.commit().map_err(database_error)?;
    Ok(PersistentJournalEnrollmentReport {
        enrolled_root_count: u32::try_from(batch.pages.len())
            .map_err(|_| invalid_batch("The volume root count exceeded its supported bound"))?,
        observation_count: batch_observation_count(batch),
        advanced_checkpoint_count: u32::try_from(batch.pages.len()).map_err(|_| {
            invalid_batch("The volume checkpoint count exceeded its supported bound")
        })?,
        ..PersistentJournalEnrollmentReport::default()
    })
}

fn batch_observation_count(batch: &PersistentJournalVolumeBatch) -> u64 {
    batch
        .pages
        .iter()
        .flat_map(|page| &page.enrollment.intents)
        .map(|intent| u64::from(intent.coalesced_observation_count))
        .sum()
}

fn validate_atomic_lineage(batch: &PersistentJournalVolumeBatch) -> Result<(), ScanError> {
    let mut owners =
        std::collections::BTreeMap::<&str, Vec<&PersistentJournalCrossRootLineage>>::new();
    for page in &batch.pages {
        for lineage in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            owners
                .entry(lineage.lineage_id.as_str())
                .or_default()
                .push(lineage);
        }
    }
    for endpoints in owners.values() {
        let [left, right] = endpoints.as_slice() else {
            return Err(invalid_batch(
                "A volume handoff must publish exactly two source-range owners",
            ));
        };
        if left.owner_source_range_id == right.owner_source_range_id
            || left.volume != right.volume
            || left.journal_id != right.journal_id
            || left.file_reference != right.file_reference
            || left.old_usn != right.old_usn
            || left.new_usn != right.new_usn
            || left.previous_carry_id != right.previous_carry_id
            || left.previous_root_id != right.previous_root_id
            || left.previous_root_generation != right.previous_root_generation
            || left.previous_relative_path != right.previous_relative_path
            || left.current_root_id != right.current_root_id
            || left.current_root_generation != right.current_root_generation
            || left.current_relative_path != right.current_relative_path
            || left.state != right.state
        {
            return Err(invalid_batch(
                "A volume handoff contains inconsistent durable endpoint evidence",
            ));
        }
    }
    Ok(())
}

fn reject_conflicting_range_coordinates(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    let conflicting = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_persistent_journal_source_ranges
               WHERE root_id = ?1 AND root_generation = ?2
                 AND volume_guid = ?3 AND volume_serial = ?4 AND journal_id = ?5
                 AND requested_start_usn = ?6 AND requested_end_usn = ?7
                 AND covered_until_usn = ?8 AND protocol_version = ?9
                 AND contract_version = ?10 AND id <> ?11
             )",
            params![
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                batch.range.batch_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if conflicting {
        Err(invalid_batch(
            "The journal range coordinates are already bound to different batch content",
        ))
    } else {
        Ok(())
    }
}

fn validate_pending_rename_mutations(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalVolumeBatch,
) -> Result<(), ScanError> {
    let mut consumed = std::collections::BTreeSet::new();
    let mut created = std::collections::BTreeSet::new();
    let carried_ids = batch
        .pages
        .iter()
        .flat_map(|page| {
            page.enrollment
                .cross_root_lineage
                .iter()
                .chain(&page.enrollment.carried_cross_root_lineage)
        })
        .filter_map(|lineage| lineage.previous_carry_id.as_deref())
        .collect::<Vec<_>>();
    for page in &batch.pages {
        for carry_id in &page.enrollment.consumed_pending_rename_ids {
            if !consumed.insert(carry_id.as_str()) {
                return Err(invalid_batch("A pending rename carry is consumed twice"));
            }
            let carry = load_pending_rename_by_id(transaction, carry_id)?;
            let Some(carry) = carry else {
                return Err(invalid_batch(
                    "A consumed pending rename carry has no durable predecessor",
                ));
            };
            let lineages = batch
                .pages
                .iter()
                .flat_map(|candidate_page| {
                    candidate_page
                        .enrollment
                        .cross_root_lineage
                        .iter()
                        .chain(&candidate_page.enrollment.carried_cross_root_lineage)
                        .map(move |lineage| (candidate_page, lineage))
                })
                .filter(|(_, lineage)| lineage.previous_carry_id.as_deref() == Some(carry_id))
                .collect::<Vec<_>>();
            let [left, right] = lineages.as_slice() else {
                return Err(invalid_batch(
                    "A consumed carry must bind exactly two carried lineage owners",
                ));
            };
            let owner_matches =
                |candidate_page: &&crate::domain::PersistentJournalVolumePage,
                 lineage: &&PersistentJournalCrossRootLineage| {
                    let is_previous = lineage.owner_source_range_id == carry.source_range_id;
                    let is_current = lineage.owner_source_range_id
                        == candidate_page.enrollment.range.batch_id
                        && lineage.current_root_id == candidate_page.enrollment.range.root_id
                        && lineage.current_root_generation
                            == candidate_page.enrollment.range.root_generation
                        && lineage.new_usn.is_some_and(|usn| {
                            usn >= candidate_page.enrollment.range.requested_start_usn
                                && usn < candidate_page.enrollment.range.covered_until_usn
                        });
                    (is_previous || is_current)
                        && lineage.volume == carry.volume
                        && lineage.journal_id == carry.journal_id
                        && lineage.file_reference == carry.file_reference
                        && lineage.old_usn == Some(carry.old_usn)
                        && lineage.previous_root_id == carry.previous_root_id
                        && lineage.previous_root_generation == carry.previous_root_generation
                        && lineage.previous_relative_path == carry.previous_relative_path
                };
            if left.1.lineage_id != right.1.lineage_id
                || left.1.owner_source_range_id == right.1.owner_source_range_id
                || !owner_matches(&left.0, &left.1)
                || !owner_matches(&right.0, &right.1)
            {
                return Err(invalid_batch(
                    "A consumed carry does not exactly match both carried lineage owners",
                ));
            }
        }
        for carry in &page.enrollment.pending_renames {
            if !created.insert(carry.carry_id.as_str())
                || consumed.contains(carry.carry_id.as_str())
            {
                return Err(invalid_batch(
                    "A pending rename carry mutation is duplicated or contradictory",
                ));
            }
        }
    }
    if carried_ids.len()
        != consumed
            .len()
            .checked_mul(2)
            .ok_or_else(|| invalid_batch("The carried lineage count overflowed"))?
        || carried_ids
            .iter()
            .any(|carry_id| !consumed.contains(carry_id))
    {
        return Err(invalid_batch(
            "Carried lineage evidence must be consumed exactly once",
        ));
    }
    let current = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_persistent_journal_pending_renames",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let next = current
        .checked_sub(i64::try_from(consumed.len()).unwrap_or(i64::MAX))
        .and_then(|value| value.checked_add(i64::try_from(created.len()).ok()?))
        .ok_or_else(|| invalid_batch("The pending rename carry count overflowed"))?;
    if !(0..=1_024).contains(&next) {
        return Err(ScanError::new(
            "persistent_journal_pending_rename_capacity",
            "The durable pending rename carry cannot exceed 1024 records",
        ));
    }
    Ok(())
}

fn load_pending_rename_by_id(
    transaction: &Transaction<'_>,
    carry_id: &str,
) -> Result<Option<PersistentJournalPendingRename>, ScanError> {
    let stored = transaction
        .query_row(
            "SELECT source_range_id, volume_guid, volume_serial, journal_id,
                    file_reference_version, file_reference, old_usn, previous_root_id,
                    previous_root_generation, previous_relative_path, is_directory,
                    enrolled_unix_ms
             FROM library_persistent_journal_pending_renames WHERE carry_id = ?1",
            [carry_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let file_reference = JournalFileReference::from_bytes(&stored.5)?;
    if i64::from(file_reference.record_version()) != stored.4 {
        return Err(invalid_batch(
            "The durable carry reference version is invalid",
        ));
    }
    let carry = PersistentJournalPendingRename {
        carry_id: carry_id.to_owned(),
        source_range_id: stored.0,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: stored.1,
            volume_serial: stored
                .2
                .parse()
                .map_err(|_| invalid_batch("The durable carry serial is invalid"))?,
        },
        journal_id: JournalIdentifier::parse_canonical(&stored.3)?,
        file_reference,
        old_usn: JournalUsn::parse_canonical(&stored.6)?,
        previous_root_id: stored.7,
        previous_root_generation: LibraryRootGeneration::new(
            u64::try_from(stored.8)
                .map_err(|_| invalid_batch("The durable carry generation is invalid"))?,
        )
        .ok_or_else(|| invalid_batch("The durable carry generation is invalid"))?,
        previous_relative_path: stored.9,
        is_directory: stored.10,
        enrolled_unix_ms: stored.11,
    };
    carry.validate()?;
    Ok(Some(carry))
}

fn insert_pending_rename(
    transaction: &Transaction<'_>,
    carry: &PersistentJournalPendingRename,
) -> Result<(), ScanError> {
    carry.validate()?;
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_pending_renames(
               carry_id, source_range_id, volume_guid, volume_serial, journal_id,
               file_reference_version, file_reference, old_usn,
               previous_root_id, previous_root_generation, previous_relative_path,
               is_directory, enrolled_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                carry.carry_id,
                carry.source_range_id,
                carry.volume.volume_guid,
                carry.volume.canonical_serial(),
                carry.journal_id.to_canonical_text(),
                i64::from(carry.file_reference.record_version()),
                carry.file_reference.as_bytes(),
                carry.old_usn.to_canonical_text(),
                carry.previous_root_id,
                sqlite_integer(carry.previous_root_generation.value(), "root generation")?,
                carry.previous_relative_path,
                carry.is_directory,
                carry.enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn validate_checkpoint_advance(
    transaction: &Transaction<'_>,
    range: &crate::domain::PersistentJournalSourceRange,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    if range.root_id != checkpoint.root_id
        || range.root_generation != checkpoint.root_generation
        || range.volume != checkpoint.volume
        || range.journal_id != checkpoint.journal_id
        || range.protocol_version != checkpoint.protocol_version
        || range.contract_version != checkpoint.contract_version
        || range.covered_until_usn != checkpoint.next_unread_usn
        || range.requested_end_usn != checkpoint.captured_exclusive_end
    {
        return Err(invalid_batch(
            "The checkpoint does not exactly close its durable source range",
        ));
    }
    require_active_root_generation(transaction, &checkpoint.root_id, checkpoint.root_generation)?;
    let current = load_checkpoint(transaction, &checkpoint.root_id, checkpoint.root_generation)?
        .ok_or_else(|| {
            invalid_batch(
                "The source range cannot establish an implicit persistent journal baseline",
            )
        })?;
    let authority = load_root_authority(transaction, checkpoint)?;
    if current.volume != range.volume
        || current.root_file_reference != checkpoint.root_file_reference
        || current.journal_id != range.journal_id
        || current.next_unread_usn != range.requested_start_usn
        || current.captured_exclusive_end > range.requested_end_usn
        || current.covered_catalog_revision > checkpoint.covered_catalog_revision
        || current.protocol_version != range.protocol_version
        || current.contract_version != range.contract_version
        || checkpoint.next_unread_usn <= current.next_unread_usn
        || checkpoint.captured_exclusive_end < current.captured_exclusive_end
        || checkpoint.updated_unix_ms < current.updated_unix_ms
        || authority.protocol_version != range.protocol_version
        || authority.contract_version != range.contract_version
        || authority.capability_state != "supported"
    {
        return Err(invalid_batch(
            "The checkpoint does not continuously advance the exact durable journal state",
        ));
    }
    Ok(())
}

fn apply_checkpoint_advance(
    transaction: &Transaction<'_>,
    range: &crate::domain::PersistentJournalSourceRange,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    let current = load_checkpoint(transaction, &checkpoint.root_id, checkpoint.root_generation)?
        .ok_or_else(|| invalid_batch("The checkpoint baseline disappeared"))?;
    let authority = load_root_authority(transaction, checkpoint)?;
    compare_and_swap_checkpoint(transaction, &current, checkpoint)?;
    let updated = transaction
        .execute(
            "UPDATE library_persistent_journal_source_ranges
             SET status = 'checkpointed', checkpointed_unix_ms = ?1
             WHERE id = ?2 AND status = 'enrolled'",
            params![checkpoint.updated_unix_ms, range.batch_id],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(invalid_batch(
            "The source range checkpoint transition raced",
        ));
    }
    let authority_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET protocol_version = ?1, contract_version = ?2,
                 capability_state = 'supported', continuity_state = 'catching_up',
                 last_failure_code = NULL, last_failure_message = NULL,
                 updated_unix_ms = ?3
             WHERE root_id = ?4 AND root_generation = ?5
               AND protocol_version = ?6 AND contract_version = ?7
               AND capability_state = ?8 AND continuity_state = ?9
               AND last_failure_code IS ?10 AND last_failure_message IS ?11
               AND updated_unix_ms = ?12
               AND EXISTS(
                 SELECT 1 FROM library_change_root_state AS active
                 WHERE active.root_id = ?4 AND active.generation = ?5 AND active.is_active = 1
               )",
            params![
                i64::from(checkpoint.protocol_version),
                i64::from(checkpoint.contract_version),
                checkpoint.updated_unix_ms,
                checkpoint.root_id,
                sqlite_integer(checkpoint.root_generation.value(), "root generation")?,
                i64::from(authority.protocol_version),
                i64::from(authority.contract_version),
                authority.capability_state,
                authority.continuity_state,
                authority.failure_code,
                authority.failure_message,
                authority.updated_unix_ms,
            ],
        )
        .map_err(database_error)?;
    if authority_updated != 1 {
        return Err(ScanError::new(
            "persistent_journal_root_authority_stale",
            "The checkpoint no longer owns the active root generation",
        ));
    }
    Ok(())
}

pub(super) fn finalize_persistent_journal_ranges(
    transaction: &Transaction<'_>,
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    super::migrations::validate_persistent_journal_lineage_rows(transaction)?;
    super::migrations::validate_persistent_journal_payload_children(transaction)?;
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
    super::migrations::validate_persistent_journal_lineage_rows(transaction)?;
    super::migrations::validate_persistent_journal_payload_children(transaction)?;

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
    super::migrations::validate_persistent_journal_payload_children(transaction)
}

fn cleanup_completed_persistent_journal_history(
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
    super::catalog_delta::cleanup_terminal_catch_up_handoffs_batch(transaction, &queue_evidence)?;
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

fn require_active_root_generation(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<(), ScanError> {
    let is_active = connection
        .query_row(
            "SELECT generation = ?1 AND is_active = 1
             FROM library_change_root_state WHERE root_id = ?2",
            params![
                sqlite_integer(root_generation.value(), "root generation")?,
                root_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !is_active {
        return Err(ScanError::new(
            "persistent_journal_root_authority_stale",
            "The persistent journal checkpoint no longer owns the active root generation",
        ));
    }
    Ok(())
}

fn require_supported_root(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    let supported = transaction
        .query_row(
            "SELECT journal.capability_state = 'supported' AND active.is_active = 1
             FROM library_persistent_journal_root_state AS journal
             JOIN library_change_root_state AS active
               ON active.root_id = journal.root_id
              AND active.generation = journal.root_generation
             WHERE journal.root_id = ?1 AND journal.root_generation = ?2",
            params![
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if supported {
        Ok(())
    } else {
        Err(ScanError::new(
            "persistent_journal_root_authority_unavailable",
            "The root has no supported persistent journal authority for this generation",
        ))
    }
}

fn source_range_exists(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<bool, ScanError> {
    let exact = transaction
        .query_row(
            "SELECT root_id = ?2 AND root_generation = ?3
                    AND volume_guid = ?4 AND volume_serial = ?5 AND journal_id = ?6
                    AND requested_start_usn = ?7 AND requested_end_usn = ?8
                    AND covered_until_usn = ?9 AND is_complete = ?10
                    AND protocol_version = ?11 AND contract_version = ?12
                    AND canonical_payload = ?13
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            params![
                batch.range.batch_id,
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                batch.range.is_complete,
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                persistent_journal_batch_payload(batch),
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?;
    match exact {
        Some(true) => Ok(true),
        Some(false) => Err(invalid_batch(
            "The journal batch identifier is already owned by different source evidence",
        )),
        None => Ok(false),
    }
}

fn insert_source_range(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_source_ranges(
               id, root_id, root_generation, volume_guid, volume_serial, journal_id,
               requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
               protocol_version, contract_version, status, enrolled_unix_ms,
               checkpointed_unix_ms, canonical_payload
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
               'enrolled', ?13, NULL, ?14
             )",
            params![
                batch.range.batch_id,
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                batch.range.is_complete,
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                batch.range.enrolled_unix_ms,
                persistent_journal_batch_payload(batch),
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_range_lifecycle(
               source_range_id, lifecycle_state, completed_unix_ms, updated_unix_ms
             ) VALUES (?1, 'pending', NULL, ?2)",
            params![batch.range.batch_id, batch.range.enrolled_unix_ms],
        )
        .map_err(database_error)?;
    Ok(())
}

fn enroll_cross_root_lineage(
    transaction: &Transaction<'_>,
    lineage: &PersistentJournalCrossRootLineage,
    enrolled_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_cross_root_lineage(
               id, volume_guid, volume_serial, file_reference_version, file_reference,
               previous_root_id, previous_root_generation, previous_relative_path,
               current_root_id, current_root_generation, current_relative_path,
               status, created_unix_ms, updated_unix_ms, journal_id, old_usn, new_usn,
               previous_carry_id
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13,
               ?14, ?15, ?16, ?17
             ) ON CONFLICT(id) DO NOTHING",
            params![
                lineage.lineage_id,
                lineage.volume.volume_guid,
                lineage.volume.canonical_serial(),
                i64::from(lineage.file_reference.record_version()),
                lineage.file_reference.as_bytes(),
                lineage.previous_root_id,
                sqlite_integer(lineage.previous_root_generation.value(), "root generation")?,
                lineage.previous_relative_path,
                lineage.current_root_id,
                sqlite_integer(lineage.current_root_generation.value(), "root generation")?,
                lineage.current_relative_path,
                lineage_state_text(lineage.state),
                enrolled_unix_ms,
                lineage.journal_id.to_canonical_text(),
                lineage.old_usn.map(JournalUsn::to_canonical_text),
                lineage.new_usn.map(JournalUsn::to_canonical_text),
                lineage.previous_carry_id,
            ],
        )
        .map_err(database_error)?;
    let exact = transaction
        .query_row(
            "SELECT volume_guid = ?2 AND volume_serial = ?3
                    AND file_reference_version = ?4 AND file_reference = ?5
                    AND previous_root_id = ?6 AND previous_root_generation = ?7
                    AND previous_relative_path = ?8 AND current_root_id = ?9
                    AND current_root_generation = ?10 AND current_relative_path = ?11
                    AND status = ?12 AND journal_id = ?13
                    AND old_usn IS ?14 AND new_usn IS ?15 AND previous_carry_id IS ?16
             FROM library_persistent_journal_cross_root_lineage WHERE id = ?1",
            params![
                lineage.lineage_id,
                lineage.volume.volume_guid,
                lineage.volume.canonical_serial(),
                i64::from(lineage.file_reference.record_version()),
                lineage.file_reference.as_bytes(),
                lineage.previous_root_id,
                sqlite_integer(lineage.previous_root_generation.value(), "root generation")?,
                lineage.previous_relative_path,
                lineage.current_root_id,
                sqlite_integer(lineage.current_root_generation.value(), "root generation")?,
                lineage.current_relative_path,
                lineage_state_text(lineage.state),
                lineage.journal_id.to_canonical_text(),
                lineage.old_usn.map(JournalUsn::to_canonical_text),
                lineage.new_usn.map(JournalUsn::to_canonical_text),
                lineage.previous_carry_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !exact {
        return Err(invalid_batch(
            "The cross-root lineage identifier conflicts with durable evidence",
        ));
    }
    let owner = transaction
        .query_row(
            "SELECT root_id, root_generation
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            [&lineage.owner_source_range_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_error)?;
    let participant_role = if lineage.previous_root_id == owner.0
        && lineage.previous_root_generation.value()
            == u64::try_from(owner.1).map_err(|_| invalid_batch("Invalid owner generation"))?
    {
        "previous"
    } else if lineage.current_root_id == owner.0
        && lineage.current_root_generation.value()
            == u64::try_from(owner.1).map_err(|_| invalid_batch("Invalid owner generation"))?
    {
        "current"
    } else {
        return Err(invalid_batch(
            "The lineage participant does not match the owner root generation",
        ));
    };
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_cross_root_ranges(
               lineage_id, source_range_id, participant_role, enrolled_unix_ms
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(lineage_id, source_range_id) DO NOTHING",
            params![
                lineage.lineage_id,
                lineage.owner_source_range_id,
                participant_role,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    link_cross_root_queue_evidence(transaction, &lineage.lineage_id, enrolled_unix_ms)?;
    if lineage.state == PersistentJournalLineageState::Completed {
        let role_count = transaction
            .query_row(
                "SELECT COUNT(DISTINCT participant_role)
                 FROM library_persistent_journal_cross_root_ranges WHERE lineage_id = ?1",
                [&lineage.lineage_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        if role_count != 2 {
            return Err(invalid_batch(
                "Completed cross-root lineage requires both durable root ranges",
            ));
        }
    }
    Ok(())
}

fn link_cross_root_queue_evidence(
    transaction: &Transaction<'_>,
    lineage_id: &str,
    enrolled_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT owner_changes.change_id, ?2, peer_ranges.source_range_id, ?3
             FROM library_persistent_journal_cross_root_ranges AS owner_ranges
             JOIN library_persistent_journal_queue_lineage AS owner_changes
               ON owner_changes.source_range_id = owner_ranges.source_range_id
             JOIN library_persistent_journal_cross_root_ranges AS peer_ranges
               ON peer_ranges.lineage_id = owner_ranges.lineage_id
             WHERE owner_ranges.lineage_id = ?1
             ON CONFLICT(change_id, catch_up_source, catch_up_watermark) DO NOTHING",
            params![
                lineage_id,
                PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT recovery_changes.id, ?2, owner_ranges.source_range_id, ?3
             FROM library_persistent_journal_cross_root_lineage AS lineage
             JOIN library_persistent_journal_cross_root_ranges AS owner_ranges
               ON owner_ranges.lineage_id = lineage.id
             JOIN library_change_queue AS recovery_changes
               ON recovery_changes.root_id = lineage.previous_root_id
              AND recovery_changes.root_generation = lineage.previous_root_generation
              AND recovery_changes.relative_path = lineage.previous_relative_path
              AND recovery_changes.first_sequence = lineage.new_usn
              AND recovery_changes.most_recent_sequence = lineage.new_usn
             JOIN library_persistent_journal_queue_lineage AS recovery_ownership
               ON recovery_ownership.change_id = recovery_changes.id
             JOIN library_persistent_journal_source_ranges AS recovery_ranges
               ON recovery_ranges.id = recovery_ownership.source_range_id
              AND recovery_ranges.volume_guid = lineage.volume_guid
              AND recovery_ranges.volume_serial = lineage.volume_serial
              AND recovery_ranges.journal_id = lineage.journal_id
              AND CAST(recovery_ranges.requested_start_usn AS INTEGER)
                    <= CAST(lineage.new_usn AS INTEGER)
              AND CAST(lineage.new_usn AS INTEGER)
                    < CAST(recovery_ranges.covered_until_usn AS INTEGER)
             WHERE lineage.id = ?1
               AND lineage.previous_carry_id IS NOT NULL
               AND lineage.new_usn IS NOT NULL
             ON CONFLICT(change_id, catch_up_source, catch_up_watermark) DO NOTHING",
            params![
                lineage_id,
                PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn load_current_recovery_opening_boundary(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<LibraryRecoveryOpeningBoundary, ScanError> {
    let checkpoint = load_checkpoint(connection, root_id, root_generation)?.ok_or_else(|| {
        ScanError::new(
            "persistent_journal_recovery_boundary_unavailable",
            "A watcher gap cannot recover without a durable current journal checkpoint",
        )
    })?;
    let root = load_root_authority_for_key(connection, root_id, root_generation)?;
    if checkpoint.continuity != PersistentJournalContinuityState::Current
        || checkpoint.next_unread_usn != checkpoint.captured_exclusive_end
        || checkpoint.failure.is_some()
        || root.capability_state != "supported"
        || root.continuity_state != "current"
        || root.protocol_version != checkpoint.protocol_version
        || root.contract_version != checkpoint.contract_version
    {
        return Err(ScanError::new(
            "persistent_journal_recovery_boundary_unavailable",
            "A watcher gap cannot recover from a non-current journal boundary",
        ));
    }
    let boundary = LibraryRecoveryOpeningBoundary {
        volume: checkpoint.volume,
        root_file_reference: checkpoint.root_file_reference,
        journal_id: checkpoint.journal_id,
        next_usn: checkpoint.next_unread_usn,
        protocol_version: checkpoint.protocol_version,
        contract_version: checkpoint.contract_version,
    };
    boundary.validate()?;
    Ok(boundary)
}

pub(super) fn insert_watcher_gap_recovery_window(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    boundary: &LibraryRecoveryOpeningBoundary,
    failure: &LibraryChangeFailure,
    authorized_unix_ms: i64,
) -> Result<(), ScanError> {
    boundary.validate()?;
    let existing = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_persistent_journal_baselines
               WHERE root_id = ?1 AND root_generation = ?2 AND phase <> 'completed'
             )",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if existing {
        return Err(ScanError::new(
            "persistent_journal_recovery_window_busy",
            "The root already owns an unfinished journal recovery window",
        ));
    }
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, closing_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms, completed_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11,
                       'inventory', ?12, ?12, NULL)",
            params![
                sqlite_integer(change_id.value(), "recovery change ID")?,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
                authorized_unix_ms,
            ],
        )
        .map_err(database_error)?;
    let checkpoint_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'recovery_required', last_failure_code = ?1,
                 last_failure_message = ?2, updated_unix_ms = ?3
             WHERE root_id = ?4 AND root_generation = ?5
               AND volume_guid = ?6 AND volume_serial = ?7
               AND root_reference_version = ?8 AND root_file_reference = ?9
               AND journal_id = ?10 AND next_unread_usn = ?11
               AND captured_exclusive_end = ?11 AND protocol_version = ?12
               AND contract_version = ?13 AND continuity_state = 'current'
               AND last_failure_code IS NULL",
            params![
                failure.code,
                failure.message,
                authorized_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
            ],
        )
        .map_err(database_error)?;
    let root_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET continuity_state = 'recovery_required', updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND capability_state = 'supported' AND continuity_state = 'current'
               AND protocol_version = ?4 AND contract_version = ?5
               AND last_failure_code IS NULL",
            params![
                authorized_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
            ],
        )
        .map_err(database_error)?;
    if checkpoint_updated != 1 || root_updated != 1 {
        return Err(ScanError::new(
            "persistent_journal_recovery_boundary_raced",
            "The current journal boundary changed before watcher-gap recovery was authorized",
        ));
    }
    Ok(())
}

fn attach_recovery_window_if_available(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    boundary: Option<&LibraryRecoveryOpeningBoundary>,
    authorized_unix_ms: i64,
) -> Result<(), ScanError> {
    let Some(boundary) = boundary else {
        return Ok(());
    };
    boundary.validate()?;
    let existing = transaction
        .query_row(
            "SELECT change_id = ?3
                    AND volume_guid = ?4 AND volume_serial = ?5
                    AND root_reference_version = ?6 AND root_file_reference = ?7
                    AND journal_id = ?8 AND opening_next_usn = ?9
                    AND protocol_version = ?10 AND contract_version = ?11
                    AND phase = 'inventory'
             FROM library_persistent_journal_baselines
             WHERE root_id = ?1 AND root_generation = ?2 AND phase <> 'completed'",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                sqlite_integer(change_id.value(), "recovery change ID")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(false) = existing {
        return Err(ScanError::new(
            "persistent_journal_recovery_window_conflict",
            "The root already owns a different unfinished recovery window",
        ));
    }
    validate_recovery_checkpoint_against_opening_boundary(
        transaction,
        change_id,
        root_id,
        root_generation,
        boundary,
    )?;
    if existing == Some(true) {
        return Ok(());
    }
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, closing_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms, completed_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11,
                       'inventory', ?12, ?12, NULL)",
            params![
                sqlite_integer(change_id.value(), "recovery change ID")?,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
                authorized_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn validate_recovery_checkpoint_against_opening_boundary(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    boundary: &LibraryRecoveryOpeningBoundary,
) -> Result<(), ScanError> {
    let authority_reason = transaction
        .query_row(
            "SELECT reason FROM library_recovery_authorities WHERE change_id = ?1",
            [sqlite_integer(change_id.value(), "recovery change ID")?],
            |row| row.get::<_, String>(0),
        )
        .map_err(database_error)?;
    let replaces_checkpoint_journal = authority_reason == "journal_reset";
    let checkpoint = load_checkpoint(transaction, root_id, root_generation)?.ok_or_else(|| {
        ScanError::new(
            "persistent_journal_recovery_checkpoint_missing",
            "A journal recovery opening boundary has no durable checkpoint",
        )
    })?;
    if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
        || checkpoint.volume != boundary.volume
        || checkpoint.root_file_reference != boundary.root_file_reference
        || checkpoint.protocol_version != boundary.protocol_version
        || checkpoint.contract_version != boundary.contract_version
        || (replaces_checkpoint_journal && checkpoint.journal_id == boundary.journal_id)
        || (!replaces_checkpoint_journal
            && (checkpoint.journal_id != boundary.journal_id
                || boundary.next_usn < checkpoint.next_unread_usn
                || boundary.next_usn < checkpoint.captured_exclusive_end))
    {
        return Err(ScanError::new(
            "persistent_journal_recovery_opening_identity_drift",
            "The recovery opening boundary does not extend the durable root checkpoint",
        ));
    }
    Ok(())
}

pub(super) fn mark_persistent_journal_recovery_required(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    failure_code: &str,
    failure_message: &str,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'recovery_required', last_failure_code = ?1,
                 last_failure_message = ?2, updated_unix_ms = ?3
             WHERE root_id = ?4 AND root_generation = ?5
               AND continuity_state IN ('current', 'catching_up', 'recovery_required')",
            params![
                failure_code,
                failure_message,
                updated_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET continuity_state = 'recovery_required', updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND capability_state = 'supported'
               AND continuity_state IN ('current', 'catching_up', 'recovery_required')",
            params![
                updated_unix_ms,
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_checkpoint(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<PersistentJournalCheckpoint>, ScanError> {
    let row = connection
        .query_row(
            "SELECT volume_guid, volume_serial, root_reference_version, root_file_reference,
                    journal_id, next_unread_usn, captured_exclusive_end,
                    covered_catalog_revision, protocol_version, contract_version,
                    continuity_state, last_failure_code, last_failure_message, updated_unix_ms
             FROM library_persistent_journal_checkpoints
             WHERE root_id = ?1 AND root_generation = ?2",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, i64>(13)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let checkpoint = PersistentJournalCheckpoint {
        root_id: root_id.to_owned(),
        root_generation,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: row.0,
            volume_serial: parse_canonical_u64(&row.1, "volume serial")?,
        },
        root_file_reference: parse_file_reference(row.2, &row.3)?,
        journal_id: JournalIdentifier::parse_canonical(&row.4)?,
        next_unread_usn: JournalUsn::parse_canonical(&row.5)?,
        captured_exclusive_end: JournalUsn::parse_canonical(&row.6)?,
        covered_catalog_revision: parse_nonnegative_u64(row.7, "catalog revision")?,
        protocol_version: parse_u16(row.8, "journal protocol version")?,
        contract_version: parse_u16(row.9, "journal contract version")?,
        continuity: parse_continuity_state(&row.10)?,
        failure: parse_failure(row.11, row.12)?,
        updated_unix_ms: row.13,
    };
    checkpoint.validate()?;
    Ok(Some(checkpoint))
}

fn compare_and_swap_checkpoint(
    transaction: &Transaction<'_>,
    current: &PersistentJournalCheckpoint,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    let updated = transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints
             SET volume_guid = ?1, volume_serial = ?2,
                 root_reference_version = ?3, root_file_reference = ?4,
                 journal_id = ?5, next_unread_usn = ?6,
                 captured_exclusive_end = ?7, covered_catalog_revision = ?8,
                 protocol_version = ?9, contract_version = ?10,
                 continuity_state = ?11, last_failure_code = ?12,
                 last_failure_message = ?13, updated_unix_ms = ?14
             WHERE root_id = ?15 AND root_generation = ?16
               AND volume_guid = ?17 AND volume_serial = ?18
               AND root_reference_version = ?19 AND root_file_reference = ?20
               AND journal_id = ?21 AND next_unread_usn = ?22
               AND captured_exclusive_end = ?23 AND covered_catalog_revision = ?24
               AND protocol_version = ?25 AND contract_version = ?26
               AND continuity_state = ?27 AND last_failure_code IS ?28
               AND last_failure_message IS ?29 AND updated_unix_ms = ?30",
            params![
                checkpoint.volume.volume_guid,
                checkpoint.volume.canonical_serial(),
                i64::from(checkpoint.root_file_reference.record_version()),
                checkpoint.root_file_reference.as_bytes(),
                checkpoint.journal_id.to_canonical_text(),
                checkpoint.next_unread_usn.to_canonical_text(),
                checkpoint.captured_exclusive_end.to_canonical_text(),
                sqlite_integer(checkpoint.covered_catalog_revision, "catalog revision")?,
                i64::from(checkpoint.protocol_version),
                i64::from(checkpoint.contract_version),
                continuity_state_text(checkpoint.continuity),
                checkpoint
                    .failure
                    .as_ref()
                    .map(|failure| failure.code.as_str()),
                checkpoint
                    .failure
                    .as_ref()
                    .map(|failure| failure.message.as_str()),
                checkpoint.updated_unix_ms,
                current.root_id,
                sqlite_integer(current.root_generation.value(), "root generation")?,
                current.volume.volume_guid,
                current.volume.canonical_serial(),
                i64::from(current.root_file_reference.record_version()),
                current.root_file_reference.as_bytes(),
                current.journal_id.to_canonical_text(),
                current.next_unread_usn.to_canonical_text(),
                current.captured_exclusive_end.to_canonical_text(),
                sqlite_integer(current.covered_catalog_revision, "catalog revision")?,
                i64::from(current.protocol_version),
                i64::from(current.contract_version),
                continuity_state_text(current.continuity),
                current
                    .failure
                    .as_ref()
                    .map(|failure| failure.code.as_str()),
                current
                    .failure
                    .as_ref()
                    .map(|failure| failure.message.as_str()),
                current.updated_unix_ms,
            ],
        )
        .map_err(database_error)?;
    if updated == 1 {
        Ok(())
    } else {
        Err(invalid_batch(
            "The persistent journal checkpoint compare-and-swap raced",
        ))
    }
}

struct CheckpointRange {
    root_id: String,
    root_generation: LibraryRootGeneration,
    volume: PersistentJournalVolumeIdentity,
    journal_id: JournalIdentifier,
    requested_start_usn: JournalUsn,
    requested_end_usn: JournalUsn,
    covered_until_usn: JournalUsn,
    protocol_version: u16,
    contract_version: u16,
    state: PersistentJournalRangeState,
}

fn load_checkpoint_range(
    transaction: &Transaction<'_>,
    source_range_id: &str,
) -> Result<CheckpointRange, ScanError> {
    let row = transaction
        .query_row(
            "SELECT root_id, root_generation, volume_guid, volume_serial, journal_id,
                    requested_start_usn, requested_end_usn, covered_until_usn,
                    protocol_version, contract_version, status
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            [source_range_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| invalid_batch("The checkpoint source range does not exist"))?;
    Ok(CheckpointRange {
        root_id: row.0,
        root_generation: parse_root_generation(row.1)?,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: row.2,
            volume_serial: parse_canonical_u64(&row.3, "volume serial")?,
        },
        journal_id: JournalIdentifier::parse_canonical(&row.4)?,
        requested_start_usn: JournalUsn::parse_canonical(&row.5)?,
        requested_end_usn: JournalUsn::parse_canonical(&row.6)?,
        covered_until_usn: JournalUsn::parse_canonical(&row.7)?,
        protocol_version: parse_u16(row.8, "journal protocol version")?,
        contract_version: parse_u16(row.9, "journal contract version")?,
        state: parse_range_state(&row.10)?,
    })
}

struct RootAuthorityState {
    protocol_version: u16,
    contract_version: u16,
    capability_state: String,
    continuity_state: String,
    failure_code: Option<String>,
    failure_message: Option<String>,
    updated_unix_ms: i64,
}

fn load_root_authority(
    transaction: &Transaction<'_>,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<RootAuthorityState, ScanError> {
    load_root_authority_for_key(transaction, &checkpoint.root_id, checkpoint.root_generation)
}

fn load_root_authority_for_key(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<RootAuthorityState, ScanError> {
    connection
        .query_row(
            "SELECT journal.protocol_version, journal.contract_version,
                    journal.capability_state, journal.continuity_state,
                    journal.last_failure_code, journal.last_failure_message,
                    journal.updated_unix_ms
             FROM library_persistent_journal_root_state AS journal
             JOIN library_change_root_state AS active
               ON active.root_id = journal.root_id
              AND active.generation = journal.root_generation
              AND active.is_active = 1
             WHERE journal.root_id = ?1 AND journal.root_generation = ?2",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| {
                Ok(RootAuthorityState {
                    protocol_version: parse_u16(row.get(0)?, "journal protocol version")
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    contract_version: parse_u16(row.get(1)?, "journal contract version")
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    capability_state: row.get(2)?,
                    continuity_state: row.get(3)?,
                    failure_code: row.get(4)?,
                    failure_message: row.get(5)?,
                    updated_unix_ms: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "persistent_journal_root_authority_stale",
                "The checkpoint no longer owns a durable root authority row",
            )
        })
}

fn load_active_baselines(
    connection: &Connection,
) -> Result<Vec<PersistentJournalBaseline>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT baseline.change_id
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             WHERE baseline.phase <> 'completed' AND authority.retired_unix_ms IS NULL
             ORDER BY baseline.root_id, baseline.root_generation",
        )
        .map_err(database_error)?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    ids.into_iter()
        .map(|id| {
            let id = LibraryChangeId::new(sqlite_unsigned(id, "baseline change ID")?)
                .ok_or_else(|| invalid_storage("baseline change ID"))?;
            load_baseline(connection, id)?
                .ok_or_else(|| invalid_storage("persistent journal baseline"))
        })
        .collect()
}

fn load_baseline_for_root(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<PersistentJournalBaseline>, ScanError> {
    let change_id = connection
        .query_row(
            "SELECT change_id FROM library_persistent_journal_baselines
             WHERE root_id = ?1 AND root_generation = ?2",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    change_id
        .map(|value| {
            LibraryChangeId::new(sqlite_unsigned(value, "baseline change ID")?)
                .ok_or_else(|| invalid_storage("baseline change ID"))
        })
        .transpose()?
        .map(|change_id| load_baseline(connection, change_id))
        .transpose()
        .map(Option::flatten)
}

fn load_baseline(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<Option<PersistentJournalBaseline>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT authority.run_id, baseline.root_id, baseline.root_generation,
                    baseline.volume_guid, baseline.volume_serial,
                    baseline.root_reference_version, baseline.root_file_reference,
                    baseline.journal_id, baseline.opening_next_usn,
                    baseline.closing_next_usn, baseline.protocol_version,
                    baseline.contract_version, baseline.phase,
                    baseline.authorized_unix_ms, baseline.updated_unix_ms,
                    baseline.completed_unix_ms
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             WHERE baseline.change_id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID")?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let baseline = PersistentJournalBaseline {
        change_id,
        run_id: stored.0,
        root_id: stored.1,
        root_generation: parse_root_generation(stored.2)?,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: stored.3,
            volume_serial: parse_canonical_u64(&stored.4, "baseline volume serial")?,
        },
        root_file_reference: parse_file_reference(stored.5, &stored.6)?,
        journal_id: JournalIdentifier::parse_canonical(&stored.7)?,
        opening_next_usn: JournalUsn::parse_canonical(&stored.8)?,
        closing_next_usn: stored
            .9
            .as_deref()
            .map(JournalUsn::parse_canonical)
            .transpose()?,
        protocol_version: parse_u16(stored.10, "baseline protocol version")?,
        contract_version: parse_u16(stored.11, "baseline contract version")?,
        phase: parse_baseline_phase(&stored.12)?,
        authorized_unix_ms: stored.13,
        updated_unix_ms: stored.14,
        completed_unix_ms: stored.15,
    };
    baseline.validate()?;
    Ok(Some(baseline))
}

pub(super) fn baseline_closing_is_covered(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    let Some(baseline) = load_baseline(connection, change_id)? else {
        return Ok(false);
    };
    let Some(closing) = baseline.closing_next_usn else {
        return Ok(false);
    };
    if !matches!(
        baseline.phase,
        PersistentJournalBaselinePhase::Replay | PersistentJournalBaselinePhase::Absence
    ) {
        return Ok(false);
    }
    let authority_is_active = connection
        .query_row(
            "SELECT retired_unix_ms IS NULL FROM library_recovery_authorities
             WHERE change_id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID")?],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !authority_is_active {
        return Ok(false);
    }
    let Some(checkpoint) =
        load_checkpoint(connection, &baseline.root_id, baseline.root_generation)?
    else {
        return Ok(false);
    };
    let identity_matches = checkpoint.volume == baseline.volume
        && checkpoint.root_file_reference == baseline.root_file_reference
        && checkpoint.journal_id == baseline.journal_id
        && checkpoint.protocol_version == baseline.protocol_version
        && checkpoint.contract_version == baseline.contract_version
        && checkpoint.next_unread_usn >= closing;
    if !identity_matches {
        return Ok(false);
    }
    if closing == baseline.opening_next_usn {
        return Ok(true);
    }
    let has_unfinished_replay = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_persistent_journal_source_ranges AS ranges
               JOIN library_persistent_journal_range_lifecycle AS lifecycle
                 ON lifecycle.source_range_id = ranges.id
               WHERE ranges.root_id = ?1
                 AND ranges.root_generation = ?2
                 AND ranges.volume_guid = ?3
                 AND ranges.volume_serial = ?4
                 AND ranges.journal_id = ?5
                 AND CAST(ranges.requested_start_usn AS INTEGER) < ?6
                 AND CAST(ranges.covered_until_usn AS INTEGER) > ?7
                 AND lifecycle.lifecycle_state <> 'completed'
             )",
            params![
                baseline.root_id,
                sqlite_integer(baseline.root_generation.value(), "root generation")?,
                baseline.volume.volume_guid,
                baseline.volume.volume_serial.to_string(),
                baseline.journal_id.to_canonical_text(),
                closing.value(),
                baseline.opening_next_usn.value(),
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    Ok(!has_unfinished_replay)
}

fn parse_baseline_phase(value: &str) -> Result<PersistentJournalBaselinePhase, ScanError> {
    match value {
        "inventory" => Ok(PersistentJournalBaselinePhase::Inventory),
        "replay" => Ok(PersistentJournalBaselinePhase::Replay),
        "absence" => Ok(PersistentJournalBaselinePhase::Absence),
        "completed" => Ok(PersistentJournalBaselinePhase::Completed),
        _ => Err(invalid_storage("persistent journal baseline phase")),
    }
}

fn parse_root_generation(value: i64) -> Result<LibraryRootGeneration, ScanError> {
    let value = parse_nonnegative_u64(value, "root generation")?;
    LibraryRootGeneration::new(value).ok_or_else(|| invalid_storage("root generation"))
}

fn parse_nonnegative_u64(value: i64, name: &str) -> Result<u64, ScanError> {
    u64::try_from(value).map_err(|_| invalid_storage(name))
}

fn parse_canonical_u64(value: &str, name: &str) -> Result<u64, ScanError> {
    let parsed = value.parse::<u64>().map_err(|_| invalid_storage(name))?;
    if parsed.to_string() != value {
        return Err(invalid_storage(name));
    }
    Ok(parsed)
}

fn parse_u16(value: i64, name: &str) -> Result<u16, ScanError> {
    u16::try_from(value).map_err(|_| invalid_storage(name))
}

fn parse_file_reference(version: i64, bytes: &[u8]) -> Result<JournalFileReference, ScanError> {
    let reference = JournalFileReference::from_bytes(bytes)?;
    if i64::from(reference.record_version()) != version {
        return Err(invalid_storage("journal file reference"));
    }
    Ok(reference)
}

fn parse_capability_state(value: &str) -> Result<PersistentJournalCapabilityState, ScanError> {
    match value {
        "unknown" => Ok(PersistentJournalCapabilityState::Unknown),
        "supported" => Ok(PersistentJournalCapabilityState::Supported),
        "live_only" => Ok(PersistentJournalCapabilityState::LiveOnly),
        _ => Err(invalid_storage("journal capability state")),
    }
}

fn parse_continuity_state(value: &str) -> Result<PersistentJournalContinuityState, ScanError> {
    match value {
        "baseline_required" => Ok(PersistentJournalContinuityState::BaselineRequired),
        "catching_up" => Ok(PersistentJournalContinuityState::CatchingUp),
        "current" => Ok(PersistentJournalContinuityState::Current),
        "recovery_required" => Ok(PersistentJournalContinuityState::RecoveryRequired),
        "live_only" => Ok(PersistentJournalContinuityState::LiveOnly),
        "unavailable" => Ok(PersistentJournalContinuityState::Unavailable),
        _ => Err(invalid_storage("journal continuity state")),
    }
}

fn parse_range_state(value: &str) -> Result<PersistentJournalRangeState, ScanError> {
    match value {
        "enrolled" => Ok(PersistentJournalRangeState::Enrolled),
        "checkpointed" => Ok(PersistentJournalRangeState::Checkpointed),
        "superseded" => Ok(PersistentJournalRangeState::Superseded),
        _ => Err(invalid_storage("journal source range state")),
    }
}

fn parse_failure(
    code: Option<String>,
    message: Option<String>,
) -> Result<Option<PersistentJournalFailure>, ScanError> {
    match (code, message) {
        (None, None) => Ok(None),
        (Some(code), Some(message)) => Ok(Some(PersistentJournalFailure { code, message })),
        _ => Err(invalid_storage("journal failure")),
    }
}

fn capability_state_text(value: PersistentJournalCapabilityState) -> &'static str {
    match value {
        PersistentJournalCapabilityState::Unknown => "unknown",
        PersistentJournalCapabilityState::Supported => "supported",
        PersistentJournalCapabilityState::LiveOnly => "live_only",
    }
}

fn continuity_state_text(value: PersistentJournalContinuityState) -> &'static str {
    match value {
        PersistentJournalContinuityState::BaselineRequired => "baseline_required",
        PersistentJournalContinuityState::CatchingUp => "catching_up",
        PersistentJournalContinuityState::Current => "current",
        PersistentJournalContinuityState::RecoveryRequired => "recovery_required",
        PersistentJournalContinuityState::LiveOnly => "live_only",
        PersistentJournalContinuityState::Unavailable => "unavailable",
    }
}

fn lineage_state_text(value: PersistentJournalLineageState) -> &'static str {
    match value {
        PersistentJournalLineageState::Pending => "pending",
        PersistentJournalLineageState::Completed => "completed",
        PersistentJournalLineageState::Superseded => "superseded",
    }
}

fn failure_columns(failure: Option<&PersistentJournalFailure>) -> (Option<&str>, Option<&str>) {
    match failure {
        Some(failure) => (Some(&failure.code), Some(&failure.message)),
        None => (None, None),
    }
}

fn map_queue_backpressure(error: ScanError, code: &str, message: &str) -> ScanError {
    if error.code == "change_queue_backpressure" {
        ScanError::new(code, message)
    } else {
        error
    }
}

fn invalid_batch(message: &str) -> ScanError {
    ScanError::new("persistent_journal_enrollment_invalid", message)
}

fn invalid_storage(name: &str) -> ScanError {
    ScanError::new(
        "catalog_persistent_journal_contract_unverifiable",
        format!("The durable {name} is outside the persistent journal contract"),
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::{TransactionBehavior, params};
    use tempfile::tempdir;

    use crate::domain::{
        JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeFailure,
        LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane, LibraryChangeOrigin,
        LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryOpeningBoundary,
        LibraryRootGeneration, MetadataInventoryFrontierEntry, MetadataInventoryPage,
        MetadataInventoryRunRequest, MetadataInventoryScope, PersistentJournalBaseline,
        PersistentJournalBaselineClosingBoundary, PersistentJournalBaselinePhase,
        PersistentJournalBaselineStartRequest, PersistentJournalCapability,
        PersistentJournalCapabilityState, PersistentJournalCheckpoint,
        PersistentJournalContinuityState, PersistentJournalCrossRootLineage,
        PersistentJournalEnrollmentBatch, PersistentJournalFailure, PersistentJournalLineageState,
        PersistentJournalPendingRename, PersistentJournalRangeState, PersistentJournalRootFailure,
        PersistentJournalRootFailureKind, PersistentJournalSourceRange,
        PersistentJournalVolumeBatch, PersistentJournalVolumeIdentity, PersistentJournalVolumePage,
        ScanRequest, persistent_journal_batch_id, persistent_journal_batch_payload,
        persistent_journal_pending_rename_id,
    };
    use crate::ports::{
        CatalogRepository, LibraryChangeQueue, MetadataInventoryRepository,
        PersistentJournalRepository,
    };

    use super::super::{SqliteCatalog, sqlite_integer};

    #[test]
    fn one_time_baseline_admission_is_atomic_and_idempotent() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        require_baseline_root(&mut catalog, "root-a");
        let request = baseline_request("root-a", 10);

        let first = catalog
            .begin_persistent_journal_baseline(&request, policy())
            .expect("admit baseline");
        let replay = catalog
            .begin_persistent_journal_baseline(&request, policy())
            .expect("replay baseline admission");
        let counts: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_change_queue),
                   (SELECT COUNT(*) FROM library_recovery_authorities),
                   (SELECT COUNT(*) FROM library_persistent_journal_baselines)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("baseline durable counts");

        assert_eq!(first, replay);
        assert_eq!(first.phase, PersistentJournalBaselinePhase::Inventory);
        assert_eq!(counts, (1, 1, 1));
        assert_eq!(
            catalog
                .load_persistent_journal_baselines()
                .expect("active baselines"),
            vec![first]
        );
    }

    #[test]
    fn first_import_opening_boundary_survives_restart_before_enumeration() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let request = first_import_scan_request("first-import-after-opening", "root-first-a");
        let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
        catalog
            .begin_scan(&request, "root-first-a", &request.root_path)
            .expect("begin first import");
        let baseline = first_import_baseline_request(&request, "root-first-a", 10);
        catalog
            .begin_persistent_journal_baseline(&baseline, policy())
            .expect("persist opening boundary before enumeration");
        drop(catalog);

        let mut reopened = SqliteCatalog::open(path).expect("reopen after opening boundary");
        assert!(
            reopened
                .first_import_change_capture_is_ready(
                    &request.scan_id,
                    "root-first-a",
                    LibraryRootGeneration::initial(),
                )
                .expect("reload first-import authority")
        );
        let resumed = reopened
            .resume_scan(&request, "root-first-a", &request.root_path)
            .expect("resume the same foreground scan");
        assert_eq!(resumed.visited_entries, 0);
        assert_eq!(
            reopened
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("metadata inventory count"),
            0
        );
    }

    #[test]
    fn first_import_published_baseline_survives_restart_without_second_inventory() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let request = first_import_scan_request("first-import-after-publish", "root-first-c");
        let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
        catalog
            .begin_scan(&request, "root-first-c", &request.root_path)
            .expect("begin first import");
        catalog
            .begin_persistent_journal_baseline(
                &first_import_baseline_request(&request, "root-first-c", 10),
                policy(),
            )
            .expect("persist opening boundary");
        catalog
            .publish_scan(&request.scan_id, "root-first-c", 0, 0)
            .expect("publish first inventory as baseline");
        drop(catalog);

        let reopened = SqliteCatalog::open(path).expect("reopen after first publication");
        assert!(
            reopened
                .metadata_inventory_is_waiting_for_closing_boundary(&request.scan_id)
                .expect("first import awaits only the closing boundary")
        );
        assert_eq!(
            reopened
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("metadata inventory count"),
            0
        );
    }

    #[test]
    fn first_import_replay_restart_waits_for_the_replayed_live_queue() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let request = first_import_scan_request("first-import-replay", "root-first-replay");
        let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
        catalog
            .begin_scan(&request, "root-first-replay", &request.root_path)
            .expect("begin first import");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &first_import_baseline_request(&request, "root-first-replay", 10),
                policy(),
            )
            .expect("persist opening boundary");
        catalog
            .publish_scan(&request.scan_id, "root-first-replay", 0, 0)
            .expect("publish first inventory");
        catalog
            .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
                baseline.change_id,
                10,
            ))
            .expect("enter replay phase with an empty journal range");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-first-replay".to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "during-scan.png".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 2_001,
                    most_recent_observed_unix_ms: 2_001,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                2_001,
                policy(),
            )
            .expect("persist live work before the replay-phase restart");
        drop(catalog);

        let mut reopened = SqliteCatalog::open(path).expect("reopen during replay");
        assert!(
            !reopened
                .finalize_ready_first_import_journal_baseline(3_000)
                .expect("pending live work must block first-import completion")
        );
        let live = reopened
            .lease_path_library_changes_in_lane(
                "root-first-replay",
                LibraryRootGeneration::initial(),
                LibraryChangeLane::Live,
                3_001,
                policy(),
            )
            .expect("lease persisted live work after restart")
            .pop()
            .expect("persisted live work");
        let catalog_revision = reopened
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("catalog revision after first publication");
        reopened
            .complete_library_change(
                live.change.id,
                live.lease_generation,
                u64::try_from(catalog_revision).expect("non-negative catalog revision"),
                3_002,
            )
            .expect("complete replayed live work");
        assert!(
            reopened
                .finalize_ready_first_import_journal_baseline(3_003)
                .expect("finalize first-import replay")
        );
        assert!(
            reopened
                .load_persistent_journal_baselines()
                .expect("load active baselines after completion")
                .is_empty()
        );
        let (baseline_phase, authority_retired_unix_ms) = reopened
            .connection
            .query_row(
                "SELECT baseline.phase, authority.retired_unix_ms
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = baseline.change_id
                 WHERE baseline.change_id = ?1",
                [i64::try_from(baseline.change_id.value()).expect("stored change ID")],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("load durable completed baseline");
        assert_eq!(baseline_phase, "completed");
        assert_eq!(authority_retired_unix_ms, Some(3_003));
        assert_eq!(
            reopened
                .load_persistent_journal_checkpoint(
                    "root-first-replay",
                    LibraryRootGeneration::initial(),
                )
                .expect("load current checkpoint")
                .expect("current checkpoint")
                .continuity,
            PersistentJournalContinuityState::Current
        );
    }

    #[test]
    fn typed_current_root_failures_admit_only_allowlisted_recovery_atomically() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let cases = [
            (
                "reset",
                PersistentJournalRootFailureKind::JournalReset,
                "journal_reset",
                true,
            ),
            (
                "trim",
                PersistentJournalRootFailureKind::JournalTrim,
                "journal_trim",
                true,
            ),
            (
                "reconstruction",
                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                "journal_reconstruction_failure",
                true,
            ),
            (
                "containment",
                PersistentJournalRootFailureKind::ContainmentFailure,
                "containment_failure",
                true,
            ),
            (
                "broker",
                PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
                "broker_after_current_failure",
                false,
            ),
        ];
        let roots = cases
            .iter()
            .map(|(root, _, _, _)| *root)
            .collect::<Vec<_>>();
        let mut catalog = catalog_with_roots(path.clone(), &roots);
        for root in &roots {
            current_root(&mut catalog, root);
        }

        for (index, (root, kind, reason, has_opening)) in cases.into_iter().enumerate() {
            let opening_boundary = has_opening.then(|| LibraryRecoveryOpeningBoundary {
                volume: volume(),
                root_file_reference: JournalFileReference::V2([1; 8]),
                journal_id: JournalIdentifier::new(
                    if kind == PersistentJournalRootFailureKind::JournalReset {
                        10
                    } else {
                        9
                    },
                )
                .expect("opening journal ID"),
                next_usn: JournalUsn::new(
                    if kind == PersistentJournalRootFailureKind::JournalReset {
                        5
                    } else {
                        20
                    },
                )
                .expect("opening USN"),
                protocol_version: 1,
                contract_version: 1,
            });
            let failure = PersistentJournalRootFailure {
                root_id: root.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                kind,
                failure: PersistentJournalFailure {
                    code: format!("typed-{root}"),
                    message: format!("Typed failure for {root}"),
                },
                opening_boundary,
            };
            let failed_unix_ms = 1_000 + i64::try_from(index).expect("case time");
            let first = catalog
                .persist_persistent_journal_root_failure(&failure, failed_unix_ms, policy())
                .expect("admit typed root failure")
                .expect("allowlisted recovery change");
            let replay = catalog
                .persist_persistent_journal_root_failure(&failure, failed_unix_ms, policy())
                .expect("replay typed root failure")
                .expect("idempotent recovery change");
            let stored: (String, String, String, String, String, i64) = catalog
                .connection
                .query_row(
                    "SELECT authority.reason, queue.status, lane.lane,
                            checkpoint.continuity_state, checkpoint.next_unread_usn,
                            (SELECT COUNT(*) FROM library_persistent_journal_baselines AS window
                             WHERE window.change_id = authority.change_id)
                     FROM library_recovery_authorities AS authority
                     JOIN library_change_queue AS queue ON queue.id = authority.change_id
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     JOIN library_persistent_journal_checkpoints AS checkpoint
                       ON checkpoint.root_id = authority.root_id
                      AND checkpoint.root_generation = authority.root_generation
                     WHERE authority.change_id = ?1",
                    [sqlite_integer(first.value(), "recovery change ID").expect("change ID")],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .expect("typed recovery state");
            assert_eq!(first, replay);
            assert_eq!(stored.0, reason);
            assert_eq!(stored.1, "pending");
            assert_eq!(stored.2, "p2_recovery");
            assert_eq!(stored.3, "recovery_required");
            assert_eq!(stored.4, "10");
            assert_eq!(stored.5, i64::from(has_opening));
            assert_eq!(
                catalog
                    .has_ready_metadata_inventory_recovery(
                        root,
                        LibraryRootGeneration::initial(),
                        failed_unix_ms,
                        policy(),
                    )
                    .expect("ready recovery state"),
                has_opening,
            );
        }

        let counts: (i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_recovery_authorities),
                   (SELECT COUNT(*) FROM library_change_queue
                    WHERE status IN ('pending', 'leased', 'retry_wait'))",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("idempotent recovery counts");
        assert_eq!(counts, (5, 5));

        drop(catalog);
        let mut catalog =
            SqliteCatalog::open(path.clone()).expect("reopen active typed recoveries");
        let reset = catalog
            .load_persistent_journal_baselines()
            .expect("load active typed recoveries")
            .into_iter()
            .find(|baseline| baseline.root_id == "reset")
            .expect("reset recovery baseline");
        assert_eq!(
            reset.journal_id,
            JournalIdentifier::new(10).expect("journal ID")
        );
        assert_eq!(
            reset.opening_next_usn,
            JournalUsn::new(5).expect("opening USN")
        );
        let replay = catalog
            .capture_persistent_journal_baseline_closing_boundary(
                &PersistentJournalBaselineClosingBoundary {
                    change_id: reset.change_id,
                    volume: reset.volume.clone(),
                    root_file_reference: reset.root_file_reference.clone(),
                    journal_id: reset.journal_id,
                    closing_next_usn: JournalUsn::new(6).expect("closing USN"),
                    protocol_version: reset.protocol_version,
                    captured_unix_ms: 2_000,
                },
            )
            .expect("capture reset closing boundary");
        assert_eq!(replay.phase, PersistentJournalBaselinePhase::Replay);
        drop(catalog);

        let catalog = SqliteCatalog::open(path).expect("reopen rebased reset recovery");
        let checkpoint = catalog
            .load_persistent_journal_checkpoint("reset", LibraryRootGeneration::initial())
            .expect("load rebased reset checkpoint")
            .expect("rebased reset checkpoint");
        assert_eq!(
            checkpoint.journal_id,
            JournalIdentifier::new(10).expect("journal ID")
        );
        assert_eq!(
            checkpoint.next_unread_usn,
            JournalUsn::new(5).expect("replay start")
        );
        assert_eq!(
            checkpoint.captured_exclusive_end,
            JournalUsn::new(6).expect("captured closing")
        );
        assert_eq!(
            checkpoint.continuity,
            PersistentJournalContinuityState::CatchingUp
        );
    }

    #[test]
    fn journal_trim_atomically_transfers_pending_live_gap_to_p2_recovery() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        let generation = LibraryRootGeneration::initial();
        current_root(&mut catalog, "root-a");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: generation,
                    kind: LibraryChangeIntentKind::FreshnessUnknown,
                    scope: LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 900,
                    most_recent_observed_unix_ms: 900,
                    first_sequence: 10,
                    most_recent_sequence: 10,
                    coalesced_observation_count: 1,
                }],
                900,
                policy(),
            )
            .expect("enqueue root live gap");
        let live = catalog
            .lease_live_authoritative_library_change("root-a", generation, 900, policy())
            .expect("lease live gap")
            .expect("live gap lease");
        assert_eq!(
            catalog
                .promote_live_watcher_gap_to_metadata_inventory(
                    live.change.id,
                    live.lease_generation,
                    &LibraryChangeFailure {
                        code: "metadata_inventory_required".to_owned(),
                        message: "The live gap requires durable ownership".to_owned(),
                    },
                    901,
                    policy(),
                )
                .expect("persist pending journal claim"),
            crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
        );
        let pending: (String, String, String, String) = catalog
            .connection
            .query_row(
                "SELECT gap.status, gap.origin, lane.lane, claim.consumer_kind
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 JOIN library_live_gap_recovery_claims AS claim
                   ON claim.gap_change_id = gap.id
                 WHERE gap.id = ?1",
                [sqlite_integer(live.change.id.value(), "live gap ID").expect("live gap ID")],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("pending live-gap claim");
        assert_eq!(
            pending,
            (
                "retry_wait".to_owned(),
                "live_notification".to_owned(),
                "p0_live".to_owned(),
                "pending_journal".to_owned(),
            )
        );

        let recovery_id = catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: "root-a".to_owned(),
                    root_generation: generation,
                    kind: PersistentJournalRootFailureKind::JournalTrim,
                    failure: PersistentJournalFailure {
                        code: "journal_trimmed_before_live_gap".to_owned(),
                        message: "The journal can no longer cover the opening live-gap USN"
                            .to_owned(),
                    },
                    opening_boundary: Some(LibraryRecoveryOpeningBoundary {
                        volume: volume(),
                        root_file_reference: JournalFileReference::V2([1; 8]),
                        journal_id: JournalIdentifier::new(9).expect("journal ID"),
                        next_usn: JournalUsn::new(10).expect("opening USN"),
                        protocol_version: 1,
                        contract_version: 1,
                    }),
                },
                902,
                policy(),
            )
            .expect("persist journal trim recovery")
            .expect("P2 recovery control");
        let recovery_id_sql =
            sqlite_integer(recovery_id.value(), "recovery change ID").expect("recovery change ID");
        type TransferredLiveGapEvidence = (
            String,
            String,
            String,
            i64,
            String,
            i64,
            Option<String>,
            String,
            String,
            String,
            String,
            String,
        );
        let transferred: TransferredLiveGapEvidence = catalog
            .connection
            .query_row(
                "SELECT gap.status, gap.origin, gap_lane.lane,
                        gap.superseded_by_change_id,
                        claim.consumer_kind, claim.recovery_change_id,
                        claim.source_range_id,
                        recovery.origin, recovery_lane.lane,
                        recovery.intent_kind, recovery.scope, authority.reason
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS gap_lane ON gap_lane.change_id = gap.id
                 JOIN library_live_gap_recovery_claims AS claim
                   ON claim.gap_change_id = gap.id
                 JOIN library_change_queue AS recovery
                   ON recovery.id = claim.recovery_change_id
                 JOIN library_change_queue_lanes AS recovery_lane
                   ON recovery_lane.change_id = recovery.id
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = recovery.id
                 WHERE gap.id = ?1",
                [sqlite_integer(live.change.id.value(), "live gap ID").expect("live gap ID")],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                    ))
                },
            )
            .expect("transferred live-gap lineage");
        assert_eq!(
            transferred,
            (
                "superseded".to_owned(),
                "live_notification".to_owned(),
                "p0_live".to_owned(),
                recovery_id_sql,
                "metadata_inventory_control".to_owned(),
                recovery_id_sql,
                None,
                "metadata_inventory".to_owned(),
                "p2_recovery".to_owned(),
                "freshness_unknown".to_owned(),
                "root".to_owned(),
                "journal_trim".to_owned(),
            )
        );
        let naked_p1_count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = 'root-a' AND lane.lane = 'p1_journal'
                   AND queue.origin = 'startup_catch_up'
                   AND queue.intent_kind = 'freshness_unknown'
                   AND queue.scope = 'root' AND queue.relative_path = ''",
                [],
                |row| row.get(0),
            )
            .expect("naked P1 gap count");
        assert_eq!(naked_p1_count, 0);
        super::super::migrations::migrate_schema(&mut catalog.connection)
            .expect("validate transferred live-gap contract");
    }

    #[test]
    fn transient_nonrecoverable_cancelled_and_live_only_fail_closed_without_p2() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["transient", "nonrecoverable", "cancelled", "live-only"],
        );
        for root in ["transient", "nonrecoverable", "cancelled"] {
            current_root(&mut catalog, root);
        }
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: "live-only".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalCapabilityState::LiveOnly,
                continuity: PersistentJournalContinuityState::LiveOnly,
                failure: Some(PersistentJournalFailure {
                    code: "live_only".to_owned(),
                    message: "Persistent continuity is unavailable".to_owned(),
                }),
                updated_unix_ms: 500,
            })
            .expect("live-only capability");
        for (root, kind) in [
            ("transient", PersistentJournalRootFailureKind::Transient),
            (
                "nonrecoverable",
                PersistentJournalRootFailureKind::NonRecoverable,
            ),
            ("cancelled", PersistentJournalRootFailureKind::Cancelled),
            (
                "live-only",
                PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
            ),
        ] {
            let result = catalog
                .persist_persistent_journal_root_failure(
                    &PersistentJournalRootFailure {
                        root_id: root.to_owned(),
                        root_generation: LibraryRootGeneration::initial(),
                        kind,
                        failure: PersistentJournalFailure {
                            code: format!("failure-{root}"),
                            message: format!("Failure for {root}"),
                        },
                        opening_boundary: None,
                    },
                    1_000,
                    policy(),
                )
                .expect("fail-closed root failure");
            assert_eq!(result, None);
        }
        let states = catalog
            .connection
            .prepare(
                "SELECT root_id, continuity_state FROM library_persistent_journal_root_state
                 WHERE root_id IN ('transient', 'nonrecoverable', 'cancelled', 'live-only')
                 ORDER BY root_id",
            )
            .expect("root states")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("root state rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("root states");
        assert_eq!(
            states,
            vec![
                ("cancelled".to_owned(), "current".to_owned()),
                ("live-only".to_owned(), "live_only".to_owned()),
                ("nonrecoverable".to_owned(), "recovery_required".to_owned(),),
                ("transient".to_owned(), "current".to_owned()),
            ]
        );
        let p2_count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_recovery_authorities",
                [],
                |row| row.get(0),
            )
            .expect("P2 authority count");
        assert_eq!(p2_count, 0);
    }

    #[test]
    fn typed_root_failure_rolls_back_state_control_and_window_when_authority_insert_fails() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        current_root(&mut catalog, "root-a");
        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER inject_journal_failure_authority_failure
                 BEFORE INSERT ON library_recovery_authorities
                 BEGIN
                   SELECT RAISE(ABORT, 'injected journal failure authority failure');
                 END;",
            )
            .expect("install failure fixture");
        let error = catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: "root-a".to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: PersistentJournalRootFailureKind::JournalReset,
                    failure: PersistentJournalFailure {
                        code: "persistent_journal_reset".to_owned(),
                        message: "The journal identity changed".to_owned(),
                    },
                    opening_boundary: Some(LibraryRecoveryOpeningBoundary {
                        volume: volume(),
                        root_file_reference: JournalFileReference::V2([1; 8]),
                        journal_id: JournalIdentifier::new(10).expect("new journal ID"),
                        next_usn: JournalUsn::new(20).expect("opening USN"),
                        protocol_version: 1,
                        contract_version: 1,
                    }),
                },
                1_000,
                policy(),
            )
            .expect_err("authority insertion must roll back the complete admission");
        assert_eq!(error.code, "metadata_inventory_recovery_authority_rejected");
        let rollback: (String, Option<String>, String, i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT checkpoint.continuity_state, checkpoint.last_failure_code,
                        root.continuity_state,
                        (SELECT COUNT(*) FROM library_change_queue),
                        (SELECT COUNT(*) FROM library_recovery_authorities),
                        (SELECT COUNT(*) FROM library_persistent_journal_baselines)
                 FROM library_persistent_journal_checkpoints AS checkpoint
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = checkpoint.root_id
                  AND root.root_generation = checkpoint.root_generation
                 WHERE checkpoint.root_id = 'root-a'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("rolled back journal failure admission");
        assert_eq!(
            rollback,
            ("current".to_owned(), None, "current".to_owned(), 0, 0, 0)
        );
    }

    #[test]
    fn recovered_fresh_opening_atomically_activates_pending_failure_authority() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        current_root(&mut catalog, "root-a");
        let failure = PersistentJournalRootFailure {
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
            failure: PersistentJournalFailure {
                code: "persistent_journal_broker_after_current_failure".to_owned(),
                message: "The broker became unavailable after Current".to_owned(),
            },
            opening_boundary: None,
        };
        let change_id = catalog
            .persist_persistent_journal_root_failure(&failure, 1_000, policy())
            .expect("persist pending broker recovery")
            .expect("pending broker recovery change");
        assert!(
            !catalog
                .has_ready_metadata_inventory_recovery(
                    "root-a",
                    LibraryRootGeneration::initial(),
                    1_000,
                    policy(),
                )
                .expect("pending recovery readiness")
        );

        let boundary = LibraryRecoveryOpeningBoundary {
            volume: volume(),
            root_file_reference: JournalFileReference::V2([1; 8]),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            next_usn: JournalUsn::new(20).expect("fresh opening USN"),
            protocol_version: 1,
            contract_version: 1,
        };
        let mut drift = boundary.clone();
        drift.volume.volume_serial = 7;
        let drift_error = catalog
            .attach_persistent_journal_recovery_opening_boundary(
                "root-a",
                LibraryRootGeneration::initial(),
                &drift,
                1_001,
            )
            .expect_err("opening identity drift must fail closed");
        assert_eq!(
            drift_error.code,
            "persistent_journal_recovery_opening_identity_drift"
        );

        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER inject_recovery_opening_failure
                 BEFORE INSERT ON library_persistent_journal_baselines
                 BEGIN
                   SELECT RAISE(ABORT, 'injected recovery opening failure');
                 END;",
            )
            .expect("install opening rollback fixture");
        catalog
            .attach_persistent_journal_recovery_opening_boundary(
                "root-a",
                LibraryRootGeneration::initial(),
                &boundary,
                1_001,
            )
            .expect_err("window insertion failure must roll back");
        assert!(
            !catalog
                .has_ready_metadata_inventory_recovery(
                    "root-a",
                    LibraryRootGeneration::initial(),
                    1_001,
                    policy(),
                )
                .expect("rolled back recovery readiness")
        );
        catalog
            .connection
            .execute_batch("DROP TRIGGER inject_recovery_opening_failure;")
            .expect("remove opening rollback fixture");

        let first = catalog
            .attach_persistent_journal_recovery_opening_boundary(
                "root-a",
                LibraryRootGeneration::initial(),
                &boundary,
                1_002,
            )
            .expect("attach recovered opening")
            .expect("recovered authority");
        let replay = catalog
            .attach_persistent_journal_recovery_opening_boundary(
                "root-a",
                LibraryRootGeneration::initial(),
                &boundary,
                1_002,
            )
            .expect("replay recovered opening")
            .expect("replayed authority");
        assert_eq!(first, change_id);
        assert_eq!(replay, change_id);
        assert!(
            catalog
                .has_ready_metadata_inventory_recovery(
                    "root-a",
                    LibraryRootGeneration::initial(),
                    1_002,
                    policy(),
                )
                .expect("activated recovery readiness")
        );
        let window_count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_baselines
                 WHERE change_id = ?1 AND phase = 'inventory'",
                [sqlite_integer(change_id.value(), "recovery change ID").expect("change ID")],
                |row| row.get(0),
            )
            .expect("recovery window count");
        assert_eq!(window_count, 1);
    }

    #[test]
    fn baseline_closing_boundary_rejects_identity_drift_and_is_idempotent() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        require_baseline_root(&mut catalog, "root-a");
        let baseline = catalog
            .begin_persistent_journal_baseline(&baseline_request("root-a", 10), policy())
            .expect("admit baseline");
        let mut mismatch = closing_boundary(baseline.change_id, 20);
        mismatch.journal_id = JournalIdentifier::new(10).expect("other journal ID");
        let error = catalog
            .capture_persistent_journal_baseline_closing_boundary(&mismatch)
            .expect_err("journal drift must fail closed");
        let checkpoint_count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_checkpoints",
                [],
                |row| row.get(0),
            )
            .expect("checkpoint count after mismatch");
        assert_eq!(error.code, "persistent_journal_baseline_closing_mismatch");
        assert_eq!(checkpoint_count, 0);

        let boundary = closing_boundary(baseline.change_id, 20);
        let captured = catalog
            .capture_persistent_journal_baseline_closing_boundary(&boundary)
            .expect("capture closing boundary");
        let replay = catalog
            .capture_persistent_journal_baseline_closing_boundary(&boundary)
            .expect("replay closing capture");
        let checkpoint = catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
            .expect("load provisional checkpoint")
            .expect("provisional checkpoint");

        assert_eq!(captured, replay);
        assert_eq!(captured.phase, PersistentJournalBaselinePhase::Replay);
        assert_eq!(
            captured.closing_next_usn,
            Some(JournalUsn::new(20).expect("USN"))
        );
        assert_eq!(
            checkpoint.next_unread_usn,
            JournalUsn::new(10).expect("USN")
        );
        assert_eq!(
            checkpoint.captured_exclusive_end,
            JournalUsn::new(20).expect("USN")
        );
        assert!(
            !catalog
                .persistent_journal_baseline_closing_is_covered(baseline.change_id)
                .expect("coverage state")
        );
    }

    #[test]
    fn empty_baseline_interval_needs_no_fsctl_range_to_cover_closing() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        require_baseline_root(&mut catalog, "root-a");
        let baseline = catalog
            .begin_persistent_journal_baseline(&baseline_request("root-a", 10), policy())
            .expect("admit baseline");
        catalog
            .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
                baseline.change_id,
                10,
            ))
            .expect("capture empty closing interval");

        assert!(
            catalog
                .persistent_journal_baseline_closing_is_covered(baseline.change_id)
                .expect("empty interval coverage")
        );
        let range_count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_source_ranges",
                [],
                |row| row.get(0),
            )
            .expect("source range count");
        assert_eq!(range_count, 0);
    }

    #[test]
    fn watcher_gap_closing_identity_and_transaction_failure_preserve_opening_window() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        current_root(&mut catalog, "root-a");
        let baseline = promote_watcher_gap(&mut catalog, "root-a");
        let mut mismatch = closing_boundary(baseline.change_id, 20);
        mismatch.volume.volume_serial = 99;
        let identity_error = catalog
            .capture_persistent_journal_baseline_closing_boundary(&mismatch)
            .expect_err("identity drift must not close the watcher window");
        assert_eq!(
            identity_error.code,
            "persistent_journal_baseline_closing_mismatch"
        );
        let initial: (String, Option<String>, String, String, String, String) = catalog
            .connection
            .query_row(
                "SELECT baseline.phase, baseline.closing_next_usn,
                        checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                        checkpoint.continuity_state, root.continuity_state
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_persistent_journal_checkpoints AS checkpoint
                   ON checkpoint.root_id = baseline.root_id
                  AND checkpoint.root_generation = baseline.root_generation
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = baseline.root_id
                  AND root.root_generation = baseline.root_generation
                 WHERE baseline.change_id = ?1",
                [
                    sqlite_integer(baseline.change_id.value(), "baseline change ID")
                        .expect("baseline change ID"),
                ],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("opening window state");
        assert_eq!(
            initial,
            (
                "inventory".to_owned(),
                None,
                "10".to_owned(),
                "10".to_owned(),
                "recovery_required".to_owned(),
                "recovery_required".to_owned(),
            )
        );

        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER inject_watcher_closing_failure
                 BEFORE UPDATE OF closing_next_usn ON library_persistent_journal_baselines
                 BEGIN
                   SELECT RAISE(ABORT, 'injected watcher closing failure');
                 END;",
            )
            .expect("install closing transaction failure");
        let closing = closing_boundary(baseline.change_id, 20);
        let transaction_error = catalog
            .capture_persistent_journal_baseline_closing_boundary(&closing)
            .expect_err("closing failure must roll back checkpoint and window");
        assert_eq!(transaction_error.code, "catalog_database_error");
        let rolled_back: (String, Option<String>, String, String, String, String) = catalog
            .connection
            .query_row(
                "SELECT baseline.phase, baseline.closing_next_usn,
                        checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                        checkpoint.continuity_state, root.continuity_state
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_persistent_journal_checkpoints AS checkpoint
                   ON checkpoint.root_id = baseline.root_id
                  AND checkpoint.root_generation = baseline.root_generation
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = baseline.root_id
                  AND root.root_generation = baseline.root_generation
                 WHERE baseline.change_id = ?1",
                [
                    sqlite_integer(baseline.change_id.value(), "baseline change ID")
                        .expect("baseline change ID"),
                ],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("rolled back closing state");
        assert_eq!(rolled_back, initial);
        catalog
            .connection
            .execute_batch("DROP TRIGGER inject_watcher_closing_failure")
            .expect("remove closing transaction failure");

        let captured = catalog
            .capture_persistent_journal_baseline_closing_boundary(&closing)
            .expect("capture watcher closing boundary");
        assert_eq!(captured.phase, PersistentJournalBaselinePhase::Replay);
        drop(catalog);
        let catalog = SqliteCatalog::open(path).expect("reopen watcher closing window");
        let reopened = catalog
            .load_persistent_journal_baselines()
            .expect("load reopened watcher window");
        assert_eq!(reopened, vec![captured]);
        let state: (String, String, String, String) = catalog
            .connection
            .query_row(
                "SELECT checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                        checkpoint.continuity_state, root.continuity_state
                 FROM library_persistent_journal_checkpoints AS checkpoint
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = checkpoint.root_id
                  AND root.root_generation = checkpoint.root_generation
                 WHERE checkpoint.root_id = 'root-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("reopened watcher catch-up state");
        assert_eq!(
            state,
            (
                "10".to_owned(),
                "20".to_owned(),
                "catching_up".to_owned(),
                "catching_up".to_owned(),
            )
        );
    }

    #[test]
    fn watcher_gap_replay_and_candidate_barriers_atomically_publish_current() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        let source_root = directory.path().join("root-a-source");
        std::fs::create_dir(&source_root).expect("watcher recovery source root");
        let source_root_path = source_root.to_string_lossy().into_owned();
        catalog
            .connection
            .execute(
                "UPDATE library_roots SET path = ?1 WHERE id = 'root-a'",
                [&source_root_path],
            )
            .expect("bind watcher recovery source root");
        current_root(&mut catalog, "root-a");
        let baseline = promote_watcher_gap(&mut catalog, "root-a");
        let authority = catalog
            .load_metadata_inventory_recovery_authority(baseline.change_id)
            .expect("load watcher recovery authority")
            .expect("watcher recovery authority");
        catalog
            .begin_metadata_inventory(&MetadataInventoryRunRequest {
                run_id: authority.run_id.clone(),
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                epoch: 1,
                scope: MetadataInventoryScope::Subtree {
                    relative_path: "album".to_owned(),
                },
                started_unix_ms: 1_002,
            })
            .expect("begin watcher inventory");
        let root_identity = crate::adapters::FileDiscovery::new(&source_root_path)
            .expect("pin watcher recovery source root")
            .metadata_inventory_root_identity()
            .expect("read watcher recovery root identity")
            .expect("stable watcher recovery root identity");
        catalog
            .connection
            .execute(
                "INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value,
                   scope_kind, scope_relative_path, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, 'root-a', 1, ?3, ?4, 'subtree', 'album',
                           'ready', 1002, 1002)",
                rusqlite::params![
                    authority.run_id,
                    sqlite_integer(authority.change_id.value(), "recovery change ID")
                        .expect("recovery change ID"),
                    root_identity.scheme,
                    root_identity.value,
                ],
            )
            .expect("persist watcher recovery root proof");
        catalog
            .stage_metadata_inventory_page(
                &authority.run_id,
                &MetadataInventoryPage {
                    page_index: 1,
                    entries: Vec::new(),
                    cursor: None,
                    is_complete: true,
                    frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
                },
                1_003,
            )
            .expect("complete watcher inventory enumeration");
        catalog
            .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
                baseline.change_id,
                20,
            ))
            .expect("capture watcher closing boundary");
        let absence_before_replay = catalog
            .authorize_metadata_inventory_absence(&authority.run_id, 1_004)
            .expect_err("absence must wait for closing replay");
        assert_eq!(
            absence_before_replay.code,
            "persistent_journal_baseline_closing_incomplete"
        );

        let mut closing_replay = atomic_volume_batch(vec![batch(
            "watcher-gap-range",
            "root-a",
            10,
            20,
            &["during-gap.jpg"],
        )]);
        closing_replay.pages[0].enrollment.range.enrolled_unix_ms = 1_005;
        rebind_volume_batch_content(&mut closing_replay);
        catalog
            .publish_persistent_journal_volume_batch(&closing_replay, 1_005, policy())
            .expect("persist closing replay range");
        assert!(
            !catalog
                .persistent_journal_baseline_closing_is_covered(baseline.change_id)
                .expect("pending replay coverage")
        );
        let absence_while_p1_pending = catalog
            .authorize_metadata_inventory_absence(&authority.run_id, 1_006)
            .expect_err("absence must wait for terminal P1 ownership");
        assert_eq!(
            absence_while_p1_pending.code,
            "persistent_journal_baseline_closing_incomplete"
        );
        let journal = catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                LibraryRootGeneration::initial(),
                crate::domain::LibraryChangeLane::Journal,
                1_006,
                policy(),
            )
            .expect("lease closing replay work");
        assert_eq!(journal.len(), 1);
        let catalog_revision = catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("catalog revision");
        let catalog_revision =
            u64::try_from(catalog_revision).expect("nonnegative catalog revision");
        assert_eq!(
            catalog
                .complete_library_change(
                    journal[0].change.id,
                    journal[0].lease_generation,
                    catalog_revision,
                    1_007,
                )
                .expect("complete closing replay work"),
            crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
        );
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("finalize replay transaction");
        super::finalize_persistent_journal_ranges(&transaction, 1_008)
            .expect("finalize closing replay range");
        transaction.commit().expect("commit replay finalization");
        assert!(
            catalog
                .persistent_journal_baseline_closing_is_covered(baseline.change_id)
                .expect("terminal replay coverage")
        );
        catalog
            .authorize_metadata_inventory_absence(&authority.run_id, 1_009)
            .expect("authorize absence after closing replay");
        let recovery = catalog
            .lease_metadata_inventory_recovery(
                "root-a",
                LibraryRootGeneration::initial(),
                1_010,
                policy(),
            )
            .expect("lease watcher recovery")
            .expect("watcher recovery lease");

        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER inject_watcher_completion_failure
                 BEFORE UPDATE OF retired_unix_ms ON library_recovery_authorities
                 BEGIN
                   SELECT RAISE(ABORT, 'injected watcher completion failure');
                 END;",
            )
            .expect("install completion transaction failure");
        let completion_error = catalog
            .finish_metadata_inventory_recovery(
                recovery.change.id,
                recovery.lease_generation,
                catalog_revision,
                1_011,
            )
            .expect_err("final authority failure must roll back Current publication");
        assert_eq!(
            completion_error.code,
            "metadata_inventory_recovery_authority_rejected"
        );
        let rolled_back: (String, String, String, String, String, Option<i64>) = catalog
            .connection
            .query_row(
                "SELECT queue.status, run.status, baseline.phase,
                        checkpoint.continuity_state, root.continuity_state,
                        authority.retired_unix_ms
                 FROM library_change_queue AS queue
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = queue.id
                 JOIN library_metadata_inventory_runs AS run
                   ON run.id = authority.run_id
                 JOIN library_persistent_journal_baselines AS baseline
                   ON baseline.change_id = authority.change_id
                 JOIN library_persistent_journal_checkpoints AS checkpoint
                   ON checkpoint.root_id = baseline.root_id
                  AND checkpoint.root_generation = baseline.root_generation
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = baseline.root_id
                  AND root.root_generation = baseline.root_generation
                 WHERE queue.id = ?1",
                [
                    sqlite_integer(recovery.change.id.value(), "recovery change ID")
                        .expect("recovery change ID"),
                ],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("rolled back Current state");
        assert_eq!(rolled_back.0, "leased");
        assert_eq!(rolled_back.1, "comparing");
        assert_eq!(rolled_back.2, "absence");
        assert_eq!(rolled_back.3, "catching_up");
        assert_eq!(rolled_back.4, "catching_up");
        assert_eq!(rolled_back.5, None);
        catalog
            .connection
            .execute_batch("DROP TRIGGER inject_watcher_completion_failure")
            .expect("remove completion transaction failure");
        assert_eq!(
            catalog
                .finish_metadata_inventory_recovery(
                    recovery.change.id,
                    recovery.lease_generation,
                    catalog_revision,
                    1_012,
                )
                .expect("atomically finish watcher recovery"),
            Some(crate::domain::LibraryChangeLeaseUpdateOutcome::Applied),
        );
        let completed: (String, String, String, String, String, Option<i64>) = catalog
            .connection
            .query_row(
                "SELECT queue.status, run.status, baseline.phase,
                        checkpoint.continuity_state, root.continuity_state,
                        authority.retired_unix_ms
                 FROM library_change_queue AS queue
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = queue.id
                 JOIN library_metadata_inventory_runs AS run
                   ON run.id = authority.run_id
                 JOIN library_persistent_journal_baselines AS baseline
                   ON baseline.change_id = authority.change_id
                 JOIN library_persistent_journal_checkpoints AS checkpoint
                   ON checkpoint.root_id = baseline.root_id
                  AND checkpoint.root_generation = baseline.root_generation
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = baseline.root_id
                  AND root.root_generation = baseline.root_generation
                 WHERE queue.id = ?1",
                [
                    sqlite_integer(recovery.change.id.value(), "recovery change ID")
                        .expect("recovery change ID"),
                ],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("completed watcher recovery state");
        assert_eq!(completed.0, "completed");
        assert_eq!(completed.1, "completed");
        assert_eq!(completed.2, "completed");
        assert_eq!(completed.3, "current");
        assert_eq!(completed.4, "current");
        assert_eq!(completed.5, Some(1_012));
    }

    #[test]
    fn atomic_volume_publication_rolls_back_before_the_second_owner() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["root-a", "root-b"],
        );
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let volume_batch = atomic_volume_batch(vec![
            batch("range-a", "root-a", 10, 20, &["a.jpg"]),
            batch("range-b", "root-b", 10, 20, &["b.jpg", "c.jpg"]),
        ]);

        let error =
            super::publish_volume_batch(&mut catalog, &volume_batch, 1_000, policy(), Some(0))
                .expect_err("injected crash must roll back the whole volume");
        let state: (i64, i64, i64, String, String) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle),
                   (SELECT COUNT(*) FROM library_change_queue),
                   (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-a'),
                   (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-b')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("rolled back volume state");
        assert_eq!(error.code, "persistent_journal_test_crash");
        assert_eq!(state, (0, 0, 0, "10".to_owned(), "10".to_owned()));
    }

    #[test]
    fn atomic_volume_capacity_failure_preserves_every_checkpoint() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["root-a", "root-b"],
        );
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let volume_batch = atomic_volume_batch(vec![
            batch("range-a", "root-a", 10, 20, &["a.jpg"]),
            batch("range-b", "root-b", 10, 20, &["b.jpg", "c.jpg"]),
        ]);
        let constrained = LibraryChangeQueuePolicy {
            max_unresolved_changes: 1,
            ..policy()
        };

        let error = catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, constrained)
            .expect_err("second owner capacity must roll back every owner");
        let state: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_change_queue),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                    WHERE next_unread_usn = '10')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("capacity rollback state");
        assert_eq!(error.code, "persistent_journal_queue_capacity");
        assert_eq!(state, (0, 0, 2));
    }

    #[test]
    fn atomic_volume_replay_binds_full_normalized_content() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let volume_batch =
            atomic_volume_batch(vec![batch("range-a", "root-a", 10, 20, &["a.jpg"])]);

        let first = catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("initial atomic publication");
        let replay = catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("same-content crash replay");
        let mut changed = volume_batch.clone();
        changed.pages[0].enrollment.intents[0].relative_path = "changed.jpg".to_owned();
        rebind_volume_batch_content(&mut changed);
        let error = catalog
            .publish_persistent_journal_volume_batch(&changed, 1_000, policy())
            .expect_err("changed normalized intent must conflict");
        let mut changed_observation = volume_batch.clone();
        changed_observation.pages[0].enrollment.intents[0].first_observed_unix_ms = 999;
        rebind_volume_batch_content(&mut changed_observation);
        let observation_error = catalog
            .publish_persistent_journal_volume_batch(&changed_observation, 1_000, policy())
            .expect_err("changed normalized intent timing must conflict");
        let checkpoint = catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
            .expect("retained checkpoint")
            .expect("checkpoint");

        assert_eq!(first.advanced_checkpoint_count, 1);
        assert_eq!(replay.advanced_checkpoint_count, 0);
        assert_eq!(replay.observation_count, 0);
        assert_eq!(error.code, "persistent_journal_enrollment_invalid");
        assert_eq!(
            observation_error.code,
            "persistent_journal_enrollment_invalid"
        );
        assert_eq!(checkpoint.next_unread_usn.value(), 20);
        assert_eq!(
            checkpoint.continuity,
            PersistentJournalContinuityState::CatchingUp
        );
    }

    #[test]
    fn atomic_volume_replay_rejects_added_or_changed_handoff_content() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["root-a", "root-b"],
        );
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        rebind_enrollment_content(&mut previous);
        rebind_enrollment_content(&mut current);
        let volume_batch = atomic_volume_batch(vec![previous, current]);

        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish normalized handoff batch");
        let replay = catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("same-content handoff crash replay");
        assert_eq!(replay.advanced_checkpoint_count, 0);

        let mut changed = volume_batch.clone();
        for page in &mut changed.pages {
            page.enrollment.cross_root_lineage[0].current_relative_path = "changed.jpg".to_owned();
        }
        rebind_volume_batch_content(&mut changed);
        let changed_error = catalog
            .publish_persistent_journal_volume_batch(&changed, 1_000, policy())
            .expect_err("changed handoff content must conflict");

        let mut added = volume_batch.clone();
        for page in &mut added.pages {
            let mut extra = page.enrollment.cross_root_lineage[0].clone();
            extra.lineage_id = "move-lineage-extra".to_owned();
            extra.file_reference = JournalFileReference::V3([3; 16]);
            extra.previous_relative_path = "old-extra.jpg".to_owned();
            extra.current_relative_path = "new-extra.jpg".to_owned();
            page.enrollment.cross_root_lineage.push(extra);
        }
        rebind_volume_batch_content(&mut added);
        let added_error = catalog
            .publish_persistent_journal_volume_batch(&added, 1_000, policy())
            .expect_err("added handoff content must conflict");

        let durable: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                    WHERE next_unread_usn = '20')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("unchanged durable handoff replay state");
        assert_eq!(changed_error.code, "persistent_journal_enrollment_invalid");
        assert_eq!(added_error.code, "persistent_journal_enrollment_invalid");
        assert_eq!(durable, (2, 1, 2));
    }

    #[test]
    fn atomic_cross_root_owners_publish_in_either_page_order() {
        for reverse in [false, true] {
            let directory = tempdir().expect("catalog directory");
            let mut catalog = catalog_with_roots(
                directory.path().join("catalog.sqlite3"),
                &["root-a", "root-b"],
            );
            support_root(&mut catalog, "root-a");
            support_root(&mut catalog, "root-b");
            let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
            previous
                .cross_root_lineage
                .push(lineage(&previous.range.batch_id));
            let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
            current
                .cross_root_lineage
                .push(lineage(&current.range.batch_id));
            let pages = if reverse {
                vec![current, previous]
            } else {
                vec![previous, current]
            };
            let volume_batch = atomic_volume_batch(pages);

            let report = catalog
                .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
                .expect("atomic cross-root publication");
            let owners: Vec<String> = catalog
                .connection
                .prepare(
                    "SELECT participant_role
                     FROM library_persistent_journal_cross_root_ranges
                     ORDER BY participant_role",
                )
                .expect("owner statement")
                .query_map([], |row| row.get(0))
                .expect("owner rows")
                .collect::<Result<_, _>>()
                .expect("owners");
            assert_eq!(report.advanced_checkpoint_count, 2);
            assert_eq!(owners, vec!["current".to_owned(), "previous".to_owned()]);
        }
    }

    #[test]
    fn v23_unprovable_lineage_migration_rolls_back_and_reopen_stays_fail_closed() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        catalog
            .publish_persistent_journal_volume_batch(
                &atomic_volume_batch(vec![previous, current]),
                1_000,
                policy(),
            )
            .expect("publish v24 lineage fixture");
        drop(catalog);

        let connection = rusqlite::Connection::open(&path).expect("open v23 lineage fixture");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 DROP TRIGGER library_persistent_journal_source_range_id_insert;
                 DROP TRIGGER library_persistent_journal_source_range_id_update;
                 ALTER TABLE library_persistent_journal_source_ranges
                   DROP COLUMN canonical_payload;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN journal_id;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN old_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN new_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage
                   DROP COLUMN previous_carry_id;
                 UPDATE schema_info SET version = 23;",
            )
            .expect("construct unprovable v23 lineage");
        drop(connection);

        for attempt in 0..2 {
            let error = match SqliteCatalog::open(path.clone()) {
                Ok(_) => panic!("unprovable v23 lineage attempt {attempt} must fail closed"),
                Err(error) => error,
            };
            assert_eq!(error.code, "persistent_journal_v23_lineage_unverifiable");
            let connection = rusqlite::Connection::open(&path).expect("inspect rollback");
            let retained: (i64, i64, i64) = connection
                .query_row(
                    "SELECT
                       (SELECT version FROM schema_info),
                       (SELECT COUNT(*) FROM pragma_table_info(
                         'library_persistent_journal_source_ranges'
                       ) WHERE name = 'canonical_payload'),
                       (SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage)",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("rolled back v23 lineage state");
            assert_eq!(retained, (23, 0, 1));
        }
    }

    #[test]
    fn durable_pending_rename_reopens_and_consumes_with_atomic_rollback() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");

        let mut old = batch("range-old", "root-a", 10, 20, &["old.jpg"]);
        let mut carry = PersistentJournalPendingRename {
            carry_id: String::new(),
            source_range_id: old.range.batch_id.clone(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            file_reference: JournalFileReference::V3([2; 16]),
            old_usn: JournalUsn::new(15).expect("OLD USN"),
            previous_root_id: "root-a".to_owned(),
            previous_root_generation: LibraryRootGeneration::initial(),
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
            enrolled_unix_ms: 1_000,
        };
        carry.carry_id = persistent_journal_pending_rename_id(&carry);
        old.pending_renames.push(carry);
        let old_volume = atomic_volume_batch(vec![old]);
        let durable_carry = old_volume.pages[0].enrollment.pending_renames[0].clone();
        catalog
            .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
            .expect("publish durable OLD carry");
        drop(catalog);

        let mut catalog = SqliteCatalog::open(path.clone()).expect("reopen durable OLD carry");
        assert_eq!(
            catalog
                .load_persistent_journal_pending_renames(
                    &volume(),
                    JournalIdentifier::new(9).expect("journal ID"),
                )
                .expect("load reopened OLD carry"),
            std::slice::from_ref(&durable_carry)
        );
        let old_replay = catalog
            .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
            .expect("replay durable OLD page");
        assert_eq!(old_replay.advanced_checkpoint_count, 0);

        let mut current = batch("range-new", "root-b", 10, 20, &["new.jpg", "sibling.jpg"]);
        let bind_carry = |mut lineage: PersistentJournalCrossRootLineage| {
            lineage.journal_id = durable_carry.journal_id;
            lineage.old_usn = Some(durable_carry.old_usn);
            lineage.new_usn = Some(JournalUsn::new(18).expect("NEW USN"));
            lineage.previous_carry_id = Some(durable_carry.carry_id.clone());
            lineage
        };
        current
            .cross_root_lineage
            .push(bind_carry(lineage(&current.range.batch_id)));
        current
            .carried_cross_root_lineage
            .push(bind_carry(lineage(&durable_carry.source_range_id)));
        current
            .consumed_pending_rename_ids
            .push(durable_carry.carry_id.clone());
        let current_volume = atomic_volume_batch(vec![current]);

        let crash =
            super::publish_volume_batch(&mut catalog, &current_volume, 1_000, policy(), Some(0))
                .expect_err("crash before carry consumption must roll back");
        assert_eq!(crash.code, "persistent_journal_test_crash");
        assert_pending_carry_rollback(&catalog, &durable_carry);

        let constrained = LibraryChangeQueuePolicy {
            max_unresolved_changes: 1,
            ..policy()
        };
        let capacity = catalog
            .publish_persistent_journal_volume_batch(&current_volume, 1_000, constrained)
            .expect_err("capacity failure must retain OLD carry and checkpoint");
        assert_eq!(capacity.code, "persistent_journal_queue_capacity");
        assert_pending_carry_rollback(&catalog, &durable_carry);

        let mut assert_mismatch = |mut mismatched: PersistentJournalVolumeBatch, name: &str| {
            rebind_volume_batch_content(&mut mismatched);
            let mismatch = catalog
                .publish_persistent_journal_volume_batch(&mismatched, 1_000, policy())
                .expect_err(name);
            assert!(
                matches!(
                    mismatch.code.as_str(),
                    "persistent_journal_enrollment_invalid" | "persistent_journal_batch_conflict"
                ),
                "unexpected mismatch code for {name}: {}",
                mismatch.code
            );
            assert_pending_carry_rollback(&catalog, &durable_carry);
        };

        let mut missing_owner = current_volume.clone();
        missing_owner.pages[0]
            .enrollment
            .carried_cross_root_lineage
            .clear();
        assert_mismatch(missing_owner, "missing carried owner must roll back");

        let mut duplicate_owner = current_volume.clone();
        let duplicate = duplicate_owner.pages[0].enrollment.cross_root_lineage[0].clone();
        duplicate_owner.pages[0]
            .enrollment
            .cross_root_lineage
            .push(duplicate);
        assert_mismatch(duplicate_owner, "duplicate carried owner must roll back");

        let mut wrong_generation = current_volume.clone();
        let enrollment = &mut wrong_generation.pages[0].enrollment;
        for lineage in enrollment
            .cross_root_lineage
            .iter_mut()
            .chain(&mut enrollment.carried_cross_root_lineage)
        {
            lineage.previous_root_generation =
                LibraryRootGeneration::new(2).expect("wrong generation");
        }
        assert_mismatch(wrong_generation, "wrong carry generation must roll back");

        let mut wrong_path = current_volume.clone();
        let enrollment = &mut wrong_path.pages[0].enrollment;
        for lineage in enrollment
            .cross_root_lineage
            .iter_mut()
            .chain(&mut enrollment.carried_cross_root_lineage)
        {
            lineage.previous_relative_path = "wrong.jpg".to_owned();
        }
        assert_mismatch(wrong_path, "wrong carry path must roll back");

        let mut wrong_reference = current_volume.clone();
        let enrollment = &mut wrong_reference.pages[0].enrollment;
        for lineage in enrollment
            .cross_root_lineage
            .iter_mut()
            .chain(&mut enrollment.carried_cross_root_lineage)
        {
            lineage.file_reference = JournalFileReference::V3([8; 16]);
        }
        assert_mismatch(wrong_reference, "wrong carry reference must roll back");

        let mut wrong_range = current_volume.clone();
        wrong_range.pages[0].enrollment.carried_cross_root_lineage[0].owner_source_range_id =
            "0".repeat(64);
        assert_mismatch(wrong_range, "wrong carry source range must roll back");

        let report = catalog
            .publish_persistent_journal_volume_batch(&current_volume, 1_000, policy())
            .expect("atomically publish carried handoff");
        assert_eq!(report.advanced_checkpoint_count, 1);
        assert!(
            catalog
                .load_persistent_journal_pending_renames(
                    &volume(),
                    JournalIdentifier::new(9).expect("journal ID"),
                )
                .expect("load consumed carries")
                .is_empty()
        );
        let durable: (i64, i64, String, String) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges),
                   (SELECT COUNT(DISTINCT participant_role)
                    FROM library_persistent_journal_cross_root_ranges),
                   (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-a'),
                   (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-b')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("durable carried handoff state");
        assert_eq!(durable, (2, 2, "20".to_owned(), "20".to_owned()));

        let current_replay = catalog
            .publish_persistent_journal_volume_batch(&current_volume, 1_000, policy())
            .expect("same-content carried handoff replay");
        let old_after_consume = catalog
            .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
            .expect("same-content OLD replay after carry cleanup");
        assert_eq!(current_replay.advanced_checkpoint_count, 0);
        assert_eq!(old_after_consume.advanced_checkpoint_count, 0);
        drop(catalog);

        let consumed = SqliteCatalog::open(path.clone()).expect("reopen consumed carry proof");
        consumed
            .connection
            .execute(
                "DELETE FROM library_persistent_journal_cross_root_lineage",
                [],
            )
            .expect("delete consumed lineage proof with owner cascade");
        assert_eq!(
            consumed
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("consumed proof owner count"),
            0
        );
        drop(consumed);
        assert_contract_reopen_fails(path, "deleted consumed carry proof");
    }

    #[test]
    fn unregister_root_releases_both_cross_root_endpoints_and_keeps_independent_survivor_state() {
        let directory = tempdir().expect("catalog directory");
        for removed_root_id in ["root-a", "root-b"] {
            let path = directory
                .path()
                .join(format!("remove-{removed_root_id}.sqlite3"));
            let (mut catalog, _, _) = catalog_with_pending_lineage_batches(path.clone());
            let survivor_root_id = if removed_root_id == "root-a" {
                "root-b"
            } else {
                "root-a"
            };
            let survivor_path = if removed_root_id == "root-a" {
                "new.jpg"
            } else {
                "old.jpg"
            };
            catalog
                .connection
                .execute(
                    "UPDATE library_persistent_journal_root_state
                     SET continuity_state = 'catching_up', updated_unix_ms = 2_000
                     WHERE root_id = ?1",
                    [survivor_root_id],
                )
                .expect("arm survivor catch-up state");
            catalog
                .connection
                .execute(
                    "UPDATE library_persistent_journal_checkpoints
                     SET captured_exclusive_end = '30', continuity_state = 'catching_up',
                         updated_unix_ms = 2_000
                     WHERE root_id = ?1",
                    [survivor_root_id],
                )
                .expect("arm survivor catch-up checkpoint");
            let independent = atomic_volume_batch(vec![batch(
                "independent-survivor",
                survivor_root_id,
                20,
                30,
                &["independent.jpg"],
            )]);
            let independent_range_id = independent.pages[0].enrollment.range.batch_id.clone();
            catalog
                .publish_persistent_journal_volume_batch(&independent, 1_000, policy())
                .expect("publish independent survivor evidence");

            assert!(
                catalog
                    .unregister_root(removed_root_id)
                    .expect("remove cross-root endpoint")
            );
            assert_root_unregister_journal_state(
                &catalog,
                removed_root_id,
                survivor_root_id,
                &independent_range_id,
            );
            let lease_unix_ms = ready_live_path_lease_unix_ms(&catalog, survivor_root_id);
            let survivor_work = catalog
                .lease_path_library_changes_in_lane(
                    survivor_root_id,
                    LibraryRootGeneration::initial(),
                    LibraryChangeLane::Live,
                    lease_unix_ms,
                    policy(),
                )
                .expect("lease survivor root-removal handoff");
            assert_eq!(survivor_work.len(), 1);
            let survivor_work = &survivor_work[0];
            assert_eq!(
                survivor_work.change.intent.kind,
                LibraryChangeIntentKind::Reconcile
            );
            assert_eq!(survivor_work.change.intent.scope, LibraryChangeScope::Path);
            assert_eq!(survivor_work.change.intent.relative_path, survivor_path);
            assert!(survivor_work.change.catch_up_source.is_none());
            assert!(survivor_work.change.catch_up_watermark.is_none());
            assert!(survivor_work.change.catch_up_lineage.is_empty());
            let catalog_revision = catalog
                .connection
                .query_row("SELECT revision FROM catalog_state", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("catalog revision after root removal");
            catalog
                .complete_library_change(
                    survivor_work.change.id,
                    survivor_work.lease_generation,
                    u64::try_from(catalog_revision).expect("nonnegative catalog revision"),
                    lease_unix_ms + 1,
                )
                .expect("complete survivor root-removal handoff");
            assert_survivor_root_unregister_handoff_completed(
                &catalog,
                survivor_root_id,
                survivor_path,
            );
            drop(catalog);

            let reopened = SqliteCatalog::open(path).expect("reopen after cross-root removal");
            assert_root_unregister_journal_state(
                &reopened,
                removed_root_id,
                survivor_root_id,
                &independent_range_id,
            );
            assert_survivor_root_unregister_handoff_completed(
                &reopened,
                survivor_root_id,
                survivor_path,
            );
        }
    }

    #[test]
    fn unregister_root_releases_both_paths_of_a_survivor_rename_from_a_retired_peer_range() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("remove-peer-range-rename.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        current.intents.push(LibraryChangeIntent {
            root_id: "root-b".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: LibraryChangeIntentKind::RenameCandidate,
            scope: LibraryChangeScope::Path,
            relative_path: "renamed-new.jpg".to_owned(),
            previous_relative_path: Some("renamed-old.jpg".to_owned()),
            origin: LibraryChangeOrigin::StartupCatchUp,
            first_observed_unix_ms: 1_000,
            most_recent_observed_unix_ms: 1_000,
            first_sequence: 2,
            most_recent_sequence: 2,
            coalesced_observation_count: 1,
        });
        let volume_batch = atomic_volume_batch(vec![previous, current]);
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish cross-root range with survivor rename");

        assert!(
            catalog
                .unregister_root("root-a")
                .expect("remove cross-root endpoint")
        );
        let lease_unix_ms = ready_live_path_lease_unix_ms(&catalog, "root-b");
        let survivor_work = catalog
            .lease_path_library_changes_in_lane(
                "root-b",
                LibraryRootGeneration::initial(),
                LibraryChangeLane::Live,
                lease_unix_ms,
                policy(),
            )
            .expect("lease survivor root-removal handoffs");
        let mut survivor_paths = survivor_work
            .iter()
            .map(|work| {
                assert_eq!(work.change.intent.kind, LibraryChangeIntentKind::Reconcile);
                assert_eq!(work.change.intent.scope, LibraryChangeScope::Path);
                assert!(work.change.catch_up_source.is_none());
                assert!(work.change.catch_up_watermark.is_none());
                assert!(work.change.catch_up_lineage.is_empty());
                work.change.intent.relative_path.clone()
            })
            .collect::<Vec<_>>();
        survivor_paths.sort();
        assert_eq!(
            survivor_paths,
            vec![
                "new.jpg".to_owned(),
                "renamed-new.jpg".to_owned(),
                "renamed-old.jpg".to_owned(),
            ]
        );
        let catalog_revision = catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("catalog revision after root removal");
        for (index, work) in survivor_work.iter().enumerate() {
            catalog
                .complete_library_change(
                    work.change.id,
                    work.lease_generation,
                    u64::try_from(catalog_revision).expect("nonnegative catalog revision"),
                    lease_unix_ms + i64::try_from(index).expect("bounded handoff index") + 1,
                )
                .expect("complete survivor rename handoff");
        }
        for survivor_path in &survivor_paths {
            assert_survivor_root_unregister_handoff_completed(&catalog, "root-b", survivor_path);
        }
        drop(catalog);

        let reopened =
            SqliteCatalog::open(path).expect("reopen with durable survivor rename handoffs");
        for survivor_path in &survivor_paths {
            assert_survivor_root_unregister_handoff_completed(&reopened, "root-b", survivor_path);
        }
    }

    #[test]
    fn unregister_root_releases_pending_carry_and_keeps_survivor_source_range() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("remove-pending-carry.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut removed = batch("removed-carry", "root-a", 10, 20, &["old.jpg"]);
        let mut carry = PersistentJournalPendingRename {
            carry_id: String::new(),
            source_range_id: removed.range.batch_id.clone(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            file_reference: JournalFileReference::V3([8; 16]),
            old_usn: JournalUsn::new(15).expect("OLD USN"),
            previous_root_id: "root-a".to_owned(),
            previous_root_generation: LibraryRootGeneration::initial(),
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
            enrolled_unix_ms: 1_000,
        };
        carry.carry_id = persistent_journal_pending_rename_id(&carry);
        removed.pending_renames.push(carry);
        let volume_batch = atomic_volume_batch(vec![
            removed,
            batch("survivor-range", "root-b", 10, 20, &["survivor.jpg"]),
        ]);
        let survivor_range_id = volume_batch.pages[1].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish pending carry and survivor range");

        assert!(
            catalog
                .unregister_root("root-a")
                .expect("remove pending-carry root")
        );
        assert_root_unregister_journal_state(&catalog, "root-a", "root-b", &survivor_range_id);
        drop(catalog);

        let reopened = SqliteCatalog::open(path).expect("reopen after pending-carry removal");
        assert_root_unregister_journal_state(&reopened, "root-a", "root-b", &survivor_range_id);
    }

    #[test]
    fn terminal_range_lifecycle_is_bounded_and_survives_reopen() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let mut start = 10_i64;
        for index in 0..70 {
            let publication_unix_ms = 10_000 + i64::from(index) * 2;
            let end = start + 1;
            let relative_path = format!("page-{index}.jpg");
            let mut volume_batch = atomic_volume_batch(vec![batch(
                &format!("range-{index}"),
                "root-a",
                start,
                end,
                &[&relative_path],
            )]);
            volume_batch.pages[0].checkpoint.updated_unix_ms = publication_unix_ms;
            catalog
                .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
                .expect("publish sequential journal page");
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("terminal lifecycle transaction");
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'completed', catalog_revision_at_success = 0,
                         updated_unix_ms = ?1
                     WHERE status = 'pending'",
                    [publication_unix_ms + 1],
                )
                .expect("terminalize page queue");
            super::finalize_persistent_journal_ranges(&transaction, publication_unix_ms + 1)
                .expect("finalize journal page");
            transaction.commit().expect("commit terminal lifecycle");
            start = end;
        }
        let retained: (i64, i64, i64, String) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle),
                   (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle
                    WHERE lifecycle_state = 'pending'),
                   (SELECT continuity_state FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-a')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("bounded lifecycle state");
        assert_eq!(retained.0, 64);
        assert_eq!(retained.0, retained.1);
        assert_eq!(retained.2, 0);
        assert_eq!(retained.3, "current");
        drop(catalog);
        let reopened = SqliteCatalog::open(path).expect("reopen bounded lifecycle");
        assert_eq!(
            reopened
                .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
                .expect("load reopened checkpoint")
                .expect("checkpoint")
                .next_unread_usn
                .value(),
            80
        );
    }

    #[test]
    fn historical_generation_cleanup_retains_pending_and_latest_terminal_proof() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let mut terminal_ids = Vec::new();
        for (index, start) in [10_i64, 20].into_iter().enumerate() {
            let mut volume_batch = atomic_volume_batch(vec![batch(
                &format!("terminal-{index}"),
                "root-a",
                start,
                start + 10,
                &[&format!("terminal-{index}.jpg")],
            )]);
            let checkpoint_unix_ms = 2_000 + 200 * i64::try_from(index).expect("bounded index");
            let terminal_unix_ms = checkpoint_unix_ms + 100;
            volume_batch.pages[0].checkpoint.updated_unix_ms = checkpoint_unix_ms;
            terminal_ids.push(volume_batch.pages[0].enrollment.range.batch_id.clone());
            catalog
                .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
                .expect("publish historical terminal page");
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("terminal lifecycle transaction");
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'completed', catalog_revision_at_success = 0,
                         updated_unix_ms = ?1
                     WHERE status = 'pending'",
                    [terminal_unix_ms],
                )
                .expect("terminalize historical queue");
            super::finalize_persistent_journal_ranges(&transaction, terminal_unix_ms)
                .expect("finalize historical page");
            transaction
                .commit()
                .expect("commit historical terminal page");
        }

        let mut pending = batch("pending-old", "root-a", 30, 40, &["pending.jpg"]);
        let mut carry = PersistentJournalPendingRename {
            carry_id: String::new(),
            source_range_id: pending.range.batch_id.clone(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            file_reference: JournalFileReference::V3([4; 16]),
            old_usn: JournalUsn::new(35).expect("OLD USN"),
            previous_root_id: "root-a".to_owned(),
            previous_root_generation: LibraryRootGeneration::initial(),
            previous_relative_path: "pending.jpg".to_owned(),
            is_directory: false,
            enrolled_unix_ms: 1_000,
        };
        carry.carry_id = persistent_journal_pending_rename_id(&carry);
        pending.pending_renames.push(carry);
        let mut pending_volume = atomic_volume_batch(vec![pending]);
        pending_volume.pages[0].checkpoint.updated_unix_ms = 2_400;
        let pending_id = pending_volume.pages[0].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&pending_volume, 1_000, policy())
            .expect("publish unresolved historical page");

        let next_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next root generation");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: next_generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "replacement.jpg".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 3_000,
                    most_recent_observed_unix_ms: 3_000,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                3_000,
                policy(),
            )
            .expect("replace root generation");
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("historical cleanup transaction");
        super::cleanup_completed_persistent_journal_history(&transaction, 3_100, 32)
            .expect("clean historical journal evidence");
        transaction.commit().expect("commit historical cleanup");

        let retained: (i64, i64, i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1),
                   EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?2),
                   EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?3),
                   (SELECT COUNT(*) FROM library_persistent_journal_pending_renames),
                   (SELECT COUNT(*) FROM library_change_root_state
                    WHERE root_id = 'root-a' AND generation = 2 AND is_active = 1)",
                params![terminal_ids[0], terminal_ids[1], pending_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("historical cleanup boundary");
        assert_eq!(retained, (0, 1, 1, 1, 1));
        drop(catalog);
        SqliteCatalog::open(path).expect("reopen historical cleanup boundary");
    }

    #[test]
    fn enrollment_survives_reopen_and_replay_advances_only_after_queue_ownership() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let batch = batch("range-a", "root-a", 10, 20, &["created.jpg", "deleted.jpg"]);

        let enrolled = catalog
            .enroll_persistent_journal_batch(&batch, 1_000, policy())
            .expect("durable journal enrollment");
        assert_eq!(enrolled.enrolled_root_count, 1);
        assert_eq!(enrolled.observation_count, 2);
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
                .expect("checkpoint before advancement"),
            Some(checkpoint("root-a", 10, 10, 500))
        );
        drop(catalog);

        let mut catalog = SqliteCatalog::open(path).expect("reopen enrolled catalog");
        let replayed = catalog
            .enroll_persistent_journal_batch(&batch, 1_000, policy())
            .expect("idempotent enrollment replay");
        assert_eq!(replayed.observation_count, 0);
        let checkpoint = checkpoint("root-a", 20, 20, 2_000);
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(&batch.range.batch_id, &checkpoint)
                .expect("advance enrolled range")
        );
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
                .expect("load durable checkpoint"),
            Some(checkpoint.clone())
        );
        assert!(
            !catalog
                .advance_persistent_journal_checkpoint(&batch.range.batch_id, &checkpoint)
                .expect("idempotent checkpoint replay")
        );
        drop(catalog);
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
            .expect("reopen checkpointed catalog");
        assert!(
            !catalog
                .advance_persistent_journal_checkpoint(&batch.range.batch_id, &checkpoint)
                .expect("crash-safe checkpoint replay")
        );
        let counts: (i64, i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_queue_lineage),
                   (SELECT COUNT(*) FROM library_change_queue),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("durable journal counts");
        assert_eq!(counts, (1, 2, 2, 1));
    }

    #[test]
    fn checkpoint_rejects_gap_and_out_of_order_ranges_without_mutation() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let gap = batch("range-gap", "root-a", 11, 20, &["gap.jpg"]);
        catalog
            .enroll_persistent_journal_batch(&gap, 1_000, policy())
            .expect("enroll gap range");
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &gap.range.batch_id,
                    &checkpoint("root-a", 20, 20, 2_000),
                )
                .is_err()
        );
        let first = batch("range-first", "root-a", 10, 20, &["first.jpg"]);
        let second = batch("range-second", "root-a", 20, 30, &["second.jpg"]);
        for enrolled in [&first, &second] {
            catalog
                .enroll_persistent_journal_batch(enrolled, 1_000, policy())
                .expect("enroll ordered range");
        }
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &second.range.batch_id,
                    &checkpoint("root-a", 30, 30, 2_000),
                )
                .is_err()
        );
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
                .expect("unchanged baseline"),
            Some(checkpoint("root-a", 10, 10, 500))
        );
    }

    #[test]
    fn checkpoint_rejects_end_revision_and_protocol_regression() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let mut partial = batch("range-partial", "root-a", 10, 30, &["partial.jpg"]);
        partial.range.covered_until_usn = JournalUsn::new(20).expect("partial covered");
        partial.range.is_complete = false;
        partial.range.batch_id = persistent_journal_batch_id(&partial);
        catalog
            .enroll_persistent_journal_batch(&partial, 1_000, policy())
            .expect("enroll partial range");
        let mut partial_checkpoint = checkpoint("root-a", 20, 30, 2_000);
        partial_checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
        catalog
            .advance_persistent_journal_checkpoint(&partial.range.batch_id, &partial_checkpoint)
            .expect("advance partial range");

        let end_regression = batch("range-end-regression", "root-a", 20, 25, &["end.jpg"]);
        catalog
            .enroll_persistent_journal_batch(&end_regression, 1_000, policy())
            .expect("enroll end regression");
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &end_regression.range.batch_id,
                    &checkpoint("root-a", 25, 25, 3_000),
                )
                .is_err()
        );

        let revision_range = batch("range-revision", "root-a", 20, 35, &["revision.jpg"]);
        catalog
            .enroll_persistent_journal_batch(&revision_range, 1_000, policy())
            .expect("enroll revision range");
        let mut revision_rollback = checkpoint("root-a", 35, 35, 3_000);
        revision_rollback.covered_catalog_revision = 0;
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &revision_range.range.batch_id,
                    &revision_rollback,
                )
                .is_err()
        );

        let mut protocol_range = batch("range-protocol", "root-a", 20, 40, &["protocol.jpg"]);
        protocol_range.range.protocol_version = 2;
        protocol_range.range.batch_id = persistent_journal_batch_id(&protocol_range);
        catalog
            .enroll_persistent_journal_batch(&protocol_range, 1_000, policy())
            .expect("enroll protocol range");
        let mut protocol_checkpoint = checkpoint("root-a", 40, 40, 3_000);
        protocol_checkpoint.protocol_version = 2;
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &protocol_range.range.batch_id,
                    &protocol_checkpoint,
                )
                .is_err()
        );
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
                .expect("retained partial checkpoint"),
            Some(partial_checkpoint)
        );
    }

    #[test]
    fn checkpoint_never_establishes_an_implicit_baseline() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        catalog
            .connection
            .execute(
                "DELETE FROM library_persistent_journal_checkpoints WHERE root_id = 'root-a'",
                [],
            )
            .expect("remove explicit baseline");
        let range = batch("range-no-baseline", "root-a", 10, 20, &["created.jpg"]);
        catalog
            .enroll_persistent_journal_batch(&range, 1_000, policy())
            .expect("enroll without baseline");
        assert!(
            catalog
                .advance_persistent_journal_checkpoint(
                    &range.range.batch_id,
                    &checkpoint("root-a", 20, 20, 2_000),
                )
                .is_err()
        );
    }

    #[test]
    fn queue_capacity_rolls_back_range_and_does_not_create_checkpoint() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let batch = batch("range-capacity", "root-a", 10, 20, &["a.jpg", "b.jpg"]);
        let constrained = LibraryChangeQueuePolicy {
            max_unresolved_changes: 1,
            ..policy()
        };

        let error = catalog
            .enroll_persistent_journal_batch(&batch, 1_000, constrained)
            .expect_err("capacity must not own a partial range");
        let counts: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_change_queue),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("rolled back capacity counts");

        assert_eq!(error.code, "persistent_journal_queue_capacity");
        assert_eq!(counts, (0, 0, 1));
    }

    #[test]
    fn retired_generation_rejects_checkpoint_reads_and_idempotent_replay() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let batch = batch("range-a", "root-a", 10, 20, &["created.jpg"]);
        let checkpoint = checkpoint("root-a", 20, 20, 2_000);
        catalog
            .enroll_persistent_journal_batch(&batch, 1_000, policy())
            .expect("durable journal enrollment");
        catalog
            .advance_persistent_journal_checkpoint(&batch.range.batch_id, &checkpoint)
            .expect("initial checkpoint advancement");

        let next_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next root generation");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: next_generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "after-replacement.jpg".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 3_000,
                    most_recent_observed_unix_ms: 3_000,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                3_000,
                policy(),
            )
            .expect("advance root generation");

        let load_error = catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
            .expect_err("retired checkpoint must not be readable");
        let replay_error = catalog
            .advance_persistent_journal_checkpoint(&batch.range.batch_id, &checkpoint)
            .expect_err("retired checkpoint must not accept idempotent replay");
        assert_eq!(load_error.code, "persistent_journal_root_authority_stale");
        assert_eq!(replay_error.code, "persistent_journal_root_authority_stale");
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_source_ranges
                     WHERE id = ?1 AND status = 'superseded'",
                    [&batch.range.batch_id],
                    |row| row.get::<_, i64>(0),
                )
                .expect("retained historical range"),
            1
        );
        drop(catalog);
        SqliteCatalog::open(path).expect("retired checkpoint state validates on reopen");
    }

    #[test]
    fn cross_root_lineage_owner_generation_must_match_source_range() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["root-a", "root-b"],
        );
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut owner = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        let mut mismatched = lineage(&owner.range.batch_id);
        mismatched.previous_root_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next root generation");
        owner.cross_root_lineage.push(mismatched);

        let error = catalog
            .enroll_persistent_journal_batch(&owner, 1_000, policy())
            .expect_err("lineage owner generation mismatch must be rejected");
        let counts: (i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("rejected lineage counts");
        assert_eq!(error.code, "persistent_journal_contract_invalid");
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn cross_root_lineage_is_owned_independently_by_both_source_ranges() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        rebind_enrollment_content(&mut previous);
        rebind_enrollment_content(&mut current);

        catalog
            .enroll_persistent_journal_batch(&previous, 1_000, policy())
            .expect("previous root enrollment");
        catalog
            .enroll_persistent_journal_batch(&current, 1_000, policy())
            .expect("current root enrollment");
        let ownership: Vec<(String, String)> = catalog
            .connection
            .prepare(
                "SELECT source_range_id, participant_role
                 FROM library_persistent_journal_cross_root_ranges
                 ORDER BY source_range_id",
            )
            .expect("ownership statement")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("ownership rows")
            .collect::<Result<_, _>>()
            .expect("ownership evidence");
        let mut expected = vec![
            (previous.range.batch_id.clone(), "previous".to_owned()),
            (current.range.batch_id.clone(), "current".to_owned()),
        ];
        expected.sort();
        assert_eq!(ownership, expected);
        let next_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next root generation");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: next_generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "after-replacement.jpg".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 2_000,
                    most_recent_observed_unix_ms: 2_000,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                2_000,
                policy(),
            )
            .expect("advance previous root generation");
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage
                     WHERE id = 'move-lineage'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("retained historical cross-root lineage"),
            1
        );
        let superseded_state: (String, String) = catalog
            .connection
            .query_row(
                "SELECT lineage.status, journal.continuity_state
                 FROM library_persistent_journal_cross_root_lineage AS lineage
                 JOIN library_persistent_journal_root_state AS journal
                   ON journal.root_id = 'root-b'
                  AND journal.root_generation = 1
                 WHERE lineage.id = 'move-lineage'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("superseded lineage authority state");
        assert_eq!(
            superseded_state,
            ("superseded".to_owned(), "catching_up".to_owned())
        );
        drop(catalog);
        SqliteCatalog::open(path).expect("cross-root ownership validates on reopen");
    }

    #[test]
    fn pending_lineage_reopen_requires_exactly_two_payload_bound_owners() {
        let directory = tempdir().expect("catalog directory");
        let legal_path = directory.path().join("legal.sqlite3");
        drop(catalog_with_pending_lineage(legal_path.clone()));
        SqliteCatalog::open(legal_path).expect("legal pending lineage reopens");

        let corruptions = [
            (
                "missing-previous",
                "DELETE FROM library_persistent_journal_cross_root_ranges
                 WHERE participant_role = 'previous'",
            ),
            (
                "missing-current",
                "DELETE FROM library_persistent_journal_cross_root_ranges
                 WHERE participant_role = 'current'",
            ),
            (
                "duplicate-role",
                "UPDATE library_persistent_journal_cross_root_ranges
                 SET participant_role = 'previous' WHERE participant_role = 'current'",
            ),
            (
                "wrong-generation",
                "UPDATE library_persistent_journal_cross_root_lineage
                 SET previous_root_generation = previous_root_generation + 1",
            ),
            (
                "payload-child",
                "UPDATE library_persistent_journal_cross_root_lineage
                 SET current_relative_path = 'payload-mismatch.jpg'",
            ),
        ];
        for (name, mutation) in corruptions {
            let path = directory.path().join(format!("{name}.sqlite3"));
            drop(catalog_with_pending_lineage(path.clone()));
            let connection = rusqlite::Connection::open(&path).expect("open corruption fixture");
            connection
                .execute_batch("PRAGMA foreign_keys = OFF")
                .expect("disable fixture foreign keys");
            connection
                .execute_batch(mutation)
                .expect("tamper durable owner evidence");
            drop(connection);
            let error = match SqliteCatalog::open(path) {
                Ok(_) => panic!("{name} corruption must fail closed"),
                Err(error) => error,
            };
            assert_eq!(
                error.code, "catalog_persistent_journal_contract_unverifiable",
                "unexpected reopen error for {name}"
            );
        }

        let wrong_range_path = directory.path().join("wrong-range.sqlite3");
        drop(catalog_with_pending_lineage(wrong_range_path.clone()));
        let connection = rusqlite::Connection::open(&wrong_range_path)
            .expect("open wrong-range corruption fixture");
        connection
            .execute_batch("PRAGMA foreign_keys = OFF")
            .expect("disable fixture foreign keys");
        connection
            .execute(
                "UPDATE library_persistent_journal_cross_root_ranges
                 SET source_range_id = ?1 WHERE participant_role = 'previous'",
                ["0".repeat(64)],
            )
            .expect("tamper owner range");
        drop(connection);
        let error = match SqliteCatalog::open(wrong_range_path) {
            Ok(_) => panic!("wrong owner range must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
    }

    #[test]
    fn retained_payload_rejects_deleted_lineage_parent_and_unconsumed_carry() {
        let directory = tempdir().expect("catalog directory");
        let lineage_path = directory.path().join("deleted-lineage.sqlite3");
        let lineage_catalog = catalog_with_pending_lineage(lineage_path.clone());
        lineage_catalog
            .connection
            .execute(
                "DELETE FROM library_persistent_journal_cross_root_lineage",
                [],
            )
            .expect("delete lineage parent with owner cascade");
        assert_eq!(
            lineage_catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("cascaded owner count"),
            0
        );
        drop(lineage_catalog);
        assert_contract_reopen_fails(lineage_path, "deleted lineage parent");

        let carry_path = directory.path().join("deleted-carry.sqlite3");
        let mut carry_catalog = catalog_with_roots(carry_path.clone(), &["root-a"]);
        support_root(&mut carry_catalog, "root-a");
        let mut enrollment = batch("pending-old", "root-a", 10, 20, &["old.jpg"]);
        let mut pending = PersistentJournalPendingRename {
            carry_id: String::new(),
            source_range_id: enrollment.range.batch_id.clone(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            file_reference: JournalFileReference::V3([7; 16]),
            old_usn: JournalUsn::new(15).expect("OLD USN"),
            previous_root_id: "root-a".to_owned(),
            previous_root_generation: LibraryRootGeneration::initial(),
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
            enrolled_unix_ms: 1_000,
        };
        pending.carry_id = persistent_journal_pending_rename_id(&pending);
        enrollment.pending_renames.push(pending);
        carry_catalog
            .publish_persistent_journal_volume_batch(
                &atomic_volume_batch(vec![enrollment]),
                1_000,
                policy(),
            )
            .expect("publish pending carry");
        carry_catalog
            .connection
            .execute("DELETE FROM library_persistent_journal_pending_renames", [])
            .expect("delete unconsumed carry");
        drop(carry_catalog);
        assert_contract_reopen_fails(carry_path, "deleted unconsumed carry");
    }

    #[test]
    fn retained_payload_rejects_duplicate_and_extra_children_after_valid_rekey() {
        let directory = tempdir().expect("catalog directory");
        for mutation in ["duplicate-lineage", "extra-lineage", "extra-carry"] {
            let path = directory.path().join(format!("{mutation}.sqlite3"));
            let (catalog, _, mut current) = catalog_with_pending_lineage_batches(path.clone());
            match mutation {
                "duplicate-lineage" => {
                    current
                        .cross_root_lineage
                        .push(current.cross_root_lineage[0].clone());
                }
                "extra-lineage" => {
                    let mut extra = lineage(&current.range.batch_id);
                    extra.lineage_id = "payload-only-lineage".to_owned();
                    current.cross_root_lineage.push(extra);
                }
                "extra-carry" => {
                    let mut pending = PersistentJournalPendingRename {
                        carry_id: String::new(),
                        source_range_id: current.range.batch_id.clone(),
                        volume: volume(),
                        journal_id: JournalIdentifier::new(9).expect("journal ID"),
                        file_reference: JournalFileReference::V3([9; 16]),
                        old_usn: JournalUsn::new(15).expect("OLD USN"),
                        previous_root_id: "root-b".to_owned(),
                        previous_root_generation: LibraryRootGeneration::initial(),
                        previous_relative_path: "payload-only.jpg".to_owned(),
                        is_directory: false,
                        enrolled_unix_ms: 1_000,
                    };
                    pending.carry_id = persistent_journal_pending_rename_id(&pending);
                    current.pending_renames.push(pending);
                }
                _ => unreachable!(),
            }
            rewrite_retained_range_payload(&catalog.connection, &mut current);
            drop(catalog);
            assert_contract_reopen_fails(path, mutation);
        }
    }

    #[test]
    fn completed_lineage_cleanup_deletes_only_a_closed_range_proof_cluster() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let (mut catalog, previous, current) = catalog_with_pending_lineage_batches(path.clone());
        let old_range_ids = [
            previous.range.batch_id.clone(),
            current.range.batch_id.clone(),
        ];
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("terminal lineage transaction");
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0,
                     updated_unix_ms = 2_100
                 WHERE status = 'pending'",
                [],
            )
            .expect("terminalize lineage queue");
        super::finalize_persistent_journal_ranges(&transaction, 2_100)
            .expect("terminalize lineage ranges");
        transaction.commit().expect("commit terminal lineage");

        let mut newer = atomic_volume_batch(vec![
            batch("newer-a", "root-a", 20, 30, &[]),
            batch("newer-b", "root-b", 20, 30, &[]),
        ]);
        for page in &mut newer.pages {
            page.enrollment.range.enrolled_unix_ms = 2_200;
            page.checkpoint.updated_unix_ms = 2_200;
        }
        rebind_volume_batch_content(&mut newer);
        let newer_range_ids = newer
            .pages
            .iter()
            .map(|page| page.enrollment.range.batch_id.clone())
            .collect::<Vec<_>>();
        catalog
            .publish_persistent_journal_volume_batch(&newer, 2_200, policy())
            .expect("publish newer terminal ranges");
        let next_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next root generation");
        for root_id in ["root-a", "root-b"] {
            catalog
                .enqueue_library_change_intents(
                    &[LibraryChangeIntent {
                        root_id: root_id.to_owned(),
                        root_generation: next_generation,
                        kind: LibraryChangeIntentKind::Reconcile,
                        scope: LibraryChangeScope::Path,
                        relative_path: format!("replacement-{root_id}.jpg"),
                        previous_relative_path: None,
                        origin: LibraryChangeOrigin::LiveNotification,
                        first_observed_unix_ms: 2_300,
                        most_recent_observed_unix_ms: 2_300,
                        first_sequence: 1,
                        most_recent_sequence: 1,
                        coalesced_observation_count: 1,
                    }],
                    2_300,
                    policy(),
                )
                .expect("retire journal root generation");
        }
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("closed cleanup transaction");
        super::cleanup_completed_persistent_journal_history(&transaction, 2_400, 32)
            .expect("delete closed range proof cluster");
        super::super::migrations::validate_persistent_journal_payload_children(&transaction)
            .expect("cleanup leaves payload-child equivalence");
        transaction.commit().expect("commit closed cleanup");

        for range_id in old_range_ids {
            assert!(
                !catalog
                    .connection
                    .query_row(
                        "SELECT EXISTS(
                           SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1
                         )",
                        [&range_id],
                        |row| row.get::<_, bool>(0),
                    )
                    .expect("old range retention")
            );
        }
        for range_id in newer_range_ids {
            assert!(
                catalog
                    .connection
                    .query_row(
                        "SELECT EXISTS(
                           SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1
                         )",
                        [&range_id],
                        |row| row.get::<_, bool>(0),
                    )
                    .expect("newer range retention")
            );
        }
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("cleaned lineage count"),
            0
        );
        drop(catalog);
        SqliteCatalog::open(path).expect("reopen after closed proof cleanup");
    }

    #[test]
    fn retained_payload_intents_require_exact_queue_rows_and_ownership() {
        for mutation in [
            "deleted-queue",
            "deleted-ownership",
            "extra-owner",
            "tampered-queue",
        ] {
            let directory = tempdir().expect("catalog directory");
            let path = directory.path().join(format!("{mutation}.sqlite3"));
            let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
            support_root(&mut catalog, "root-a");
            let first = atomic_volume_batch(vec![batch(
                "range-a",
                "root-a",
                10,
                20,
                &["a.jpg", "b.jpg"],
            )]);
            let first_id = first.pages[0].enrollment.range.batch_id.clone();
            catalog
                .publish_persistent_journal_volume_batch(&first, 1_000, policy())
                .expect("publish retained intent evidence");
            match mutation {
                "deleted-queue" => {
                    catalog
                        .connection
                        .execute(
                            "DELETE FROM library_change_queue
                             WHERE id = (
                               SELECT change_id
                               FROM library_persistent_journal_queue_lineage
                               WHERE source_range_id = ?1 ORDER BY change_id LIMIT 1
                             )",
                            [&first_id],
                        )
                        .expect("delete queue row and cascading ownership");
                }
                "deleted-ownership" => {
                    catalog
                        .connection
                        .execute(
                            "DELETE FROM library_persistent_journal_queue_lineage
                             WHERE source_range_id = ?1 AND change_id = (
                               SELECT change_id
                               FROM library_persistent_journal_queue_lineage
                               WHERE source_range_id = ?1 ORDER BY change_id LIMIT 1
                             )",
                            [&first_id],
                        )
                        .expect("delete queue ownership");
                }
                "extra-owner" => {
                    let mut second = atomic_volume_batch(vec![batch(
                        "range-b",
                        "root-a",
                        20,
                        30,
                        &["other.jpg"],
                    )]);
                    second.pages[0].enrollment.range.enrolled_unix_ms = 2_000;
                    second.pages[0].checkpoint.updated_unix_ms = 2_000;
                    rebind_volume_batch_content(&mut second);
                    let second_id = second.pages[0].enrollment.range.batch_id.clone();
                    catalog
                        .publish_persistent_journal_volume_batch(&second, 2_000, policy())
                        .expect("publish independent queue owner");
                    let second_change_id = catalog
                        .connection
                        .query_row(
                            "SELECT change_id
                             FROM library_persistent_journal_queue_lineage
                             WHERE source_range_id = ?1 ORDER BY change_id LIMIT 1",
                            [&second_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .expect("second change ID");
                    catalog
                        .connection
                        .execute(
                            "INSERT INTO library_change_queue_catch_up_lineage(
                               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                             ) VALUES (?1, 'persistent_journal_v1', ?2, 1000)",
                            params![second_change_id, first_id],
                        )
                        .expect("attach extra generic queue owner");
                    catalog
                        .connection
                        .execute(
                            "INSERT INTO library_persistent_journal_queue_lineage(
                               source_range_id, change_id, enrolled_unix_ms
                             ) VALUES (?1, ?2, 1000)",
                            params![first_id, second_change_id],
                        )
                        .expect("attach extra persistent queue owner");
                }
                "tampered-queue" => {
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_change_queue
                             SET relative_path = 'tampered.jpg'
                             WHERE id = (
                               SELECT change_id
                               FROM library_persistent_journal_queue_lineage
                               WHERE source_range_id = ?1 ORDER BY change_id LIMIT 1
                             )",
                            [&first_id],
                        )
                        .expect("tamper queue intent content");
                }
                _ => unreachable!(),
            }
            assert_terminalizer_and_reopen_reject_intent_corruption(catalog, path, mutation);
        }
    }

    #[test]
    fn retained_payload_rejects_extra_or_missing_intent_after_valid_rekey() {
        for mutation in ["extra-intent", "missing-intent"] {
            let directory = tempdir().expect("catalog directory");
            let path = directory.path().join(format!("{mutation}.sqlite3"));
            let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
            support_root(&mut catalog, "root-a");
            let volume_batch = atomic_volume_batch(vec![batch(
                "range-a",
                "root-a",
                10,
                20,
                &["a.jpg", "b.jpg"],
            )]);
            let mut enrollment = volume_batch.pages[0].enrollment.clone();
            catalog
                .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
                .expect("publish retained payload intents");
            if mutation == "extra-intent" {
                enrollment.intents.push(LibraryChangeIntent {
                    relative_path: "payload-only.jpg".to_owned(),
                    first_sequence: 3,
                    most_recent_sequence: 3,
                    ..enrollment.intents[0].clone()
                });
            } else {
                enrollment.intents.pop().expect("remove payload intent");
            }
            rewrite_retained_range_payload(&catalog.connection, &mut enrollment);
            assert_terminalizer_and_reopen_reject_intent_corruption(catalog, path, mutation);
        }
    }

    #[test]
    fn cross_root_peer_queue_evidence_requires_exact_enrollment_provenance() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("peer-provenance.sqlite3");
        let catalog = catalog_with_pending_lineage(path.clone());
        let updated = catalog
            .connection
            .execute(
                "UPDATE library_change_queue_catch_up_lineage
                 SET enrolled_unix_ms = enrolled_unix_ms + 1
                 WHERE rowid = (
                   SELECT lineage.rowid
                   FROM library_change_queue_catch_up_lineage AS lineage
                   JOIN library_persistent_journal_queue_lineage AS ownership
                     ON ownership.change_id = lineage.change_id
                    AND ownership.source_range_id <> lineage.catch_up_watermark
                   WHERE lineage.catch_up_source = 'persistent_journal_v1'
                   ORDER BY lineage.rowid LIMIT 1
                 )",
                [],
            )
            .expect("tamper peer evidence enrollment provenance");
        assert_eq!(updated, 1);
        assert_terminalizer_and_reopen_reject_intent_corruption(
            catalog,
            path,
            "tampered cross-root peer enrollment provenance",
        );
    }

    #[test]
    fn coalesced_payload_queue_survives_retry_terminalization_and_cluster_cleanup() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("coalesced.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let volume_batch = atomic_volume_batch(vec![batch(
            "range-a",
            "root-a",
            10,
            20,
            &["same.jpg", "same.jpg"],
        )]);
        let old_range_id = volume_batch.pages[0].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish coalesced payload queue");
        let queue_shape: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_change_queue
                    WHERE catch_up_watermark = ?1),
                   (SELECT COUNT(*) FROM library_persistent_journal_queue_lineage
                    WHERE source_range_id = ?1),
                   (SELECT coalesced_observation_count FROM library_change_queue
                    WHERE catch_up_watermark = ?1)",
                [&old_range_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("coalesced durable queue shape");
        assert_eq!(queue_shape, (1, 1, 2));

        let leased = catalog
            .lease_library_changes("root-a", LibraryRootGeneration::initial(), 1_000, policy())
            .expect("lease coalesced queue")
            .pop()
            .expect("coalesced lease");
        catalog
            .retry_library_change(
                leased.change.id,
                leased.lease_generation,
                &LibraryChangeFailure {
                    code: "path_locked".to_owned(),
                    message: "The path remained locked.".to_owned(),
                },
                1_001,
                policy(),
            )
            .expect("retain retry-wait queue evidence");
        {
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("retry terminalizer transaction");
            super::finalize_persistent_journal_ranges(&transaction, 1_002)
                .expect("retry-wait evidence remains valid");
            transaction.commit().expect("commit retry terminalizer");
        }
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT lifecycle_state
                     FROM library_persistent_journal_range_lifecycle
                     WHERE source_range_id = ?1",
                    [&old_range_id],
                    |row| row.get::<_, String>(0),
                )
                .expect("retry lifecycle"),
            "pending"
        );

        let retried = catalog
            .lease_library_changes("root-a", LibraryRootGeneration::initial(), 10_000, policy())
            .expect("lease ready retry")
            .pop()
            .expect("retry becomes ready");
        catalog
            .complete_library_change(
                retried.change.id,
                retried.lease_generation,
                retried.change.catalog_revision_at_enqueue,
                10_001,
            )
            .expect("complete coalesced queue");
        {
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("terminal proof transaction");
            super::finalize_persistent_journal_ranges(&transaction, 10_002)
                .expect("terminalize exact queue ownership");
            transaction.commit().expect("commit terminal proof");
        }
        let mut newer = atomic_volume_batch(vec![batch("range-b", "root-a", 20, 30, &[])]);
        newer.pages[0].enrollment.range.enrolled_unix_ms = 11_000;
        newer.pages[0].checkpoint.updated_unix_ms = 11_000;
        rebind_volume_batch_content(&mut newer);
        let newer_range_id = newer.pages[0].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&newer, 11_000, policy())
            .expect("publish newer terminal proof");
        let next_generation = LibraryRootGeneration::initial()
            .next()
            .expect("next generation");
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root-a".to_owned(),
                    root_generation: next_generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "replacement.jpg".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 12_000,
                    most_recent_observed_unix_ms: 12_000,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                12_000,
                policy(),
            )
            .expect("retire completed persistent generation");
        {
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("cluster cleanup transaction");
            super::cleanup_completed_persistent_journal_history(&transaction, 12_001, 32)
                .expect("cleanup closed payload and queue cluster");
            transaction.commit().expect("commit cluster cleanup");
        }
        let retained: (bool, bool, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1),
                   EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?2),
                   (SELECT COUNT(*) FROM library_change_queue_catch_up_lineage
                    WHERE catch_up_source = 'persistent_journal_v1'
                      AND catch_up_watermark = ?1)",
                params![old_range_id, newer_range_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("closed cluster cleanup boundary");
        assert_eq!(retained, (false, true, 0));
        drop(catalog);
        SqliteCatalog::open(path).expect("reopen cleaned coalesced queue proof");
    }

    #[test]
    fn lineage_state_requires_exact_endpoint_lifecycle_proof() {
        for mutation in [
            "premature-completed",
            "one-end-completed",
            "tampered-superseded",
        ] {
            let directory = tempdir().expect("catalog directory");
            let path = directory.path().join(format!("{mutation}.sqlite3"));
            let catalog = catalog_with_pending_lineage(path.clone());
            match mutation {
                "premature-completed" => {
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_persistent_journal_cross_root_lineage
                             SET status = 'completed'",
                            [],
                        )
                        .expect("forge premature completed lineage");
                }
                "one-end-completed" => {
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_change_queue
                             SET status = 'completed', catalog_revision_at_success = 0",
                            [],
                        )
                        .expect("terminalize queue fixtures");
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_persistent_journal_range_lifecycle
                             SET lifecycle_state = 'completed', completed_unix_ms = 2000,
                                 updated_unix_ms = 2000
                             WHERE source_range_id = (
                               SELECT source_range_id
                               FROM library_persistent_journal_cross_root_ranges
                               WHERE participant_role = 'previous'
                             )",
                            [],
                        )
                        .expect("complete only previous owner lifecycle");
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_persistent_journal_cross_root_lineage
                             SET status = 'completed'",
                            [],
                        )
                        .expect("forge one-ended completed lineage");
                }
                "tampered-superseded" => {
                    catalog
                        .connection
                        .execute(
                            "UPDATE library_persistent_journal_cross_root_lineage
                             SET status = 'superseded'",
                            [],
                        )
                        .expect("forge superseded lineage under active owners");
                }
                _ => unreachable!(),
            }
            assert_terminalizer_and_reopen_reject_intent_corruption(catalog, path, mutation);
        }
    }

    #[test]
    fn lineage_completes_only_after_both_endpoint_orders_and_reopens() {
        for order in [["root-a", "root-b"], ["root-b", "root-a"]] {
            let directory = tempdir().expect("catalog directory");
            let path = directory.path().join(format!("{}-first.sqlite3", order[0]));
            let mut catalog = catalog_with_pending_lineage(path.clone());
            for (index, root_id) in order.into_iter().enumerate() {
                let transaction = catalog
                    .connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .expect("endpoint completion transaction");
                transaction
                    .execute(
                        "UPDATE library_change_queue
                         SET status = 'completed', catalog_revision_at_success = 0,
                             updated_unix_ms = ?1
                         WHERE root_id = ?2 AND status = 'pending'",
                        params![
                            2_000 + i64::try_from(index).expect("completion index"),
                            root_id
                        ],
                    )
                    .expect("complete endpoint queue");
                super::finalize_persistent_journal_ranges(
                    &transaction,
                    2_000 + i64::try_from(index).expect("completion index"),
                )
                .expect("terminalize endpoint lifecycle");
                transaction.commit().expect("commit endpoint completion");
                let expected = if index == 0 { "pending" } else { "completed" };
                assert_eq!(
                    catalog
                        .connection
                        .query_row(
                            "SELECT status
                             FROM library_persistent_journal_cross_root_lineage",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .expect("lineage status"),
                    expected
                );
                drop(catalog);
                catalog = SqliteCatalog::open(path.clone()).expect("reopen endpoint order");
            }
            let terminal: (i64, i64) = catalog
                .connection
                .query_row(
                    "SELECT
                       (SELECT COUNT(*)
                        FROM library_persistent_journal_range_lifecycle
                        WHERE lifecycle_state = 'completed'),
                       (SELECT COUNT(*)
                        FROM library_persistent_journal_root_state
                        WHERE continuity_state = 'current')",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("terminal endpoint proof");
            assert_eq!(terminal, (2, 2));
        }
    }

    #[test]
    fn terminalizer_refuses_to_publish_current_after_owner_loss() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_pending_lineage(directory.path().join("catalog.sqlite3"));
        catalog
            .connection
            .execute(
                "DELETE FROM library_persistent_journal_cross_root_ranges
                 WHERE participant_role = 'previous'",
                [],
            )
            .expect("remove previous owner");
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("terminal transaction");
        let error = super::finalize_persistent_journal_ranges(&transaction, 2_000)
            .expect_err("owner loss must block terminal publication");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        transaction.rollback().expect("roll back terminal attempt");
        let states = catalog
            .connection
            .prepare(
                "SELECT continuity_state FROM library_persistent_journal_checkpoints
                 ORDER BY root_id",
            )
            .expect("checkpoint state statement")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("checkpoint states")
            .collect::<Result<Vec<_>, _>>()
            .expect("checkpoint state rows");
        assert_eq!(states, ["catching_up", "catching_up"]);

        let mut parent_loss =
            catalog_with_pending_lineage(directory.path().join("deleted-parent-terminal.sqlite3"));
        parent_loss
            .connection
            .execute(
                "DELETE FROM library_persistent_journal_cross_root_lineage",
                [],
            )
            .expect("delete lineage parent with owner cascade");
        let transaction = parent_loss
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("parent-loss terminal transaction");
        let error = super::finalize_persistent_journal_ranges(&transaction, 2_000)
            .expect_err("missing payload child parent must block terminal publication");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        transaction
            .rollback()
            .expect("roll back parent-loss terminal attempt");
        let states = parent_loss
            .connection
            .prepare(
                "SELECT continuity_state FROM library_persistent_journal_checkpoints
                 ORDER BY root_id",
            )
            .expect("parent-loss checkpoint statement")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("parent-loss checkpoint states")
            .collect::<Result<Vec<_>, _>>()
            .expect("parent-loss checkpoint rows");
        assert_eq!(states, ["catching_up", "catching_up"]);
    }

    #[test]
    fn source_range_id_triggers_reject_null_and_nontext_insert_and_update() {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let volume_batch = atomic_volume_batch(vec![batch(
            "canonical-range",
            "root-a",
            10,
            20,
            &["pending.jpg"],
        )]);
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish canonical range");
        let insert_sql = "INSERT INTO library_persistent_journal_source_ranges(
               id, root_id, root_generation, volume_guid, volume_serial, journal_id,
               requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
               protocol_version, contract_version, status, enrolled_unix_ms,
               checkpointed_unix_ms, canonical_payload
             )
             SELECT ?1, root_id, root_generation, volume_guid, volume_serial, journal_id,
                    requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
                    protocol_version, contract_version, status, enrolled_unix_ms,
                    checkpointed_unix_ms, canonical_payload
             FROM library_persistent_journal_source_ranges LIMIT 1";
        for invalid in [
            rusqlite::types::Value::Null,
            rusqlite::types::Value::Integer(7),
        ] {
            catalog
                .connection
                .execute(insert_sql, [invalid.clone()])
                .expect_err("invalid insert ID type must be rejected");
            catalog
                .connection
                .execute(
                    "UPDATE library_persistent_journal_source_ranges SET id = ?1",
                    [invalid],
                )
                .expect_err("invalid update ID type must be rejected");
        }
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM library_persistent_journal_source_ranges",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("canonical range count"),
            1
        );
    }

    #[test]
    fn empty_complete_and_partial_pages_terminalize_in_the_publication_transaction() {
        let directory = tempdir().expect("catalog directory");
        let mut complete =
            catalog_with_roots(directory.path().join("complete.sqlite3"), &["root-a"]);
        support_root(&mut complete, "root-a");
        let complete_batch =
            atomic_volume_batch(vec![batch("empty-complete", "root-a", 10, 20, &[])]);
        complete
            .publish_persistent_journal_volume_batch(&complete_batch, 1_000, policy())
            .expect("publish empty complete page");
        let complete_state: (String, String) = complete
            .connection
            .query_row(
                "SELECT checkpoints.continuity_state, lifecycle.lifecycle_state
                 FROM library_persistent_journal_checkpoints AS checkpoints
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.root_id = checkpoints.root_id
                  AND ranges.root_generation = checkpoints.root_generation
                  AND ranges.covered_until_usn = checkpoints.next_unread_usn
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE checkpoints.root_id = 'root-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("empty complete lifecycle");
        assert_eq!(
            complete_state,
            ("current".to_owned(), "completed".to_owned())
        );

        let mut partial = catalog_with_roots(directory.path().join("partial.sqlite3"), &["root-a"]);
        support_root(&mut partial, "root-a");
        let mut first = batch("empty-partial", "root-a", 10, 30, &[]);
        first.range.covered_until_usn = JournalUsn::new(20).expect("partial covered");
        first.range.is_complete = false;
        first.range.batch_id = persistent_journal_batch_id(&first);
        let first = atomic_volume_batch(vec![first]);
        partial
            .publish_persistent_journal_volume_batch(&first, 1_000, policy())
            .expect("publish empty partial page");
        let partial_state: (String, String, String) = partial
            .connection
            .query_row(
                "SELECT checkpoints.continuity_state, checkpoints.next_unread_usn,
                        lifecycle.lifecycle_state
                 FROM library_persistent_journal_checkpoints AS checkpoints
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.root_id = checkpoints.root_id
                  AND ranges.root_generation = checkpoints.root_generation
                  AND ranges.covered_until_usn = checkpoints.next_unread_usn
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE checkpoints.root_id = 'root-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("empty partial lifecycle");
        assert_eq!(
            partial_state,
            (
                "catching_up".to_owned(),
                "20".to_owned(),
                "completed".to_owned()
            )
        );
    }

    #[test]
    fn v23_empty_range_rekeys_and_current_payload_tampering_fails_closed() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let batch = atomic_volume_batch(vec![batch("legacy-empty", "root-a", 10, 20, &[])]);
        let current_id = batch.pages[0].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&batch, 1_000, policy())
            .expect("publish current seed");
        drop(catalog);

        let connection = rusqlite::Connection::open(&path).expect("open v23 fixture");
        remove_v25_contract_for_legacy_fixture(&connection);
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 DROP TRIGGER library_persistent_journal_source_range_id_insert;
                 DROP TRIGGER library_persistent_journal_source_range_id_update;
                 ALTER TABLE library_persistent_journal_source_ranges
                   DROP COLUMN canonical_payload;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN journal_id;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN old_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN new_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage
                   DROP COLUMN previous_carry_id;
                 UPDATE library_persistent_journal_range_lifecycle
                   SET source_range_id = 'legacy-range' WHERE source_range_id <> 'legacy-range';
                 UPDATE library_persistent_journal_source_ranges
                   SET id = 'legacy-range' WHERE id <> 'legacy-range';
                 UPDATE schema_info SET version = 23;",
            )
            .expect("construct valid legacy v23 range");
        drop(connection);

        let catalog = SqliteCatalog::open(path.clone()).expect("migrate and rekey v23 range");
        let migrated: (String, Vec<u8>, String, i64) = catalog
            .connection
            .query_row(
                "SELECT ranges.id, ranges.canonical_payload, lifecycle.lifecycle_state,
                        (SELECT version FROM schema_info)
                 FROM library_persistent_journal_source_ranges AS ranges
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated canonical range");
        assert_ne!(migrated.0, "legacy-range");
        assert_eq!(migrated.0, current_id);
        assert_eq!(migrated.0.len(), 64);
        assert_eq!(migrated.2, "completed");
        assert_eq!(migrated.3, super::super::SCHEMA_VERSION);
        catalog
            .connection
            .execute(
                "UPDATE library_persistent_journal_source_ranges SET id = 'x'",
                [],
            )
            .expect_err("current schema must reject an arbitrary source-range ID");
        drop(catalog);

        let connection = rusqlite::Connection::open(&path).expect("open range tamper fixture");
        connection
            .execute(
                "UPDATE library_persistent_journal_source_ranges
                 SET enrolled_unix_ms = enrolled_unix_ms + 1",
                [],
            )
            .expect("tamper payload-bound range coordinate");
        drop(connection);
        let error = match SqliteCatalog::open(path.clone()) {
            Ok(_) => panic!("payload-bound range tampering must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        let connection = rusqlite::Connection::open(&path).expect("restore range fixture");
        connection
            .execute(
                "UPDATE library_persistent_journal_source_ranges
                 SET enrolled_unix_ms = enrolled_unix_ms - 1",
                [],
            )
            .expect("restore payload-bound range coordinate");
        drop(connection);

        let connection = rusqlite::Connection::open(&path).expect("open tamper fixture");
        connection
            .execute(
                "UPDATE library_persistent_journal_source_ranges
                 SET canonical_payload = X'00'",
                [],
            )
            .expect("tamper canonical payload");
        drop(connection);
        let error = match SqliteCatalog::open(path) {
            Ok(_) => panic!("tampered payload must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
    }

    #[test]
    fn v23_backfill_keeps_nonterminal_range_pending_after_rekey_and_reopen() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
        support_root(&mut catalog, "root-a");
        let batch = atomic_volume_batch(vec![batch(
            "legacy-pending",
            "root-a",
            10,
            20,
            &["pending.jpg"],
        )]);
        let old_id = batch.pages[0].enrollment.range.batch_id.clone();
        catalog
            .publish_persistent_journal_volume_batch(&batch, 1_000, policy())
            .expect("publish pending current seed");
        drop(catalog);

        let connection = rusqlite::Connection::open(&path).expect("open v23 fixture");
        remove_v25_contract_for_legacy_fixture(&connection);
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 DROP TRIGGER library_persistent_journal_source_range_id_insert;
                 DROP TRIGGER library_persistent_journal_source_range_id_update;
                 ALTER TABLE library_persistent_journal_source_ranges
                   DROP COLUMN canonical_payload;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN journal_id;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN old_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN new_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage
                   DROP COLUMN previous_carry_id;",
            )
            .expect("remove v24 columns");
        for sql in [
            "UPDATE library_persistent_journal_queue_lineage
             SET source_range_id = 'legacy-pending' WHERE source_range_id = ?1",
            "UPDATE library_persistent_journal_range_lifecycle
             SET source_range_id = 'legacy-pending' WHERE source_range_id = ?1",
            "UPDATE library_change_queue_catch_up_lineage
             SET catch_up_watermark = 'legacy-pending'
             WHERE catch_up_source = 'persistent_journal_v1' AND catch_up_watermark = ?1",
            "UPDATE library_change_queue
             SET catch_up_watermark = 'legacy-pending'
             WHERE catch_up_source = 'persistent_journal_v1' AND catch_up_watermark = ?1",
            "UPDATE library_persistent_journal_source_ranges
             SET id = 'legacy-pending' WHERE id = ?1",
        ] {
            connection
                .execute(sql, [&old_id])
                .expect("rekey pending v23 fixture");
        }
        connection
            .execute("UPDATE schema_info SET version = 23", [])
            .expect("mark v23 fixture");
        drop(connection);

        let catalog = SqliteCatalog::open(path.clone()).expect("migrate pending v23 range");
        let migrated: (String, String, String, i64) = catalog
            .connection
            .query_row(
                "SELECT ranges.id, lifecycle.lifecycle_state, checkpoints.continuity_state,
                        (SELECT COUNT(*) FROM library_change_queue
                         WHERE status IN ('pending', 'leased', 'retry_wait'))
                 FROM library_persistent_journal_source_ranges AS ranges
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 JOIN library_persistent_journal_checkpoints AS checkpoints
                   ON checkpoints.root_id = ranges.root_id
                  AND checkpoints.root_generation = ranges.root_generation",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated pending lifecycle");
        assert_ne!(migrated.0, "legacy-pending");
        assert_eq!(migrated.1, "pending");
        assert_eq!(migrated.2, "catching_up");
        assert_eq!(migrated.3, 1);
        drop(catalog);
        SqliteCatalog::open(path).expect("reopen migrated pending range");
    }

    fn remove_v25_contract_for_legacy_fixture(connection: &rusqlite::Connection) {
        super::super::migrations::downgrade_source_revision_contract_to_v30_for_test(connection);
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
                 DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
                 DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
                 DROP TABLE IF EXISTS library_live_gap_recovery_claims;
                 DROP TABLE IF EXISTS library_live_gap_recovery_contract;
                 DROP INDEX IF EXISTS library_scan_publication_namespace_root;
                 DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
                 DROP TABLE IF EXISTS library_root_publication_namespaces;
                 DROP TABLE IF EXISTS library_root_publication_namespace_contract;
                 DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard;
                 DROP TRIGGER library_metadata_inventory_spool_binding_update_guard;
                 DROP INDEX library_metadata_inventory_spool_entries_order;
                 DROP TABLE library_metadata_inventory_spool_entries;
                 DROP INDEX library_metadata_inventory_spool_directories_state;
                 DROP TABLE library_metadata_inventory_spool_directories;
                 DROP TABLE library_metadata_inventory_spools;
                 DROP TABLE library_metadata_inventory_spool_contract;
                 DROP TRIGGER library_metadata_inventory_candidate_owner_update_guard;
                 DROP TRIGGER library_metadata_inventory_candidate_owner_insert_guard;
                 DROP INDEX library_metadata_inventory_candidate_owners_change;
                 DROP TABLE library_metadata_inventory_candidate_owners;
                 DROP INDEX library_metadata_inventory_frontier_state;
                 DROP TABLE library_metadata_inventory_frontier;
                 DROP TABLE library_recovery_execution_contract;
                 DROP TRIGGER library_persistent_journal_baseline_update_guard;
                 DROP TRIGGER library_persistent_journal_baseline_insert_guard;
                 DROP INDEX library_persistent_journal_baselines_root;
                 DROP TABLE library_persistent_journal_baselines;
                 DROP TRIGGER library_recovery_authority_update_guard;
                 DROP TRIGGER library_recovery_authority_insert_guard;
                 DROP INDEX library_recovery_authorities_root;
                 DROP TABLE library_recovery_authorities;
                 DROP TABLE library_recovery_authority_contract;
                 DROP TRIGGER library_change_queue_lane_origin_update;
                 DROP TRIGGER library_change_queue_lane_insert;
                 DROP TRIGGER library_change_queue_lane_update_guard;
                 DROP TRIGGER library_change_queue_lane_insert_guard;
                 DROP INDEX library_change_queue_lanes_eligible;
                 DROP TABLE library_change_queue_lanes;
                 DROP TABLE library_change_lane_contract;",
            )
            .expect("remove v25 contract from legacy fixture");
    }

    fn catalog_with_roots(path: std::path::PathBuf, root_ids: &[&str]) -> SqliteCatalog {
        let mut catalog = SqliteCatalog::open(path).expect("catalog");
        for root_id in root_ids {
            let request = ScanRequest {
                scan_id: format!("scan-{root_id}"),
                root_path: format!("C:/{root_id}"),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            };
            catalog
                .begin_scan(&request, root_id, &request.root_path)
                .expect("begin root baseline");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: (*root_id).to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    protocol_version: 1,
                    contract_version: 1,
                    state: PersistentJournalCapabilityState::LiveOnly,
                    continuity: PersistentJournalContinuityState::LiveOnly,
                    failure: Some(PersistentJournalFailure {
                        code: "test_live_observer_only".to_owned(),
                        message: "The fixed catalog fixture owns a trusted live observer seam"
                            .to_owned(),
                    }),
                    updated_unix_ms: super::super::unix_time_ms(),
                })
                .expect("arm fixture first-import handoff");
            catalog
                .publish_scan(&request.scan_id, root_id, 0, 0)
                .expect("publish root baseline");
        }
        catalog
    }

    fn support_root(catalog: &mut SqliteCatalog, root_id: &str) {
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::CatchingUp,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("supported root capability");
        let baseline = checkpoint(root_id, 10, 10, 500);
        catalog
            .connection
            .execute(
                "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   last_failure_code, last_failure_message, updated_unix_ms
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   ?13, NULL, NULL, ?14
                 )",
                rusqlite::params![
                    baseline.root_id,
                    i64::try_from(baseline.root_generation.value()).expect("root generation"),
                    baseline.volume.volume_guid,
                    baseline.volume.canonical_serial(),
                    i64::from(baseline.root_file_reference.record_version()),
                    baseline.root_file_reference.as_bytes(),
                    baseline.journal_id.to_canonical_text(),
                    baseline.next_unread_usn.to_canonical_text(),
                    baseline.captured_exclusive_end.to_canonical_text(),
                    i64::try_from(baseline.covered_catalog_revision).expect("catalog revision"),
                    i64::from(baseline.protocol_version),
                    i64::from(baseline.contract_version),
                    super::continuity_state_text(baseline.continuity),
                    baseline.updated_unix_ms,
                ],
            )
            .expect("seed persistent journal baseline");
    }

    fn current_root(catalog: &mut SqliteCatalog, root_id: &str) {
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("current root capability");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&checkpoint(root_id, 10, 10, 500))
            .expect("current root checkpoint");
    }

    fn promote_watcher_gap(
        catalog: &mut SqliteCatalog,
        root_id: &str,
    ) -> PersistentJournalBaseline {
        let generation = LibraryRootGeneration::initial();
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Subtree,
                    relative_path: "album".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 1_000,
                    most_recent_observed_unix_ms: 1_000,
                    first_sequence: 11,
                    most_recent_sequence: 11,
                    coalesced_observation_count: 1,
                }],
                1_000,
                policy(),
            )
            .expect("enqueue watcher gap");
        let live = catalog
            .lease_authoritative_library_change(root_id, generation, 1_000, policy())
            .expect("lease watcher gap")
            .expect("watcher gap lease");
        assert_eq!(
            catalog
                .promote_live_watcher_gap_to_metadata_inventory(
                    live.change.id,
                    live.lease_generation,
                    &LibraryChangeFailure {
                        code: "metadata_inventory_required".to_owned(),
                        message: "The bounded watcher scope could not be reconstructed".to_owned(),
                    },
                    1_001,
                    policy(),
                )
                .expect("promote watcher gap"),
            crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
        );
        catalog
            .load_persistent_journal_baselines()
            .expect("load watcher recovery window")
            .into_iter()
            .find(|baseline| baseline.root_id == root_id)
            .expect("watcher recovery window")
    }

    fn require_baseline_root(catalog: &mut SqliteCatalog, root_id: &str) {
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("baseline-required root capability");
    }

    fn baseline_request(
        root_id: &str,
        opening_next_usn: i64,
    ) -> PersistentJournalBaselineStartRequest {
        PersistentJournalBaselineStartRequest {
            run_id: format!("baseline-{root_id}"),
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            authority_reason: crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
            volume: volume(),
            root_file_reference: JournalFileReference::V2([1; 8]),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            opening_next_usn: JournalUsn::new(opening_next_usn).expect("opening USN"),
            protocol_version: 1,
            contract_version: 1,
            authorized_unix_ms: 1_000,
        }
    }

    fn first_import_scan_request(scan_id: &str, root_id: &str) -> ScanRequest {
        ScanRequest {
            scan_id: scan_id.to_owned(),
            root_path: format!("C:/{root_id}"),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        }
    }

    fn first_import_baseline_request(
        request: &ScanRequest,
        root_id: &str,
        opening_next_usn: i64,
    ) -> PersistentJournalBaselineStartRequest {
        PersistentJournalBaselineStartRequest {
            run_id: request.scan_id.clone(),
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            authority_reason: crate::domain::LibraryRecoveryAuthorityReason::FirstImportBoundary,
            volume: volume(),
            root_file_reference: JournalFileReference::V2([1; 8]),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            opening_next_usn: JournalUsn::new(opening_next_usn).expect("opening USN"),
            protocol_version: 1,
            contract_version: 1,
            authorized_unix_ms: 1_000,
        }
    }

    fn closing_boundary(
        change_id: crate::domain::LibraryChangeId,
        closing_next_usn: i64,
    ) -> PersistentJournalBaselineClosingBoundary {
        PersistentJournalBaselineClosingBoundary {
            change_id,
            volume: volume(),
            root_file_reference: JournalFileReference::V2([1; 8]),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            closing_next_usn: JournalUsn::new(closing_next_usn).expect("closing USN"),
            protocol_version: 1,
            captured_unix_ms: 2_000,
        }
    }

    fn batch(
        batch_id: &str,
        root_id: &str,
        start: i64,
        end: i64,
        paths: &[&str],
    ) -> PersistentJournalEnrollmentBatch {
        let mut batch = PersistentJournalEnrollmentBatch {
            range: PersistentJournalSourceRange {
                batch_id: batch_id.to_owned(),
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                volume: volume(),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                requested_start_usn: JournalUsn::new(start).expect("start USN"),
                requested_end_usn: JournalUsn::new(end).expect("end USN"),
                covered_until_usn: JournalUsn::new(end).expect("covered USN"),
                is_complete: true,
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalRangeState::Enrolled,
                enrolled_unix_ms: 1_000,
                checkpointed_unix_ms: None,
            },
            intents: paths
                .iter()
                .enumerate()
                .map(|(index, path)| LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: (*path).to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::StartupCatchUp,
                    first_observed_unix_ms: 1_000,
                    most_recent_observed_unix_ms: 1_000,
                    first_sequence: u64::try_from(index + 1).expect("sequence"),
                    most_recent_sequence: u64::try_from(index + 1).expect("sequence"),
                    coalesced_observation_count: 1,
                })
                .collect(),
            cross_root_lineage: Vec::new(),
            carried_cross_root_lineage: Vec::new(),
            pending_renames: Vec::new(),
            consumed_pending_rename_ids: Vec::new(),
        };
        batch.range.batch_id = persistent_journal_batch_id(&batch);
        batch
    }

    fn checkpoint(
        root_id: &str,
        next_unread_usn: i64,
        captured_exclusive_end: i64,
        updated_unix_ms: i64,
    ) -> PersistentJournalCheckpoint {
        PersistentJournalCheckpoint {
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume(),
            root_file_reference: JournalFileReference::V2([1; 8]),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            next_unread_usn: JournalUsn::new(next_unread_usn).expect("next USN"),
            captured_exclusive_end: JournalUsn::new(captured_exclusive_end).expect("end USN"),
            covered_catalog_revision: 1,
            protocol_version: 1,
            contract_version: 1,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms,
        }
    }

    fn assert_root_unregister_journal_state(
        catalog: &SqliteCatalog,
        removed_root_id: &str,
        survivor_root_id: &str,
        survivor_range_id: &str,
    ) {
        let state: (i64, i64, i64, i64, i64, i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_roots WHERE id = ?1),
                   (SELECT COUNT(*) FROM library_persistent_journal_root_state
                    WHERE root_id = ?1),
                   (SELECT COUNT(*) FROM library_persistent_journal_root_state
                    WHERE root_id = ?2),
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges
                    WHERE id = ?3 AND root_id = ?2),
                   (SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage),
                   (SELECT COUNT(*) FROM library_persistent_journal_pending_renames),
                   (SELECT COUNT(*)
                    FROM library_change_queue_catch_up_lineage AS lineage
                    LEFT JOIN library_persistent_journal_source_ranges AS ranges
                      ON ranges.id = lineage.catch_up_watermark
                    WHERE lineage.catch_up_source = 'persistent_journal_v1'
                      AND ranges.id IS NULL),
                   (SELECT COUNT(*) FROM library_change_queue
                    WHERE root_id = ?1
                      AND status IN ('pending', 'leased', 'retry_wait'))",
                params![removed_root_id, survivor_root_id, survivor_range_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("root unregister journal state");
        assert_eq!(state, (0, 0, 1, 1, 0, 0, 0, 0));
    }

    fn assert_survivor_root_unregister_handoff_completed(
        catalog: &SqliteCatalog,
        survivor_root_id: &str,
        survivor_path: &str,
    ) {
        let state = catalog
            .connection
            .query_row(
                "SELECT queue.status, queue.catch_up_source, queue.catch_up_watermark
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND queue.root_generation = 1
                   AND lane.lane = 'p0_live' AND queue.intent_kind = 'reconcile'
                   AND queue.scope = 'path' AND queue.relative_path = ?2",
                params![survivor_root_id, survivor_path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .expect("completed survivor root-removal handoff");
        assert_eq!(state, ("completed".to_owned(), None, None));
    }

    fn ready_live_path_lease_unix_ms(catalog: &SqliteCatalog, root_id: &str) -> i64 {
        catalog
            .connection
            .query_row(
                "SELECT COALESCE(MAX(queue.ready_unix_ms), 0) + 1
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND queue.status = 'pending'
                   AND lane.lane = 'p0_live'",
                [root_id],
                |row| row.get(0),
            )
            .expect("ready live path lease time")
    }

    fn assert_pending_carry_rollback(
        catalog: &SqliteCatalog,
        expected: &PersistentJournalPendingRename,
    ) {
        assert_eq!(
            catalog
                .load_persistent_journal_pending_renames(
                    &volume(),
                    JournalIdentifier::new(9).expect("journal ID"),
                )
                .expect("load retained OLD carry"),
            std::slice::from_ref(expected)
        );
        let state: (i64, i64, String) = catalog
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges),
                   (SELECT COUNT(*) FROM library_persistent_journal_source_ranges
                    WHERE root_id = 'root-b'),
                   (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'root-b')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("rolled back carried handoff state");
        assert_eq!(state, (0, 0, "10".to_owned()));
    }

    fn atomic_volume_batch(
        batches: Vec<PersistentJournalEnrollmentBatch>,
    ) -> PersistentJournalVolumeBatch {
        PersistentJournalVolumeBatch {
            pages: batches
                .into_iter()
                .map(|mut enrollment| {
                    let previous_id = enrollment.range.batch_id.clone();
                    let content_id = persistent_journal_batch_id(&enrollment);
                    enrollment.range.batch_id = content_id.clone();
                    for lineage in &mut enrollment.cross_root_lineage {
                        if lineage.owner_source_range_id == previous_id {
                            lineage.owner_source_range_id.clone_from(&content_id);
                        }
                    }
                    for pending in &mut enrollment.pending_renames {
                        if pending.source_range_id == previous_id {
                            pending.source_range_id.clone_from(&content_id);
                        }
                    }
                    let mut checkpoint = checkpoint(
                        &enrollment.range.root_id,
                        enrollment.range.covered_until_usn.value(),
                        enrollment.range.requested_end_usn.value(),
                        2_000,
                    );
                    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
                    PersistentJournalVolumePage {
                        enrollment,
                        checkpoint,
                    }
                })
                .collect(),
        }
    }

    fn rebind_volume_batch_content(batch: &mut PersistentJournalVolumeBatch) {
        for page in &mut batch.pages {
            let previous_id = page.enrollment.range.batch_id.clone();
            let content_id = persistent_journal_batch_id(&page.enrollment);
            page.enrollment.range.batch_id.clone_from(&content_id);
            for lineage in &mut page.enrollment.cross_root_lineage {
                if lineage.owner_source_range_id == previous_id {
                    lineage.owner_source_range_id.clone_from(&content_id);
                }
            }
            for pending in &mut page.enrollment.pending_renames {
                if pending.source_range_id == previous_id {
                    pending.source_range_id.clone_from(&content_id);
                }
            }
        }
    }

    fn rebind_enrollment_content(batch: &mut PersistentJournalEnrollmentBatch) {
        let previous_id = batch.range.batch_id.clone();
        let content_id = persistent_journal_batch_id(batch);
        batch.range.batch_id.clone_from(&content_id);
        for lineage in &mut batch.cross_root_lineage {
            if lineage.owner_source_range_id == previous_id {
                lineage.owner_source_range_id.clone_from(&content_id);
            }
        }
        for pending in &mut batch.pending_renames {
            if pending.source_range_id == previous_id {
                pending.source_range_id.clone_from(&content_id);
            }
        }
    }

    fn lineage(owner_source_range_id: &str) -> PersistentJournalCrossRootLineage {
        PersistentJournalCrossRootLineage {
            lineage_id: "move-lineage".to_owned(),
            owner_source_range_id: owner_source_range_id.to_owned(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal"),
            file_reference: JournalFileReference::V3([2; 16]),
            old_usn: None,
            new_usn: None,
            previous_carry_id: None,
            previous_root_id: "root-a".to_owned(),
            previous_root_generation: LibraryRootGeneration::initial(),
            previous_relative_path: "old.jpg".to_owned(),
            current_root_id: "root-b".to_owned(),
            current_root_generation: LibraryRootGeneration::initial(),
            current_relative_path: "new.jpg".to_owned(),
            state: PersistentJournalLineageState::Pending,
        }
    }

    fn catalog_with_pending_lineage(path: std::path::PathBuf) -> SqliteCatalog {
        catalog_with_pending_lineage_batches(path).0
    }

    fn catalog_with_pending_lineage_batches(
        path: std::path::PathBuf,
    ) -> (
        SqliteCatalog,
        PersistentJournalEnrollmentBatch,
        PersistentJournalEnrollmentBatch,
    ) {
        let mut catalog = catalog_with_roots(path, &["root-a", "root-b"]);
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        let volume_batch = atomic_volume_batch(vec![previous, current]);
        let previous = volume_batch.pages[0].enrollment.clone();
        let current = volume_batch.pages[1].enrollment.clone();
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish pending two-owner lineage");
        (catalog, previous, current)
    }

    fn rewrite_retained_range_payload(
        connection: &rusqlite::Connection,
        enrollment: &mut PersistentJournalEnrollmentBatch,
    ) {
        let old_id = enrollment.range.batch_id.clone();
        rebind_enrollment_content(enrollment);
        let new_id = enrollment.range.batch_id.clone();
        let payload = persistent_journal_batch_payload(enrollment);
        connection
            .execute_batch("PRAGMA foreign_keys = OFF")
            .expect("disable fixture foreign keys");
        connection
            .execute(
                "UPDATE library_persistent_journal_source_ranges
                 SET id = ?1, canonical_payload = ?2 WHERE id = ?3",
                params![new_id, payload, old_id],
            )
            .expect("rekey tampered source range");
        for table in [
            "library_persistent_journal_queue_lineage",
            "library_persistent_journal_cross_root_ranges",
            "library_persistent_journal_range_lifecycle",
            "library_persistent_journal_pending_renames",
        ] {
            connection
                .execute(
                    &format!("UPDATE {table} SET source_range_id = ?1 WHERE source_range_id = ?2"),
                    params![new_id, old_id],
                )
                .expect("rekey retained range child");
        }
        connection
            .execute(
                "UPDATE library_change_queue_catch_up_lineage
                 SET catch_up_watermark = ?1
                 WHERE catch_up_source = ?2 AND catch_up_watermark = ?3",
                params![new_id, super::PERSISTENT_JOURNAL_CATCH_UP_SOURCE, old_id],
            )
            .expect("rekey retained queue watermark");
        connection
            .execute(
                "UPDATE library_change_queue
                 SET catch_up_watermark = ?1
                 WHERE catch_up_source = ?2 AND catch_up_watermark = ?3",
                params![new_id, super::PERSISTENT_JOURNAL_CATCH_UP_SOURCE, old_id],
            )
            .expect("rekey retained queue evidence");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("restore fixture foreign keys");
    }

    fn assert_terminalizer_and_reopen_reject_intent_corruption(
        mut catalog: SqliteCatalog,
        path: std::path::PathBuf,
        corruption: &str,
    ) {
        let error = {
            let transaction = catalog
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("corrupt terminalizer transaction");
            let error = super::finalize_persistent_journal_ranges(&transaction, 9_000)
                .expect_err("corrupt retained proof must block terminalization");
            drop(transaction);
            error
        };
        let current_count = catalog
            .connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_persistent_journal_root_state
                 WHERE continuity_state = 'current'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("corrupt continuity state");
        assert_eq!(
            error.code, "catalog_persistent_journal_contract_unverifiable",
            "unexpected terminalizer error for {corruption}"
        );
        assert_eq!(current_count, 0, "{corruption} must not publish Current");
        drop(catalog);
        assert_contract_reopen_fails(path, corruption);
    }

    fn assert_contract_reopen_fails(path: std::path::PathBuf, corruption: &str) {
        let error = match SqliteCatalog::open(path) {
            Ok(_) => panic!("{corruption} must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code, "catalog_persistent_journal_contract_unverifiable",
            "unexpected {corruption} error"
        );
    }

    fn volume() -> PersistentJournalVolumeIdentity {
        PersistentJournalVolumeIdentity {
            volume_guid: "volume-a".to_owned(),
            volume_serial: u64::MAX,
        }
    }

    fn policy() -> LibraryChangeQueuePolicy {
        LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..LibraryChangeQueuePolicy::default()
        }
    }
}

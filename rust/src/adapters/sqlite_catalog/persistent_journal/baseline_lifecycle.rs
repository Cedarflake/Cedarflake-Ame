use rusqlite::params;

use crate::domain::{
    LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, PersistentJournalBaseline,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselinePhase,
    PersistentJournalBaselineStartRequest, ScanError,
};

use super::super::change_queue::{cleanup_for_enqueue, enqueue_intents_in_transaction};

use super::super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

use super::{
    baseline_capability, capability, invalid_batch, load_baseline, load_baseline_for_root,
    load_checkpoint, map_queue_backpressure, recovery_checkpoint, require_active_root_generation,
};

impl SqliteCatalog {
    pub(crate) fn begin_journal_baseline_with_admission<Permit>(
        &mut self,
        request: &PersistentJournalBaselineStartRequest,
        policy: LibraryChangeQueuePolicy,
        acquire: impl FnOnce() -> Result<Permit, ScanError>,
    ) -> Result<PersistentJournalBaseline, ScanError> {
        request.validate()?;
        if !policy.is_valid() {
            return Err(invalid_batch("The baseline queue policy is invalid"));
        }
        let transaction = self.begin_write()?;
        let _permit = acquire()?;
        require_active_root_generation(&transaction, &request.root_id, request.root_generation)?;
        if request.authority_reason
            == crate::domain::LibraryRecoveryAuthorityReason::FirstImportBoundary
        {
            capability::require_running_first_import(
                &transaction,
                &request.root_id,
                request.root_generation,
                &request.run_id,
            )?;
        }
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
        baseline_capability::acquire(&transaction, request)?;
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
}

pub(super) fn capture_closing_boundary(
    catalog: &mut SqliteCatalog,
    boundary: &PersistentJournalBaselineClosingBoundary,
) -> Result<PersistentJournalBaseline, ScanError> {
    boundary.validate()?;
    let transaction = catalog.begin_write()?;
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
    let (authority_reason, recovery_scope) = transaction
        .query_row(
            "SELECT authority.reason, queue.scope
             FROM library_recovery_authorities AS authority
             JOIN library_change_queue AS queue ON queue.id = authority.change_id
             WHERE authority.change_id = ?1",
            [sqlite_integer(
                boundary.change_id.value(),
                "baseline change ID",
            )?],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(database_error)?;
    let is_recovery_window = !matches!(
        authority_reason.as_str(),
        "existing_root_baseline" | "first_import_boundary"
    );
    let recovery_checkpoint = if is_recovery_window {
        let checkpoint =
            load_checkpoint(&transaction, &baseline.root_id, baseline.root_generation)?
                .ok_or_else(|| {
                    ScanError::new(
                        "persistent_journal_baseline_closing_mismatch",
                        "The recovery closing boundary has no durable checkpoint",
                    )
                })?;
        Some(checkpoint)
    } else {
        None
    };
    let catalog_revision = super::super::load_catalog_revision(&transaction)?;
    let checkpoint_updated = if is_recovery_window {
        let checkpoint = recovery_checkpoint
            .as_ref()
            .expect("recovery windows retain a validated checkpoint");
        let replay_start = recovery_checkpoint::replay_start(
            checkpoint,
            &baseline,
            &authority_reason,
            &recovery_scope,
        )?;
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

use rusqlite::{OptionalExtension, params};

use super::super::change_queue::{
    insert_persistent_journal_recovery_control, transfer_pending_live_gap_claims_to_recovery,
};
use super::super::metadata_inventory::insert_metadata_inventory_recovery_authority;
use super::super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};
use super::{
    attach_recovery_window_if_available, invalid_batch, load_checkpoint,
    load_root_authority_for_key, mark_persistent_journal_recovery_required,
    require_active_root_generation,
};
use crate::domain::{
    LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthority,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind, ScanError,
};

pub(super) fn persist(
    catalog: &mut SqliteCatalog,
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
    let transaction = catalog.begin_write_in_lane(crate::domain::LibraryChangeLane::Journal)?;
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
                 'broker_after_current_failure', 'watcher_uncovered_gap', 'existing_root_baseline'
               )
               AND queue.status IN ('pending', 'leased', 'retry_wait')
               AND lane.lane = 'p2_recovery'
               AND (?3 = 0 OR queue.scope = 'root')
             ORDER BY authority.change_id LIMIT 1",
            params![
                failure.root_id,
                sqlite_integer(failure.root_generation.value(), "root generation")?,
                matches!(
                    failure.kind,
                    PersistentJournalRootFailureKind::AutomaticMetadataFallback(_)
                ),
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(change_id) = existing {
        let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "recovery change ID")?)
            .ok_or_else(|| invalid_batch("The recovery change ID is invalid"))?;
        attach_or_fall_back(&transaction, change_id, failure, failed_unix_ms)?;
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
    let change_id =
        insert_persistent_journal_recovery_control(&transaction, &intent, failed_unix_ms, policy)?;
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
    attach_or_fall_back(&transaction, change_id, failure, failed_unix_ms)?;
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

fn attach_or_fall_back(
    transaction: &rusqlite::Transaction<'_>,
    change_id: LibraryChangeId,
    failure: &PersistentJournalRootFailure,
    now: i64,
) -> Result<(), ScanError> {
    if matches!(
        failure.kind,
        PersistentJournalRootFailureKind::AutomaticMetadataFallback(_)
    ) {
        super::metadata_fallback::admit(
            transaction,
            change_id,
            &failure.root_id,
            failure.root_generation,
            now,
        )
    } else {
        attach_recovery_window_if_available(
            transaction,
            change_id,
            &failure.root_id,
            failure.root_generation,
            failure.opening_boundary.as_ref(),
            now,
        )
    }
}

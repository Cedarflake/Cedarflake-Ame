use super::*;

mod capacity;
mod retained;
pub(in crate::adapters::sqlite_catalog) use capacity::wake_metadata_inventory_capacity_deferrals;
pub(super) use capacity::{
    defer_for_capacity, is_typed_capacity_deferred_live_gap, shape_predicate,
};
pub(super) use retained::eligible_query;
pub(super) use retained::{candidate as retained_candidate, transfer_one as transfer_retained};

fn is_live_only(
    connection: &Connection,
    root_id: &str,
    generation: LibraryRootGeneration,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_persistent_journal_root_state AS journal
         JOIN library_change_root_state AS root
           ON root.root_id = journal.root_id AND root.generation = journal.root_generation
         WHERE journal.root_id = ?1 AND journal.root_generation = ?2
           AND root.is_active = 1 AND journal.capability_state = 'live_only'
           AND journal.continuity_state = 'live_only')",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?
            ],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn promote(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    failure: &LibraryChangeFailure,
    promoted_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    validate_policy(policy)?;
    validate_failure(failure)?;
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Live)?;
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
        match super::super::persistent_journal::load_current_recovery_opening_boundary(
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
                        message:
                            "The P0 live gap is durably waiting for continuous P1 journal ownership"
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
    let outcome = promote_scoped(&transaction, &current, failure, promoted_unix_ms, policy)?;
    transaction.commit().map_err(database_error)?;
    Ok(outcome)
}

fn promote_scoped(
    transaction: &Transaction<'_>,
    current: &crate::domain::DurableLibraryChange,
    failure: &LibraryChangeFailure,
    promoted_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    let change_id = current.id;
    let lease_generation = current.lease_generation;
    let opening_boundary =
        match super::super::persistent_journal::load_current_recovery_opening_boundary(
            transaction,
            &current.intent.root_id,
            current.intent.root_generation,
        ) {
            Ok(boundary) => Some(boundary),
            Err(error)
                if error.code == "persistent_journal_recovery_boundary_unavailable"
                    && is_live_only(
                        transaction,
                        &current.intent.root_id,
                        current.intent.root_generation,
                    )? =>
            {
                None
            }
            Err(error) if error.code == "persistent_journal_recovery_boundary_unavailable" => {
                super::super::persistent_journal::mark_persistent_journal_recovery_required(
                    transaction,
                    &current.intent.root_id,
                    current.intent.root_generation,
                    &error.code,
                    &error.message,
                    promoted_unix_ms,
                )?;
                retry_leased_change_in_transaction(
                    transaction,
                    change_id,
                    lease_generation,
                    failure,
                    promoted_unix_ms,
                    policy,
                )?;
                return Ok(LibraryChangeLeaseUpdateOutcome::Applied);
            }
            Err(error) => return Err(error),
        };
    let active = load_active_changes(
        transaction,
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
        transaction,
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
    transfer_catch_up_lineage(transaction, [change_id], recovery_change_id)?;
    insert_metadata_inventory_recovery_authority(
        transaction,
        &LibraryRecoveryAuthority {
            change_id: recovery_change_id,
            run_id: format!("watcher-gap-promotion-{}", recovery_change_id.value()),
            root_id: recovery_intent.root_id.clone(),
            root_generation: recovery_intent.root_generation,
            reason: LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
            opening_boundary: opening_boundary.clone(),
            authorized_unix_ms: promoted_unix_ms,
            retired_unix_ms: None,
        },
    )?;
    if let Some(opening_boundary) = &opening_boundary {
        super::super::persistent_journal::insert_watcher_gap_recovery_window(
            transaction,
            recovery_change_id,
            &recovery_intent.root_id,
            recovery_intent.root_generation,
            opening_boundary,
            failure,
            promoted_unix_ms,
        )?;
    }
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
        transaction,
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
    Ok(LibraryChangeLeaseUpdateOutcome::Applied)
}

use super::*;
use crate::domain::JournalScopeRecoveryOutcome;

pub(super) fn promote(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    failure: &LibraryChangeFailure,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    validate_policy(policy)?;
    validate_failure(failure)?;
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Journal)?;
    let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
    if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
        transaction.commit().map_err(database_error)?;
        return Ok(outcome);
    }
    let parent = load_change(
        &transaction,
        sqlite_integer(change_id.value(), "change ID")?,
    )?;
    let journal_lane: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_change_queue_lanes
         WHERE change_id = ?1 AND lane = 'p1_journal')",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if parent.intent.origin != LibraryChangeOrigin::StartupCatchUp
        || parent.intent.scope == LibraryChangeScope::Path
        || !journal_lane
        || failure.code != "metadata_inventory_required"
    {
        return Err(ScanError::new(
            "journal_scope_recovery_promotion_invalid",
            "Only an oversized leased journal namespace task may create scoped recovery",
        ));
    }
    let existing: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_journal_scope_recoveries
         WHERE parent_change_id = ?1)",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if existing {
        return Err(ScanError::new(
            "journal_scope_recovery_already_started",
            "The journal task already owns ordered scoped recovery",
        ));
    }
    if !has_capacity(&transaction, &parent, policy)? {
        yield_parent(&transaction, &parent, now_unix_ms, true)?;
        transaction.commit().map_err(database_error)?;
        return Ok(LibraryChangeLeaseUpdateOutcome::Applied);
    }
    let child = insert_child(
        &transaction,
        &parent,
        &parent.intent.relative_path,
        parent.intent.scope,
        now_unix_ms,
        policy,
    )?;
    transaction
        .execute(
            "INSERT INTO library_journal_scope_recoveries(parent_change_id, current_change_id)
         VALUES (?1, ?2)",
            params![
                sqlite_integer(change_id.value(), "change ID")?,
                sqlite_integer(child.value(), "recovery change ID")?
            ],
        )
        .map_err(database_error)?;
    // Journal range lineage stays on the nonterminal parent until both namespaces finish.
    yield_parent(&transaction, &parent, now_unix_ms, false)?;
    transaction.commit().map_err(database_error)?;
    Ok(LibraryChangeLeaseUpdateOutcome::Applied)
}

pub(super) fn advance(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    catalog_revision: u64,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<Option<JournalScopeRecoveryOutcome>, ScanError> {
    validate_policy(policy)?;
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Journal)?;
    let children = transaction
        .query_row(
            "SELECT current_change_id, previous_change_id
         FROM library_journal_scope_recoveries WHERE parent_change_id = ?1",
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    let Some((current, previous)) = children else {
        transaction.commit().map_err(database_error)?;
        return Ok(None);
    };
    let outcome = classify_lease_update(
        &transaction,
        change_id,
        lease_generation,
        Some(catalog_revision),
    )?;
    if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
        transaction.commit().map_err(database_error)?;
        return Ok(Some(JournalScopeRecoveryOutcome::Rejected(outcome)));
    }
    let parent = load_change(
        &transaction,
        sqlite_integer(change_id.value(), "change ID")?,
    )?;
    let completed: bool = transaction
        .query_row(
            "SELECT status = 'completed' FROM library_change_queue WHERE id = ?1",
            [previous.unwrap_or(current)],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !completed {
        yield_parent(&transaction, &parent, now_unix_ms, false)?;
        transaction.commit().map_err(database_error)?;
        return Ok(Some(JournalScopeRecoveryOutcome::Deferred));
    }
    if previous.is_none()
        && let Some(path) = &parent.intent.previous_relative_path
    {
        if !has_capacity(&transaction, &parent, policy)? {
            yield_parent(&transaction, &parent, now_unix_ms, true)?;
            transaction.commit().map_err(database_error)?;
            return Ok(Some(JournalScopeRecoveryOutcome::Deferred));
        }
        let child = insert_child(
            &transaction,
            &parent,
            path,
            LibraryChangeScope::Subtree,
            now_unix_ms,
            policy,
        )?;
        transaction
            .execute(
                "UPDATE library_journal_scope_recoveries SET previous_change_id = ?2
             WHERE parent_change_id = ?1 AND previous_change_id IS NULL",
                params![
                    sqlite_integer(change_id.value(), "change ID")?,
                    sqlite_integer(child.value(), "recovery change ID")?
                ],
            )
            .map_err(database_error)?;
        yield_parent(&transaction, &parent, now_unix_ms, false)?;
        transaction.commit().map_err(database_error)?;
        return Ok(Some(JournalScopeRecoveryOutcome::Deferred));
    }
    transaction
        .execute(
            "UPDATE library_change_queue SET status = 'completed', lease_expires_unix_ms = NULL,
           catalog_revision_at_success = ?1, updated_unix_ms = ?2
         WHERE id = ?3 AND status = 'leased' AND lease_generation = ?4",
            params![
                sqlite_integer(catalog_revision, "catalog revision")?,
                now_unix_ms,
                sqlite_integer(change_id.value(), "change ID")?,
                sqlite_integer(lease_generation, "lease generation")?
            ],
        )
        .map_err(database_error)?;
    wake_metadata_inventory_capacity_deferrals(
        &transaction,
        &parent.intent.root_id,
        parent.intent.root_generation,
        now_unix_ms,
    )?;
    transaction.commit().map_err(database_error)?;
    Ok(Some(JournalScopeRecoveryOutcome::Completed))
}

fn has_capacity(
    transaction: &Transaction<'_>,
    parent: &crate::domain::DurableLibraryChange,
    policy: LibraryChangeQueuePolicy,
) -> Result<bool, ScanError> {
    let active = load_active_changes(
        transaction,
        &parent.intent.root_id,
        parent.intent.root_generation,
        policy.max_unresolved_changes,
    )?;
    Ok(lane_admission_allows(
        policy,
        LibraryChangeLane::Recovery,
        active_lane_counts(&active).adding(LibraryChangeLane::Recovery, 1),
    ))
}

fn insert_child(
    transaction: &Transaction<'_>,
    parent: &crate::domain::DurableLibraryChange,
    relative_path: &str,
    scope: LibraryChangeScope,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeId, ScanError> {
    let mut intent = parent.intent.clone();
    intent.origin = LibraryChangeOrigin::MetadataInventory;
    intent.kind = if scope == LibraryChangeScope::Root {
        LibraryChangeIntentKind::FreshnessUnknown
    } else {
        LibraryChangeIntentKind::Reconcile
    };
    intent.scope = scope;
    intent.relative_path = relative_path.to_owned();
    intent.previous_relative_path = None;
    validate_intent_batch(std::slice::from_ref(&intent), &intent)?;
    let child = insert_change(
        transaction,
        &intent,
        now_unix_ms,
        parent.catalog_revision_at_enqueue,
        None,
        policy,
    )?;
    insert_metadata_inventory_recovery_authority(
        transaction,
        &LibraryRecoveryAuthority {
            change_id: child,
            run_id: format!("journal-scope-{}", child.value()),
            root_id: intent.root_id,
            root_generation: intent.root_generation,
            reason: LibraryRecoveryAuthorityReason::JournalReconstructionFailure,
            opening_boundary: None,
            authorized_unix_ms: now_unix_ms,
            retired_unix_ms: None,
        },
    )?;
    Ok(child)
}

fn yield_parent(
    transaction: &Transaction<'_>,
    parent: &crate::domain::DurableLibraryChange,
    now_unix_ms: i64,
    capacity_wait: bool,
) -> Result<(), ScanError> {
    let (status, retry_at, failure) = if capacity_wait {
        (
            "retry_wait",
            Some(now_unix_ms.saturating_add(250)),
            "journal_scope_capacity_deferred",
        )
    } else {
        ("pending", None, "journal_scope_waiting_for_inventory")
    };
    transaction
        .execute(
            "UPDATE library_change_queue SET status = ?1, next_retry_unix_ms = ?2,
           attempt_count = CASE WHEN attempt_count > 0 THEN attempt_count - 1 ELSE 0 END,
           lease_expires_unix_ms = NULL, last_failure_code = ?3,
           last_failure_message = 'The journal namespace is awaiting bounded metadata recovery',
           updated_unix_ms = ?4
         WHERE id = ?5 AND status = 'leased' AND lease_generation = ?6",
            params![
                status,
                retry_at,
                failure,
                now_unix_ms,
                sqlite_integer(parent.id.value(), "change ID")?,
                sqlite_integer(parent.lease_generation, "lease generation")?
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

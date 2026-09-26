use super::*;

pub(in crate::adapters::sqlite_catalog::change_queue) fn shape_predicate(alias: &str) -> String {
    format!("{alias}.origin = 'live_notification'
      AND {alias}.previous_relative_path IS NULL
      AND (({alias}.intent_kind = 'freshness_unknown' AND {alias}.scope = 'root' AND {alias}.relative_path = '')
        OR ({alias}.intent_kind = 'reconcile' AND {alias}.scope = 'subtree' AND {alias}.relative_path <> ''))
      AND EXISTS(SELECT 1 FROM library_change_queue_lanes AS capacity_lane
        WHERE capacity_lane.change_id = {alias}.id AND capacity_lane.lane = 'p0_live')
      AND NOT EXISTS(SELECT 1 FROM library_live_gap_recovery_claims AS capacity_claim
        WHERE capacity_claim.gap_change_id = {alias}.id)")
}

pub(in crate::adapters::sqlite_catalog::change_queue) fn defer_for_capacity(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    deferral: LibraryChangeCapacityDeferral,
    deferred_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    validate_policy(policy)?;
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Live)?;
    let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
    if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
        transaction.commit().map_err(database_error)?;
        return Ok(outcome);
    }
    let is_bounded_live_gap = transaction
        .query_row(
            &format!(
                "SELECT {} FROM library_change_queue WHERE id = ?1",
                shape_predicate("library_change_queue")
            ),
            [sqlite_integer(change_id.value(), "change ID")?],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !is_bounded_live_gap || deferral != LibraryChangeCapacityDeferral::MetadataInventoryLane {
        return Err(ScanError::new(
            "change_queue_capacity_deferral_invalid",
            "Only a leased P0 bounded live gap may wait for P2 metadata-inventory capacity",
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

pub(in crate::adapters::sqlite_catalog::change_queue) fn is_typed_capacity_deferred_live_gap(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM library_change_queue AS queue
              WHERE queue.id = ?1 AND queue.status IN ('leased', 'retry_wait')
                AND queue.last_failure_code = ?2 AND {})",
                shape_predicate("queue")
            ),
            params![
                sqlite_integer(change_id.value(), "change ID")?,
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
            ],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog) fn wake_metadata_inventory_capacity_deferrals(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    released_unix_ms: i64,
) -> Result<u32, ScanError> {
    let updated = transaction
        .execute(
            &format!(
                "UPDATE library_change_queue SET next_retry_unix_ms = ?1, updated_unix_ms = ?1
              WHERE root_id = ?2 AND root_generation = ?3 AND status = 'retry_wait'
                AND last_failure_code = ?4 AND {}
                AND (next_retry_unix_ms IS NULL OR next_retry_unix_ms > ?1)",
                shape_predicate("library_change_queue")
            ),
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

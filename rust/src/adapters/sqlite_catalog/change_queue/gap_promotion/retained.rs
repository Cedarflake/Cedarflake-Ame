use super::*;

pub(in crate::adapters::sqlite_catalog::change_queue) fn eligible_query(
    filter: &str,
    attempts: &str,
) -> String {
    format!("SELECT queue.id FROM library_change_queue AS queue
         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
         JOIN library_change_root_state AS active
           ON active.root_id = queue.root_id AND active.generation = queue.root_generation
         JOIN library_persistent_journal_root_state AS journal
           ON journal.root_id = queue.root_id AND journal.root_generation = queue.root_generation
         JOIN library_roots AS root ON root.id = queue.root_id
         JOIN library_root_publication_namespaces AS namespace
           ON namespace.root_id = queue.root_id AND namespace.root_generation = queue.root_generation
         WHERE active.is_active = 1
           AND root.active_scan_id IS NOT NULL
           AND journal.capability_state = 'live_only' AND journal.continuity_state = 'live_only'
           AND queue.status = 'retry_wait' AND queue.attempt_count >= {attempts}
           AND queue.next_retry_unix_ms IS NULL AND queue.lease_expires_unix_ms IS NULL
           AND queue.last_failure_code = 'metadata_inventory_required'
           AND queue.origin = 'live_notification' AND lane.lane = 'p0_live'
           AND queue.intent_kind = 'reconcile' AND queue.scope = 'subtree'
           AND queue.relative_path <> '' AND queue.previous_relative_path IS NULL
           AND queue.authoritative_scan_id IS NULL AND queue.superseded_by_change_id IS NULL
           AND NOT EXISTS(SELECT 1 FROM library_live_gap_recovery_claims AS claim
                          WHERE claim.gap_change_id = queue.id)
           AND NOT EXISTS(SELECT 1 FROM library_recovery_authorities AS authority
                          WHERE authority.change_id = queue.id)
           AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_candidate_owners AS owner
                          WHERE owner.change_id = queue.id)
 AND ({filter}) ")
}

pub(in crate::adapters::sqlite_catalog::change_queue) fn candidate(
    connection: &Connection,
    root_id: &str,
    generation: LibraryRootGeneration,
    policy: LibraryChangeQueuePolicy,
) -> Result<Option<i64>, ScanError> {
    let generation = sqlite_integer(generation.value(), "root generation")?;
    let selected = connection
        .query_row(
            &format!(
                "{} ORDER BY queue.id LIMIT 1",
                eligible_query("queue.root_id = ?1 AND queue.root_generation = ?2", "?3")
            ),
            params![root_id, generation, i64::from(policy.max_attempts)],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let mut counts = active_lane_counts(&[]).adding(LibraryChangeLane::Recovery, 1);
    let mut statement = connection
        .prepare(
            "SELECT lane.lane, COUNT(*) FROM library_change_queue AS queue
         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
         WHERE queue.root_id = ?1 AND queue.root_generation = ?2 AND queue.id <> ?3
           AND queue.status IN ('pending', 'leased', 'retry_wait') GROUP BY lane.lane",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![root_id, generation, selected], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(database_error)?;
    for row in rows {
        let (lane, count) = row.map_err(database_error)?;
        counts = counts.adding(parse_admission_lane(&lane)?, count as usize);
    }
    Ok(lane_admission_allows(policy, LibraryChangeLane::Recovery, counts).then_some(selected))
}

pub(in crate::adapters::sqlite_catalog::change_queue) fn transfer_one(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    if candidate(&catalog.connection, root_id, generation, policy)?.is_none() {
        return Ok(());
    }
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Live)?;
    if let Some(id) = candidate(&transaction, root_id, generation, policy)? {
        let current = load_change(&transaction, id)?;
        validate_intent_batch(std::slice::from_ref(&current.intent), &current.intent)?;
        let failure = current.last_failure.as_ref().ok_or_else(|| {
            ScanError::new(
                "live_gap_retained_evidence_missing",
                "The retained gap has no failure evidence",
            )
        })?;
        // Transfer the original evidence without restarting its exhausted lease lifecycle.
        // The P2 source guard still proves the current namespace before inventory or absence.
        promote_scoped(&transaction, &current, failure, now_unix_ms, policy)?;
    }
    transaction.commit().map_err(database_error)
}

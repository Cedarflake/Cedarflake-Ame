use super::*;

impl SqliteCatalog {
    pub(super) fn lease_library_changes_matching(
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
        let defer_unlineaged = first_import_replay::blocks_unlineaged_live_changes(
            &self.connection,
            root_id,
            root_generation,
        )?;
        let pass_is_needed = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_root_state AS state
                   WHERE state.root_id = ?1 AND state.generation = ?2 AND state.is_active = 1
                 ) AND EXISTS(
                   SELECT 1 FROM library_change_queue AS queue
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND (?7 = 0 OR NOT EXISTS(
                       SELECT 1 FROM library_change_queue_lanes AS lane
                       WHERE lane.change_id = queue.id AND lane.lane = 'p0_live'
                     ) OR EXISTS(
                       SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                       WHERE lineage.change_id = queue.id
                         AND lineage.catch_up_source = 'persistent_journal_v1'
                     ))
                     AND NOT EXISTS(
                       SELECT 1 FROM library_journal_scope_recoveries AS recovery
                       JOIN library_change_queue AS child
                         ON child.id = COALESCE(recovery.previous_change_id, recovery.current_change_id)
                       WHERE recovery.parent_change_id = queue.id AND child.status <> 'completed'
                     )
                     AND (
                       (queue.status = 'retry_wait' AND queue.attempt_count >= ?3
                         AND queue.next_retry_unix_ms IS NOT NULL)
                       OR (
                         (
                           ?5 = 0
                           OR (?5 = 1 AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown')
                           OR (?5 = 10 AND queue.scope <> 'path'
                             AND EXISTS(SELECT 1 FROM library_change_queue_lanes AS lane
                               WHERE lane.change_id = queue.id AND lane.lane = 'p1_journal'))
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
                                   OR (authority.reason = 'journal_reconstruction_failure' AND EXISTS(
                                     SELECT 1 FROM library_journal_scope_recoveries AS recovery
                                     WHERE recovery.current_change_id = authority.change_id
                                       OR recovery.previous_change_id = authority.change_id
                                   ))
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
                    defer_unlineaged,
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
            | LeaseSelection::LiveAuthoritative
            | LeaseSelection::JournalAuthoritative => 1,
            LeaseSelection::All
            | LeaseSelection::Path
            | LeaseSelection::PathLane(_)
            | LeaseSelection::RecoveryCandidates
            | LeaseSelection::UnownedRecoveryPaths => i64::from(policy.max_lease_batch),
        };
        let defer_unlineaged = first_import_replay::blocks_unlineaged_live_changes(
            &transaction,
            root_id,
            root_generation,
        )?;
        let change_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT id
                 FROM library_change_queue
                  WHERE root_id = ?1 AND root_generation = ?2
                    AND (?8 = 0 OR NOT EXISTS(
                      SELECT 1 FROM library_change_queue_lanes AS lane
                      WHERE lane.change_id = library_change_queue.id AND lane.lane = 'p0_live'
                    ) OR EXISTS(
                      SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                      WHERE lineage.change_id = library_change_queue.id
                        AND lineage.catch_up_source = 'persistent_journal_v1'
                    ))
                    AND NOT EXISTS(
                      SELECT 1 FROM library_journal_scope_recoveries AS recovery
                      JOIN library_change_queue AS child
                        ON child.id = COALESCE(recovery.previous_change_id, recovery.current_change_id)
                      WHERE recovery.parent_change_id = library_change_queue.id
                        AND child.status <> 'completed'
                    )
                    AND attempt_count < ?3
                    AND (?6 <> 6 OR ?7 IS NULL OR id = ?7)
                    AND (
                     ?6 = 0
                     OR (?6 = 1 AND scope = 'path' AND intent_kind <> 'freshness_unknown')
                     OR (?6 = 10 AND scope <> 'path'
                       AND EXISTS(SELECT 1 FROM library_change_queue_lanes AS lane
                         WHERE lane.change_id = library_change_queue.id AND lane.lane = 'p1_journal'))
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
                             OR (authority.reason = 'journal_reconstruction_failure' AND EXISTS(
                               SELECT 1 FROM library_journal_scope_recoveries AS recovery
                               WHERE recovery.current_change_id = authority.change_id
                                 OR recovery.previous_change_id = authority.change_id
                             ))
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
                        defer_unlineaged,
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
pub(super) enum LeaseSelection {
    All,
    Path,
    Authoritative,
    PathLane(LibraryChangeLane),
    RecoveryAuthoritative,
    RecoveryCandidates,
    LiveAuthoritative,
    JournalAuthoritative,
    UnownedRecoveryPaths,
}

impl LeaseSelection {
    pub(super) const fn sql_value(self) -> i64 {
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
            Self::JournalAuthoritative => 10,
        }
    }
}

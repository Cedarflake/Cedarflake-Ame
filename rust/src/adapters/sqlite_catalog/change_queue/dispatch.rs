use rusqlite::params;

use crate::domain::{
    LeasedLibraryChange, LibraryChangeId, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration, ScanError,
};
use crate::ports::LibraryChangeDispatch;

use super::{
    LeaseSelection, SqliteCatalog, database_error, gap_promotion, load_change, mark_superseded,
    merge_newer_evidence, merge_older_evidence, root_generation_is_current, sqlite_integer,
    update_change, validate_intent_batch, validate_policy, validate_root_id,
};

impl LibraryChangeDispatch for SqliteCatalog {
    fn compact_legacy_unowned_recovery_controls(
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

    fn lease_metadata_inventory_recovery(
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

    fn lease_live_authoritative_library_change(
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
        if leased.is_empty() {
            gap_promotion::transfer_retained(self, root_id, root_generation, now_unix_ms, policy)?;
        }
        Ok(leased.pop())
    }

    fn lease_journal_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::JournalAuthoritative,
        )
        .map(|mut changes| changes.pop())
    }
}

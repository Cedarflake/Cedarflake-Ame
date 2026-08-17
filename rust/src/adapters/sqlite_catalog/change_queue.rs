use rusqlite::params;

use crate::domain::{
    LeasedLibraryChange, LibraryChangeEnqueueReport, LibraryChangeFailure, LibraryChangeId,
    LibraryChangeIntent, LibraryChangeLeaseUpdateOutcome, LibraryChangeQueueMetrics,
    LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError,
};
use crate::ports::LibraryChangeQueue;

use super::{SqliteCatalog, database_error, load_catalog_revision, sqlite_integer, sqlite_u32};

mod coalescing;
mod persistence;

use coalescing::{
    enqueue_one, validate_failure, validate_intent_batch, validate_policy, validate_root_id,
};
pub(super) use persistence::retire_root_change_queue;
use persistence::{
    GenerationDisposition, classify_lease_update, cleanup_terminal_records,
    establish_root_generation, load_change, load_metrics, next_retry_deadline,
    recover_expired_leases, root_generation_is_current,
};

impl LibraryChangeQueue for SqliteCatalog {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        validate_policy(policy)?;
        let Some(first) = intents.first() else {
            return Ok(LibraryChangeEnqueueReport::default());
        };
        validate_intent_batch(intents, first)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let retention_millis = i64::try_from(policy.terminal_retention_millis).unwrap_or(i64::MAX);
        cleanup_terminal_records(
            &transaction,
            enqueued_unix_ms.saturating_sub(retention_millis),
            policy.cleanup_batch,
        )?;
        let mut report = LibraryChangeEnqueueReport::default();
        match establish_root_generation(
            &transaction,
            &first.root_id,
            first.root_generation,
            enqueued_unix_ms,
        )? {
            GenerationDisposition::Current { superseded_count } => {
                report.superseded_count = superseded_count;
            }
            GenerationDisposition::Stale => {
                report.stale_generation_count = u32::try_from(intents.len()).unwrap_or(u32::MAX);
                transaction.commit().map_err(database_error)?;
                return Ok(report);
            }
        }
        let catalog_revision = load_catalog_revision(&transaction)?;
        for intent in intents {
            enqueue_one(
                &transaction,
                intent,
                enqueued_unix_ms,
                catalog_revision,
                policy,
                &mut report,
            )?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(report)
    }

    fn lease_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        if !root_generation_is_current(&transaction, root_id, root_generation)? {
            transaction.commit().map_err(database_error)?;
            return Ok(Vec::new());
        }
        recover_expired_leases(&transaction, root_id, root_generation, now_unix_ms, policy)?;
        let limit = i64::from(policy.max_lease_batch);
        let change_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT id
                     FROM library_change_queue
                     WHERE root_id = ?1 AND root_generation = ?2
                       AND attempt_count < ?3
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

    fn complete_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let outcome = classify_lease_update(
            &transaction,
            change_id,
            lease_generation,
            Some(catalog_revision_at_success),
        )?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
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
        let transaction = self.connection.transaction().map_err(database_error)?;
        let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
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
            transaction
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
        }
        transaction.commit().map_err(database_error)?;
        Ok(outcome)
    }

    fn load_library_change_queue_metrics(
        &self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        validate_policy(policy)?;
        load_metrics(&self.connection, now_unix_ms, policy)
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        terminal_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError> {
        cleanup_terminal_records(&self.connection, terminal_before_unix_ms, limit)
    }
}

#[cfg(test)]
mod tests;

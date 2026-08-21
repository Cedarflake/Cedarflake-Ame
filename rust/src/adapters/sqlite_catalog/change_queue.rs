use rusqlite::{Transaction, TransactionBehavior, params};

#[cfg(test)]
use crate::domain::LibraryChangeCatchUpQueueBatch;
use crate::domain::{
    LeasedLibraryChange, LibraryChangeCatchUpEvidence, LibraryChangeEnqueueReport,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeLeaseUpdateOutcome,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError,
};
use crate::ports::LibraryChangeQueue;

use super::{SqliteCatalog, database_error, load_catalog_revision, sqlite_integer, sqlite_u32};

mod coalescing;
mod persistence;

use coalescing::{
    enqueue_one, validate_failure, validate_intent_batch, validate_policy, validate_root_id,
};
use persistence::{
    GenerationDisposition, classify_lease_update, cleanup_terminal_records,
    enforce_retry_attempt_limit, establish_root_generation, load_change, load_metrics,
    load_root_metrics, next_retry_deadline, recover_expired_leases, root_generation_is_current,
};
pub(super) use persistence::{activate_root_change_queue, retire_root_change_queue};

impl LibraryChangeQueue for SqliteCatalog {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        enqueue_intents(self, intents, None, enqueued_unix_ms, policy)
    }

    #[cfg(test)]
    fn enqueue_library_change_intents_with_catch_up(
        &mut self,
        intents: &[LibraryChangeIntent],
        evidence: &LibraryChangeCatchUpEvidence,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        validate_catch_up_evidence(evidence)?;
        enqueue_intents(self, intents, Some(evidence), enqueued_unix_ms, policy)
    }

    #[cfg(test)]
    fn enqueue_library_change_catch_up_batches(
        &mut self,
        batches: &[LibraryChangeCatchUpQueueBatch],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LibraryChangeEnqueueReport>, ScanError> {
        validate_policy(policy)?;
        for batch in batches {
            if let Some(evidence) = &batch.evidence {
                validate_catch_up_evidence(evidence)?;
            }
            validate_enqueue_batch(&batch.intents)?;
        }
        if batches.is_empty() {
            return Ok(Vec::new());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
        let mut reports = Vec::with_capacity(batches.len());
        for batch in batches {
            reports.push(enqueue_intents_in_transaction(
                &transaction,
                &batch.intents,
                batch.evidence.as_ref(),
                enqueued_unix_ms,
                policy,
            )?);
        }
        transaction.commit().map_err(database_error)?;
        Ok(reports)
    }

    fn lease_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::All,
        )
    }

    fn lease_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LeasedLibraryChange>, ScanError> {
        self.lease_library_changes_matching(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
            LeaseSelection::Path,
        )
    }

    fn lease_authoritative_library_change(
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
            LeaseSelection::Authoritative,
        )?;
        Ok(leased.pop())
    }

    fn complete_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
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
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
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

    fn defer_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferred_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let outcome = classify_lease_update(&transaction, change_id, lease_generation, None)?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'pending', ready_unix_ms = ?1,
                         attempt_count = CASE
                           WHEN attempt_count > 0 THEN attempt_count - 1 ELSE 0 END,
                         next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                         updated_unix_ms = ?1
                     WHERE id = ?2 AND status = 'leased' AND lease_generation = ?3",
                    params![
                        deferred_unix_ms,
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

    fn load_library_change_root_queue_metrics(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        load_root_metrics(
            &self.connection,
            root_id,
            root_generation,
            now_unix_ms,
            policy,
        )
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        terminal_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let deleted = cleanup_terminal_records(&transaction, terminal_before_unix_ms, limit)?;
        transaction.commit().map_err(database_error)?;
        Ok(deleted)
    }
}

fn enqueue_intents(
    catalog: &mut SqliteCatalog,
    intents: &[LibraryChangeIntent],
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeEnqueueReport, ScanError> {
    validate_policy(policy)?;
    validate_enqueue_batch(intents)?;
    if intents.is_empty() {
        return Ok(LibraryChangeEnqueueReport::default());
    }
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
    let report =
        enqueue_intents_in_transaction(&transaction, intents, evidence, enqueued_unix_ms, policy)?;
    transaction.commit().map_err(database_error)?;
    Ok(report)
}

pub(super) fn validate_enqueue_batch(intents: &[LibraryChangeIntent]) -> Result<(), ScanError> {
    let Some(first) = intents.first() else {
        return Ok(());
    };
    validate_intent_batch(intents, first)
}

fn cleanup_for_enqueue(
    transaction: &Transaction<'_>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    let retention_millis = i64::try_from(policy.terminal_retention_millis).unwrap_or(i64::MAX);
    cleanup_terminal_records(
        transaction,
        enqueued_unix_ms.saturating_sub(retention_millis),
        policy.cleanup_batch,
    )?;
    Ok(())
}

pub(super) fn enqueue_intents_in_transaction(
    transaction: &Transaction<'_>,
    intents: &[LibraryChangeIntent],
    evidence: Option<&LibraryChangeCatchUpEvidence>,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeEnqueueReport, ScanError> {
    let first = intents.first().ok_or_else(|| {
        ScanError::new(
            "change_queue_batch_empty",
            "A transactional change queue batch must contain at least one intent",
        )
    })?;
    let mut report = LibraryChangeEnqueueReport::default();
    match establish_root_generation(
        transaction,
        &first.root_id,
        first.root_generation,
        enqueued_unix_ms,
    )? {
        GenerationDisposition::Current { superseded_count } => {
            report.superseded_count = superseded_count;
        }
        GenerationDisposition::Stale => {
            report.stale_generation_count = u32::try_from(intents.len()).unwrap_or(u32::MAX);
            return Ok(report);
        }
    }
    let catalog_revision = load_catalog_revision(transaction)?;
    for intent in intents {
        enqueue_one(
            transaction,
            intent,
            enqueued_unix_ms,
            catalog_revision,
            policy,
            evidence,
            &mut report,
        )?;
    }
    Ok(report)
}

#[cfg(test)]
fn validate_catch_up_evidence(evidence: &LibraryChangeCatchUpEvidence) -> Result<(), ScanError> {
    if evidence.source.trim().is_empty()
        || evidence.source.len() > 128
        || evidence.watermark.trim().is_empty()
        || evidence.watermark.len() > 1_024
    {
        Err(ScanError::new(
            "library_change_catch_up_evidence_invalid",
            "Catch-up queue evidence exceeds its bounded storage contract",
        ))
    } else {
        Ok(())
    }
}

impl SqliteCatalog {
    pub(crate) fn has_ready_authoritative_library_change(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<bool, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        self.connection
            .query_row(
                "SELECT EXISTS(
               SELECT 1 FROM library_change_queue
                   WHERE root_id = ?1 AND root_generation = ?2
                     AND (scope <> 'path' OR intent_kind = 'freshness_unknown')
                     AND (
                       (attempt_count < ?3 AND status = 'pending' AND ready_unix_ms <= ?4)
                       OR (attempt_count < ?3 AND status = 'retry_wait'
                         AND next_retry_unix_ms IS NOT NULL AND next_retry_unix_ms <= ?4)
                       OR (status = 'leased' AND lease_expires_unix_ms IS NOT NULL
                         AND lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                    i64::from(policy.max_attempts),
                    now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    fn lease_library_changes_matching(
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
        let pass_is_needed = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_root_state AS state
                   WHERE state.root_id = ?1 AND state.generation = ?2 AND state.is_active = 1
                 ) AND EXISTS(
                   SELECT 1 FROM library_change_queue AS queue
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND (
                       (queue.status = 'retry_wait' AND queue.attempt_count >= ?3
                         AND queue.next_retry_unix_ms IS NOT NULL)
                       OR (
                         (
                           ?5 = 0
                           OR (?5 = 1 AND queue.scope = 'path'
                             AND queue.intent_kind <> 'freshness_unknown')
                           OR (?5 = 2 AND (queue.scope <> 'path'
                             OR queue.intent_kind = 'freshness_unknown'))
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
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !pass_is_needed {
            return Ok(Vec::new());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
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
        let limit = match selection {
            LeaseSelection::Authoritative => 1,
            LeaseSelection::All | LeaseSelection::Path => i64::from(policy.max_lease_batch),
        };
        let change_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT id
                 FROM library_change_queue
                 WHERE root_id = ?1 AND root_generation = ?2
                   AND attempt_count < ?3
                   AND (
                     ?6 = 0
                     OR (?6 = 1 AND scope = 'path' AND intent_kind <> 'freshness_unknown')
                     OR (?6 = 2 AND (scope <> 'path' OR intent_kind = 'freshness_unknown'))
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
enum LeaseSelection {
    All,
    Path,
    Authoritative,
}

impl LeaseSelection {
    const fn sql_value(self) -> i64 {
        match self {
            Self::All => 0,
            Self::Path => 1,
            Self::Authoritative => 2,
        }
    }
}

#[cfg(test)]
mod tests;

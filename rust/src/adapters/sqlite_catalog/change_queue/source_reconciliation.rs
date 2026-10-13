use rusqlite::params;

use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibraryChangeScope, PreviewRequest, ScanError,
};
use crate::ports::{
    SourceReconciliationAdmission, SourceReconciliationRepository, SourceReconciliationScope,
};

use super::super::{
    SqliteCatalog, database_error, load_catalog_revision, source_revision_token, sqlite_integer,
};
use super::coalescing::{
    active_lane_counts, lane_admission_allows, queue_backpressure, validate_intent_batch,
    validate_policy,
};
use super::persistence::{
    insert_change, load_active_changes, load_change, root_generation_is_current,
};

impl SourceReconciliationRepository for SqliteCatalog {
    fn admit_source_reconciliation(
        &mut self,
        request: &PreviewRequest,
        intent: &LibraryChangeIntent,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<SourceReconciliationAdmission, ScanError> {
        validate_policy(policy)?;
        validate_intent_batch(std::slice::from_ref(intent), intent)?;
        let scope = SourceReconciliationScope::from_intent(intent).ok_or_else(|| {
            ScanError::new(
                "source_reconciliation_scope_invalid",
                "Source reconciliation requires a guarded existing catalog path",
            )
        })?;
        if intent.root_id != request.expected_root_id {
            return Err(ScanError::new(
                "source_reconciliation_scope_invalid",
                "Source reconciliation requires one existing catalog path",
            ));
        }
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let current = transaction
            .query_row(
                "SELECT EXISTS(
               SELECT 1 FROM asset_locations AS location
               JOIN library_roots AS root ON root.id = location.root_id
                 AND root.active_scan_id = location.scan_id
               WHERE location.location_id = ?1 AND location.root_id = ?2
                 AND location.scan_id = ?3 AND location.source_generation = ?4
                 AND location.source_revision_token IS ?5 AND location.relative_path = ?6
             )",
                params![
                    request.location_id,
                    request.expected_root_id,
                    request.expected_scan_id,
                    sqlite_integer(request.expected_source_generation, "source generation")?,
                    request
                        .expected_source_revision
                        .as_ref()
                        .map(source_revision_token)
                        .transpose()?,
                    scope.previous_path()
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !current
            || !root_generation_is_current(&transaction, &intent.root_id, intent.root_generation)?
        {
            return Ok(SourceReconciliationAdmission::RequestSuperseded);
        }
        let active = load_active_changes(
            &transaction,
            &intent.root_id,
            intent.root_generation,
            policy.max_unresolved_changes,
        )?;
        if active.iter().any(|change| {
            change.intent.scope == LibraryChangeScope::Path
                && (change.intent.relative_path == intent.relative_path
                    || change.intent.relative_path == scope.previous_path()
                    || change.intent.previous_relative_path.as_deref()
                        == Some(&intent.relative_path)
                    || change.intent.previous_relative_path.as_deref()
                        == Some(scope.previous_path()))
        }) {
            return Ok(SourceReconciliationAdmission::ExistingPathWork);
        }
        let counts = active_lane_counts(&active).adding(LibraryChangeLane::Recovery, 1);
        if !lane_admission_allows(policy, LibraryChangeLane::Recovery, counts)
            || active.len() >= LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES as usize
        {
            return Err(queue_backpressure());
        }
        let id = insert_change(
            &transaction,
            intent,
            now_unix_ms,
            load_catalog_revision(&transaction)?,
            None,
            policy,
        )?;
        let id = sqlite_integer(id.value(), "change ID")?;
        let expires = now_unix_ms
            .saturating_add(i64::try_from(policy.lease_duration_millis).unwrap_or(i64::MAX));
        let claimed = transaction.execute(
            "UPDATE library_change_queue
             SET status = 'leased', attempt_count = attempt_count + 1,
                 next_retry_unix_ms = NULL, lease_generation = lease_generation + 1,
                 lease_expires_unix_ms = ?1, updated_unix_ms = ?2
             WHERE id = ?3 AND root_id = ?4 AND relative_path = ?5
               AND scope = 'path' AND intent_kind IN ('reconcile', 'rename_candidate')
               AND origin = 'consistency_audit' AND status = 'pending'
               AND ready_unix_ms <= ?2 AND attempt_count < ?6
               AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_candidate_owners WHERE change_id = ?3)
               AND NOT EXISTS(SELECT 1 FROM library_recovery_authorities WHERE change_id = ?3)",
            params![expires, now_unix_ms, id, intent.root_id, intent.relative_path, i64::from(policy.max_attempts)],
        ).map_err(database_error)?;
        if claimed != 1 {
            return Ok(SourceReconciliationAdmission::LeaseUnavailable);
        }
        let change = load_change(&transaction, id)?;
        let leased = LeasedLibraryChange {
            lease_generation: change.lease_generation,
            lease_expires_unix_ms: expires,
            change,
        };
        transaction.commit().map_err(database_error)?;
        Ok(SourceReconciliationAdmission::Leased(Box::new(leased)))
    }
}

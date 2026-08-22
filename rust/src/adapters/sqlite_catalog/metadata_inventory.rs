use std::collections::BTreeSet;
use std::path::{Component, Path};

use rusqlite::{
    ErrorCode, OptionalExtension, Row, Transaction, TransactionBehavior, params, params_from_iter,
};

use crate::domain::{
    FileIdentityEvidence, LibraryRootGeneration, MetadataInventoryCleanupReport,
    MetadataInventoryComparisonStatus, MetadataInventoryComparisonUpdate, MetadataInventoryEntry,
    MetadataInventoryEntryKind, MetadataInventoryPage, MetadataInventoryPlaceholderState,
    MetadataInventoryRun, MetadataInventoryRunRequest, MetadataInventoryRunStatus,
    MetadataInventoryScope, MetadataInventoryStartRequest, ScanError,
};
use crate::ports::MetadataInventoryRepository;

use super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

const MAX_PAGE_ENTRIES: u32 = 4_096;
const MAX_CLEANUP_RUNS: u32 = 128;
type StoredEntryParts<'a> = (
    &'static str,
    Option<i64>,
    Option<&'a str>,
    Option<&'a str>,
    &'static str,
);

impl MetadataInventoryRepository for SqliteCatalog {
    fn begin_next_metadata_inventory(
        &mut self,
        request: &MetadataInventoryStartRequest,
    ) -> Result<MetadataInventoryRun, ScanError> {
        validate_start_request(request)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(existing) = load_run(&transaction, &request.run_id)? {
            let is_active = matches!(
                existing.status,
                MetadataInventoryRunStatus::Running | MetadataInventoryRunStatus::Comparing
            );
            if is_active
                && existing.request.root_id == request.root_id
                && existing.request.root_generation == request.root_generation
                && existing.request.scope == request.scope
            {
                validate_active_root(&transaction, &existing.request)?;
                transaction.commit().map_err(database_error)?;
                return Ok(existing);
            }
            return Err(ScanError::new(
                "metadata_inventory_run_duplicate",
                "The metadata inventory run identity already exists",
            ));
        }
        let latest_epoch = transaction
            .query_row(
                "SELECT MAX(epoch)
                 FROM library_metadata_inventory_runs
                 WHERE root_id = ?1 AND root_generation = ?2",
                params![
                    request.root_id,
                    sqlite_integer(
                        request.root_generation.value(),
                        "metadata inventory root generation",
                    )?,
                ],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(database_error)?
            .map(|epoch| sqlite_unsigned(epoch, "metadata inventory epoch"))
            .transpose()?
            .unwrap_or(0);
        let epoch = latest_epoch.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_epoch_overflow",
                "The metadata inventory epoch exceeded the supported range",
            )
        })?;
        let run_request = MetadataInventoryRunRequest {
            run_id: request.run_id.clone(),
            root_id: request.root_id.clone(),
            root_generation: request.root_generation,
            epoch,
            scope: request.scope.clone(),
            started_unix_ms: request.started_unix_ms,
        };
        let run = begin_metadata_inventory_transaction(&transaction, &run_request)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn begin_metadata_inventory(
        &mut self,
        request: &MetadataInventoryRunRequest,
    ) -> Result<MetadataInventoryRun, ScanError> {
        validate_run_request(request)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = begin_metadata_inventory_transaction(&transaction, request)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn stage_metadata_inventory_page(
        &mut self,
        run_id: &str,
        page: &MetadataInventoryPage,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        validate_page(page)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Running {
            return Err(ScanError::new(
                "metadata_inventory_run_not_running",
                "Metadata inventory pages require a running inventory",
            ));
        }
        if run.next_page_index != page.page_index {
            return Err(ScanError::new(
                "metadata_inventory_page_sequence_mismatch",
                "The metadata inventory page does not match the durable cursor",
            ));
        }
        validate_active_root(&transaction, &run.request)?;
        let mut paths = BTreeSet::new();
        for entry in &page.entries {
            validate_entry(&run.request.scope, entry)?;
            if !paths.insert(entry.relative_path.as_str()) {
                return Err(ScanError::new(
                    "metadata_inventory_page_duplicate",
                    "A metadata inventory page contains a duplicate relative path",
                ));
            }
            let (entry_kind, file_size, identity_scheme, identity_value, placeholder_state) =
                entry_parts(entry)?;
            transaction
                .execute(
                    "INSERT INTO library_metadata_inventory_entries(
                       run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                       file_identity_scheme, file_identity_value, placeholder_state,
                       is_reparse_point, staged_page_index, staged_unix_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        run_id,
                        entry.relative_path,
                        entry_kind,
                        file_size,
                        entry.modified_unix_ms,
                        identity_scheme,
                        identity_value,
                        placeholder_state,
                        i64::from(entry.is_reparse_point),
                        sqlite_integer(page.page_index, "metadata inventory page index")?,
                        updated_unix_ms,
                    ],
                )
                .map_err(|error| {
                    if matches!(
                        error,
                        rusqlite::Error::SqliteFailure(ref failure, _)
                            if failure.code == ErrorCode::ConstraintViolation
                    ) {
                        ScanError::new(
                            "metadata_inventory_entry_conflict",
                            "The metadata inventory observed the same path more than once",
                        )
                    } else {
                        database_error(error)
                    }
                })?;
        }
        let next_page_index = page.page_index.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_page_overflow",
                "The metadata inventory page counter overflowed",
            )
        })?;
        let added_count = u64::try_from(page.entries.len()).map_err(|_| {
            ScanError::new(
                "metadata_inventory_entry_count_overflow",
                "The metadata inventory page count exceeded the supported range",
            )
        })?;
        let staged_entry_count =
            run.staged_entry_count
                .checked_add(added_count)
                .ok_or_else(|| {
                    ScanError::new(
                        "metadata_inventory_entry_count_overflow",
                        "The metadata inventory entry count overflowed",
                    )
                })?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = CASE WHEN ?2 THEN 'comparing' ELSE 'running' END,
                     next_page_index = ?3,
                     enumeration_cursor = ?4,
                     staged_entry_count = ?5,
                     enumeration_complete = CASE WHEN ?2 THEN 1 ELSE 0 END,
                     updated_unix_ms = ?6
                 WHERE id = ?1 AND status = 'running' AND next_page_index = ?7",
                params![
                    run_id,
                    page.is_complete,
                    sqlite_integer(next_page_index, "metadata inventory next page index")?,
                    page.cursor,
                    sqlite_integer(staged_entry_count, "metadata inventory staged entry count",)?,
                    updated_unix_ms,
                    sqlite_integer(page.page_index, "metadata inventory page index")?,
                ],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn authorize_metadata_inventory_absence(
        &mut self,
        run_id: &str,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing || !run.enumeration_complete {
            return Err(ScanError::new(
                "metadata_inventory_enumeration_incomplete",
                "Absence cannot become authoritative before enumeration completes",
            ));
        }
        validate_active_root(&transaction, &run.request)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET absence_authority = 1, updated_unix_ms = ?2
                 WHERE id = ?1 AND status = 'comparing' AND enumeration_complete = 1",
                params![run_id, updated_unix_ms],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn load_pending_metadata_inventory_entries(
        &self,
        run_id: &str,
        limit: u32,
    ) -> Result<Vec<MetadataInventoryEntry>, ScanError> {
        validate_window_limit(limit)?;
        let run = require_run(&self.connection, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing || !run.enumeration_complete {
            return Err(ScanError::new(
                "metadata_inventory_not_comparing",
                "Metadata inventory comparison requires complete enumeration",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT relative_path, entry_kind, file_size, modified_unix_ms,
                        file_identity_scheme, file_identity_value, placeholder_state,
                        is_reparse_point
                 FROM library_metadata_inventory_entries
                 WHERE run_id = ?1 AND comparison_status = 'pending'
                 ORDER BY relative_path
                 LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![run_id, i64::from(limit)], stored_entry)
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
            .into_iter()
            .map(StoredEntry::into_domain)
            .collect()
    }

    fn load_metadata_inventory_previous_path(
        &self,
        run_id: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<String>, ScanError> {
        if identity.scheme.is_empty() || identity.value.is_empty() {
            return Err(ScanError::new(
                "metadata_inventory_identity_invalid",
                "Metadata inventory file identity evidence must be complete",
            ));
        }
        self.connection
            .query_row(
                "SELECT locations.relative_path
                 FROM library_metadata_inventory_runs AS runs
                 JOIN library_roots AS roots ON roots.id = runs.root_id
                 JOIN asset_locations AS locations
                   ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id
                 WHERE runs.id = ?1
                   AND locations.file_identity_scheme = ?2
                   AND locations.file_identity_value = ?3
                   AND NOT EXISTS(
                     SELECT 1 FROM library_metadata_inventory_entries AS entries
                     WHERE entries.run_id = runs.id
                       AND entries.relative_path = locations.relative_path
                       AND entries.entry_kind = 'file'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_metadata_inventory_entries AS entries
                     WHERE entries.run_id = runs.id
                       AND entries.candidate_previous_relative_path = locations.relative_path
                   )
                 ORDER BY locations.relative_path
                 LIMIT 1",
                params![run_id, identity.scheme, identity.value],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)
    }

    fn load_metadata_inventory_previous_paths(
        &self,
        run_id: &str,
        identities: &[FileIdentityEvidence],
    ) -> Result<Vec<(FileIdentityEvidence, String)>, ScanError> {
        if identities.is_empty() {
            return Ok(Vec::new());
        }
        if identities.len() > MAX_PAGE_ENTRIES as usize
            || identities
                .iter()
                .any(|identity| identity.scheme.is_empty() || identity.value.is_empty())
        {
            return Err(ScanError::new(
                "metadata_inventory_identity_window_invalid",
                "Metadata inventory identity windows must contain at most 4096 complete identities",
            ));
        }
        let requested_values = std::iter::repeat_n("(?, ?)", identities.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "WITH requested(file_identity_scheme, file_identity_value) AS (
               VALUES {requested_values}
             )
             SELECT locations.file_identity_scheme, locations.file_identity_value,
                    locations.relative_path
             FROM requested
             JOIN library_metadata_inventory_runs AS runs ON runs.id = ?
             JOIN library_roots AS roots ON roots.id = runs.root_id
             JOIN asset_locations AS locations
               ON locations.root_id = roots.id
              AND locations.scan_id = roots.active_scan_id
              AND locations.file_identity_scheme = requested.file_identity_scheme
              AND locations.file_identity_value = requested.file_identity_value
             WHERE NOT EXISTS(
               SELECT 1 FROM library_metadata_inventory_entries AS entries
               WHERE entries.run_id = runs.id
                 AND entries.relative_path = locations.relative_path
                 AND entries.entry_kind = 'file'
             )
               AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_entries AS entries
                 WHERE entries.run_id = runs.id
                   AND entries.candidate_previous_relative_path = locations.relative_path
               )
             ORDER BY locations.file_identity_scheme, locations.file_identity_value,
                      locations.relative_path"
        );
        let parameters = identities
            .iter()
            .flat_map(|identity| [identity.scheme.as_str(), identity.value.as_str()])
            .chain(std::iter::once(run_id));
        let mut statement = self.connection.prepare(&query).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(parameters), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(database_error)?;
        let mut claimed_identities = BTreeSet::new();
        let mut previous_paths = Vec::with_capacity(identities.len());
        for row in rows {
            let (scheme, value, relative_path) = row.map_err(database_error)?;
            if claimed_identities.insert((scheme.clone(), value.clone())) {
                previous_paths.push((FileIdentityEvidence { scheme, value }, relative_path));
            }
        }
        Ok(previous_paths)
    }

    fn record_metadata_inventory_comparisons(
        &mut self,
        run_id: &str,
        updates: &[MetadataInventoryComparisonUpdate],
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        if updates.is_empty() || updates.len() > MAX_PAGE_ENTRIES as usize {
            return Err(ScanError::new(
                "metadata_inventory_comparison_batch_invalid",
                "Metadata inventory comparison batches must stay within one page",
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing {
            return Err(ScanError::new(
                "metadata_inventory_not_comparing",
                "Metadata inventory entries can only be compared in the comparing state",
            ));
        }
        validate_active_root(&transaction, &run.request)?;
        let mut candidate_count = 0_u64;
        let mut last_relative_path = None;
        for update in updates {
            validate_relative_path(&update.relative_path, false)?;
            if let Some(previous) = update.candidate_previous_relative_path.as_deref() {
                validate_relative_path(previous, false)?;
            }
            if update.status == MetadataInventoryComparisonStatus::Unchanged
                && update.candidate_previous_relative_path.is_some()
            {
                return Err(ScanError::new(
                    "metadata_inventory_comparison_invalid",
                    "Only an enqueued rename candidate may retain a previous path",
                ));
            }
            let status = match update.status {
                MetadataInventoryComparisonStatus::Unchanged => "unchanged",
                MetadataInventoryComparisonStatus::Enqueued => {
                    candidate_count = candidate_count.checked_add(1).ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_candidate_count_overflow",
                            "The metadata inventory candidate count overflowed",
                        )
                    })?;
                    "enqueued"
                }
            };
            let updated = transaction
                .execute(
                    "UPDATE library_metadata_inventory_entries
                     SET comparison_status = ?3, candidate_previous_relative_path = ?4
                     WHERE run_id = ?1 AND relative_path = ?2
                       AND comparison_status = 'pending'",
                    params![
                        run_id,
                        update.relative_path,
                        status,
                        update.candidate_previous_relative_path,
                    ],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "metadata_inventory_comparison_conflict",
                    "The metadata inventory entry was already compared or is missing",
                ));
            }
            last_relative_path = Some(update.relative_path.as_str());
        }
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET comparison_cursor = ?2,
                     candidate_count = candidate_count + ?3,
                     updated_unix_ms = ?4
                 WHERE id = ?1 AND status = 'comparing'",
                params![
                    run_id,
                    last_relative_path,
                    sqlite_integer(candidate_count, "metadata inventory candidate count")?,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn load_metadata_inventory_absence_candidates(
        &self,
        run_id: &str,
        after_relative_path: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError> {
        validate_window_limit(limit)?;
        if let Some(after) = after_relative_path {
            validate_relative_path(after, false)?;
        }
        let run = require_run(&self.connection, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing || !run.absence_authority {
            return Err(ScanError::new(
                "metadata_inventory_absence_not_authoritative",
                "Absence candidates require complete inventory authority",
            ));
        }
        load_absence_candidates(&self.connection, &run, after_relative_path, limit)
    }

    fn advance_metadata_inventory_absence_cursor(
        &mut self,
        run_id: &str,
        expected_cursor: Option<&str>,
        next_cursor: &str,
        candidate_count: u64,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        validate_relative_path(next_cursor, false)?;
        if let Some(expected) = expected_cursor {
            validate_relative_path(expected, false)?;
            if next_cursor <= expected {
                return Err(ScanError::new(
                    "metadata_inventory_absence_cursor_invalid",
                    "The absence cursor must advance monotonically",
                ));
            }
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing
            || !run.absence_authority
            || run.absence_cursor.as_deref() != expected_cursor
        {
            return Err(ScanError::new(
                "metadata_inventory_absence_cursor_conflict",
                "The metadata inventory absence cursor changed before publication",
            ));
        }
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET absence_cursor = ?2,
                     candidate_count = candidate_count + ?3,
                     updated_unix_ms = ?4
                 WHERE id = ?1 AND status = 'comparing'",
                params![
                    run_id,
                    next_cursor,
                    sqlite_integer(candidate_count, "metadata inventory candidate count")?,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn complete_metadata_inventory(
        &mut self,
        run_id: &str,
        completed_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing || !run.absence_authority {
            return Err(ScanError::new(
                "metadata_inventory_completion_not_authoritative",
                "Metadata inventory completion requires complete absence authority",
            ));
        }
        validate_active_root(&transaction, &run.request)?;
        let has_pending = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_entries
                   WHERE run_id = ?1 AND comparison_status = 'pending'
                 )",
                [run_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        let remaining_absence =
            load_absence_candidates(&transaction, &run, run.absence_cursor.as_deref(), 1)?;
        if has_pending || !remaining_absence.is_empty() {
            return Err(ScanError::new(
                "metadata_inventory_comparison_incomplete",
                "Metadata inventory completion requires every staged and absent path to be compared",
            ));
        }
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE id = ?1 AND status = 'comparing' AND absence_authority = 1",
                params![run_id, completed_unix_ms],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn terminate_metadata_inventory(
        &mut self,
        run_id: &str,
        status: MetadataInventoryRunStatus,
        issue: Option<(&str, &str)>,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        let status = match status {
            MetadataInventoryRunStatus::Failed => "failed",
            MetadataInventoryRunStatus::Cancelled => "cancelled",
            MetadataInventoryRunStatus::Superseded => "superseded",
            _ => {
                return Err(ScanError::new(
                    "metadata_inventory_terminal_status_invalid",
                    "Metadata inventory termination requires a terminal failure state",
                ));
            }
        };
        if issue.is_some_and(|(code, message)| {
            code.is_empty()
                || code.len() > 128
                || message.is_empty()
                || message.len() > 4_096
                || code.contains('\0')
                || message.contains('\0')
        }) {
            return Err(ScanError::new(
                "metadata_inventory_issue_invalid",
                "Metadata inventory issues must contain bounded text",
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        if !matches!(
            run.status,
            MetadataInventoryRunStatus::Running | MetadataInventoryRunStatus::Comparing
        ) {
            return Err(ScanError::new(
                "metadata_inventory_run_terminal",
                "The metadata inventory run is already terminal",
            ));
        }
        let (issue_code, issue_message) = issue.unzip();
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = ?2, last_issue_code = ?3, last_issue_message = ?4,
                     updated_unix_ms = ?5
                 WHERE id = ?1 AND status IN ('running', 'comparing')",
                params![run_id, status, issue_code, issue_message, updated_unix_ms],
            )
            .map_err(database_error)?;
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(run)
    }

    fn load_metadata_inventory_run(
        &self,
        run_id: &str,
    ) -> Result<Option<MetadataInventoryRun>, ScanError> {
        load_run(&self.connection, run_id)
    }

    fn cleanup_terminal_metadata_inventories(
        &mut self,
        terminal_before_unix_ms: i64,
        entry_limit: u32,
        run_limit: u32,
    ) -> Result<MetadataInventoryCleanupReport, ScanError> {
        if entry_limit == 0
            || entry_limit > MAX_PAGE_ENTRIES
            || run_limit == 0
            || run_limit > MAX_CLEANUP_RUNS
        {
            return Err(ScanError::new(
                "metadata_inventory_cleanup_limit_invalid",
                "Metadata inventory cleanup limits exceed the bounded contract",
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let removed_entry_count = transaction
            .execute(
                "DELETE FROM library_metadata_inventory_entries
                 WHERE rowid IN (
                   SELECT entries.rowid
                   FROM library_metadata_inventory_entries AS entries
                   JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
                   WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                   ORDER BY runs.updated_unix_ms, runs.id, entries.relative_path
                   LIMIT ?1
                 )",
                [i64::from(entry_limit)],
            )
            .map_err(database_error)?;
        let removed_run_count = transaction
            .execute(
                "DELETE FROM library_metadata_inventory_runs
                 WHERE id IN (
                   SELECT runs.id
                   FROM library_metadata_inventory_runs AS runs
                   WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                     AND runs.updated_unix_ms < ?1
                     AND NOT EXISTS(
                       SELECT 1 FROM library_metadata_inventory_entries AS entries
                       WHERE entries.run_id = runs.id
                     )
                   ORDER BY runs.updated_unix_ms, runs.id
                   LIMIT ?2
                 )",
                params![terminal_before_unix_ms, i64::from(run_limit)],
            )
            .map_err(database_error)?;
        let has_more = transaction
            .query_row(
                "SELECT
                   EXISTS(
                     SELECT 1
                     FROM library_metadata_inventory_entries AS entries
                     JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
                     WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                   )
                   OR EXISTS(
                     SELECT 1
                     FROM library_metadata_inventory_runs AS runs
                     WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                       AND runs.updated_unix_ms < ?1
                       AND NOT EXISTS(
                         SELECT 1 FROM library_metadata_inventory_entries AS entries
                         WHERE entries.run_id = runs.id
                       )
                   )",
                [terminal_before_unix_ms],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(MetadataInventoryCleanupReport {
            removed_entry_count: u32::try_from(removed_entry_count).map_err(|_| {
                ScanError::new(
                    "metadata_inventory_cleanup_count_overflow",
                    "Metadata inventory entry cleanup count overflowed",
                )
            })?,
            removed_run_count: u32::try_from(removed_run_count).map_err(|_| {
                ScanError::new(
                    "metadata_inventory_cleanup_count_overflow",
                    "Metadata inventory run cleanup count overflowed",
                )
            })?,
            has_more,
        })
    }
}

fn begin_metadata_inventory_transaction(
    transaction: &Transaction<'_>,
    request: &MetadataInventoryRunRequest,
) -> Result<MetadataInventoryRun, ScanError> {
    validate_active_root(transaction, request)?;
    let existing_run = transaction
        .query_row(
            "SELECT id FROM library_metadata_inventory_runs WHERE id = ?1",
            [&request.run_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if existing_run.is_some() {
        return Err(ScanError::new(
            "metadata_inventory_run_exists",
            "The metadata inventory run already exists",
        ));
    }
    let active_run = transaction
        .query_row(
            "SELECT id, root_generation, epoch
             FROM library_metadata_inventory_runs
             WHERE root_id = ?1 AND status IN ('running', 'comparing')",
            [&request.root_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    if let Some((active_id, active_generation, active_epoch)) = active_run {
        let active_generation = sqlite_unsigned(
            active_generation,
            "active metadata inventory root generation",
        )?;
        let active_epoch = sqlite_unsigned(active_epoch, "active metadata inventory epoch")?;
        let request_generation = request.root_generation.value();
        if active_generation > request_generation
            || active_generation == request_generation && active_epoch >= request.epoch
        {
            return Err(ScanError::new(
                "metadata_inventory_active_conflict",
                "A current or newer metadata inventory already owns this root",
            ));
        }
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'superseded',
                     last_issue_code = 'metadata_inventory_newer_epoch',
                     last_issue_message = 'A newer metadata inventory superseded this run',
                     updated_unix_ms = ?2
                 WHERE id = ?1 AND status IN ('running', 'comparing')",
                params![active_id, request.started_unix_ms],
            )
            .map_err(database_error)?;
    }
    let (scope_kind, scope_relative_path) = scope_parts(&request.scope);
    transaction
        .execute(
            "INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, started_unix_ms, updated_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', 1, ?7, ?7)",
            params![
                request.run_id,
                request.root_id,
                sqlite_integer(
                    request.root_generation.value(),
                    "metadata inventory root generation",
                )?,
                sqlite_integer(request.epoch, "metadata inventory epoch")?,
                scope_kind,
                scope_relative_path,
                request.started_unix_ms,
            ],
        )
        .map_err(database_error)?;
    load_run(transaction, &request.run_id)?.ok_or_else(|| {
        ScanError::new(
            "metadata_inventory_run_missing",
            "The metadata inventory run was not persisted",
        )
    })
}

fn validate_active_root(
    transaction: &Transaction<'_>,
    request: &MetadataInventoryRunRequest,
) -> Result<(), ScanError> {
    let root = transaction
        .query_row(
            "SELECT state.generation, state.is_active,
                    EXISTS(
                      SELECT 1 FROM scan_runs AS scans
                      WHERE scans.id = roots.active_scan_id AND scans.status = 'completed'
                    ),
                    EXISTS(
                      SELECT 1 FROM scan_runs AS scans
                      WHERE scans.root_id = roots.id AND scans.status IN ('running', 'paused')
                    )
             FROM library_roots AS roots
             JOIN library_change_root_state AS state ON state.root_id = roots.id
             WHERE roots.id = ?1",
            [&request.root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((generation, is_active, has_published_scan, has_running_scan)) = root else {
        return Err(ScanError::new(
            "metadata_inventory_root_missing",
            "The metadata inventory root is no longer registered",
        ));
    };
    if !is_active
        || !has_published_scan
        || has_running_scan
        || sqlite_unsigned(generation, "metadata inventory root generation")?
            != request.root_generation.value()
    {
        return Err(ScanError::new(
            "metadata_inventory_root_stale",
            "The metadata inventory root generation or publication boundary changed",
        ));
    }
    Ok(())
}

fn load_absence_candidates(
    connection: &rusqlite::Connection,
    run: &MetadataInventoryRun,
    after_relative_path: Option<&str>,
    limit: u32,
) -> Result<Vec<String>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT locations.relative_path
             FROM library_metadata_inventory_runs AS runs
             JOIN library_roots AS roots ON roots.id = runs.root_id
             JOIN asset_locations AS locations
               ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id
             WHERE runs.id = ?1
               AND (?2 IS NULL OR locations.relative_path > ?2)
               AND (
                 runs.scope_kind = 'root'
                 OR locations.relative_path = runs.scope_relative_path
                 OR substr(locations.relative_path, 1, length(runs.scope_relative_path) + 1)
                      = runs.scope_relative_path || '/'
               )
               AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_entries AS entries
                 WHERE entries.run_id = runs.id
                   AND entries.relative_path = locations.relative_path
                   AND entries.entry_kind = 'file'
               )
               AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_entries AS entries
                 WHERE entries.run_id = runs.id
                   AND entries.candidate_previous_relative_path = locations.relative_path
               )
             ORDER BY locations.relative_path
             LIMIT ?3",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![run.request.run_id, after_relative_path, i64::from(limit)],
            |row| row.get::<_, String>(0),
        )
        .map_err(database_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
}

fn load_run(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<Option<MetadataInventoryRun>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                    status, next_page_index, enumeration_cursor, comparison_cursor,
                    absence_cursor, staged_entry_count, candidate_count,
                    enumeration_complete, absence_authority, started_unix_ms,
                    updated_unix_ms, completed_unix_ms, last_issue_code, last_issue_message
             FROM library_metadata_inventory_runs WHERE id = ?1",
            [run_id],
            stored_run,
        )
        .optional()
        .map_err(database_error)?;
    stored.map(StoredRun::into_domain).transpose()
}

fn require_run(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<MetadataInventoryRun, ScanError> {
    load_run(connection, run_id)?.ok_or_else(|| {
        ScanError::new(
            "metadata_inventory_run_missing",
            "The metadata inventory run does not exist",
        )
    })
}

struct StoredRun {
    id: String,
    root_id: String,
    root_generation: i64,
    epoch: i64,
    scope_kind: String,
    scope_relative_path: String,
    status: String,
    next_page_index: i64,
    enumeration_cursor: Option<String>,
    comparison_cursor: Option<String>,
    absence_cursor: Option<String>,
    staged_entry_count: i64,
    candidate_count: i64,
    enumeration_complete: bool,
    absence_authority: bool,
    started_unix_ms: i64,
    updated_unix_ms: i64,
    completed_unix_ms: Option<i64>,
    last_issue_code: Option<String>,
    last_issue_message: Option<String>,
}

impl StoredRun {
    fn into_domain(self) -> Result<MetadataInventoryRun, ScanError> {
        let root_generation = LibraryRootGeneration::new(sqlite_unsigned(
            self.root_generation,
            "metadata inventory root generation",
        )?)
        .ok_or_else(|| {
            ScanError::new(
                "catalog_metadata_inventory_generation_invalid",
                "The stored metadata inventory root generation is invalid",
            )
        })?;
        let scope = match self.scope_kind.as_str() {
            "root" if self.scope_relative_path.is_empty() => MetadataInventoryScope::Root,
            "subtree" if !self.scope_relative_path.is_empty() => MetadataInventoryScope::Subtree {
                relative_path: self.scope_relative_path,
            },
            _ => {
                return Err(ScanError::new(
                    "catalog_metadata_inventory_scope_invalid",
                    "The stored metadata inventory scope is invalid",
                ));
            }
        };
        Ok(MetadataInventoryRun {
            request: MetadataInventoryRunRequest {
                run_id: self.id,
                root_id: self.root_id,
                root_generation,
                epoch: sqlite_unsigned(self.epoch, "metadata inventory epoch")?,
                scope,
                started_unix_ms: self.started_unix_ms,
            },
            status: parse_run_status(&self.status)?,
            next_page_index: sqlite_unsigned(
                self.next_page_index,
                "metadata inventory next page index",
            )?,
            enumeration_cursor: self.enumeration_cursor,
            comparison_cursor: self.comparison_cursor,
            absence_cursor: self.absence_cursor,
            staged_entry_count: sqlite_unsigned(
                self.staged_entry_count,
                "metadata inventory staged entry count",
            )?,
            candidate_count: sqlite_unsigned(
                self.candidate_count,
                "metadata inventory candidate count",
            )?,
            enumeration_complete: self.enumeration_complete,
            absence_authority: self.absence_authority,
            updated_unix_ms: self.updated_unix_ms,
            completed_unix_ms: self.completed_unix_ms,
            last_issue_code: self.last_issue_code,
            last_issue_message: self.last_issue_message,
        })
    }
}

fn stored_run(row: &Row<'_>) -> rusqlite::Result<StoredRun> {
    Ok(StoredRun {
        id: row.get(0)?,
        root_id: row.get(1)?,
        root_generation: row.get(2)?,
        epoch: row.get(3)?,
        scope_kind: row.get(4)?,
        scope_relative_path: row.get(5)?,
        status: row.get(6)?,
        next_page_index: row.get(7)?,
        enumeration_cursor: row.get(8)?,
        comparison_cursor: row.get(9)?,
        absence_cursor: row.get(10)?,
        staged_entry_count: row.get(11)?,
        candidate_count: row.get(12)?,
        enumeration_complete: row.get(13)?,
        absence_authority: row.get(14)?,
        started_unix_ms: row.get(15)?,
        updated_unix_ms: row.get(16)?,
        completed_unix_ms: row.get(17)?,
        last_issue_code: row.get(18)?,
        last_issue_message: row.get(19)?,
    })
}

struct StoredEntry {
    relative_path: String,
    entry_kind: String,
    file_size: Option<i64>,
    modified_unix_ms: i64,
    file_identity_scheme: Option<String>,
    file_identity_value: Option<String>,
    placeholder_state: String,
    is_reparse_point: bool,
}

impl StoredEntry {
    fn into_domain(self) -> Result<MetadataInventoryEntry, ScanError> {
        let kind = match self.entry_kind.as_str() {
            "file" => MetadataInventoryEntryKind::File,
            "directory" => MetadataInventoryEntryKind::Directory,
            "other" => MetadataInventoryEntryKind::Other,
            _ => {
                return Err(ScanError::new(
                    "catalog_metadata_inventory_entry_kind_invalid",
                    "The stored metadata inventory entry kind is invalid",
                ));
            }
        };
        let placeholder_state = match self.placeholder_state.as_str() {
            "available" => MetadataInventoryPlaceholderState::Available,
            "offline" => MetadataInventoryPlaceholderState::Offline,
            "recall_on_open" => MetadataInventoryPlaceholderState::RecallOnOpen,
            "recall_on_data_access" => MetadataInventoryPlaceholderState::RecallOnDataAccess,
            _ => {
                return Err(ScanError::new(
                    "catalog_metadata_inventory_placeholder_invalid",
                    "The stored metadata inventory placeholder state is invalid",
                ));
            }
        };
        let file_identity = match (self.file_identity_scheme, self.file_identity_value) {
            (Some(scheme), Some(value)) => Some(FileIdentityEvidence { scheme, value }),
            (None, None) => None,
            _ => {
                return Err(ScanError::new(
                    "catalog_metadata_inventory_identity_invalid",
                    "The stored metadata inventory file identity is incomplete",
                ));
            }
        };
        Ok(MetadataInventoryEntry {
            relative_path: self.relative_path,
            kind,
            file_size: self
                .file_size
                .map(|value| sqlite_unsigned(value, "metadata inventory file size"))
                .transpose()?,
            modified_unix_ms: self.modified_unix_ms,
            file_identity,
            placeholder_state,
            is_reparse_point: self.is_reparse_point,
        })
    }
}

fn stored_entry(row: &Row<'_>) -> rusqlite::Result<StoredEntry> {
    Ok(StoredEntry {
        relative_path: row.get(0)?,
        entry_kind: row.get(1)?,
        file_size: row.get(2)?,
        modified_unix_ms: row.get(3)?,
        file_identity_scheme: row.get(4)?,
        file_identity_value: row.get(5)?,
        placeholder_state: row.get(6)?,
        is_reparse_point: row.get(7)?,
    })
}

fn parse_run_status(value: &str) -> Result<MetadataInventoryRunStatus, ScanError> {
    match value {
        "running" => Ok(MetadataInventoryRunStatus::Running),
        "comparing" => Ok(MetadataInventoryRunStatus::Comparing),
        "completed" => Ok(MetadataInventoryRunStatus::Completed),
        "failed" => Ok(MetadataInventoryRunStatus::Failed),
        "cancelled" => Ok(MetadataInventoryRunStatus::Cancelled),
        "superseded" => Ok(MetadataInventoryRunStatus::Superseded),
        _ => Err(ScanError::new(
            "catalog_metadata_inventory_status_invalid",
            "The stored metadata inventory status is invalid",
        )),
    }
}

fn validate_run_request(request: &MetadataInventoryRunRequest) -> Result<(), ScanError> {
    if request.run_id.is_empty()
        || request.run_id.len() > 256
        || request.run_id.contains('\0')
        || request.root_id.is_empty()
        || request.root_id.len() > 256
        || request.root_id.contains('\0')
        || request.epoch == 0
    {
        return Err(ScanError::new(
            "metadata_inventory_request_invalid",
            "Metadata inventory identity and epoch must be bounded and non-empty",
        ));
    }
    match &request.scope {
        MetadataInventoryScope::Root => Ok(()),
        MetadataInventoryScope::Subtree { relative_path } => {
            validate_relative_path(relative_path, false)
        }
    }
}

fn validate_start_request(request: &MetadataInventoryStartRequest) -> Result<(), ScanError> {
    validate_run_request(&MetadataInventoryRunRequest {
        run_id: request.run_id.clone(),
        root_id: request.root_id.clone(),
        root_generation: request.root_generation,
        epoch: 1,
        scope: request.scope.clone(),
        started_unix_ms: request.started_unix_ms,
    })
}

fn validate_page(page: &MetadataInventoryPage) -> Result<(), ScanError> {
    if page.page_index == 0
        || page.entries.len() > MAX_PAGE_ENTRIES as usize
        || page.entries.is_empty() && !page.is_complete
        || page.cursor.as_deref()
            != page
                .entries
                .last()
                .map(|entry| entry.relative_path.as_str())
    {
        return Err(ScanError::new(
            "metadata_inventory_page_invalid",
            "Metadata inventory page shape or cursor is invalid",
        ));
    }
    Ok(())
}

fn validate_entry(
    scope: &MetadataInventoryScope,
    entry: &MetadataInventoryEntry,
) -> Result<(), ScanError> {
    validate_relative_path(&entry.relative_path, false)?;
    if let MetadataInventoryScope::Subtree { relative_path } = scope
        && entry.relative_path != *relative_path
        && !entry
            .relative_path
            .strip_prefix(relative_path)
            .is_some_and(|suffix| suffix.starts_with('/'))
    {
        return Err(ScanError::new(
            "metadata_inventory_entry_outside_scope",
            "A metadata inventory entry escaped its owning subtree",
        ));
    }
    if (entry.kind == MetadataInventoryEntryKind::File) != entry.file_size.is_some() {
        return Err(ScanError::new(
            "metadata_inventory_entry_state_invalid",
            "Only file inventory entries carry a file size",
        ));
    }
    if entry.kind == MetadataInventoryEntryKind::Directory
        && (entry.is_reparse_point
            || entry.placeholder_state != MetadataInventoryPlaceholderState::Available)
    {
        return Err(ScanError::new(
            "metadata_inventory_directory_unverifiable",
            "An untraversed directory cannot contribute complete absence authority",
        ));
    }
    Ok(())
}

fn entry_parts(entry: &MetadataInventoryEntry) -> Result<StoredEntryParts<'_>, ScanError> {
    let entry_kind = match entry.kind {
        MetadataInventoryEntryKind::File => "file",
        MetadataInventoryEntryKind::Directory => "directory",
        MetadataInventoryEntryKind::Other => "other",
    };
    let file_size = entry
        .file_size
        .map(|value| sqlite_integer(value, "metadata inventory file size"))
        .transpose()?;
    let (identity_scheme, identity_value) =
        entry
            .file_identity
            .as_ref()
            .map_or((None, None), |identity| {
                (
                    Some(identity.scheme.as_str()),
                    Some(identity.value.as_str()),
                )
            });
    if identity_scheme.is_some_and(|value| value.is_empty() || value.len() > 128)
        || identity_value.is_some_and(|value| value.is_empty() || value.len() > 1_024)
    {
        return Err(ScanError::new(
            "metadata_inventory_identity_invalid",
            "Metadata inventory file identity evidence must be complete",
        ));
    }
    let placeholder_state = match entry.placeholder_state {
        MetadataInventoryPlaceholderState::Available => "available",
        MetadataInventoryPlaceholderState::Offline => "offline",
        MetadataInventoryPlaceholderState::RecallOnOpen => "recall_on_open",
        MetadataInventoryPlaceholderState::RecallOnDataAccess => "recall_on_data_access",
    };
    Ok((
        entry_kind,
        file_size,
        identity_scheme,
        identity_value,
        placeholder_state,
    ))
}

fn scope_parts(scope: &MetadataInventoryScope) -> (&'static str, &str) {
    match scope {
        MetadataInventoryScope::Root => ("root", ""),
        MetadataInventoryScope::Subtree { relative_path } => ("subtree", relative_path),
    }
}

fn validate_window_limit(limit: u32) -> Result<(), ScanError> {
    if limit == 0 || limit > MAX_PAGE_ENTRIES {
        return Err(ScanError::new(
            "metadata_inventory_window_limit_invalid",
            "Metadata inventory windows must contain between 1 and 4096 entries",
        ));
    }
    Ok(())
}

fn validate_relative_path(value: &str, allow_empty: bool) -> Result<(), ScanError> {
    let path = Path::new(value);
    let drive_qualified = value.as_bytes().get(1) == Some(&b':')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic);
    if (!allow_empty && value.is_empty())
        || value.contains('\0')
        || value.contains('\\')
        || drive_qualified
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
                    | Component::CurDir
            )
        })
    {
        return Err(ScanError::new(
            "metadata_inventory_relative_path_invalid",
            "Metadata inventory paths must be normalized relative paths",
        ));
    }
    Ok(())
}

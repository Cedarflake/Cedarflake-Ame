use std::collections::BTreeSet;
use std::path::{Component, Path};

#[cfg(test)]
use std::cell::RefCell;

use blake3::Hasher;
use rusqlite::types::Value;
use rusqlite::{
    Connection, ErrorCode, OptionalExtension, Row, Transaction, params, params_from_iter,
};

use crate::adapters::{DurableLocalMetadataInventory, PublicationGuardedFileDiscovery};
use crate::domain::{
    FileIdentityEvidence, JournalIdentifier, JournalUsn, LeasedLibraryChange,
    LibraryChangeEnqueueReport, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLane, LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthority,
    LibraryRecoveryAuthorityReason, LibraryRecoveryOpeningBoundary, LibraryRootGeneration,
    MetadataInventoryCleanupReport, MetadataInventoryComparisonStatus,
    MetadataInventoryComparisonUpdate, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryFrontierEntry, MetadataInventoryFrontierState, MetadataInventoryPage,
    MetadataInventoryPlaceholderState, MetadataInventoryRun, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, MetadataInventoryScope, MetadataInventoryStartRequest, ScanError,
};
use crate::ports::{MetadataInventoryAbsencePublicationRequest, MetadataInventoryRepository};

use super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

const MAX_PAGE_ENTRIES: u32 = 4_096;
const MAX_CLEANUP_RUNS: u32 = 128;
const METADATA_ENTRY_INSERT_ROWS_PER_STATEMENT: usize = 64;
const METADATA_INVENTORY_SPOOL_PAGE_SQL: &str =
    "SELECT relative_path, entry_kind, file_size, modified_unix_ms,
            file_identity_scheme, file_identity_value, placeholder_state,
            is_reparse_point
     FROM library_metadata_inventory_spool_entries
     WHERE run_id = ?1
       AND relative_path > COALESCE(?2, '')
     ORDER BY relative_path LIMIT ?3";
type StoredEntryParts<'a> = (
    &'static str,
    Option<i64>,
    Option<&'a str>,
    Option<&'a str>,
    &'static str,
);

#[cfg(test)]
thread_local! {
    static BEFORE_METADATA_INVENTORY_SPOOL_COMMIT_HOOK: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_before_metadata_inventory_spool_commit_hook(hook: impl FnOnce() + 'static) {
    BEFORE_METADATA_INVENTORY_SPOOL_COMMIT_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
}

#[cfg(test)]
fn run_before_metadata_inventory_spool_commit_hook() {
    BEFORE_METADATA_INVENTORY_SPOOL_COMMIT_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

pub(super) fn insert_metadata_inventory_recovery_authority(
    transaction: &Transaction<'_>,
    authority: &LibraryRecoveryAuthority,
) -> Result<(), ScanError> {
    validate_recovery_authority(authority)?;
    let (opening_journal_id, opening_next_usn) = if matches!(
        authority.reason,
        LibraryRecoveryAuthorityReason::ExistingRootBaseline
            | LibraryRecoveryAuthorityReason::FirstImportBoundary
    ) {
        authority
            .opening_boundary
            .as_ref()
            .map_or((None, None), |boundary| {
                (
                    Some(boundary.journal_id.to_canonical_text()),
                    Some(boundary.next_usn.to_canonical_text()),
                )
            })
    } else {
        (None, None)
    };
    transaction
        .execute(
            "INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason,
               opening_journal_id, opening_next_usn, authorized_unix_ms, retired_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                sqlite_integer(authority.change_id.value(), "recovery authority change id")?,
                authority.run_id,
                authority.root_id,
                sqlite_integer(
                    authority.root_generation.value(),
                    "recovery authority root generation",
                )?,
                recovery_authority_reason_text(authority.reason),
                opening_journal_id,
                opening_next_usn,
                authority.authorized_unix_ms,
                authority.retired_unix_ms,
            ],
        )
        .map_err(recovery_authority_database_error)?;
    Ok(())
}

impl MetadataInventoryRepository for SqliteCatalog {
    fn open_metadata_inventory_source(
        &self,
        root_path: &str,
        scope: &MetadataInventoryScope,
        run: &MetadataInventoryRun,
        authority: &LeasedLibraryChange,
        expected_publication_identity: Option<&FileIdentityEvidence>,
        expected_source_identity: &FileIdentityEvidence,
    ) -> Result<Box<dyn crate::ports::MetadataInventorySource>, ScanError> {
        DurableLocalMetadataInventory::open(
            self.validated_session(),
            root_path,
            scope,
            run,
            authority,
            expected_publication_identity,
            expected_source_identity,
        )
        .map(|source| Box::new(source) as Box<dyn crate::ports::MetadataInventorySource>)
    }

    fn load_metadata_inventory_root_identity(
        &self,
        run_id: &str,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        self.connection
            .query_row(
                "SELECT root_identity_scheme, root_identity_value
                 FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [run_id],
                |row| {
                    Ok(FileIdentityEvidence {
                        scheme: row.get(0)?,
                        value: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(database_error)
    }

    fn authorize_metadata_inventory_recovery(
        &mut self,
        authority: &LibraryRecoveryAuthority,
    ) -> Result<LibraryRecoveryAuthority, ScanError> {
        validate_recovery_authority(authority)?;
        let transaction = self.begin_write()?;
        if let Some(existing) = load_recovery_authority(&transaction, authority.change_id)? {
            if existing.root_id != authority.root_id
                || existing.root_generation != authority.root_generation
                || existing.reason != authority.reason
                || existing.opening_boundary != authority.opening_boundary
                || existing.retired_unix_ms.is_some()
            {
                return Err(ScanError::new(
                    "metadata_inventory_recovery_authority_conflict",
                    "The recovery change already has different durable authority",
                ));
            }
            if existing.run_id != authority.run_id {
                transaction
                    .execute(
                        "UPDATE library_recovery_authorities
                         SET run_id = ?2
                         WHERE change_id = ?1 AND retired_unix_ms IS NULL",
                        params![
                            sqlite_integer(
                                authority.change_id.value(),
                                "recovery authority change id",
                            )?,
                            authority.run_id,
                        ],
                    )
                    .map_err(recovery_authority_database_error)?;
                let updated = load_recovery_authority(&transaction, authority.change_id)?
                    .ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_recovery_authority_missing",
                            "The durable recovery authority no longer exists",
                        )
                    })?;
                transaction.commit().map_err(database_error)?;
                return Ok(updated);
            }
            transaction.commit().map_err(database_error)?;
            return Ok(existing);
        }
        insert_metadata_inventory_recovery_authority(&transaction, authority)?;
        let stored =
            load_recovery_authority(&transaction, authority.change_id)?.ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_recovery_authority_missing",
                    "The durable recovery authority was not stored",
                )
            })?;
        transaction.commit().map_err(database_error)?;
        Ok(stored)
    }

    fn load_metadata_inventory_recovery_authority(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<Option<LibraryRecoveryAuthority>, ScanError> {
        load_recovery_authority(&self.connection, change_id)
    }

    fn retire_metadata_inventory_recovery_authority(
        &mut self,
        change_id: LibraryChangeId,
        retired_unix_ms: i64,
    ) -> Result<LibraryRecoveryAuthority, ScanError> {
        let transaction = self.begin_write()?;
        let authority = load_recovery_authority(&transaction, change_id)?.ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_authority_missing",
                "The durable recovery authority no longer exists",
            )
        })?;
        if retired_unix_ms < authority.authorized_unix_ms {
            return Err(ScanError::new(
                "metadata_inventory_recovery_authority_time_invalid",
                "Recovery authority cannot retire before it was authorized",
            ));
        }
        if authority.retired_unix_ms.is_none() {
            transaction
                .execute(
                    "UPDATE library_recovery_authorities
                     SET retired_unix_ms = ?2
                     WHERE change_id = ?1 AND retired_unix_ms IS NULL",
                    params![
                        sqlite_integer(change_id.value(), "recovery authority change id")?,
                        retired_unix_ms,
                    ],
                )
                .map_err(recovery_authority_database_error)?;
        }
        let retired = load_recovery_authority(&transaction, change_id)?.ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_authority_missing",
                "The durable recovery authority no longer exists",
            )
        })?;
        transaction.commit().map_err(database_error)?;
        Ok(retired)
    }

    fn metadata_inventory_is_waiting_for_closing_boundary(
        &self,
        run_id: &str,
    ) -> Result<bool, ScanError> {
        if run_id.is_empty() {
            return Ok(false);
        }
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_metadata_inventory_runs AS run
                   JOIN library_recovery_authorities AS authority
                     ON authority.run_id = run.id
                   JOIN library_persistent_journal_baselines AS baseline
                     ON baseline.change_id = authority.change_id
                   WHERE run.id = ?1
                     AND run.status = 'comparing'
                     AND run.enumeration_complete = 1
                     AND run.absence_authority = 0
                     AND authority.retired_unix_ms IS NULL
                     AND baseline.phase = 'inventory'
                     AND NOT EXISTS (
                       SELECT 1 FROM library_metadata_inventory_entries AS entry
                       WHERE entry.run_id = run.id
                         AND entry.comparison_status = 'pending'
                     )
                 )",
                [run_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    fn metadata_inventory_recovery_allows_absence(
        &self,
        change_id: LibraryChangeId,
    ) -> Result<bool, ScanError> {
        let is_baseline = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_persistent_journal_baselines
                   WHERE change_id = ?1 AND phase <> 'completed'
                 )",
                [sqlite_integer(
                    change_id.value(),
                    "recovery authority change id",
                )?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !is_baseline {
            return Ok(true);
        }
        super::persistent_journal::baseline_closing_is_covered(&self.connection, change_id)
    }

    fn finish_metadata_inventory_recovery(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<Option<LibraryChangeLeaseUpdateOutcome>, ScanError> {
        let (root_path, expected_root_identity) = self
            .connection
            .query_row(
                "SELECT root.path, spool.root_identity_scheme, spool.root_identity_value
                 FROM library_recovery_authorities AS authority
                 JOIN library_metadata_inventory_spools AS spool
                   ON spool.run_id = authority.run_id
                 JOIN library_roots AS root ON root.id = authority.root_id
                 WHERE authority.change_id = ?1 AND authority.retired_unix_ms IS NULL",
                [sqlite_integer(
                    change_id.value(),
                    "recovery authority change id",
                )?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        FileIdentityEvidence {
                            scheme: row.get(1)?,
                            value: row.get(2)?,
                        },
                    ))
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_root_identity_missing",
                    "Recovery finalization lacks its durable pinned-root identity",
                )
            })?;
        let _finalization_root =
            PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                &root_path,
                &expected_root_identity,
            )?;
        let transaction = self.begin_write()?;
        let authority = load_recovery_authority(&transaction, change_id)?.ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_authority_missing",
                "The durable recovery authority no longer exists",
            )
        })?;
        if completed_unix_ms < authority.authorized_unix_ms {
            return Err(ScanError::new(
                "metadata_inventory_recovery_authority_time_invalid",
                "Recovery authority cannot complete before it was authorized",
            ));
        }
        let run = require_run(&transaction, &authority.run_id)?;
        if !matches!(
            run.status,
            MetadataInventoryRunStatus::Comparing | MetadataInventoryRunStatus::Completed
        ) || run.request.root_id != authority.root_id
            || run.request.root_generation != authority.root_generation
        {
            return Err(ScanError::new(
                "metadata_inventory_recovery_incomplete",
                "Recovery authority cannot retire before its inventory completes",
            ));
        }
        if run.status == MetadataInventoryRunStatus::Comparing {
            if !run.enumeration_complete || !run.absence_authority {
                return Err(ScanError::new(
                    "metadata_inventory_recovery_incomplete",
                    "Recovery authority cannot retire before inventory publication completes",
                ));
            }
            let has_pending = transaction
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM library_metadata_inventory_entries
                       WHERE run_id = ?1 AND comparison_status = 'pending'
                     )",
                    [&authority.run_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            let remaining_absence =
                load_absence_candidates(&transaction, &run, run.absence_cursor.as_deref(), 1)?;
            if has_pending || !remaining_absence.is_empty() {
                return Err(ScanError::new(
                    "metadata_inventory_recovery_incomplete",
                    "Recovery authority cannot retire before every inventory page is published",
                ));
            }
        }
        let is_baseline = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_persistent_journal_baselines
                   WHERE change_id = ?1
                 )",
                [sqlite_integer(
                    change_id.value(),
                    "recovery authority change id",
                )?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if is_baseline {
            let phase = transaction
                .query_row(
                    "SELECT phase FROM library_persistent_journal_baselines
                     WHERE change_id = ?1",
                    [sqlite_integer(
                        change_id.value(),
                        "recovery authority change id",
                    )?],
                    |row| row.get::<_, String>(0),
                )
                .map_err(database_error)?;
            if phase != "absence"
                || !super::persistent_journal::baseline_closing_is_covered(&transaction, change_id)?
            {
                return Err(ScanError::new(
                    "persistent_journal_baseline_closing_incomplete",
                    "The closing journal boundary must be durably covered before baseline completion",
                ));
            }
        }
        if metadata_inventory_has_unresolved_candidates(&transaction, &run)? {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        }
        let outcome = super::change_queue::classify_lease_update(
            &transaction,
            change_id,
            lease_generation,
            Some(catalog_revision_at_success),
        )?;
        if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
            transaction.commit().map_err(database_error)?;
            return Ok(Some(outcome));
        }
        let completed = transaction
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
        if completed != 1 {
            return Err(ScanError::new(
                "metadata_inventory_recovery_completion_raced",
                "The leased recovery changed during atomic completion",
            ));
        }
        if run.status == MetadataInventoryRunStatus::Comparing {
            let completed_run = transaction
                .execute(
                    "UPDATE library_metadata_inventory_runs
                     SET status = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                     WHERE id = ?1 AND status = 'comparing'
                       AND enumeration_complete = 1 AND absence_authority = 1",
                    params![authority.run_id, completed_unix_ms],
                )
                .map_err(database_error)?;
            if completed_run != 1 {
                return Err(ScanError::new(
                    "metadata_inventory_recovery_completion_raced",
                    "The inventory publication changed during atomic completion",
                ));
            }
        }
        super::establish_root_publication_namespace(
            &transaction,
            &authority.root_id,
            sqlite_integer(authority.root_generation.value(), "root generation")?,
            &expected_root_identity,
            "metadata_inventory",
            catalog_revision_at_success,
            completed_unix_ms,
        )?;
        if is_baseline {
            let baseline_identity = transaction
                .query_row(
                    "SELECT root_id, root_generation
                     FROM library_persistent_journal_baselines
                     WHERE change_id = ?1 AND phase = 'absence'",
                    [sqlite_integer(change_id.value(), "baseline change ID")?],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .map_err(database_error)?;
            let updated = transaction
                .execute(
                    "UPDATE library_persistent_journal_baselines
                     SET phase = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                     WHERE change_id = ?1 AND phase = 'absence'
                       AND completed_unix_ms IS NULL",
                    params![
                        sqlite_integer(change_id.value(), "baseline change ID")?,
                        completed_unix_ms,
                    ],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "persistent_journal_baseline_completion_raced",
                    "The one-time baseline changed during atomic completion",
                ));
            }
            let checkpoint_updated = transaction
                .execute(
                    "UPDATE library_persistent_journal_checkpoints
                     SET continuity_state = 'current', updated_unix_ms = ?1
                     WHERE root_id = ?2 AND root_generation = ?3
                       AND continuity_state = 'catching_up'
                       AND next_unread_usn = captured_exclusive_end
                       AND last_failure_code IS NULL",
                    params![completed_unix_ms, baseline_identity.0, baseline_identity.1],
                )
                .map_err(database_error)?;
            if checkpoint_updated != 1 {
                return Err(ScanError::new(
                    "persistent_journal_baseline_completion_raced",
                    "The baseline checkpoint changed during atomic completion",
                ));
            }
            let root_state_updated = transaction
                .execute(
                    "UPDATE library_persistent_journal_root_state
                     SET continuity_state = 'current', updated_unix_ms = ?1
                     WHERE root_id = ?2 AND root_generation = ?3
                       AND capability_state = 'supported'
                       AND continuity_state = 'catching_up'
                       AND last_failure_code IS NULL
                       AND EXISTS (
                         SELECT 1 FROM library_change_root_state AS active
                         WHERE active.root_id = ?2 AND active.generation = ?3
                           AND active.is_active = 1
                       )",
                    params![completed_unix_ms, baseline_identity.0, baseline_identity.1],
                )
                .map_err(database_error)?;
            if root_state_updated != 1 {
                return Err(ScanError::new(
                    "persistent_journal_baseline_completion_raced",
                    "The baseline root authority changed during atomic completion",
                ));
            }
        }
        transaction
            .execute(
                "DELETE FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [&authority.run_id],
            )
            .map_err(database_error)?;
        let retired = transaction
            .execute(
                "UPDATE library_recovery_authorities
                 SET retired_unix_ms = ?2
                 WHERE change_id = ?1 AND retired_unix_ms IS NULL",
                params![
                    sqlite_integer(change_id.value(), "recovery authority change id")?,
                    completed_unix_ms,
                ],
            )
            .map_err(recovery_authority_database_error)?;
        if retired != 1 {
            return Err(ScanError::new(
                "metadata_inventory_recovery_completion_raced",
                "Recovery authority changed during atomic completion",
            ));
        }
        super::change_queue::wake_metadata_inventory_capacity_deferrals(
            &transaction,
            &authority.root_id,
            authority.root_generation,
            completed_unix_ms,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(Some(outcome))
    }

    fn begin_next_metadata_inventory(
        &mut self,
        request: &MetadataInventoryStartRequest,
    ) -> Result<MetadataInventoryRun, ScanError> {
        validate_start_request(request)?;
        let transaction = self.begin_write()?;
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
        let transaction = self.begin_write()?;
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
        let transaction = self.begin_write()?;
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
        insert_metadata_inventory_page_entries(&transaction, &run, page, updated_unix_ms)?;
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
        replace_metadata_inventory_frontier(&transaction, run_id, &page.frontier, updated_unix_ms)?;
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
        let transaction = self.begin_write()?;
        let run = require_run(&transaction, run_id)?;
        if run.status != MetadataInventoryRunStatus::Comparing || !run.enumeration_complete {
            return Err(ScanError::new(
                "metadata_inventory_enumeration_incomplete",
                "Absence cannot become authoritative before enumeration completes",
            ));
        }
        validate_active_root(&transaction, &run.request)?;
        let baseline_change_id = transaction
            .query_row(
                "SELECT authority.change_id
                 FROM library_recovery_authorities AS authority
                 JOIN library_persistent_journal_baselines AS baseline
                   ON baseline.change_id = authority.change_id
                 WHERE authority.run_id = ?1 AND authority.retired_unix_ms IS NULL
                   AND baseline.phase IN ('replay', 'absence')",
                [run_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(change_id) = baseline_change_id {
            let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "baseline change ID")?)
                .ok_or_else(|| {
                    ScanError::new(
                        "persistent_journal_baseline_corrupt",
                        "The baseline change ID is invalid",
                    )
                })?;
            if !super::persistent_journal::baseline_closing_is_covered(&transaction, change_id)? {
                return Err(ScanError::new(
                    "persistent_journal_baseline_closing_incomplete",
                    "Absence cannot become authoritative before closing replay completes",
                ));
            }
            transaction
                .execute(
                    "UPDATE library_persistent_journal_baselines
                     SET phase = 'absence', updated_unix_ms = ?2
                     WHERE change_id = ?1 AND phase = 'replay'",
                    params![
                        sqlite_integer(change_id.value(), "baseline change ID")?,
                        updated_unix_ms,
                    ],
                )
                .map_err(database_error)?;
        }
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
        let transaction = self.begin_write()?;
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

    fn publish_metadata_inventory_comparison_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        run_id: &str,
        intents: &[LibraryChangeIntent],
        updates: &[MetadataInventoryComparisonUpdate],
        updated_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<(LibraryChangeEnqueueReport, MetadataInventoryRun)>, ScanError> {
        validate_comparison_publication(intents, updates)?;
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        require_recovery_candidate_publication(&transaction, authority, run_id)?;
        let mut pending_updates = Vec::with_capacity(updates.len());
        let mut candidates_to_publish = Vec::new();
        let mut candidate_iter = intents.iter();
        let mut candidate_count = 0_u64;

        for (index, update) in updates.iter().enumerate() {
            let desired_status = comparison_status_text(update.status);
            let current = transaction
                .query_row(
                    "SELECT comparison_status, candidate_previous_relative_path
                     FROM library_metadata_inventory_entries
                     WHERE run_id = ?1 AND relative_path = ?2",
                    params![run_id, update.relative_path],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()
                .map_err(database_error)?
                .ok_or_else(|| {
                    ScanError::new(
                        "metadata_inventory_comparison_conflict",
                        "The metadata inventory entry is missing",
                    )
                })?;
            let is_pending = current.0 == "pending";
            if !is_pending
                && (current.0 != desired_status
                    || current.1.as_deref() != update.candidate_previous_relative_path.as_deref())
            {
                return Err(ScanError::new(
                    "metadata_inventory_comparison_conflict",
                    "The metadata inventory entry has a different durable comparison result",
                ));
            }
            pending_updates.push(is_pending);

            if update.status == MetadataInventoryComparisonStatus::Enqueued {
                let intent = candidate_iter.next().ok_or_else(|| {
                    ScanError::new(
                        "metadata_inventory_candidate_batch_mismatch",
                        "Every enqueued comparison must have one candidate intent",
                    )
                })?;
                let candidate_key = metadata_inventory_candidate_key(
                    "present",
                    &update.relative_path,
                    update.candidate_previous_relative_path.as_deref(),
                );
                let owner_exists = transaction
                    .query_row(
                        "SELECT EXISTS(
                           SELECT 1 FROM library_metadata_inventory_candidate_owners
                           WHERE run_id = ?1 AND candidate_key = ?2
                         )",
                        params![run_id, candidate_key],
                        |row| row.get::<_, bool>(0),
                    )
                    .map_err(database_error)?;
                if is_pending || !owner_exists {
                    candidates_to_publish.push((index, candidate_key, intent.clone()));
                }
                if is_pending {
                    candidate_count = candidate_count.checked_add(1).ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_candidate_count_overflow",
                            "The metadata inventory candidate count overflowed",
                        )
                    })?;
                }
            }
        }

        let candidate_intents = candidates_to_publish
            .iter()
            .map(|(_, _, intent)| intent.clone())
            .collect::<Vec<_>>();
        let Some((enqueue_report, change_ids)) =
            super::change_queue::enqueue_metadata_inventory_candidates_in_transaction(
                &transaction,
                authority,
                &candidate_intents,
                updated_unix_ms,
                policy,
            )?
        else {
            return Ok(None);
        };
        if change_ids.len() != candidates_to_publish.len() {
            return Err(ScanError::new(
                "metadata_inventory_candidate_publication_incomplete",
                "The durable queue did not return every published candidate identity",
            ));
        }

        for (update, is_pending) in updates.iter().zip(pending_updates) {
            if !is_pending {
                continue;
            }
            let updated = transaction
                .execute(
                    "UPDATE library_metadata_inventory_entries
                     SET comparison_status = ?3, candidate_previous_relative_path = ?4
                     WHERE run_id = ?1 AND relative_path = ?2
                       AND comparison_status = 'pending'",
                    params![
                        run_id,
                        update.relative_path,
                        comparison_status_text(update.status),
                        update.candidate_previous_relative_path,
                    ],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "metadata_inventory_comparison_conflict",
                    "The metadata inventory entry changed during candidate publication",
                ));
            }
        }
        for ((_, candidate_key, intent), change_id) in
            candidates_to_publish.iter().zip(change_ids.iter())
        {
            upsert_metadata_inventory_candidate_owner(
                &transaction,
                run_id,
                candidate_key,
                *change_id,
                "present",
                &intent.relative_path,
                intent.previous_relative_path.as_deref(),
                updated_unix_ms,
            )?;
        }
        if let Some(last_relative_path) = updates.last().map(|update| update.relative_path.as_str())
        {
            transaction
                .execute(
                    "UPDATE library_metadata_inventory_runs
                     SET comparison_cursor = CASE
                           WHEN comparison_cursor IS NULL OR comparison_cursor < ?2 THEN ?2
                           ELSE comparison_cursor
                         END,
                         candidate_count = candidate_count + ?3,
                         updated_unix_ms = CASE
                           WHEN comparison_cursor IS NULL OR comparison_cursor < ?2 OR ?3 > 0
                             THEN ?4
                           ELSE updated_unix_ms
                         END
                     WHERE id = ?1 AND status = 'comparing'",
                    params![
                        run_id,
                        last_relative_path,
                        sqlite_integer(candidate_count, "metadata inventory candidate count")?,
                        updated_unix_ms,
                    ],
                )
                .map_err(database_error)?;
        }
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(Some((enqueue_report, run)))
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
        let transaction = self.begin_write()?;
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

    fn publish_metadata_inventory_absence_candidates(
        &mut self,
        authority: &LeasedLibraryChange,
        request: MetadataInventoryAbsencePublicationRequest<'_>,
    ) -> Result<Option<(LibraryChangeEnqueueReport, MetadataInventoryRun)>, ScanError> {
        let MetadataInventoryAbsencePublicationRequest {
            run_id,
            expected_cursor,
            next_cursor,
            intents,
            updated_unix_ms,
            policy,
        } = request;
        validate_absence_publication(intents, expected_cursor, next_cursor)?;
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run = require_recovery_candidate_publication(&transaction, authority, run_id)?;
        if !run.absence_authority {
            return Err(ScanError::new(
                "metadata_inventory_absence_not_authoritative",
                "Absence candidates require complete inventory authority",
            ));
        }
        let is_replay = run.absence_cursor.as_deref() == Some(next_cursor);
        if !is_replay && run.absence_cursor.as_deref() != expected_cursor {
            return Err(ScanError::new(
                "metadata_inventory_absence_cursor_conflict",
                "The metadata inventory absence cursor changed before publication",
            ));
        }
        if !is_replay {
            let expected_paths = load_absence_candidates(
                &transaction,
                &run,
                expected_cursor,
                u32::try_from(intents.len()).map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_absence_batch_invalid",
                        "The absence candidate page exceeds the supported bound",
                    )
                })?,
            )?;
            if expected_paths.len() != intents.len()
                || expected_paths
                    .iter()
                    .zip(intents)
                    .any(|(path, intent)| path != &intent.relative_path)
            {
                return Err(ScanError::new(
                    "metadata_inventory_absence_page_conflict",
                    "The absence candidate page no longer matches the durable cursor",
                ));
            }
        }

        let mut candidates_to_publish = Vec::new();
        for intent in intents {
            let candidate_key =
                metadata_inventory_candidate_key("absence", &intent.relative_path, None);
            let owner_exists = transaction
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM library_metadata_inventory_candidate_owners
                       WHERE run_id = ?1 AND candidate_key = ?2
                     )",
                    params![run_id, candidate_key],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if !is_replay || !owner_exists {
                candidates_to_publish.push((candidate_key, intent.clone()));
            }
        }
        let candidate_intents = candidates_to_publish
            .iter()
            .map(|(_, intent)| intent.clone())
            .collect::<Vec<_>>();
        let Some((enqueue_report, change_ids)) =
            super::change_queue::enqueue_metadata_inventory_candidates_in_transaction(
                &transaction,
                authority,
                &candidate_intents,
                updated_unix_ms,
                policy,
            )?
        else {
            return Ok(None);
        };
        if change_ids.len() != candidates_to_publish.len() {
            return Err(ScanError::new(
                "metadata_inventory_candidate_publication_incomplete",
                "The durable queue did not return every published absence identity",
            ));
        }
        for ((candidate_key, intent), change_id) in
            candidates_to_publish.iter().zip(change_ids.iter())
        {
            upsert_metadata_inventory_candidate_owner(
                &transaction,
                run_id,
                candidate_key,
                *change_id,
                "absence",
                &intent.relative_path,
                None,
                updated_unix_ms,
            )?;
        }
        if !is_replay {
            let updated = transaction
                .execute(
                    "UPDATE library_metadata_inventory_runs
                     SET absence_cursor = ?2,
                         candidate_count = candidate_count + ?3,
                         updated_unix_ms = ?4
                     WHERE id = ?1 AND status = 'comparing'
                       AND absence_cursor IS ?5",
                    params![
                        run_id,
                        next_cursor,
                        sqlite_integer(
                            u64::try_from(intents.len()).map_err(|_| {
                                ScanError::new(
                                    "metadata_inventory_candidate_count_overflow",
                                    "The metadata inventory absence count overflowed",
                                )
                            })?,
                            "metadata inventory candidate count",
                        )?,
                        updated_unix_ms,
                        expected_cursor,
                    ],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "metadata_inventory_absence_cursor_conflict",
                    "The metadata inventory absence cursor changed during publication",
                ));
            }
        }
        let run = require_run(&transaction, run_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(Some((enqueue_report, run)))
    }

    fn complete_metadata_inventory(
        &mut self,
        run_id: &str,
        completed_unix_ms: i64,
    ) -> Result<MetadataInventoryRun, ScanError> {
        let transaction = self.begin_write()?;
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
        let has_recovery_authority = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_recovery_authorities
                   WHERE run_id = ?1 AND retired_unix_ms IS NULL
                 )",
                [run_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !has_recovery_authority {
            transaction
                .execute(
                    "UPDATE library_metadata_inventory_runs
                     SET status = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                     WHERE id = ?1 AND status = 'comparing' AND absence_authority = 1",
                    params![run_id, completed_unix_ms],
                )
                .map_err(database_error)?;
        }
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
        let transaction = self.begin_write()?;
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
                "DELETE FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [run_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = ?2, last_issue_code = ?3, last_issue_message = ?4,
                     absence_authority = 0, updated_unix_ms = ?5
                 WHERE id = ?1 AND status IN ('running', 'comparing')",
                params![run_id, status, issue_code, issue_message, updated_unix_ms],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_frontier
                 SET state = 'completed', resume_after_relative_path = NULL,
                     enumerated_entry_count = 0, updated_unix_ms = ?2
                 WHERE run_id = ?1",
                params![run_id, updated_unix_ms],
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
        let transaction = self.begin_write()?;
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

impl SqliteCatalog {
    pub(crate) fn initialize_metadata_inventory_spool(
        &mut self,
        run: &MetadataInventoryRun,
        authority: &LeasedLibraryChange,
        root_identity: &FileIdentityEvidence,
        initial_entry: Option<&MetadataInventoryEntry>,
        initial_directory: Option<&str>,
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        if run.status != MetadataInventoryRunStatus::Running
            || run.request.root_id != authority.change.intent.root_id
            || run.request.root_generation != authority.change.intent.root_generation
        {
            return Err(ScanError::new(
                "metadata_inventory_spool_authority_mismatch",
                "The source spool does not match the active recovery authority",
            ));
        }
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let authority_change_id = sqlite_integer(
            authority.change.id.value(),
            "metadata inventory spool authority change id",
        )?;
        let (scope_kind, scope_relative_path) = scope_parts(&run.request.scope);
        let existing = transaction
            .query_row(
                "SELECT authority_change_id, root_id, root_generation,
                        root_identity_scheme, root_identity_value,
                        scope_kind, scope_relative_path
                 FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [&run.request.run_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        if let Some(existing) = existing {
            if existing
                != (
                    authority_change_id,
                    run.request.root_id.clone(),
                    sqlite_integer(
                        run.request.root_generation.value(),
                        "metadata inventory spool root generation",
                    )?,
                    root_identity.scheme.clone(),
                    root_identity.value.clone(),
                    scope_kind.to_owned(),
                    scope_relative_path.to_owned(),
                )
            {
                return Err(ScanError::new(
                    "metadata_inventory_spool_authority_mismatch",
                    "The durable source spool is bound to different recovery work",
                ));
            }
            transaction
                .execute(
                    "DELETE FROM library_metadata_inventory_spool_entries
                     WHERE run_id = ?1 AND directory_relative_path IN (
                       SELECT relative_directory
                       FROM library_metadata_inventory_spool_directories
                       WHERE run_id = ?1 AND state = 'enumerating'
                     )",
                    [&run.request.run_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE library_metadata_inventory_spool_directories
                     SET state = 'pending', directory_identity_scheme = NULL,
                         directory_identity_value = NULL, source_entry_count = 0,
                         updated_unix_ms = ?2
                     WHERE run_id = ?1 AND state = 'enumerating'",
                    params![run.request.run_id, updated_unix_ms],
                )
                .map_err(database_error)?;
            #[cfg(test)]
            run_before_metadata_inventory_spool_commit_hook();
            transaction.commit().map_err(database_error)?;
            return Ok(());
        }
        let state = if initial_directory.is_some() {
            "enumerating"
        } else {
            "ready"
        };
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value,
                   scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    run.request.run_id,
                    authority_change_id,
                    run.request.root_id,
                    sqlite_integer(
                        run.request.root_generation.value(),
                        "metadata inventory spool root generation",
                    )?,
                    root_identity.scheme,
                    root_identity.value,
                    scope_kind,
                    scope_relative_path,
                    state,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if let Some(relative_directory) = initial_directory {
            transaction
                .execute(
                    "INSERT INTO library_metadata_inventory_spool_directories(
                       run_id, ordinal, relative_directory, state, source_entry_count,
                       created_unix_ms, updated_unix_ms
                     ) VALUES (?1, 0, ?2, 'pending', 0, ?3, ?3)",
                    params![run.request.run_id, relative_directory, updated_unix_ms],
                )
                .map_err(database_error)?;
        }
        if let Some(entry) = initial_entry {
            insert_spool_entries(
                &transaction,
                &run.request.scope,
                &run.request.run_id,
                None,
                std::slice::from_ref(entry),
                updated_unix_ms,
            )?;
        }
        #[cfg(test)]
        run_before_metadata_inventory_spool_commit_hook();
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn metadata_inventory_spool_is_ready(
        &self,
        run_id: &str,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT state = 'ready'
                 FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_spool_missing",
                    "The durable metadata inventory source spool is missing",
                )
            })
    }

    pub(crate) fn next_metadata_inventory_spool_directory(
        &self,
        run_id: &str,
    ) -> Result<Option<String>, ScanError> {
        self.connection
            .query_row(
                "SELECT relative_directory
                 FROM library_metadata_inventory_spool_directories
                 WHERE run_id = ?1 AND state = 'pending'
                 ORDER BY ordinal LIMIT 1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)
    }

    pub(crate) fn begin_metadata_inventory_spool_directory(
        &mut self,
        run_id: &str,
        relative_directory: &str,
        identity: &FileIdentityEvidence,
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let updated = transaction
            .execute(
                "UPDATE library_metadata_inventory_spool_directories
                 SET state = 'enumerating', directory_identity_scheme = ?3,
                     directory_identity_value = ?4, source_entry_count = 0,
                     updated_unix_ms = ?5
                 WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'pending'",
                params![
                    run_id,
                    relative_directory,
                    identity.scheme,
                    identity.value,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "metadata_inventory_spool_directory_raced",
                "The next durable source directory is no longer pending",
            ));
        }
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn append_metadata_inventory_spool_entries(
        &mut self,
        run_id: &str,
        relative_directory: &str,
        entries: &[MetadataInventoryEntry],
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run = require_running_spool_run(&transaction, run_id, relative_directory)?;
        insert_spool_entries(
            &transaction,
            &run.request.scope,
            run_id,
            Some(relative_directory),
            entries,
            updated_unix_ms,
        )?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_spool_directories
                 SET source_entry_count = source_entry_count + ?3, updated_unix_ms = ?4
                 WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'enumerating'",
                params![
                    run_id,
                    relative_directory,
                    sqlite_integer(
                        u64::try_from(entries.len()).map_err(|_| {
                            ScanError::new(
                                "metadata_inventory_spool_count_overflow",
                                "The source spool batch count overflowed",
                            )
                        })?,
                        "metadata inventory source spool batch count",
                    )?,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn complete_metadata_inventory_spool_directory(
        &mut self,
        run_id: &str,
        relative_directory: &str,
        identity: &FileIdentityEvidence,
        final_entries: &[MetadataInventoryEntry],
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run = require_running_spool_run(&transaction, run_id, relative_directory)?;
        let stored_identity = transaction
            .query_row(
                "SELECT directory_identity_scheme, directory_identity_value
                 FROM library_metadata_inventory_spool_directories
                 WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'enumerating'",
                params![run_id, relative_directory],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(database_error)?;
        if stored_identity != (identity.scheme.clone(), identity.value.clone()) {
            return Err(ScanError::new(
                "metadata_inventory_spool_directory_identity_changed",
                "The source directory identity changed before spool completion",
            ));
        }
        insert_spool_entries(
            &transaction,
            &run.request.scope,
            run_id,
            Some(relative_directory),
            final_entries,
            updated_unix_ms,
        )?;
        let final_count = sqlite_integer(
            u64::try_from(final_entries.len()).map_err(|_| {
                ScanError::new(
                    "metadata_inventory_spool_count_overflow",
                    "The final source spool batch count overflowed",
                )
            })?,
            "metadata inventory final source spool batch count",
        )?;
        let updated = transaction
            .execute(
                "UPDATE library_metadata_inventory_spool_directories
                 SET state = 'completed', source_entry_count = source_entry_count + ?3,
                     updated_unix_ms = ?4
                 WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'enumerating'",
                params![run_id, relative_directory, final_count, updated_unix_ms],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "metadata_inventory_spool_directory_raced",
                "The source spool directory completion raced with recovery",
            ));
        }
        transaction
            .execute(
                "WITH base(next_ordinal) AS (
                   SELECT COALESCE(MAX(ordinal) + 1, 0)
                   FROM library_metadata_inventory_spool_directories WHERE run_id = ?1
                 ), children(relative_path, child_offset) AS (
                   SELECT relative_path, ROW_NUMBER() OVER (ORDER BY relative_path) - 1
                   FROM library_metadata_inventory_spool_entries
                   WHERE run_id = ?1 AND directory_relative_path = ?2
                     AND entry_kind = 'directory' AND placeholder_state = 'available'
                     AND is_reparse_point = 0
                 )
                 INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state, source_entry_count,
                   created_unix_ms, updated_unix_ms
                 )
                 SELECT ?1, base.next_ordinal + children.child_offset,
                        children.relative_path, 'pending', 0, ?3, ?3
                 FROM children CROSS JOIN base",
                params![run_id, relative_directory, updated_unix_ms],
            )
            .map_err(metadata_inventory_spool_write_error)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_spools
                 SET state = CASE WHEN EXISTS(
                       SELECT 1 FROM library_metadata_inventory_spool_directories
                       WHERE run_id = ?1 AND state IN ('pending', 'enumerating')
                     ) THEN 'enumerating' ELSE 'ready' END,
                     updated_unix_ms = ?2
                 WHERE run_id = ?1",
                params![run_id, updated_unix_ms],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn load_metadata_inventory_spool_page(
        &self,
        run_id: &str,
        limit: u32,
    ) -> Result<MetadataInventoryPage, ScanError> {
        validate_window_limit(limit)?;
        let run = require_run(&self.connection, run_id)?;
        if run.status != MetadataInventoryRunStatus::Running
            || !self.metadata_inventory_spool_is_ready(run_id)?
        {
            return Err(ScanError::new(
                "metadata_inventory_spool_not_ready",
                "The durable source spool is not ready for ordered paging",
            ));
        }
        let page_size = i64::from(limit).checked_add(1).ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_spool_page_limit_overflow",
                "The output spool sentinel page limit overflowed",
            )
        })?;
        let mut statement = self
            .connection
            .prepare(METADATA_INVENTORY_SPOOL_PAGE_SQL)
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![run_id, run.enumeration_cursor.as_deref(), page_size],
                stored_entry,
            )
            .map_err(database_error)?;
        let mut entries = rows
            .map(|row| {
                row.map_err(database_error)
                    .and_then(StoredEntry::into_domain)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output_limit = usize::try_from(limit).map_err(|_| {
            ScanError::new(
                "metadata_inventory_spool_page_limit_overflow",
                "The output spool page limit does not fit this platform",
            )
        })?;
        let is_complete = entries.len() <= output_limit;
        if !is_complete {
            entries.pop();
        }
        let cursor = entries.last().map(|entry| entry.relative_path.clone());
        let directory_identity = self
            .connection
            .query_row(
                "SELECT directory_identity_scheme, directory_identity_value
                 FROM library_metadata_inventory_spool_directories
                 WHERE run_id = ?1 AND state = 'completed'
                 ORDER BY ordinal LIMIT 1",
                [run_id],
                |row| {
                    Ok(FileIdentityEvidence {
                        scheme: row.get(0)?,
                        value: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(database_error)?;
        let frontier = vec![MetadataInventoryFrontierEntry {
            ordinal: 0,
            relative_directory: run.request.scope.relative_path().to_owned(),
            state: if is_complete {
                MetadataInventoryFrontierState::Completed
            } else {
                MetadataInventoryFrontierState::Enumerating
            },
            directory_identity,
            resume_after_relative_path: if is_complete { None } else { cursor.clone() },
            enumerated_entry_count: if is_complete {
                0
            } else {
                run.staged_entry_count
                    .checked_add(u64::try_from(entries.len()).map_err(|_| {
                        ScanError::new(
                            "metadata_inventory_spool_count_overflow",
                            "The output spool frontier count overflowed",
                        )
                    })?)
                    .ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_spool_count_overflow",
                            "The output spool frontier count overflowed",
                        )
                    })?
            },
        }];
        Ok(MetadataInventoryPage {
            page_index: run.next_page_index,
            entries,
            cursor,
            is_complete,
            frontier,
        })
    }

    #[cfg(test)]
    pub(crate) fn metadata_inventory_spool_page_query_plan(
        &self,
        run_id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError> {
        validate_window_limit(limit)?;
        let query = format!("EXPLAIN QUERY PLAN {METADATA_INVENTORY_SPOOL_PAGE_SQL}");
        let mut statement = self.connection.prepare(&query).map_err(database_error)?;
        let rows = statement
            .query_map(params![run_id, cursor, i64::from(limit) + 1], |row| {
                row.get::<_, String>(3)
            })
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }
}

fn require_running_spool_run(
    transaction: &Transaction<'_>,
    run_id: &str,
    relative_directory: &str,
) -> Result<MetadataInventoryRun, ScanError> {
    let run = require_run(transaction, run_id)?;
    let is_active = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_spools AS spool
               JOIN library_metadata_inventory_spool_directories AS directory
                 ON directory.run_id = spool.run_id
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = spool.authority_change_id
               WHERE spool.run_id = ?1 AND spool.state = 'enumerating'
                 AND directory.relative_directory = ?2
                 AND directory.state = 'enumerating'
                 AND authority.run_id = spool.run_id
                 AND authority.retired_unix_ms IS NULL
             )",
            params![run_id, relative_directory],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if run.status != MetadataInventoryRunStatus::Running || !is_active {
        return Err(ScanError::new(
            "metadata_inventory_spool_authority_mismatch",
            "The source directory no longer belongs to active recovery work",
        ));
    }
    Ok(run)
}

fn insert_spool_entries(
    transaction: &Transaction<'_>,
    scope: &MetadataInventoryScope,
    run_id: &str,
    relative_directory: Option<&str>,
    entries: &[MetadataInventoryEntry],
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    for entry in entries {
        validate_entry(scope, entry)?;
        let (entry_kind, file_size, identity_scheme, identity_value, placeholder_state) =
            entry_parts(entry)?;
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spool_entries(
                   run_id, directory_relative_path, relative_path, entry_kind, file_size,
                   modified_unix_ms, file_identity_scheme, file_identity_value,
                   placeholder_state, is_reparse_point, staged_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    run_id,
                    relative_directory,
                    entry.relative_path,
                    entry_kind,
                    file_size,
                    entry.modified_unix_ms,
                    identity_scheme,
                    identity_value,
                    placeholder_state,
                    entry.is_reparse_point,
                    updated_unix_ms,
                ],
            )
            .map_err(metadata_inventory_spool_write_error)?;
    }
    Ok(())
}

fn metadata_inventory_spool_write_error(error: rusqlite::Error) -> ScanError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(ref failure, _)
            if failure.code == ErrorCode::ConstraintViolation
    ) {
        ScanError::new(
            "metadata_inventory_spool_identity_ambiguous",
            "The source spool observed an ambiguous or repeated path",
        )
    } else {
        database_error(error)
    }
}

fn require_recovery_candidate_publication(
    transaction: &Transaction<'_>,
    authority: &LeasedLibraryChange,
    run_id: &str,
) -> Result<MetadataInventoryRun, ScanError> {
    let run = require_run(transaction, run_id)?;
    if run.status != MetadataInventoryRunStatus::Comparing
        || !run.enumeration_complete
        || run.request.root_id != authority.change.intent.root_id
        || run.request.root_generation != authority.change.intent.root_generation
    {
        return Err(ScanError::new(
            "metadata_inventory_candidate_authority_mismatch",
            "Recovery candidate publication does not match the active inventory run",
        ));
    }
    validate_active_root(transaction, &run.request)?;
    let has_authority = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_recovery_authorities AS recovery
               JOIN library_change_queue_lanes AS lane
                 ON lane.change_id = recovery.change_id
               WHERE recovery.change_id = ?1
                 AND recovery.run_id = ?2
                 AND recovery.root_id = ?3
                 AND recovery.root_generation = ?4
                 AND recovery.retired_unix_ms IS NULL
                 AND lane.lane = 'p2_recovery'
             )",
            params![
                sqlite_integer(authority.change.id.value(), "recovery authority change id")?,
                run_id,
                run.request.root_id,
                sqlite_integer(
                    run.request.root_generation.value(),
                    "recovery authority root generation",
                )?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_authority {
        return Err(ScanError::new(
            "metadata_inventory_recovery_not_authorized",
            "Candidate publication requires matching unretired P2 recovery authority",
        ));
    }
    Ok(run)
}

fn metadata_inventory_has_unresolved_candidates(
    transaction: &Transaction<'_>,
    run: &MetadataInventoryRun,
) -> Result<bool, ScanError> {
    transaction
        .query_row(
            "WITH RECURSIVE resolution(
               candidate_key, owned_relative_path, owned_previous_relative_path,
               change_id, status, superseded_by_change_id, scope, work_relative_path,
               work_previous_relative_path, root_id, root_generation, depth, visited
             ) AS (
               SELECT owner.candidate_key, owner.relative_path, owner.previous_relative_path,
                      queue.id, queue.status, queue.superseded_by_change_id, queue.scope,
                      queue.relative_path, queue.previous_relative_path,
                      queue.root_id, queue.root_generation, 0,
                      printf(',%d,', queue.id)
               FROM library_metadata_inventory_candidate_owners AS owner
               JOIN library_change_queue AS queue ON queue.id = owner.change_id
               WHERE owner.run_id = ?1
               UNION ALL
               SELECT resolution.candidate_key, resolution.owned_relative_path,
                      resolution.owned_previous_relative_path,
                      target.id, target.status, target.superseded_by_change_id, target.scope,
                      target.relative_path, target.previous_relative_path,
                      target.root_id, target.root_generation, resolution.depth + 1,
                      resolution.visited || target.id || ','
               FROM resolution
               JOIN library_change_queue AS target
                 ON target.id = resolution.superseded_by_change_id
               WHERE resolution.status = 'superseded'
                 AND resolution.depth < 255
                 AND instr(resolution.visited, printf(',%d,', target.id)) = 0
             )
             SELECT
               EXISTS(
                 SELECT 1 FROM library_metadata_inventory_runs AS inventory
                 WHERE inventory.id = ?1
                   AND inventory.candidate_count <> (
                     SELECT COUNT(*)
                     FROM library_metadata_inventory_candidate_owners AS owner
                     WHERE owner.run_id = inventory.id
                   )
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_candidate_owners AS owner
                 WHERE owner.run_id = ?1
                   AND NOT EXISTS(
                     SELECT 1 FROM resolution
                     WHERE resolution.candidate_key = owner.candidate_key
                       AND resolution.status = 'completed'
                       AND resolution.root_id = ?2
                       AND resolution.root_generation = ?3
                       AND (
                         resolution.scope = 'root'
                         OR (
                           resolution.scope = 'subtree'
                           AND (
                             owner.relative_path = resolution.work_relative_path
                             OR substr(
                               owner.relative_path,
                               1,
                               length(resolution.work_relative_path) + 1
                             ) = resolution.work_relative_path || '/'
                           )
                         )
                         OR (
                           resolution.scope = 'path'
                           AND owner.relative_path = resolution.work_relative_path
                           AND owner.previous_relative_path
                                 IS resolution.work_previous_relative_path
                         )
                       )
                   )
               )",
            params![
                run.request.run_id,
                run.request.root_id,
                sqlite_integer(
                    run.request.root_generation.value(),
                    "metadata inventory root generation",
                )?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}

fn validate_comparison_publication(
    intents: &[LibraryChangeIntent],
    updates: &[MetadataInventoryComparisonUpdate],
) -> Result<(), ScanError> {
    if updates.is_empty() || updates.len() > MAX_PAGE_ENTRIES as usize {
        return Err(ScanError::new(
            "metadata_inventory_comparison_batch_invalid",
            "Metadata inventory comparison batches must stay within one page",
        ));
    }
    let expected_candidates = updates
        .iter()
        .filter(|update| update.status == MetadataInventoryComparisonStatus::Enqueued)
        .count();
    if intents.len() != expected_candidates {
        return Err(ScanError::new(
            "metadata_inventory_candidate_batch_mismatch",
            "Every enqueued comparison must have exactly one candidate intent",
        ));
    }
    let mut candidate_iter = intents.iter();
    let mut previous_path = None;
    for update in updates {
        validate_relative_path(&update.relative_path, false)?;
        if previous_path.is_some_and(|previous| previous >= update.relative_path.as_str()) {
            return Err(ScanError::new(
                "metadata_inventory_comparison_order_invalid",
                "Comparison pages must use stable strictly increasing path order",
            ));
        }
        previous_path = Some(update.relative_path.as_str());
        if let Some(previous) = update.candidate_previous_relative_path.as_deref() {
            validate_relative_path(previous, false)?;
        }
        match update.status {
            MetadataInventoryComparisonStatus::Unchanged => {
                if update.candidate_previous_relative_path.is_some() {
                    return Err(ScanError::new(
                        "metadata_inventory_comparison_invalid",
                        "Only an enqueued rename candidate may retain a previous path",
                    ));
                }
            }
            MetadataInventoryComparisonStatus::Enqueued => {
                let intent = candidate_iter.next().ok_or_else(|| {
                    ScanError::new(
                        "metadata_inventory_candidate_batch_mismatch",
                        "Every enqueued comparison must have one candidate intent",
                    )
                })?;
                validate_metadata_inventory_candidate_intent(
                    intent,
                    &update.relative_path,
                    update.candidate_previous_relative_path.as_deref(),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_absence_publication(
    intents: &[LibraryChangeIntent],
    expected_cursor: Option<&str>,
    next_cursor: &str,
) -> Result<(), ScanError> {
    if intents.is_empty() || intents.len() > MAX_PAGE_ENTRIES as usize {
        return Err(ScanError::new(
            "metadata_inventory_absence_batch_invalid",
            "Absence candidate batches must stay within one non-empty page",
        ));
    }
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
    let mut previous_path = expected_cursor;
    for intent in intents {
        validate_metadata_inventory_candidate_intent(intent, &intent.relative_path, None)?;
        if previous_path.is_some_and(|previous| previous >= intent.relative_path.as_str()) {
            return Err(ScanError::new(
                "metadata_inventory_absence_order_invalid",
                "Absence candidates must use stable strictly increasing path order",
            ));
        }
        previous_path = Some(intent.relative_path.as_str());
    }
    if intents.last().map(|intent| intent.relative_path.as_str()) != Some(next_cursor) {
        return Err(ScanError::new(
            "metadata_inventory_absence_cursor_invalid",
            "The absence cursor must identify the final published candidate",
        ));
    }
    Ok(())
}

fn validate_metadata_inventory_candidate_intent(
    intent: &LibraryChangeIntent,
    relative_path: &str,
    previous_relative_path: Option<&str>,
) -> Result<(), ScanError> {
    let expected_kind = if previous_relative_path.is_some() {
        LibraryChangeIntentKind::RenameCandidate
    } else {
        LibraryChangeIntentKind::Reconcile
    };
    if intent.origin != LibraryChangeOrigin::MetadataInventory
        || intent.scope != LibraryChangeScope::Path
        || intent.kind != expected_kind
        || intent.relative_path != relative_path
        || intent.previous_relative_path.as_deref() != previous_relative_path
    {
        return Err(ScanError::new(
            "metadata_inventory_candidate_batch_mismatch",
            "Candidate intent identity does not match its inventory comparison",
        ));
    }
    Ok(())
}

const fn comparison_status_text(status: MetadataInventoryComparisonStatus) -> &'static str {
    match status {
        MetadataInventoryComparisonStatus::Unchanged => "unchanged",
        MetadataInventoryComparisonStatus::Enqueued => "enqueued",
    }
}

fn metadata_inventory_candidate_key(
    role: &str,
    relative_path: &str,
    previous_relative_path: Option<&str>,
) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"ame-metadata-inventory-candidate-v1\0");
    hasher.update(role.as_bytes());
    hasher.update(b"\0");
    hasher.update(relative_path.as_bytes());
    hasher.update(b"\0");
    if let Some(previous) = previous_relative_path {
        hasher.update(b"1\0");
        hasher.update(previous.as_bytes());
    } else {
        hasher.update(b"0\0");
    }
    format!("v1:{}", hasher.finalize().to_hex())
}

#[expect(
    clippy::too_many_arguments,
    reason = "candidate identity and durable queue ownership are one atomic invariant"
)]
fn upsert_metadata_inventory_candidate_owner(
    transaction: &Transaction<'_>,
    run_id: &str,
    candidate_key: &str,
    change_id: LibraryChangeId,
    candidate_role: &str,
    relative_path: &str,
    previous_relative_path: Option<&str>,
    owned_unix_ms: i64,
) -> Result<(), ScanError> {
    let updated = transaction
        .execute(
            "INSERT INTO library_metadata_inventory_candidate_owners(
               run_id, candidate_key, change_id, candidate_role, relative_path,
               previous_relative_path, owned_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(run_id, candidate_key) DO UPDATE SET
               change_id = excluded.change_id
             WHERE library_metadata_inventory_candidate_owners.candidate_role
                     = excluded.candidate_role
               AND library_metadata_inventory_candidate_owners.relative_path
                     = excluded.relative_path
               AND library_metadata_inventory_candidate_owners.previous_relative_path
                     IS excluded.previous_relative_path",
            params![
                run_id,
                candidate_key,
                sqlite_integer(change_id.value(), "metadata inventory candidate change id")?,
                candidate_role,
                relative_path,
                previous_relative_path,
                owned_unix_ms,
            ],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(ScanError::new(
            "metadata_inventory_candidate_owner_conflict",
            "The stable inventory candidate key belongs to different path work",
        ));
    }
    Ok(())
}

fn load_recovery_authority(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<Option<LibraryRecoveryAuthority>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT authority.run_id, authority.root_id, authority.root_generation,
                    authority.reason, authority.opening_journal_id,
                    authority.opening_next_usn, authority.authorized_unix_ms,
                    authority.retired_unix_ms, baseline.volume_guid,
                    baseline.volume_serial, baseline.root_file_reference,
                    baseline.journal_id, baseline.opening_next_usn,
                    baseline.protocol_version, baseline.contract_version
             FROM library_recovery_authorities AS authority
             LEFT JOIN library_persistent_journal_baselines AS baseline
               ON baseline.change_id = authority.change_id
             WHERE authority.change_id = ?1",
            [sqlite_integer(
                change_id.value(),
                "recovery authority change id",
            )?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<Vec<u8>>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((
        run_id,
        root_id,
        root_generation,
        reason,
        opening_journal_id,
        opening_next_usn,
        authorized_unix_ms,
        retired_unix_ms,
        volume_guid,
        volume_serial,
        root_file_reference,
        baseline_journal_id,
        baseline_opening_next_usn,
        protocol_version,
        contract_version,
    )) = stored
    else {
        return Ok(None);
    };
    let stored_opening_pair = match (opening_journal_id, opening_next_usn) {
        (Some(journal_id), Some(next_usn)) => Some((journal_id, next_usn)),
        (None, None) => None,
        _ => {
            return Err(ScanError::new(
                "metadata_inventory_recovery_authority_corrupt",
                "The recovery opening boundary is incomplete",
            ));
        }
    };
    let opening_boundary = match (
        volume_guid,
        volume_serial,
        root_file_reference,
        baseline_journal_id,
        baseline_opening_next_usn,
        protocol_version,
        contract_version,
    ) {
        (
            Some(volume_guid),
            Some(volume_serial),
            Some(root_file_reference),
            Some(journal_id),
            Some(next_usn),
            Some(protocol_version),
            Some(contract_version),
        ) => {
            if stored_opening_pair
                .as_ref()
                .is_some_and(|stored| stored.0 != journal_id || stored.1 != next_usn)
            {
                return Err(ScanError::new(
                    "metadata_inventory_recovery_authority_corrupt",
                    "The recovery opening boundary disagrees with its journal window",
                ));
            }
            Some(LibraryRecoveryOpeningBoundary {
                volume: crate::domain::PersistentJournalVolumeIdentity {
                    volume_guid,
                    volume_serial: volume_serial.parse().map_err(|_| {
                        ScanError::new(
                            "metadata_inventory_recovery_authority_corrupt",
                            "The recovery opening volume serial is invalid",
                        )
                    })?,
                },
                root_file_reference: crate::domain::JournalFileReference::from_bytes(
                    &root_file_reference,
                )
                .map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_recovery_authority_corrupt",
                        "The recovery opening root identity is invalid",
                    )
                })?,
                journal_id: JournalIdentifier::parse_canonical(&journal_id).map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_recovery_authority_corrupt",
                        "The recovery opening journal identifier is invalid",
                    )
                })?,
                next_usn: JournalUsn::parse_canonical(&next_usn).map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_recovery_authority_corrupt",
                        "The recovery opening journal boundary is invalid",
                    )
                })?,
                protocol_version: u16::try_from(protocol_version).map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_recovery_authority_corrupt",
                        "The recovery opening protocol version is invalid",
                    )
                })?,
                contract_version: u16::try_from(contract_version).map_err(|_| {
                    ScanError::new(
                        "metadata_inventory_recovery_authority_corrupt",
                        "The recovery opening contract version is invalid",
                    )
                })?,
            })
        }
        (None, None, None, None, None, None, None) => None,
        _ => {
            return Err(ScanError::new(
                "metadata_inventory_recovery_authority_corrupt",
                "The recovery opening journal identity is incomplete",
            ));
        }
    };
    let authority = LibraryRecoveryAuthority {
        change_id,
        run_id,
        root_id,
        root_generation: LibraryRootGeneration::new(sqlite_unsigned(
            root_generation,
            "recovery authority root generation",
        )?)
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_authority_corrupt",
                "The recovery root generation is invalid",
            )
        })?,
        reason: parse_recovery_authority_reason(&reason)?,
        opening_boundary,
        authorized_unix_ms,
        retired_unix_ms,
    };
    validate_recovery_authority(&authority)?;
    Ok(Some(authority))
}

fn validate_recovery_authority(authority: &LibraryRecoveryAuthority) -> Result<(), ScanError> {
    if authority.run_id.is_empty()
        || authority.run_id.len() > 512
        || authority.root_id.is_empty()
        || authority.root_id.len() > 512
        || (authority.reason.requires_opening_boundary() && authority.opening_boundary.is_none())
        || authority
            .retired_unix_ms
            .is_some_and(|retired| retired < authority.authorized_unix_ms)
    {
        return Err(ScanError::new(
            "metadata_inventory_recovery_authority_invalid",
            "The metadata recovery authority is invalid",
        ));
    }
    if let Some(boundary) = &authority.opening_boundary {
        boundary.validate()?;
    }
    Ok(())
}

const fn recovery_authority_reason_text(reason: LibraryRecoveryAuthorityReason) -> &'static str {
    match reason {
        LibraryRecoveryAuthorityReason::ExistingRootBaseline => "existing_root_baseline",
        LibraryRecoveryAuthorityReason::FirstImportBoundary => "first_import_boundary",
        LibraryRecoveryAuthorityReason::JournalGap => "journal_gap",
        LibraryRecoveryAuthorityReason::JournalReset => "journal_reset",
        LibraryRecoveryAuthorityReason::JournalTrim => "journal_trim",
        LibraryRecoveryAuthorityReason::JournalReconstructionFailure => {
            "journal_reconstruction_failure"
        }
        LibraryRecoveryAuthorityReason::ContainmentFailure => "containment_failure",
        LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure => "broker_after_current_failure",
        LibraryRecoveryAuthorityReason::WatcherUncoveredGap => "watcher_uncovered_gap",
    }
}

fn parse_recovery_authority_reason(
    value: &str,
) -> Result<LibraryRecoveryAuthorityReason, ScanError> {
    match value {
        "existing_root_baseline" => Ok(LibraryRecoveryAuthorityReason::ExistingRootBaseline),
        "first_import_boundary" => Ok(LibraryRecoveryAuthorityReason::FirstImportBoundary),
        "journal_gap" => Ok(LibraryRecoveryAuthorityReason::JournalGap),
        "journal_reset" => Ok(LibraryRecoveryAuthorityReason::JournalReset),
        "journal_trim" => Ok(LibraryRecoveryAuthorityReason::JournalTrim),
        "journal_reconstruction_failure" => {
            Ok(LibraryRecoveryAuthorityReason::JournalReconstructionFailure)
        }
        "containment_failure" => Ok(LibraryRecoveryAuthorityReason::ContainmentFailure),
        "broker_after_current_failure" => {
            Ok(LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure)
        }
        "watcher_uncovered_gap" => Ok(LibraryRecoveryAuthorityReason::WatcherUncoveredGap),
        _ => Err(ScanError::new(
            "metadata_inventory_recovery_authority_corrupt",
            "The metadata recovery reason is invalid",
        )),
    }
}

fn recovery_authority_database_error(error: rusqlite::Error) -> ScanError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(ref failure, _)
            if failure.code == ErrorCode::ConstraintViolation
    ) {
        ScanError::new(
            "metadata_inventory_recovery_authority_rejected",
            "The catalog rejected recovery work without matching P2 authority",
        )
    } else {
        database_error(error)
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
    let request_authority_change_id = load_metadata_inventory_authority_owner_for_run(
        transaction,
        &request.run_id,
        &request.root_id,
        request.root_generation,
    )?;
    let unfinished_baseline_change_id = transaction
        .query_row(
            "SELECT change_id
             FROM library_persistent_journal_baselines
             WHERE root_id = ?1 AND root_generation = ?2
               AND phase <> 'completed'",
            params![
                request.root_id,
                sqlite_integer(
                    request.root_generation.value(),
                    "metadata inventory root generation",
                )?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?
        .map(|change_id| {
            sqlite_unsigned(change_id, "metadata inventory baseline change ID").map(|_| change_id)
        })
        .transpose()?;
    if unfinished_baseline_change_id.is_some()
        && unfinished_baseline_change_id != request_authority_change_id
    {
        return Err(ScanError::new(
            "metadata_inventory_baseline_authority_conflict",
            "The unfinished journal baseline belongs to a different recovery authority",
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
        if active_generation == request_generation {
            let active_authority_change_id = load_metadata_inventory_authority_owner_for_run(
                transaction,
                &active_id,
                &request.root_id,
                request.root_generation,
            )?;
            if active_authority_change_id.is_some()
                && active_authority_change_id != request_authority_change_id
            {
                return Err(ScanError::new(
                    "metadata_inventory_active_authority_conflict",
                    "A different recovery authority still owns the active metadata inventory",
                ));
            }
            if unfinished_baseline_change_id.is_some()
                && active_authority_change_id.is_some()
                && unfinished_baseline_change_id != active_authority_change_id
            {
                return Err(ScanError::new(
                    "metadata_inventory_baseline_authority_conflict",
                    "The active metadata inventory does not own the unfinished journal baseline",
                ));
            }
        }
        transaction
            .execute(
                "DELETE FROM library_metadata_inventory_spools WHERE run_id = ?1",
                [&active_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'superseded',
                     absence_authority = 0,
                     last_issue_code = 'metadata_inventory_newer_epoch',
                     last_issue_message = 'A newer metadata inventory superseded this run',
                     updated_unix_ms = ?2
                 WHERE id = ?1 AND status IN ('running', 'comparing')",
                params![active_id, request.started_unix_ms],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_frontier
                 SET state = 'completed', resume_after_relative_path = NULL,
                     enumerated_entry_count = 0, updated_unix_ms = ?2
                 WHERE run_id = ?1",
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

fn load_metadata_inventory_authority_owner_for_run(
    transaction: &Transaction<'_>,
    run_id: &str,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<i64>, ScanError> {
    let root_generation = sqlite_integer(
        root_generation.value(),
        "metadata inventory authority root generation",
    )?;
    let authority = transaction
        .query_row(
            "SELECT authority.change_id, authority.root_id, authority.root_generation,
                    authority.retired_unix_ms, queue.root_id, queue.root_generation,
                    queue.status,
                    (SELECT COUNT(*) FROM library_change_queue_lanes AS lanes
                     WHERE lanes.change_id = authority.change_id),
                    (SELECT COUNT(*) FROM library_change_queue_lanes AS lanes
                     WHERE lanes.change_id = authority.change_id
                       AND lanes.lane = 'p2_recovery')
             FROM library_recovery_authorities AS authority
             LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
             WHERE authority.run_id = ?1",
            [run_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((
        change_id,
        authority_root_id,
        authority_root_generation,
        retired_unix_ms,
        queue_root_id,
        queue_root_generation,
        queue_status,
        lane_count,
        recovery_lane_count,
    )) = authority
    else {
        return Ok(None);
    };
    sqlite_unsigned(change_id, "metadata inventory authority change ID")?;
    if authority_root_id != root_id
        || authority_root_generation != root_generation
        || retired_unix_ms.is_some()
        || queue_root_id.as_deref() != Some(root_id)
        || queue_root_generation != Some(root_generation)
        || !matches!(
            queue_status.as_deref(),
            Some("pending" | "leased" | "retry_wait")
        )
        || lane_count != 1
        || recovery_lane_count != 1
    {
        return Err(ScanError::new(
            "metadata_inventory_recovery_authority_corrupt",
            "The metadata recovery authority does not own valid unresolved P2 work",
        ));
    }
    Ok(Some(change_id))
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

fn load_metadata_inventory_frontier(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<Vec<MetadataInventoryFrontierEntry>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT ordinal, relative_directory, state, directory_identity_scheme,
                    directory_identity_value, resume_after_relative_path,
                    enumerated_entry_count
             FROM library_metadata_inventory_frontier
             WHERE run_id = ?1
             ORDER BY ordinal",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(database_error)?;
    let mut frontier = Vec::new();
    for row in rows {
        let (
            ordinal,
            relative_directory,
            state,
            identity_scheme,
            identity_value,
            resume_after_relative_path,
            enumerated_entry_count,
        ) = row.map_err(database_error)?;
        let ordinal = sqlite_unsigned(ordinal, "metadata inventory frontier ordinal")?;
        if ordinal != frontier.len() as u64 {
            return Err(frontier_corrupt());
        }
        let state = match state.as_str() {
            "pending" => MetadataInventoryFrontierState::Pending,
            "enumerating" => MetadataInventoryFrontierState::Enumerating,
            "completed" => MetadataInventoryFrontierState::Completed,
            _ => return Err(frontier_corrupt()),
        };
        let directory_identity = match (identity_scheme, identity_value) {
            (Some(scheme), Some(value)) => Some(FileIdentityEvidence { scheme, value }),
            (None, None) => None,
            _ => return Err(frontier_corrupt()),
        };
        frontier.push(MetadataInventoryFrontierEntry {
            ordinal,
            relative_directory,
            state,
            directory_identity,
            resume_after_relative_path,
            enumerated_entry_count: sqlite_unsigned(
                enumerated_entry_count,
                "metadata inventory frontier entry count",
            )?,
        });
    }
    Ok(frontier)
}

fn replace_metadata_inventory_frontier(
    transaction: &Transaction<'_>,
    run_id: &str,
    frontier: &[MetadataInventoryFrontierEntry],
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM library_metadata_inventory_frontier WHERE run_id = ?1",
            [run_id],
        )
        .map_err(database_error)?;
    for entry in frontier {
        let state = match entry.state {
            MetadataInventoryFrontierState::Pending => "pending",
            MetadataInventoryFrontierState::Enumerating => "enumerating",
            MetadataInventoryFrontierState::Completed => "completed",
        };
        let (identity_scheme, identity_value) =
            entry
                .directory_identity
                .as_ref()
                .map_or((None, None), |identity| {
                    (
                        Some(identity.scheme.as_str()),
                        Some(identity.value.as_str()),
                    )
                });
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_frontier(
                   run_id, ordinal, relative_directory, state,
                   directory_identity_scheme, directory_identity_value,
                   resume_after_relative_path, enumerated_entry_count, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    run_id,
                    sqlite_integer(entry.ordinal, "metadata inventory frontier ordinal")?,
                    entry.relative_directory,
                    state,
                    identity_scheme,
                    identity_value,
                    entry.resume_after_relative_path,
                    sqlite_integer(
                        entry.enumerated_entry_count,
                        "metadata inventory frontier entry count",
                    )?,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn frontier_corrupt() -> ScanError {
    ScanError::new(
        "catalog_metadata_inventory_frontier_invalid",
        "The stored metadata inventory frontier is invalid",
    )
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
    let Some(stored) = stored else {
        return Ok(None);
    };
    let mut run = stored.into_domain()?;
    run.frontier = load_metadata_inventory_frontier(connection, run_id)?;
    Ok(Some(run))
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
            frontier: Vec::new(),
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
    if page.frontier.is_empty() || page.frontier.len() > 1_025 {
        return Err(frontier_corrupt());
    }
    let mut saw_pending = false;
    for (ordinal, entry) in page.frontier.iter().enumerate() {
        if entry.ordinal != ordinal as u64
            || saw_pending
            || entry.directory_identity.as_ref().is_some_and(|identity| {
                identity.scheme.is_empty()
                    || identity.scheme.len() > 128
                    || identity.value.is_empty()
                    || identity.value.len() > 512
            })
        {
            return Err(frontier_corrupt());
        }
        validate_relative_path(&entry.relative_directory, true)?;
        if let Some(cursor) = entry.resume_after_relative_path.as_deref() {
            validate_relative_path(cursor, false)?;
        }
        match entry.state {
            MetadataInventoryFrontierState::Pending => {
                if entry.resume_after_relative_path.is_some()
                    || entry.enumerated_entry_count != 0
                    || ordinal + 1 != page.frontier.len()
                {
                    return Err(frontier_corrupt());
                }
                saw_pending = true;
            }
            MetadataInventoryFrontierState::Enumerating => {
                if page.is_complete {
                    return Err(frontier_corrupt());
                }
            }
            MetadataInventoryFrontierState::Completed => {
                if !page.is_complete
                    || page.frontier.len() != 1
                    || entry.resume_after_relative_path.is_some()
                    || entry.enumerated_entry_count != 0
                {
                    return Err(frontier_corrupt());
                }
            }
        }
    }
    if page.is_complete
        != (page.frontier.len() == 1
            && page.frontier[0].state == MetadataInventoryFrontierState::Completed)
    {
        return Err(frontier_corrupt());
    }
    Ok(())
}

fn insert_metadata_inventory_page_entries(
    transaction: &Transaction<'_>,
    run: &MetadataInventoryRun,
    page: &MetadataInventoryPage,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    let mut paths = BTreeSet::new();
    for entry in &page.entries {
        validate_entry(&run.request.scope, entry)?;
        if !paths.insert(entry.relative_path.as_str()) {
            return Err(ScanError::new(
                "metadata_inventory_page_duplicate",
                "A metadata inventory page contains a duplicate relative path",
            ));
        }
    }
    let page_index = sqlite_integer(page.page_index, "metadata inventory page index")?;
    for entries in page
        .entries
        .chunks(METADATA_ENTRY_INSERT_ROWS_PER_STATEMENT)
    {
        let mut sql = String::from(
            "INSERT INTO library_metadata_inventory_entries(
               run_id, relative_path, entry_kind, file_size, modified_unix_ms,
               file_identity_scheme, file_identity_value, placeholder_state,
               is_reparse_point, staged_page_index, staged_unix_ms
             ) VALUES ",
        );
        let mut parameters = Vec::with_capacity(entries.len().saturating_mul(11));
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                sql.push(',');
            }
            sql.push_str("(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
            let (entry_kind, file_size, identity_scheme, identity_value, placeholder_state) =
                entry_parts(entry)?;
            parameters.extend([
                Value::Text(run.request.run_id.clone()),
                Value::Text(entry.relative_path.clone()),
                Value::Text(entry_kind.to_owned()),
                file_size.map_or(Value::Null, Value::Integer),
                Value::Integer(entry.modified_unix_ms),
                identity_scheme.map_or(Value::Null, |value| Value::Text(value.to_owned())),
                identity_value.map_or(Value::Null, |value| Value::Text(value.to_owned())),
                Value::Text(placeholder_state.to_owned()),
                Value::Integer(i64::from(entry.is_reparse_point)),
                Value::Integer(page_index),
                Value::Integer(updated_unix_ms),
            ]);
        }
        transaction
            .prepare_cached(&sql)
            .map_err(database_error)?
            .execute(params_from_iter(parameters.iter()))
            .map_err(metadata_inventory_entry_write_error)?;
    }
    Ok(())
}

fn metadata_inventory_entry_write_error(error: rusqlite::Error) -> ScanError {
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

use rusqlite::{ErrorCode, Transaction, params};

use crate::domain::{
    FileIdentityEvidence, LibraryChangeLane, MetadataInventoryEntry, MetadataInventoryRun,
    MetadataInventoryScope, ScanError,
};

use super::spool_execution::SpoolState;
use super::{
    MetadataInventorySpoolExecution, SqliteCatalog, database_error, entry_parts, sqlite_integer,
    validate_entry,
};

impl SqliteCatalog {
    pub(crate) fn begin_metadata_inventory_spool_directory(
        &mut self,
        execution: &MetadataInventorySpoolExecution,
        relative_directory: &str,
        identity: &FileIdentityEvidence,
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let run_id = execution.run_id();
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        require_running_spool_run(&transaction, execution, relative_directory, "pending")?;
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
        execution: &MetadataInventorySpoolExecution,
        relative_directory: &str,
        entries: &[MetadataInventoryEntry],
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let run_id = execution.run_id();
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run =
            require_running_spool_run(&transaction, execution, relative_directory, "enumerating")?;
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
        execution: &MetadataInventorySpoolExecution,
        relative_directory: &str,
        identity: &FileIdentityEvidence,
        final_entries: &[MetadataInventoryEntry],
        updated_unix_ms: i64,
    ) -> Result<(), ScanError> {
        let run_id = execution.run_id();
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run =
            require_running_spool_run(&transaction, execution, relative_directory, "enumerating")?;
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
                       WHERE run_id = ?1 AND state IN ('pending', 'enumerating', 'resetting')
                     ) THEN 'enumerating' ELSE 'ready' END,
                     updated_unix_ms = ?2
                 WHERE run_id = ?1",
                params![run_id, updated_unix_ms],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }
}

fn require_running_spool_run(
    transaction: &Transaction<'_>,
    execution: &MetadataInventorySpoolExecution,
    relative_directory: &str,
    directory_state: &str,
) -> Result<MetadataInventoryRun, ScanError> {
    let (run, state) = execution.require_spool(transaction)?;
    let is_active = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM library_metadata_inventory_spool_directories
           WHERE run_id = ?1 AND relative_directory = ?2 AND state = ?3
         )",
            params![execution.run_id(), relative_directory, directory_state],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if state != SpoolState::Enumerating || !is_active {
        return Err(ScanError::new(
            "metadata_inventory_spool_authority_mismatch",
            "The source directory no longer belongs to active recovery work",
        ));
    }
    Ok(run)
}

pub(super) fn insert_spool_entries(
    transaction: &Transaction<'_>,
    scope: &MetadataInventoryScope,
    run_id: &str,
    relative_directory: Option<&str>,
    entries: &[MetadataInventoryEntry],
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    for entry in entries {
        validate_entry(scope, entry)?;
        let (entry_kind, file_size, identity_scheme, identity_value, placeholder_state, revision) =
            entry_parts(entry)?;
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spool_entries(
                   run_id, directory_relative_path, relative_path, entry_kind, file_size,
                   modified_unix_ms, file_identity_scheme, file_identity_value,
                   placeholder_state, is_reparse_point, staged_unix_ms, source_revision_token
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                    revision,
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

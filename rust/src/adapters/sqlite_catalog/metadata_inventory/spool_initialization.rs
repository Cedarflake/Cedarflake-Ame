use rusqlite::{OptionalExtension, params};

use crate::domain::{
    FileIdentityEvidence, LeasedLibraryChange, LibraryChangeLane, MetadataInventoryEntry,
    MetadataInventoryRun, MetadataInventoryRunStatus, ScanError,
};

#[cfg(test)]
use super::run_before_metadata_inventory_spool_commit_hook;
use super::spool_write::insert_spool_entries;
use super::{
    MetadataInventorySpoolExecution, SqliteCatalog, database_error, scope_parts, sqlite_integer,
};

impl SqliteCatalog {
    pub(crate) fn initialize_metadata_inventory_spool(
        &mut self,
        run: &MetadataInventoryRun,
        authority: &LeasedLibraryChange,
        root_identity: &FileIdentityEvidence,
        initial_entry: Option<&MetadataInventoryEntry>,
        initial_directory: Option<&str>,
        updated_unix_ms: i64,
    ) -> Result<MetadataInventorySpoolExecution, ScanError> {
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
        let execution = MetadataInventorySpoolExecution::capture(run, authority);
        execution.require_current_run(&transaction)?;
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
            return Ok(execution);
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
        transaction.commit().map_err(database_error)?;
        Ok(execution)
    }
}

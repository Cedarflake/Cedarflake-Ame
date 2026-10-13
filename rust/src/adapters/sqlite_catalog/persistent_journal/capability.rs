use rusqlite::{Transaction, params};

use crate::domain::{
    LibraryChangeLane, LibraryRootGeneration, PersistentJournalCapability, ScanError,
};

use super::super::{SqliteCatalog, database_error, sqlite_integer};
use super::{capability_state_text, continuity_state_text, failure_columns};

pub(super) fn require_running_first_import(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
    scan_id: &str,
) -> Result<(), ScanError> {
    let owns_import = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM scan_runs AS scan
           JOIN library_roots AS root ON root.id = scan.root_id
           JOIN library_change_root_state AS state ON state.root_id = root.id
           WHERE scan.id = ?1 AND scan.root_id = ?2
             AND scan.root_generation_at_start = ?3
             AND scan.scan_owner = 'foreground' AND scan.status = 'running'
             AND root.active_scan_id IS NULL
             AND state.generation = ?3 AND state.is_active = 1
         )",
            params![
                scan_id,
                root_id,
                sqlite_integer(generation.value(), "root generation")?
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !owns_import {
        return Err(ScanError::new(
            "persistent_journal_first_import_inactive",
            "The journal opening no longer belongs to the running first import",
        ));
    }
    Ok(())
}

impl SqliteCatalog {
    pub(crate) fn save_first_import_journal_capability<Permit>(
        &mut self,
        capability: &PersistentJournalCapability,
        scan_id: &str,
        acquire: impl FnOnce() -> Result<Permit, ScanError>,
    ) -> Result<(), ScanError> {
        self.save_journal_capability(capability, Some(scan_id), acquire)
    }

    pub(super) fn save_journal_capability<Permit>(
        &mut self,
        capability: &PersistentJournalCapability,
        first_import_scan_id: Option<&str>,
        acquire: impl FnOnce() -> Result<Permit, ScanError>,
    ) -> Result<(), ScanError> {
        capability.validate()?;
        let (failure_code, failure_message) = failure_columns(capability.failure.as_ref());
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Journal)?;
        let _permit = acquire()?;
        if let Some(scan_id) = first_import_scan_id {
            require_running_first_import(
                &transaction,
                &capability.root_id,
                capability.root_generation,
                scan_id,
            )?;
        }
        let updated = transaction
            .execute(
                "WITH progress AS MATERIALIZED (
               SELECT state.continuity_state, state.last_failure_code, state.last_failure_message
               FROM library_persistent_journal_root_state AS state
               WHERE state.root_id = ?8 AND state.root_generation = ?9
                 AND state.capability_state = 'supported' AND ?3 = 'supported'
                 AND state.protocol_version = ?1 AND state.contract_version = ?2
                 AND (
                   EXISTS (SELECT 1 FROM library_persistent_journal_checkpoints AS checkpoint
                     WHERE checkpoint.root_id = state.root_id
                       AND checkpoint.root_generation = state.root_generation)
                   OR EXISTS (
                     SELECT 1 FROM library_persistent_journal_baselines AS baseline
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = baseline.change_id
                     WHERE baseline.root_id = state.root_id
                       AND baseline.root_generation = state.root_generation
                       AND baseline.phase <> 'completed' AND authority.retired_unix_ms IS NULL
                   )
                 )
             )
             UPDATE library_persistent_journal_root_state
             SET protocol_version = ?1, contract_version = ?2,
                 capability_state = ?3,
                 continuity_state = COALESCE((SELECT continuity_state FROM progress), ?4),
                 last_failure_code = CASE WHEN EXISTS(SELECT 1 FROM progress)
                   THEN (SELECT last_failure_code FROM progress) ELSE ?5 END,
                 last_failure_message = CASE WHEN EXISTS(SELECT 1 FROM progress)
                   THEN (SELECT last_failure_message FROM progress) ELSE ?6 END,
                 updated_unix_ms = MAX(updated_unix_ms, ?7)
             WHERE root_id = ?8 AND root_generation = ?9
               AND EXISTS(
                 SELECT 1 FROM library_change_root_state AS active
                 WHERE active.root_id = ?8 AND active.generation = ?9 AND active.is_active = 1
               )",
                params![
                    i64::from(capability.protocol_version),
                    i64::from(capability.contract_version),
                    capability_state_text(capability.state),
                    continuity_state_text(capability.continuity),
                    failure_code,
                    failure_message,
                    capability.updated_unix_ms,
                    capability.root_id,
                    sqlite_integer(capability.root_generation.value(), "root generation")?,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_root_authority_stale",
                "The persistent journal capability no longer owns the active root generation",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(())
    }
}

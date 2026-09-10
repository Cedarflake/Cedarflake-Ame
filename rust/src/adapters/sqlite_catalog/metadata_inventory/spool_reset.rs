use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{LibraryChangeLane, ScanError};

use super::spool_execution::with_read_spool;
use super::{
    MetadataInventorySpoolExecution, SqliteCatalog, database_error, validate_window_limit,
};

impl SqliteCatalog {
    pub(crate) fn reset_metadata_inventory_spool_batch(
        &mut self,
        execution: &MetadataInventorySpoolExecution,
        limit: u32,
        updated_unix_ms: i64,
    ) -> Result<bool, ScanError> {
        validate_window_limit(limit)?;
        if !with_read_spool(self, execution, |connection, _, _| {
            Ok(next_resetting_directory(connection, execution)?.is_some())
        })? {
            return Ok(false);
        }
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        execution.require_spool(&transaction)?;
        let directory = next_resetting_directory(&transaction, execution)?;
        let Some(directory) = directory else {
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        };
        transaction
            .execute(
                "DELETE FROM library_metadata_inventory_spool_entries WHERE rowid IN (
               SELECT rowid FROM library_metadata_inventory_spool_entries
               WHERE run_id = ?1 AND directory_relative_path = ?2 LIMIT ?3
             )",
                params![execution.run_id(), directory, limit],
            )
            .map_err(database_error)?;
        // An interrupted directory cannot be read, appended, or declared complete until every
        // observation from its old OS enumerator has been retired. Reopen preserves this debt.
        transaction.execute(
            "UPDATE library_metadata_inventory_spool_directories
             SET state = 'pending', directory_identity_scheme = NULL, directory_identity_value = NULL,
                 source_entry_count = 0, updated_unix_ms = MAX(updated_unix_ms, ?3)
             WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'resetting'
               AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_spool_entries
                 WHERE run_id = ?1 AND directory_relative_path = ?2)",
            params![execution.run_id(), directory, updated_unix_ms],
        ).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }
}

fn next_resetting_directory(
    connection: &Connection,
    execution: &MetadataInventorySpoolExecution,
) -> Result<Option<String>, ScanError> {
    connection
        .query_row(
            "SELECT relative_directory FROM library_metadata_inventory_spool_directories
         WHERE run_id = ?1 AND state = 'resetting' ORDER BY ordinal LIMIT 1",
            [execution.run_id()],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)
}

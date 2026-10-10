use rusqlite::{OptionalExtension, params};

use crate::domain::{
    FileIdentityEvidence, MetadataInventoryFrontierEntry, MetadataInventoryFrontierState,
    MetadataInventoryPage, ScanError,
};

use super::spool_execution::{SpoolState, with_read_spool};
use super::{
    MetadataInventorySpoolExecution, SqliteCatalog, StoredEntry, database_error, stored_entry,
    validate_window_limit,
};

const METADATA_INVENTORY_SPOOL_PAGE_SQL: &str =
    "SELECT relative_path, entry_kind, file_size, modified_unix_ms,
            file_identity_scheme, file_identity_value, placeholder_state,
            is_reparse_point, source_revision_token
     FROM library_metadata_inventory_spool_entries
     WHERE run_id = ?1
       AND relative_path > COALESCE(?2, '')
     ORDER BY relative_path LIMIT ?3";

impl SqliteCatalog {
    pub(crate) fn metadata_inventory_spool_is_ready(
        &self,
        execution: &MetadataInventorySpoolExecution,
    ) -> Result<bool, ScanError> {
        with_read_spool(
            self,
            execution,
            |_, _, state| Ok(state == SpoolState::Ready),
        )
    }

    pub(crate) fn next_metadata_inventory_spool_directory(
        &self,
        execution: &MetadataInventorySpoolExecution,
    ) -> Result<Option<String>, ScanError> {
        with_read_spool(self, execution, |connection, _, _| {
            connection
                .query_row(
                    "SELECT relative_directory
                 FROM library_metadata_inventory_spool_directories
                 WHERE run_id = ?1 AND state = 'pending'
                 ORDER BY ordinal LIMIT 1",
                    [execution.run_id()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(database_error)
        })
    }

    pub(crate) fn load_metadata_inventory_spool_page(
        &self,
        execution: &MetadataInventorySpoolExecution,
        limit: u32,
    ) -> Result<MetadataInventoryPage, ScanError> {
        validate_window_limit(limit)?;
        with_read_spool(self, execution, |connection, run, state| {
            let run_id = execution.run_id();
            if state != SpoolState::Ready {
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
            let mut statement = connection
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
            let directory_identity = connection
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

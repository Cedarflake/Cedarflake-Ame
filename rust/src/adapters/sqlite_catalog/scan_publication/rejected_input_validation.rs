use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::params;

use crate::domain::{DiscoveredFile, ExpectedFileState, ScanError};

use super::super::{
    SqliteCatalog, database_error, source_revision_token, sqlite_integer, sqlite_unsigned,
    stored_file_identity, stored_source_revision,
};

static NEXT_ROSTER_ID: AtomicU64 = AtomicU64::new(1);

/// Negative inspection evidence belongs to this connection, not a retained scan checkpoint.
pub(crate) struct RejectedInputValidationRoster {
    table: String,
}

impl RejectedInputValidationRoster {
    pub(crate) fn create(catalog: &SqliteCatalog) -> Result<Self, ScanError> {
        let table = format!(
            "ame_rejected_input_validation_{}",
            NEXT_ROSTER_ID.fetch_add(1, Ordering::Relaxed)
        );
        catalog
            .connection
            .execute_batch(&format!(
                "PRAGMA temp_store = FILE; PRAGMA temp.cache_size = -2048;
             CREATE TEMP TABLE {table} (
               relative_path TEXT PRIMARY KEY, absolute_path TEXT NOT NULL,
               file_size INTEGER NOT NULL, modified_unix_ms INTEGER NOT NULL,
               identity_scheme TEXT, identity_value TEXT, source_revision TEXT
             ) WITHOUT ROWID;"
            ))
            .map_err(database_error)?;
        Ok(Self { table })
    }

    pub(crate) fn record(
        &self,
        catalog: &SqliteCatalog,
        file: &DiscoveredFile,
    ) -> Result<(), ScanError> {
        let revision = file
            .source_revision
            .as_ref()
            .map(source_revision_token)
            .transpose()?;
        catalog.connection.execute(
            &format!("INSERT INTO temp.{} VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(relative_path) DO UPDATE SET absolute_path = excluded.absolute_path,
                  file_size = excluded.file_size, modified_unix_ms = excluded.modified_unix_ms,
                  identity_scheme = excluded.identity_scheme, identity_value = excluded.identity_value,
                  source_revision = excluded.source_revision", self.table),
            params![file.relative_path, file.absolute_path, sqlite_integer(file.file_size, "rejected file size")?,
                file.modified_unix_ms, file.file_identity.as_ref().map(|identity| &identity.scheme),
                file.file_identity.as_ref().map(|identity| &identity.value), revision],
        ).map_err(database_error)?;
        Ok(())
    }

    pub(crate) fn load_window(
        &self,
        catalog: &SqliteCatalog,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, ExpectedFileState)>, ScanError> {
        let page = if after.is_some() {
            "WHERE relative_path > ?1 ORDER BY relative_path LIMIT ?2"
        } else {
            "ORDER BY relative_path LIMIT ?1"
        };
        let mut statement = catalog
            .connection
            .prepare(&format!(
                "SELECT relative_path, absolute_path, file_size, modified_unix_ms,
                    identity_scheme, identity_value, source_revision
             FROM temp.{} {page}",
                self.table
            ))
            .map_err(database_error)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        };
        let rows = match after {
            Some(after) => statement.query_map(params![after, i64::from(limit)], map_row),
            None => statement.query_map([i64::from(limit)], map_row),
        }
        .map_err(database_error)?;
        rows.map(|row| {
            let (path, absolute_path, size, modified_unix_ms, scheme, identity, revision) =
                row.map_err(database_error)?;
            Ok((
                path,
                ExpectedFileState {
                    absolute_path,
                    file_size: sqlite_unsigned(size, "rejected file size")?,
                    modified_unix_ms,
                    file_identity: stored_file_identity(scheme, identity)?,
                    source_revision: stored_source_revision(revision)?,
                },
            ))
        })
        .collect()
    }
}

#[cfg(test)]
#[path = "rejected_input_validation_tests.rs"]
mod tests;

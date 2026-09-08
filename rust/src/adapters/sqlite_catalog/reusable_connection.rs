use rusqlite::Connection;

use super::{
    SCHEMA_VERSION, SQLITE_APPLICATION_ID, ScanError, SqliteCatalog, SqliteCatalogSession,
    SqliteDatabaseIdentity, catalog_database_identity, catalog_pragma_integer,
    catalog_schema_cookie, database_error, operation_diagnostics::measure,
    stale_catalog_session_error,
};

impl SqliteCatalogSession {
    pub(crate) fn revalidate_connection(&self, catalog: &SqliteCatalog) -> Result<(), ScanError> {
        let original = &catalog.session;
        if catalog.path != self.path
            || original.path != self.path
            || original.database_identity != self.database_identity
            || original.application_id != self.application_id
            || original.user_version != self.user_version
            || original.schema_cookie != self.schema_cookie
            || !std::sync::Arc::ptr_eq(&catalog.write_admission, &self.write_admission)
        {
            return Err(stale_catalog_session_error());
        }
        if !catalog.connection.is_autocommit()
            || catalog.connection.is_busy()
            || !catalog.pending_locations.is_empty()
            || !catalog.pending_authoritative_retry_paths.is_empty()
        {
            return Err(ScanError::new(
                "catalog_connection_not_reusable",
                "The catalog connection still owns a transaction, active statement, or pending publication",
            ));
        }
        let before_identity = measure("reusable_identity_before", || {
            catalog_database_identity(&self.path)
        })?;
        if before_identity != self.database_identity {
            return Err(stale_catalog_session_error());
        }
        self.validate_connection_proof(&catalog.connection, &before_identity)
    }

    pub(super) fn validate_connection_proof(
        &self,
        connection: &Connection,
        before_identity: &SqliteDatabaseIdentity,
    ) -> Result<(), ScanError> {
        let journal_mode = measure("proof_journal_mode", || {
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .map_err(database_error)
        })?;
        let version = measure("proof_schema_info", || {
            connection
                .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(database_error)
        })?;
        let application_id = measure("proof_application_id", || {
            catalog_pragma_integer(connection, "application_id")
        })?;
        let user_version = measure("proof_user_version", || {
            catalog_pragma_integer(connection, "user_version")
        })?;
        let schema_cookie = measure("proof_schema_cookie", || catalog_schema_cookie(connection))?;
        let after_identity = measure("proof_identity_after", || {
            catalog_database_identity(&self.path)
        })?;
        if !journal_mode.eq_ignore_ascii_case("wal")
            || version != SCHEMA_VERSION
            || application_id != self.application_id
            || application_id != SQLITE_APPLICATION_ID
            || user_version != self.user_version
            || user_version != SCHEMA_VERSION
            || schema_cookie != self.schema_cookie
            || before_identity != &after_identity
        {
            return Err(stale_catalog_session_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

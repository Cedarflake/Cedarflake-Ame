use rusqlite::{Connection, config::DbConfig};

use super::{SQLITE_BUSY_TIMEOUT, ScanError, SqliteCatalog, database_error, read_retry};

mod initial_import;
pub(super) use initial_import::ImportCheckpointPolicy;

// Smaller checkpoints limit the accumulated work performed by the committing writer.
pub(super) const AUTOMATIC_CHECKPOINT_PAGES: i32 = 256;
pub(super) const INITIAL_IMPORT_CHECKPOINT_PAGES: i32 = 32_768;
const CATALOG_PAGE_CACHE_KIB: i64 = 32 * 1024;

pub(super) fn configure_catalog_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(database_error)?;
    configure_page_cache(connection)?;
    configure_automatic_checkpoint(connection, AUTOMATIC_CHECKPOINT_PAGES)
}

fn configure_page_cache(connection: &Connection) -> Result<(), ScanError> {
    // Negative cache_size values budget KiB; pages are allocated only on demand.
    connection
        .pragma_update(None, "cache_size", -CATALOG_PAGE_CACHE_KIB)
        .map_err(database_error)
}

fn configure_automatic_checkpoint(connection: &Connection, pages: i32) -> Result<(), ScanError> {
    connection
        .pragma_update(None, "wal_autocheckpoint", pages)
        .map_err(database_error)?;
    #[cfg(test)]
    super::operation_diagnostics::observe_checkpoint(connection).map_err(database_error)?;
    Ok(())
}

impl SqliteCatalog {
    pub(crate) fn retain_wal_on_close(&self) -> Result<(), ScanError> {
        // A retained runtime retires its handle without adding checkpoint work to shutdown.
        self.connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
            .map_err(database_error)?;
        Ok(())
    }
}

pub(super) fn configure_catalog_read_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(read_retry::PROTOCOL_READ_DEADLINE)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = ON;")
        .map_err(database_error)?;
    configure_page_cache(connection)
}

#[cfg(test)]
mod tests;

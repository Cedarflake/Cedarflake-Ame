use rusqlite::Connection;

use super::{SQLITE_BUSY_TIMEOUT, ScanError, database_error, read_retry};

// Smaller checkpoints limit the accumulated work performed by the committing writer.
pub(super) const AUTOMATIC_CHECKPOINT_PAGES: i32 = 256;

pub(super) fn configure_catalog_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(database_error)?;
    connection
        .pragma_update(None, "wal_autocheckpoint", AUTOMATIC_CHECKPOINT_PAGES)
        .map_err(database_error)?;
    #[cfg(test)]
    super::operation_diagnostics::observe_checkpoint(connection).map_err(database_error)?;
    Ok(())
}

pub(super) fn configure_catalog_read_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(read_retry::PROTOCOL_READ_DEADLINE)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = ON;")
        .map_err(database_error)
}

#[cfg(test)]
mod tests;

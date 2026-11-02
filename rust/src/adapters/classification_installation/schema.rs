use std::path::Path;

use rusqlite::Connection;

use crate::domain::ScanError;

use super::storage_error;

pub(super) fn open(path: &Path) -> Result<Connection, ScanError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage_error)?;
    }
    let mut connection = Connection::open(path).map_err(storage_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(storage_error)?;
    let transaction = connection.transaction().map_err(storage_error)?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    if version > 2 {
        return Err(storage_error(
            "The classification installation format is newer than supported",
        ));
    }
    if version == 0 {
        transaction
            .execute_batch(
                "CREATE TABLE installation (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                models TEXT NOT NULL CHECK(length(models) > 0),
                runtime TEXT NOT NULL CHECK(length(runtime) > 0)
            ) STRICT;",
            )
            .map_err(storage_error)?;
    }
    if version < 2 {
        transaction
            .execute_batch(
                "CREATE TABLE download_intent (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                generation INTEGER NOT NULL CHECK(generation > 0),
                package TEXT NOT NULL CHECK(length(package) BETWEEN 1 AND 128),
                pending INTEGER NOT NULL CHECK(pending IN (0, 1))
            ) STRICT;
            PRAGMA user_version = 2;",
            )
            .map_err(storage_error)?;
    }
    transaction.commit().map_err(storage_error)?;
    Ok(connection)
}

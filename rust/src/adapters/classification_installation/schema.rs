use std::path::Path;

use rusqlite::{Connection, TransactionBehavior};

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
    if schema_version(&connection)? == 3 {
        return Ok(connection);
    }
    // Recheck after acquiring write authority so simultaneous first opens share one initialization.
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let version = schema_version(&transaction)?;
    if version == 0 {
        let empty: bool = transaction
            .query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*')",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !empty {
            return Err(storage_error(
                "The model settings database has no supported format",
            ));
        }
        transaction
            .execute_batch(
                "CREATE TABLE installation (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                models TEXT NOT NULL CHECK(length(models) > 0),
                runtime TEXT NOT NULL CHECK(length(runtime) > 0)
            ) STRICT;
            CREATE TABLE download_intent (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                generation INTEGER NOT NULL CHECK(generation > 0),
                package TEXT NOT NULL CHECK(length(package) BETWEEN 1 AND 128),
                pending INTEGER NOT NULL CHECK(pending IN (0, 1))
            ) STRICT;
            CREATE TABLE acceleration (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                gpu_enabled INTEGER NOT NULL CHECK(gpu_enabled IN (0, 1))
            ) STRICT;
            INSERT INTO acceleration VALUES (1, 1);
            PRAGMA user_version = 3;",
            )
            .map_err(storage_error)?;
    }
    transaction.commit().map_err(storage_error)?;
    Ok(connection)
}

fn schema_version(connection: &Connection) -> Result<u32, ScanError> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    if version != 0 && version != 3 {
        return Err(storage_error(
            "This version of Ame does not support the model settings format",
        ));
    }
    Ok(version)
}

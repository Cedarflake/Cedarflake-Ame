use rusqlite::{Connection, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::storage_versions::SETTINGS_SCHEMA_VERSION;

use super::settings_error;

pub(super) fn initialize(connection: &mut Connection) -> Result<(), ScanError> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(settings_error)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(settings_error)?;
    let empty: bool = transaction
        .query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*')",
            [],
            |row| row.get(0),
        )
        .map_err(settings_error)?;
    if empty {
        transaction
            .execute_batch(
                "CREATE TABLE storage_settings_schema (version INTEGER NOT NULL);
                 INSERT INTO storage_settings_schema VALUES (2);
                 CREATE TABLE storage_settings (
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   catalog_path TEXT NOT NULL,
                   preview_root TEXT NOT NULL,
                   preview_budget_bytes INTEGER NOT NULL CHECK(preview_budget_bytes > 0)
                 );
                 CREATE TABLE preview_root_ownership (
                   preview_root TEXT PRIMARY KEY,
                   state TEXT NOT NULL CHECK(state IN ('pending', 'retired')),
                   recorded_unix_ms INTEGER NOT NULL
                 );",
            )
            .map_err(settings_error)?;
    }
    let (count, version): (i64, Option<i64>) = transaction
        .query_row(
            "SELECT COUNT(*), MIN(version) FROM storage_settings_schema",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(settings_error)?;
    if count != 1 || version != Some(SETTINGS_SCHEMA_VERSION) {
        return Err(ScanError::new(
            "storage_settings_schema_unsupported",
            format!(
                "Expected one settings schema {SETTINGS_SCHEMA_VERSION}, found {count} rows with version {version:?}"
            ),
        ));
    }
    transaction.commit().map_err(settings_error)
}

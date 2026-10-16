use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_object_sql_matches};

// User choices deliberately have no cascading relationship to derived evidence or locations.
const TABLE: &str = "CREATE TABLE exact_duplicate_decisions (
    scheme TEXT NOT NULL,
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
    preferred_location_id TEXT,
    preferred_asset_id TEXT,
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    updated_unix_ms INTEGER NOT NULL,
    CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)),
    PRIMARY KEY(scheme, digest, byte_length)
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 41)?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'exact_duplicate_decisions')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        validate(transaction)?;
    } else {
        transaction.execute_batch(TABLE).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 42; PRAGMA user_version = 42;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 42)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "table", "exact_duplicate_decisions", TABLE)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_duplicate_decisions_contract_invalid",
        "Exact duplicate choices require their durable content-keyed table",
    ))
}

#[cfg(test)]
mod tests;

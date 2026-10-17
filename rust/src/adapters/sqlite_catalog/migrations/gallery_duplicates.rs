use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_object_sql_matches};

const TABLE: &str = "CREATE TABLE gallery_content_fingerprints (
    source_generation INTEGER NOT NULL,
    scheme TEXT NOT NULL,
    PRIMARY KEY(source_generation, scheme),
    FOREIGN KEY(source_generation, scheme)
      REFERENCES content_fingerprints(source_generation, scheme)
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 40)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    let published = transaction
        .execute(
            "INSERT INTO gallery_content_fingerprints
         SELECT source_generation, scheme FROM content_fingerprints",
            [],
        )
        .map_err(database_error)?;
    if published > 0 {
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 41; PRAGMA user_version = 41;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 41)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "table", "gallery_content_fingerprints", TABLE)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_gallery_duplicates_contract_invalid",
        "Gallery grouping requires an atomically published fingerprint projection",
    ))
}

#[cfg(test)]
mod tests;

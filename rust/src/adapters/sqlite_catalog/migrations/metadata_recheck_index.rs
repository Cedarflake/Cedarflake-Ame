use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;
use super::{current_schema, database_error};

const INDEX_NAME: &str = "asset_locations_pending_metadata";
const INDEX_DDL: &str = "CREATE INDEX asset_locations_pending_metadata
    ON asset_locations(root_id, scan_id, relative_path)
    WHERE metadata_engine_id = 'ame-invalidated-media-metadata'";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 43)?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [INDEX_NAME],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !exists {
        transaction
            .execute_batch(INDEX_DDL)
            .map_err(database_error)?;
    }
    validate(transaction)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 44; PRAGMA user_version = 44;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 44)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "index", INDEX_NAME, INDEX_DDL)? {
        return Err(ScanError::new(
            "catalog_metadata_recheck_index_invalid",
            "Pending metadata recovery requires its bounded candidate index",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn downgrade_to_v43_for_test(connection: &Connection) {
    super::retained_gallery_membership::downgrade_to_v44_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() == 44 {
        connection
            .execute_batch(
                "DROP INDEX asset_locations_pending_metadata;
             UPDATE schema_info SET version = 43; PRAGMA user_version = 43;",
            )
            .unwrap();
    }
}

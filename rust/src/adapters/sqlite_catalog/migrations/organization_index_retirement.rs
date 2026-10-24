use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::organization_reconciliation::{GENERATION_INDEX, ROOT_GENERATION_INDEX};
use super::{current_schema, database_error, schema_shape};

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 49)?;
    // Exact-content reads already index source_generation. Keep the separate root/scan keyset
    // index, but do not pay another per-location index for the same generation lookup.
    transaction
        .execute_batch(
            "DROP INDEX asset_locations_organization_generation;
             UPDATE schema_info SET version = 50;
             PRAGMA user_version = 50;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 50)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let retired_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [GENERATION_INDEX.0],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let root_index_matches = schema_shape::schema_object_sql_matches(
        connection,
        "index",
        ROOT_GENERATION_INDEX.0,
        ROOT_GENERATION_INDEX.1,
    )?;
    if retired_exists || !root_index_matches {
        return Err(ScanError::new(
            "catalog_organization_index_contract_invalid",
            "Organization publication requires a root keyset index and reuses the exact source index",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_to_v49_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 50 {
        let transaction = connection.unchecked_transaction().unwrap();
        transaction.execute_batch(GENERATION_INDEX.1).unwrap();
        transaction
            .execute_batch("UPDATE schema_info SET version = 49; PRAGMA user_version = 49;")
            .unwrap();
        transaction.commit().unwrap();
    }
}

#[cfg(test)]
mod tests;

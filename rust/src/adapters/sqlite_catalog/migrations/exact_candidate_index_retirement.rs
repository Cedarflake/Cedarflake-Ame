use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error};

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 42)?;
    // Candidate keyset traversal already uses asset_locations_location_id. The wider
    // index cannot cover source revalidation and duplicates its per-location storage.
    transaction
        .execute_batch(
            "DROP INDEX asset_locations_exact_cursor;
             UPDATE schema_info SET version = 43;
             PRAGMA user_version = 43;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 43)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE name = 'asset_locations_exact_cursor')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        return Err(ScanError::new(
            "catalog_exact_candidate_index_contract_invalid",
            "Exact candidates must reuse the catalog location cursor index",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::adapters::sqlite_catalog::migrations) fn downgrade_to_v42_for_test(
    connection: &Connection,
) {
    super::metadata_recheck_index::downgrade_to_v43_for_test(connection);
    let transaction = connection.unchecked_transaction().unwrap();
    current_schema::validate_schema_version(&transaction, 43).unwrap();
    transaction
        .execute_batch(super::exact_duplicates::LEGACY_CURSOR_INDEX.2)
        .unwrap();
    transaction
        .execute_batch("UPDATE schema_info SET version = 42; PRAGMA user_version = 42;")
        .unwrap();
    current_schema::validate_schema_version(&transaction, 42).unwrap();
    transaction.commit().unwrap();
}

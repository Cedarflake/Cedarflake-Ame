use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const NAME: &str = "exact_duplicate_preferred_location";
const INDEX: &str = "CREATE INDEX exact_duplicate_preferred_location
    ON exact_duplicate_decisions(preferred_location_id, preferred_asset_id)
    WHERE preferred_location_id IS NOT NULL";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 58)?;
    transaction.execute_batch(INDEX).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 59; PRAGMA user_version = 59;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 59)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_shape::schema_object_sql_matches(connection, "index", NAME, INDEX)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_preferred_copy_index_invalid",
        "Preferred-copy reads require their location and asset lookup index",
    ))
}

#[cfg(test)]
pub(super) fn downgrade_to_v58_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 59 {
        connection
            .execute_batch(
                "DROP INDEX exact_duplicate_preferred_location;
                 UPDATE schema_info SET version = 58; PRAGMA user_version = 58;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

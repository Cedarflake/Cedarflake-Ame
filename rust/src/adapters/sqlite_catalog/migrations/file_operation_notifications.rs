use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::invalid;

use super::{current_schema, database_error, schema_shape};

const INDEX: &str = "CREATE INDEX file_operation_removal_source ON file_operation_items(
    json_extract(authorization, '$.catalog.location_id'),
    json_extract(authorization, '$.catalog.source_generation'))
    WHERE phase IN ('executing', 'native_completed', 'uncertain')
      AND json_extract(authorization, '$.action') IN ('recycle', 'delete_permanently')";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 79)?;
    transaction.execute_batch(INDEX).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 80; PRAGMA user_version = 80;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 80)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "index",
        "file_operation_removal_source",
        INDEX,
    )? {
        return Err(invalid("The operation notification index is invalid"));
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::adapters::sqlite_catalog) fn downgrade_for_test(connection: &Connection) {
    super::file_operation_exports::downgrade_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 80 {
        connection
            .execute_batch(
                "DROP INDEX file_operation_removal_source;
                 UPDATE schema_info SET version = 79; PRAGMA user_version = 79;",
            )
            .unwrap();
    }
}

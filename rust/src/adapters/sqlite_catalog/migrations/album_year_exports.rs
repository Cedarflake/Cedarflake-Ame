use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, file_operation_exports};

pub(super) fn batches_schema() -> String {
    file_operation_exports::BATCHES.replace("'source'", "'source', 'album_year'")
}

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 81)?;
    replace_tables(transaction, &batches_schema())?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 82; PRAGMA user_version = 82;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 82)
}

fn replace_tables(connection: &Connection, batches: &str) -> Result<(), ScanError> {
    // Keep history on disk in the same transaction while widening its rule constraint.
    connection
        .execute_batch(
            "CREATE TABLE file_operation_exports_upgrade AS SELECT * FROM file_operation_exports;
             CREATE TABLE file_operation_skips_upgrade AS SELECT * FROM file_operation_export_skips;
             DROP TABLE file_operation_export_skips;
             DROP TABLE file_operation_exports;",
        )
        .map_err(database_error)?;
    for (_, name, sql) in file_operation_exports::OBJECTS {
        let sql = if *name == "file_operation_exports" {
            batches
        } else {
            sql
        };
        connection.execute_batch(sql).map_err(database_error)?;
    }
    connection
        .execute_batch(
            "INSERT INTO file_operation_exports SELECT * FROM file_operation_exports_upgrade;
             INSERT INTO file_operation_export_skips SELECT * FROM file_operation_skips_upgrade;
             DROP TABLE file_operation_skips_upgrade;
             DROP TABLE file_operation_exports_upgrade;",
        )
        .map_err(database_error)
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    super::organization_group_filters::downgrade_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() == 82 {
        let transaction = connection.unchecked_transaction().unwrap();
        replace_tables(&transaction, file_operation_exports::BATCHES).unwrap();
        transaction
            .execute_batch("UPDATE schema_info SET version = 81; PRAGMA user_version = 81;")
            .unwrap();
        transaction.commit().unwrap();
    }
}

#[cfg(test)]
mod tests;

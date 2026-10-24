use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const INDEX: (&str, &str) = (
    "organization_bindings_native_version",
    "CREATE INDEX organization_bindings_native_version ON organization_bindings(
       file_identity_scheme, file_identity_value, source_revision_token, byte_length, record_id)",
);

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 52)?;
    transaction.execute_batch(INDEX.1).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 53; PRAGMA user_version = 53;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 53)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(connection, "index", INDEX.0, INDEX.1)? {
        return Err(ScanError::new(
            "catalog_organization_native_recovery_invalid",
            "Organization recovery requires the native-version evidence index",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_to_v52_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 53 {
        connection
            .execute_batch(
                "DROP INDEX organization_bindings_native_version;
             UPDATE schema_info SET version = 52; PRAGMA user_version = 52;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

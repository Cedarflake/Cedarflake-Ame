use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error};

mod confirmation;
mod schema;
mod validation;

pub(super) use confirmation::{
    migrate as migrate_confirmation, migrate_transaction as migrate_confirmation_transaction,
};
pub(super) use validation::{validate, validate_rows};

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 73)?;
    for (_, _, ddl) in schema::OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO similarity_dispatch VALUES (1, NULL, NULL, 0)",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 74; PRAGMA user_version = 74;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 74)
}

#[cfg(test)]
pub(super) fn downgrade_to_v73_for_test(connection: &Connection) {
    confirmation::downgrade_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() != 74 {
        return;
    }
    for (kind, name, _) in schema::OBJECTS.iter().rev() {
        connection
            .execute_batch(&format!("DROP {kind} {name}"))
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 73; PRAGMA user_version = 73;")
        .unwrap();
}

#[cfg(test)]
mod tests;

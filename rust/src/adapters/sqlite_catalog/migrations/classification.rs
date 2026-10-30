use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

mod schema;

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 64)?;
    for (_, _, statement) in schema::OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO classification_state(singleton, revision) VALUES (1, 0);
         UPDATE schema_info SET version = 65; PRAGMA user_version = 65;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 65)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let present: bool = connection
        .query_row(
            "SELECT COUNT(*) = 1 FROM classification_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !present {
        return Err(invalid());
    }
    for (kind, name, _) in schema::OBJECTS {
        if *kind != "table" {
            continue;
        }
        let failed: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if failed {
            return Err(invalid());
        }
    }
    super::super::classification::validation::catalog(connection)
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_classification_invalid",
        "Classification records require a valid schema and complete relationships",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v64_for_test(connection: &Connection) {
    super::classification_work::downgrade_to_v65_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() != 65 {
        return;
    }
    connection
        .execute(
            "UPDATE classification_state SET active_run_id = NULL, active_rules = NULL",
            [],
        )
        .unwrap();
    for (kind, name, _) in schema::OBJECTS.iter().rev() {
        connection
            .execute_batch(&format!("DROP {} {name}", kind.to_uppercase()))
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 64; PRAGMA user_version = 64;")
        .unwrap();
}

#[cfg(test)]
mod tests;

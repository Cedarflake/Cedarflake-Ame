use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape::schema_object_sql_matches};

mod schema;

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 46)?;
    for (kind, name, ddl) in schema::OBJECTS {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if exists {
            if !schema_object_sql_matches(transaction, kind, name, ddl)? {
                return Err(invalid());
            }
        } else {
            transaction.execute_batch(ddl).map_err(database_error)?;
        }
    }
    transaction
        .execute_batch(
            "INSERT OR IGNORE INTO organization_state(singleton, revision) VALUES (1, 0);
             UPDATE schema_info SET version = 47; PRAGMA user_version = 47;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 47)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in schema::OBJECTS {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    let has_state: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_state WHERE singleton = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !has_state {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_contract_invalid",
        "Durable organization requires its versioned records, bindings and operation receipts",
    )
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, table, _) in schema::OBJECTS
        .iter()
        .filter(|(kind, _, _)| *kind == "table")
    {
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    let invalid_revision: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_records AS record, organization_state AS state
           WHERE record.organization_revision > state.revision OR record.favorite_revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operations AS operation, organization_state AS state
           WHERE operation.revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operation_changes AS change, organization_state AS state
           WHERE change.applied_revision > state.revision)", [], |row| row.get(0)
    ).map_err(database_error)?;
    if invalid_revision {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::adapters::sqlite_catalog) fn downgrade_to_v46_for_test(connection: &Connection) {
    super::organization_transitions::downgrade_to_v47_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() == 47 {
        for (kind, name, _) in schema::OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 46; PRAGMA user_version = 46;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

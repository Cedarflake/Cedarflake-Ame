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
    current_schema::validate_schema_version(transaction, 54)?;
    for (_, _, ddl) in schema::OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO organization_unassociated_records(record_id, group_id)
         SELECT member.record_id, member.group_id FROM organization_records AS record
         JOIN organization_record_groups AS member USING(record_id)
         WHERE record.unassociated_since_unix_ms IS NOT NULL;
         UPDATE schema_info SET version = 55; PRAGMA user_version = 55;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 55)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_rows: bool = connection.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM organization_records AS record
           JOIN organization_record_groups AS member USING(record_id)
           WHERE record.unassociated_since_unix_ms IS NOT NULL
             AND NOT EXISTS(SELECT 1 FROM organization_unassociated_records AS indexed
               WHERE indexed.record_id = member.record_id AND indexed.group_id = member.group_id)
         ) OR EXISTS(
           SELECT 1 FROM organization_unassociated_records AS indexed
           LEFT JOIN organization_records AS record USING(record_id)
           LEFT JOIN organization_record_groups AS member USING(record_id)
           WHERE record.unassociated_since_unix_ms IS NULL OR member.group_id IS NOT indexed.group_id
         )",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_rows {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_association_index_invalid",
        "Organization association lookup must match retained records and group ownership",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v54_for_test(connection: &Connection) {
    super::organization_history::downgrade_to_v55_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 55 {
        for (kind, name, _) in schema::OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 54; PRAGMA user_version = 54;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

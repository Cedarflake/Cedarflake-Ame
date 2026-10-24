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
    current_schema::validate_schema_version(transaction, 53)?;
    for (_, _, ddl) in schema::OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 54; PRAGMA user_version = 54;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 54)
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
    for (kind, name, _) in schema::OBJECTS {
        if *kind != "table" {
            continue;
        }
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    let invalid_ownership: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_recovery_admissions AS history
           WHERE NOT EXISTS(SELECT 1 FROM organization_recovery_admission_sources AS owner
             WHERE owner.source_generation = history.source_generation))
         OR EXISTS(SELECT 1 FROM organization_recovery_admissions AS history
           JOIN organization_admissions AS current USING(admission_id))
         OR EXISTS(SELECT 1 FROM (
           SELECT root_id FROM organization_recovery_admission_sources
           UNION SELECT root_id FROM organization_recovery_duplicate_sources
           UNION SELECT root_id FROM organization_recovery_versions
         ) AS source WHERE NOT EXISTS(SELECT 1 FROM library_roots AS current WHERE current.id = source.root_id)
           AND NOT EXISTS(SELECT 1 FROM organization_recovery_roots AS retained WHERE retained.root_id = source.root_id))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_ownership {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_restoration_invalid",
        "Organization restoration requires valid recovery records and associations",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v53_for_test(connection: &Connection) {
    super::organization_association_index::downgrade_to_v54_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 54 {
        for (kind, name, _) in schema::OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 53; PRAGMA user_version = 53;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

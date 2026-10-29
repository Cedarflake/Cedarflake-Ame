use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const NAME: &str = "organization_admission_clock";
const TABLE: &str = "CREATE TABLE organization_admission_clock (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    restored_prefix INTEGER NOT NULL CHECK (restored_prefix >= 0)
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 59)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    transaction
        .execute_batch(
            "INSERT INTO organization_admission_clock VALUES (1, 0);
         UPDATE schema_info SET version = 60; PRAGMA user_version = 60;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 60)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_shape::schema_object_sql_matches(connection, "table", NAME, TABLE)? {
        return Ok(());
    }
    Err(invalid())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let valid: bool = connection
        .query_row(
            "SELECT COUNT(*) = 1 AND COALESCE(MIN(singleton = 1 AND restored_prefix >= 0
           AND restored_prefix <= (SELECT COALESCE(MAX(value), 0) FROM (
             SELECT seq AS value FROM sqlite_sequence WHERE name = 'organization_admissions'
             UNION ALL SELECT MAX(admission_id) FROM organization_recovery_admissions))), 0)
         FROM organization_admission_clock",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if valid { Ok(()) } else { Err(invalid()) }
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_admission_clock_invalid",
        "The organization admission boundary requires its canonical clock",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v59_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 60 {
        connection
            .execute_batch(
                "DROP TABLE organization_admission_clock;
             UPDATE schema_info SET version = 59; PRAGMA user_version = 59;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

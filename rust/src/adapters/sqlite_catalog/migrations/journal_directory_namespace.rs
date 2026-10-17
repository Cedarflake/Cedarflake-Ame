use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::schema_shape::schema_object_sql_matches;
use super::{current_schema, database_error};
use crate::domain::ScanError;

const TABLE: &str = "scan_directory_journal_authority";
const DDL: &str = "CREATE TABLE scan_directory_journal_authority (
    scan_id TEXT PRIMARY KEY NOT NULL,
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    journal_id BLOB NOT NULL CHECK(typeof(journal_id) = 'blob' AND length(journal_id) = 8
        AND journal_id != X'0000000000000000'),
    covered_until_usn INTEGER NOT NULL CHECK(covered_until_usn >= 0),
    FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 35)?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [TABLE],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        validate(transaction)?;
    } else {
        transaction.execute_batch(DDL).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 36; PRAGMA user_version = 36;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 36)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "table", TABLE, DDL)? {
        return Err(ScanError::new(
            "catalog_journal_directory_contract_invalid",
            "Journal directory coverage requires its canonical schema",
        ));
    }
    Ok(())
}

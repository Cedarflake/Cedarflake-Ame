use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::super::change_queue::root_retirement::{
    REMOVED_ROOT_AUTHORITY_IDS_SQL, removed_root_authority_retirement_sql,
};
use super::current_schema::validate_current_schema_structure;
use super::database_error;

pub(super) fn repair_needed(connection: &Connection) -> Result<bool, ScanError> {
    validate_current_schema_structure(connection)?;
    connection
        .query_row(
            &format!("SELECT EXISTS({REMOVED_ROOT_AUTHORITY_IDS_SQL})"),
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn repair_in_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    // Recheck structure under the writer snapshot. The caller's full validation must succeed
    // before committing; unrelated corruption cannot be hidden by this monotonic retirement.
    validate_current_schema_structure(transaction)?;
    transaction
        .execute(&removed_root_authority_retirement_sql(), [])
        .map_err(database_error)?;
    Ok(())
}

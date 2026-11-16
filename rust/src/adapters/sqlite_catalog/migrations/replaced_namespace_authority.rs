use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::super::change_queue::namespace_retirement::UNSTARTED_AUTHORITY_IDS;
use super::{SCHEMA_VERSION, current_schema, database_error, schema_shape};

fn replacement_ids() -> String {
    format!(
        "{UNSTARTED_AUTHORITY_IDS}
         AND EXISTS (
           SELECT 1 FROM library_change_root_state AS state
           JOIN library_roots AS root ON root.id = state.root_id
           WHERE state.root_id = authority.root_id AND state.is_active = 1
             AND state.generation > authority.root_generation
         )"
    )
}

pub(super) fn needed(connection: &Connection, version: i64) -> Result<bool, ScanError> {
    current_schema::validate_schema_structure(connection, version)?;
    connection
        .query_row(
            &format!("SELECT EXISTS({})", replacement_ids()),
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn apply(transaction: &Transaction<'_>, version: i64) -> Result<(), ScanError> {
    current_schema::validate_schema_structure(transaction, version)?;
    transaction
        .execute(
            &format!(
                "UPDATE library_recovery_authorities
                 SET retired_unix_ms = MAX(authorized_unix_ms,
                   (SELECT updated_unix_ms FROM library_change_queue
                    WHERE id = library_recovery_authorities.change_id),
                   (SELECT updated_unix_ms FROM library_change_root_state AS state
                    WHERE state.root_id = library_recovery_authorities.root_id))
                 WHERE change_id IN ({})",
                replacement_ids()
            ),
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn repair_before_upgrade(connection: &mut Connection) -> Result<(), ScanError> {
    let version = schema_shape::schema_version(connection)?;
    if !(81..SCHEMA_VERSION).contains(&version) || !needed(connection, version)? {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    apply(&transaction, version)?;
    current_schema::validate_schema_version(&transaction, version)?;
    transaction.commit().map_err(database_error)
}

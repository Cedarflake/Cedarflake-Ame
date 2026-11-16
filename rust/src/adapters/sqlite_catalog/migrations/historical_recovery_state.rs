use rusqlite::{Connection, TransactionBehavior};

use crate::domain::ScanError;

use super::{
    SCHEMA_VERSION, current_schema, database_error, replaced_namespace_authority, schema_shape,
    terminal_journal_handoff,
};

pub(super) fn repair_before_upgrade(connection: &mut Connection) -> Result<(), ScanError> {
    let version = schema_shape::schema_version(connection)?;
    if !(37..SCHEMA_VERSION).contains(&version) {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let version = schema_shape::schema_version(&transaction)?;
    if !(37..SCHEMA_VERSION).contains(&version) {
        return Ok(());
    }
    let replaced_namespace =
        version >= 81 && replaced_namespace_authority::needed(&transaction, version)?;
    let terminal_handoff = terminal_journal_handoff::needed(&transaction, version)?;
    if !replaced_namespace && !terminal_handoff {
        return Ok(());
    }
    if replaced_namespace {
        replaced_namespace_authority::apply(&transaction, version)?;
    }
    if terminal_handoff {
        terminal_journal_handoff::apply(&transaction, version)?;
    }
    current_schema::validate_schema_version(&transaction, version)?;
    transaction.commit().map_err(database_error)
}

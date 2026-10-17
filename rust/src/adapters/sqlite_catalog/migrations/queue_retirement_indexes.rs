use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;
use super::{current_schema, database_error};

const INDEXES: &[(&str, &str)] = &[
    (
        "library_change_queue_superseded_by",
        "CREATE INDEX library_change_queue_superseded_by
         ON library_change_queue(superseded_by_change_id)",
    ),
    (
        "library_media_admission_rechecks_change",
        "CREATE INDEX library_media_admission_rechecks_change
         ON library_media_admission_rechecks(change_id)",
    ),
    (
        "library_media_admission_rechecks_asset",
        "CREATE INDEX library_media_admission_rechecks_asset
         ON library_media_admission_rechecks(asset_id)",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    super::repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 38)?;
    super::repair_retired_live_gap_claims_transaction(transaction, 38)?;
    super::retired_root_authority::repair_for_schema(transaction, 38)?;
    current_schema::validate_schema_version(transaction, 38)?;
    for (name, ddl) in INDEXES {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if exists {
            require_index(transaction, name, ddl)?;
        } else {
            transaction.execute_batch(ddl).map_err(database_error)?;
        }
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 39; PRAGMA user_version = 39;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 39)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, ddl) in INDEXES {
        require_index(connection, name, ddl)?;
    }
    Ok(())
}

fn require_index(connection: &Connection, name: &str, ddl: &str) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "index", name, ddl)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_queue_retirement_index_invalid",
        "Queue and media retirement require their bounded ownership indexes",
    ))
}

#[cfg(test)]
mod tests;

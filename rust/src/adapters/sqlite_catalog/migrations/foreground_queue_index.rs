use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_object_sql_matches};

const INDEX_NAME: &str = "library_change_queue_active_origin";
const INDEX_DDL: &str = "CREATE INDEX library_change_queue_active_origin
    ON library_change_queue(root_id, root_generation, origin, status, id)
    WHERE status IN ('pending', 'leased', 'retry_wait')";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    super::repair_prerelease_v24_source_range_id_triggers(connection)?;
    super::repair_prerelease_v26_recovery_window_schema(connection)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    super::repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 33)?;
    super::repair_retired_live_gap_claims_transaction(transaction, 33)?;
    super::retired_root_authority::repair_for_schema(transaction, 33)?;
    current_schema::validate_schema_version(transaction, 33)?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [INDEX_NAME],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        if !schema_object_sql_matches(transaction, "index", INDEX_NAME, INDEX_DDL)? {
            return Err(ScanError::new(
                "catalog_queue_index_conflict",
                "The foreground queue index conflicts with an existing schema object",
            ));
        }
    } else {
        transaction
            .execute_batch(INDEX_DDL)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 34; PRAGMA user_version = 34;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 34)
}

#[cfg(test)]
pub(in crate::adapters::sqlite_catalog::migrations) fn downgrade_to_v33_for_test(
    connection: &Connection,
) {
    connection
        .execute_batch(
            "DROP TABLE IF EXISTS library_media_admission_rechecks;
             DROP TABLE IF EXISTS library_journal_scope_recoveries;
             DROP INDEX IF EXISTS library_change_queue_active_origin;
             UPDATE schema_info SET version = 33; PRAGMA user_version = 33;",
        )
        .expect("retain a v33 fixture without the new index");
}

#[cfg(test)]
mod tests;

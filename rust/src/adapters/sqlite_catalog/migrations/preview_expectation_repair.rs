use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::database_error;
use super::schema_shape::table_columns_match;

pub(super) fn repair(connection: &mut Connection) -> Result<(), ScanError> {
    if marker_is_complete(connection)? {
        return Ok(());
    }
    repair_missing_v19_preview_expectation_marker(connection)
}

pub(super) fn marker_is_complete(connection: &Connection) -> Result<bool, ScanError> {
    let marker_type = connection
        .query_row(
            "SELECT type FROM sqlite_master
             WHERE name = 'library_change_preview_repair_contract'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(marker_type) = marker_type {
        let marker_valid = marker_type == "table"
            && table_columns_match(
                connection,
                "library_change_preview_repair_contract",
                &[
                    ("singleton", "INTEGER", false, 1),
                    ("complete", "INTEGER", true, 0),
                ],
            )?
            && connection
                .query_row(
                    "SELECT singleton = 1 AND complete = 1
                     FROM library_change_preview_repair_contract",
                    [],
                    |row| row.get::<_, bool>(0),
                )
                .optional()
                .map_err(database_error)?
                .unwrap_or(false);
        if !marker_valid {
            return Err(ScanError::new(
                "catalog_change_catch_up_contract_unverifiable",
                "The catalog preview repair marker is malformed",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

fn repair_missing_v19_preview_expectation_marker(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    if marker_is_complete(&transaction)? {
        return transaction.commit().map_err(database_error);
    }
    transaction
        .execute_batch(
            "UPDATE library_change_catch_up_handoffs
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path =
                     library_change_catch_up_handoffs.preview_path
               );
             UPDATE library_change_scan_handoff_items
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path =
                     library_change_scan_handoff_items.preview_path
               );
             DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM library_roots AS roots
               JOIN asset_locations AS locations
                 ON locations.root_id = roots.id
                AND locations.scan_id = roots.active_scan_id
               WHERE locations.preview_status = 'ready'
                 AND NOT EXISTS (
                   SELECT 1 FROM preview_artifacts AS artifacts
                   WHERE artifacts.lifecycle_state = 'ready'
                     AND artifacts.artifact_path = locations.preview_path
                 )
             );
             UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND scan_id = (
                 SELECT roots.active_scan_id
                 FROM library_roots AS roots
                 WHERE roots.id = asset_locations.root_id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path = asset_locations.preview_path
               );
             CREATE TABLE library_change_preview_repair_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_change_preview_repair_contract(singleton, complete)
             VALUES (1, 1);",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn add_marker(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_preview_repair_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_change_preview_repair_contract(singleton, complete)
             VALUES (1, 1);",
        )
        .map_err(database_error)
}

#[cfg(test)]
mod tests;

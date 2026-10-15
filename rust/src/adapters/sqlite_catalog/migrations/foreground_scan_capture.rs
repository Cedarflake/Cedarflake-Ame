use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;

use super::{current_schema, database_error, unix_time_ms};

const CAPTURED_CONTROLS: &str = "
    SELECT control.id, control.root_id, control.root_generation,
           authority.run_id, control.catalog_revision_at_success AS revision,
           scan.completed_unix_ms, scan.change_queue_high_watermark
    FROM library_recovery_authorities AS authority
    JOIN library_change_queue AS control ON control.id = authority.change_id
    JOIN library_metadata_inventory_runs AS inventory ON inventory.id = authority.run_id
    JOIN library_roots AS root ON root.id = control.root_id
    JOIN library_change_root_state AS state ON state.root_id = root.id
    JOIN scan_runs AS scan ON scan.id = root.active_scan_id
    WHERE authority.retired_unix_ms IS NULL
      AND authority.root_id = control.root_id
      AND authority.root_generation = control.root_generation
      AND inventory.root_id = control.root_id
      AND inventory.root_generation = control.root_generation
      AND inventory.status IN ('running', 'comparing')
      AND state.is_active = 1 AND state.generation = control.root_generation
      AND control.status = 'completed' AND control.authoritative_scan_id IS NULL
      AND control.catalog_revision_at_success IS NOT NULL
      AND control.catalog_revision_at_success <= state.catalog_revision
      AND scan.root_id = root.id AND scan.root_generation_at_start = state.generation
      AND scan.scan_owner = 'foreground' AND scan.status = 'completed'
      AND scan.change_queue_high_watermark >= control.id
      AND scan.completed_unix_ms = control.updated_unix_ms
      AND 1 = (SELECT COUNT(*) FROM library_change_queue_lanes WHERE change_id = control.id)
      AND EXISTS (
        SELECT 1 FROM library_change_queue_lanes
        WHERE change_id = control.id AND lane = 'p2_recovery'
      )";

pub(super) fn repair_needed(connection: &Connection) -> Result<bool, ScanError> {
    current_schema::validate_current_schema_structure(connection)?;
    connection
        .query_row(&format!("SELECT EXISTS({CAPTURED_CONTROLS})"), [], |row| {
            row.get(0)
        })
        .map_err(database_error)
}

pub(super) fn repair_in_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_current_schema_structure(transaction)?;
    let now = unix_time_ms();
    // The completed foreground scan is the witness for the erroneous queue completion. Restore
    // only work completed by that publication; keep the inventory and its staged progress intact.
    transaction
        .execute(
            &format!(
                "UPDATE library_change_queue AS candidate
                 SET status = 'pending', ready_unix_ms = ?1, next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     catalog_revision_at_success = NULL, updated_unix_ms = ?1
                 WHERE candidate.status = 'completed'
                   AND candidate.authoritative_scan_id IS NULL
                   AND EXISTS (
                     SELECT 1 FROM library_metadata_inventory_candidate_owners AS owner
                     JOIN ({CAPTURED_CONTROLS}) AS captured ON captured.run_id = owner.run_id
                     WHERE owner.change_id = candidate.id
                       AND candidate.root_id = captured.root_id
                       AND candidate.root_generation = captured.root_generation
                       AND candidate.id <= captured.change_queue_high_watermark
                       AND candidate.catalog_revision_at_success = captured.revision
                       AND candidate.updated_unix_ms = captured.completed_unix_ms
                   )"
            ),
            params![now],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            &format!(
                "UPDATE library_change_queue
                 SET status = 'pending', ready_unix_ms = ?1, next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     catalog_revision_at_success = NULL, updated_unix_ms = ?1
                 WHERE id IN (SELECT id FROM ({CAPTURED_CONTROLS}))"
            ),
            params![now],
        )
        .map_err(database_error)?;
    Ok(())
}

use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, current_schema, database_error, scan_resume_inventory};

const REPAIRABLE_LOCATIONS: &str = "
    SELECT retained.scan_id, retained.location_id
    FROM scan_resume_pending_locations AS pending
    JOIN asset_locations AS retained
      ON retained.scan_id = pending.scan_id AND retained.location_id = pending.location_id
    JOIN scan_runs AS scan ON scan.id = retained.scan_id AND scan.root_id = retained.root_id
    JOIN library_roots AS root ON root.id = scan.root_id
    JOIN library_change_root_state AS state ON state.root_id = root.id
    WHERE scan.scan_owner = 'foreground' AND scan.status IN ('running', 'paused')
      AND root.active_scan_id IS NULL AND state.is_active = 1
      AND state.generation = scan.root_generation_at_start
      AND retained.file_identity_scheme = 'windows-file-id-128-v1'
      AND length(retained.file_identity_value) = 49
      AND substr(retained.file_identity_value, 17, 1) = ':'
      AND substr(retained.file_identity_value, 1, 16) NOT GLOB '*[^0-9a-f]*'
      AND substr(retained.file_identity_value, 18, 32) NOT GLOB '*[^0-9a-f]*'
      AND retained.source_generation > 0
      AND length(retained.source_revision_token) = 50
      AND substr(retained.source_revision_token, 1, 34) = 'windows-file-change-time-100ns-v1:'
      AND substr(retained.source_revision_token, 35) NOT GLOB '*[^0-9a-f]*'
      AND EXISTS (
        SELECT 1 FROM asset_locations AS current
        WHERE current.scan_id = retained.scan_id AND current.root_id = retained.root_id
          AND current.asset_id = retained.asset_id
          AND current.file_identity_scheme = retained.file_identity_scheme
          AND current.file_identity_value = retained.file_identity_value
          AND current.source_generation > retained.source_generation
          AND length(current.source_revision_token) = 50
          AND substr(current.source_revision_token, 1, 34) = 'windows-file-change-time-100ns-v1:'
          AND substr(current.source_revision_token, 35) NOT GLOB '*[^0-9a-f]*'
          AND NOT EXISTS (
            SELECT 1 FROM scan_resume_pending_locations AS unverified
            WHERE unverified.scan_id = current.scan_id AND unverified.location_id = current.location_id
          )
      )";

pub(super) fn repair_needed(connection: &Connection) -> Result<bool, ScanError> {
    scan_resume_inventory::validate_structure(connection)?;
    let has_retained_locations: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM scan_resume_pending_locations)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !has_retained_locations {
        return Ok(false);
    }
    connection
        .query_row(
            &format!("SELECT EXISTS ({REPAIRABLE_LOCATIONS})"),
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn repair_in_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    if !repair_needed(transaction)? {
        return Ok(());
    }
    current_schema::validate_current_schema_structure(transaction)?;
    super::validate_pre_live_gap_schema_contract(transaction, SCHEMA_VERSION)?;
    super::live_gap_contract::validate(transaction)?;
    scan_resume_inventory::validate_rows(transaction)?;
    // Retire only unverified derived aliases. Their newer sibling preserves the asset;
    // explicit resumption still has to establish every current namespace member.
    transaction.execute(
        &format!("DELETE FROM asset_locations WHERE (scan_id, location_id) IN ({REPAIRABLE_LOCATIONS})"),
        [],
    ).map_err(database_error)?;
    Ok(())
}

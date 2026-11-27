use rusqlite::Connection;

use crate::domain::ScanError;

use super::database_error;

pub(super) const GENERATION_INDEX: (&str, &str) = (
    "asset_locations_organization_generation",
    "CREATE INDEX asset_locations_organization_generation
       ON asset_locations(source_generation, root_id, scan_id)",
);

pub(super) const ROOT_GENERATION_INDEX: (&str, &str) = (
    "asset_locations_organization_root_generation",
    "CREATE INDEX asset_locations_organization_root_generation
       ON asset_locations(root_id, scan_id, source_generation)",
);

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_bindings AS binding
           JOIN organization_record_groups AS member ON member.record_id = binding.record_id
           WHERE binding.digest IS NOT NULL
           GROUP BY binding.fingerprint_scheme, binding.digest, binding.byte_length
           HAVING MIN(member.group_id) <> MAX(member.group_id))
         OR EXISTS(SELECT 1 FROM organization_conflicts AS conflict
           JOIN organization_record_groups AS member ON member.record_id = conflict.record_id
           JOIN organization_records AS record ON record.record_id = member.group_id
           WHERE member.record_id <> member.group_id OR record.disposition <> 'deferred' OR record.album_id IS NOT NULL)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid {
        return Err(ScanError::new(
            "catalog_organization_reconciliation_invalid",
            "Organization groups require coherent content bindings and deferred conflicts",
        ));
    }
    Ok(())
}

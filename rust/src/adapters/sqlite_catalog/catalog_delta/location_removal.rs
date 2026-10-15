use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::super::database_error;

pub(super) fn remove(
    transaction: &Transaction<'_>,
    active_scan_id: &str,
    running_scan_id: Option<&str>,
    root_id: &str,
    location_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations
             WHERE location_id = ?3
               AND EXISTS (
                 SELECT 1 FROM asset_locations AS locations
                 WHERE locations.scan_id = ?1 AND locations.root_id = ?2
                   AND locations.location_id = preview_artifact_locations.location_id
               )",
            params![active_scan_id, root_id, location_id],
        )
        .map_err(database_error)?;

    // Bind both parts of the location primary key. Combining the scans with OR can
    // select the root/scan index and revisit the entire root for each removed item.
    for scan_id in std::iter::once(active_scan_id).chain(running_scan_id) {
        transaction
            .execute(
                "DELETE FROM asset_locations
                 WHERE scan_id = ?1 AND location_id = ?2 AND root_id = ?3",
                params![scan_id, location_id, root_id],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

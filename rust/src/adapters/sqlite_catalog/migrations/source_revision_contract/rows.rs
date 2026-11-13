use rusqlite::Connection;

use crate::domain::ScanError;

use super::database_error;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    // A single location cannot disagree with another alias. Identify repeated identities through
    // their narrow indexes before reading full state for the cross-root and within-scan proofs.
    let invalid = connection.query_row(
        "WITH active_aliases AS MATERIALIZED (
           SELECT locations.file_identity_scheme, locations.file_identity_value
           FROM asset_locations AS locations INDEXED BY asset_locations_active_file_identity
           JOIN library_roots AS roots
             ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id
           WHERE locations.file_identity_scheme IS NOT NULL
           GROUP BY locations.file_identity_scheme, locations.file_identity_value
           HAVING COUNT(*) > 1
         ), scan_aliases AS MATERIALIZED (
           SELECT scan_id, file_identity_scheme, file_identity_value
           FROM asset_locations INDEXED BY asset_locations_file_identity
           WHERE file_identity_scheme IS NOT NULL
           GROUP BY scan_id, file_identity_scheme, file_identity_value
           HAVING COUNT(*) > 1
         )
         SELECT EXISTS(
           SELECT 1 FROM asset_locations
           WHERE source_generation IS NULL OR source_generation <= 0
             OR (source_revision_token IS NOT NULL AND (
               length(source_revision_token) <> 50
               OR source_revision_token NOT GLOB
                 'windows-file-change-time-100ns-v1:[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'
             ))
         ) OR EXISTS(
           SELECT 1 FROM preview_artifacts
           WHERE lifecycle_state = 'ready'
             AND (source_generation IS NULL OR source_generation <= 0
               OR source_revision_token IS NULL)
         ) OR EXISTS(
           SELECT 1 FROM active_aliases AS aliases CROSS JOIN asset_locations AS locations
             ON locations.file_identity_scheme = aliases.file_identity_scheme
             AND locations.file_identity_value IS aliases.file_identity_value
           JOIN library_roots AS roots
             ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id
           GROUP BY locations.file_identity_scheme, locations.file_identity_value
           HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
             OR COUNT(DISTINCT locations.source_revision_token) > 1
             OR MIN(locations.file_size) <> MAX(locations.file_size)
             OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
         ) OR EXISTS(
           SELECT 1 FROM scan_aliases AS aliases CROSS JOIN asset_locations AS locations
             ON locations.scan_id = aliases.scan_id
             AND locations.file_identity_scheme = aliases.file_identity_scheme
             AND locations.file_identity_value IS aliases.file_identity_value
           GROUP BY locations.scan_id, locations.file_identity_scheme, locations.file_identity_value
           HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
             OR COUNT(DISTINCT locations.source_revision_token) > 1
             OR MIN(locations.file_size) <> MAX(locations.file_size)
             OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
         ) OR (SELECT next_source_generation FROM catalog_state) <= COALESCE(
           (SELECT MAX(source_generation) FROM asset_locations), 0
         )",
        [], |row| row.get::<_, bool>(0),
    ).map_err(database_error)?;
    if invalid {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source revision and generation contract",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

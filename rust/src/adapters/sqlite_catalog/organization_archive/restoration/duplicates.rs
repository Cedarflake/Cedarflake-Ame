use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::database_error;

pub(super) fn retain(connection: &Connection, now: i64) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_recovery_duplicate_decisions
         SELECT source.scheme, source.digest, source.byte_length, location.retained_id, asset.retained_id,
           source.ignored, source.updated_unix_ms, ?1
         FROM recovery.archive_duplicate_decision AS source
         LEFT JOIN temp.organization_restore_identifiers AS location
           ON location.kind = 'location' AND location.original_id = source.preferred_location_id
         LEFT JOIN temp.organization_restore_identifiers AS asset
           ON asset.kind = 'asset' AND asset.original_id = source.preferred_asset_id
         WHERE NOT EXISTS(SELECT 1 FROM exact_duplicate_decisions AS current
           WHERE current.scheme = source.scheme AND current.digest = source.digest AND current.byte_length = source.byte_length)
         ON CONFLICT(scheme, digest, byte_length) DO NOTHING", [now],
    ).map_err(database_error)?;
    connection.execute_batch(
        "INSERT INTO organization_recovery_duplicate_sources
         SELECT retained.scheme, retained.digest, retained.byte_length, COALESCE(root.current_root_id, root.retained_root_id)
         FROM recovery.archive_duplicate_decision AS source
         JOIN organization_recovery_duplicate_decisions AS retained USING(scheme, digest, byte_length)
         JOIN recovery.archive_version_evidence AS evidence
           ON (evidence.location_id = source.preferred_location_id AND evidence.asset_id = source.preferred_asset_id)
             OR (source.preferred_location_id IS NULL AND evidence.fingerprint_scheme = source.scheme
               AND evidence.digest = source.digest AND evidence.byte_length = source.byte_length)
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = evidence.root_id
         WHERE 1 ON CONFLICT(scheme, digest, byte_length, root_id) DO NOTHING;",
    ).map_err(database_error)
}

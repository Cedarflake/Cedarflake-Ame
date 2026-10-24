use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::super::database_error;
use super::super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};

pub(super) fn reconcile(connection: &Connection) -> Result<(), ScanError> {
    let pending: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_recovery_duplicate_decisions)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !pending {
        return Ok(());
    }
    connection.execute(
        &format!(
            "INSERT INTO organization_recovery_duplicate_sources
             SELECT choice.scheme, choice.digest, choice.byte_length, l.root_id
             FROM temp.organization_recovery_batch AS batch CROSS JOIN {CURRENT_LOCATIONS}
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             CROSS JOIN organization_recovery_versions AS evidence
               ON evidence.file_identity_scheme = l.file_identity_scheme AND evidence.file_identity_value = l.file_identity_value
                 AND evidence.source_revision_token = l.source_revision_token AND evidence.byte_length = l.file_size
             JOIN organization_recovery_duplicate_decisions AS choice
               ON choice.scheme = evidence.fingerprint_scheme AND choice.digest = evidence.digest AND choice.byte_length = evidence.byte_length
                 AND choice.preferred_location_id = evidence.location_id AND choice.preferred_asset_id = evidence.asset_id
             WHERE l.source_generation = batch.source_generation AND choice.scheme = ?1
               AND l.file_identity_scheme = 'windows-file-id-128-v1'
               AND l.source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
               AND (f.digest IS NULL OR f.digest = choice.digest)
             ON CONFLICT(scheme, digest, byte_length, root_id) DO NOTHING"
        ), [EXACT_FINGERPRINT_SCHEME],
    ).map_err(database_error)?;
    connection.execute(
        &format!(
            "INSERT INTO exact_duplicate_decisions
             SELECT choice.scheme, choice.digest, choice.byte_length, l.location_id, l.asset_id,
               choice.ignored, choice.updated_unix_ms
             FROM temp.organization_recovery_batch AS batch
             CROSS JOIN {CURRENT_LOCATIONS}
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             CROSS JOIN organization_recovery_versions AS evidence
               ON evidence.file_identity_scheme = l.file_identity_scheme AND evidence.file_identity_value = l.file_identity_value
                 AND evidence.source_revision_token = l.source_revision_token AND evidence.byte_length = l.file_size
             JOIN organization_recovery_duplicate_decisions AS choice
               ON choice.scheme = evidence.fingerprint_scheme AND choice.digest = evidence.digest AND choice.byte_length = evidence.byte_length
                 AND choice.preferred_location_id = evidence.location_id AND choice.preferred_asset_id = evidence.asset_id
             WHERE l.source_generation = batch.source_generation AND choice.scheme = ?1
               AND evidence.native_copy_count = 1 AND l.file_identity_scheme = 'windows-file-id-128-v1'
               AND l.source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
               AND (f.digest IS NULL OR f.digest = choice.digest)
               AND (SELECT COUNT(*) FROM (
                 SELECT 1 FROM asset_locations AS alias
                 JOIN library_roots AS owner ON owner.id = alias.root_id AND owner.active_scan_id = alias.scan_id
                 WHERE alias.file_identity_scheme = l.file_identity_scheme AND alias.file_identity_value = l.file_identity_value
                   AND alias.source_revision_token = l.source_revision_token AND alias.file_size = l.file_size LIMIT 2)) = 1
             ON CONFLICT(scheme, digest, byte_length) DO NOTHING"
        ), [EXACT_FINGERPRINT_SCHEME],
    ).map_err(database_error)?;
    connection.execute(
        &format!(
            "INSERT INTO exact_duplicate_decisions
             SELECT choice.scheme, choice.digest, choice.byte_length, NULL, NULL, choice.ignored, choice.updated_unix_ms
             FROM temp.organization_recovery_batch AS batch
             CROSS JOIN {CURRENT_LOCATIONS}
             JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             JOIN gallery_content_fingerprints AS published ON published.source_generation = f.source_generation AND published.scheme = f.scheme
             JOIN organization_recovery_duplicate_decisions AS choice
               ON choice.scheme = f.scheme AND choice.digest = f.digest AND choice.byte_length = f.byte_length
             WHERE l.source_generation = batch.source_generation AND choice.preferred_location_id IS NULL
             ON CONFLICT(scheme, digest, byte_length) DO NOTHING"
        ), [EXACT_FINGERPRINT_SCHEME],
    ).map_err(database_error)?;
    connection.execute_batch(
        "DELETE FROM organization_recovery_duplicate_decisions AS retained
         WHERE EXISTS(SELECT 1 FROM exact_duplicate_decisions AS current
           WHERE current.scheme = retained.scheme AND current.digest = retained.digest AND current.byte_length = retained.byte_length);",
    ).map_err(database_error)
}

use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::database_error;
use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::{reconciliation, records};

pub(super) fn publish_generation(
    connection: &Connection,
    generation: i64,
) -> Result<(), ScanError> {
    if !needs_recovery(connection, generation)? {
        return Ok(());
    }
    records::prepare_members(connection)?;
    connection.execute(
        &format!(
            "INSERT OR IGNORE INTO temp.organization_command_members
             SELECT l.asset_id, l.source_generation, l.root_id, l.file_size, l.file_identity_scheme,
               l.file_identity_value, l.source_revision_token, f.scheme, f.digest
             FROM {CURRENT_LOCATIONS} LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             WHERE l.source_generation = ?2"
        ), params![EXACT_FINGERPRINT_SCHEME, generation],
    ).map_err(database_error)?;
    records::validate_members(connection)?;
    collect_bindings(connection)?;
    if contradicts_content(connection)? {
        return Ok(());
    }
    reconciliation::prepare_groups(connection)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO temp.organization_merge_groups
         SELECT member.group_id FROM temp.organization_native_bindings AS binding
         JOIN organization_record_groups AS member ON member.record_id = binding.record_id",
            [],
        )
        .map_err(database_error)?;
    if let Some(resolved) = reconciliation::merge(connection, None)? {
        records::attach_members(connection, &resolved.record_id)?;
    }
    Ok(())
}

fn needs_recovery(connection: &Connection, generation: i64) -> Result<bool, ScanError> {
    // Source reconciliation assigns one generation to every current alias of a physical version.
    // Compare distinct canonical groups once, rather than every current/historical alias pair.
    connection.query_row(
        &format!(
            "WITH native_version AS MATERIALIZED (
               SELECT l.file_identity_scheme, l.file_identity_value, l.source_revision_token, l.file_size
               FROM {CURRENT_LOCATIONS}
               WHERE l.source_generation = ?1 AND l.file_identity_scheme = 'windows-file-id-128-v1'
                 AND l.source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
               LIMIT 1
             ), known_groups AS MATERIALIZED (
               SELECT DISTINCT mapping.group_id FROM native_version AS version
               CROSS JOIN organization_bindings AS binding
                 ON binding.file_identity_scheme = version.file_identity_scheme
                   AND binding.file_identity_value = version.file_identity_value
                   AND binding.source_revision_token = version.source_revision_token
                   AND binding.byte_length = version.file_size
               CROSS JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
             )
             SELECT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
               LEFT JOIN organization_bindings AS attached
                 ON attached.asset_id = l.asset_id AND attached.source_generation = l.source_generation
               LEFT JOIN organization_record_groups AS own ON own.record_id = attached.record_id
               WHERE l.source_generation = ?1 AND EXISTS(SELECT 1 FROM known_groups)
                 AND (own.group_id IS NULL OR EXISTS(
                   SELECT 1 FROM known_groups AS known WHERE known.group_id <> own.group_id)))"
        ), [generation], |row| row.get(0),
    ).map_err(database_error)
}

fn collect_bindings(connection: &Connection) -> Result<(), ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_native_bindings (
           asset_id TEXT NOT NULL, source_generation INTEGER NOT NULL, record_id TEXT NOT NULL,
           fingerprint_scheme TEXT, digest TEXT, PRIMARY KEY(asset_id, source_generation)
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_native_bindings;
         INSERT OR IGNORE INTO temp.organization_native_bindings
         SELECT binding.asset_id, binding.source_generation, binding.record_id,
                binding.fingerprint_scheme, binding.digest
         FROM (SELECT DISTINCT file_identity_scheme, file_identity_value, source_revision_token, byte_length
               FROM temp.organization_command_members) AS member
         CROSS JOIN organization_bindings AS binding
           ON binding.file_identity_scheme = member.file_identity_scheme
             AND binding.file_identity_value = member.file_identity_value
             AND binding.source_revision_token = member.source_revision_token
             AND binding.byte_length = member.byte_length;",
    ).map_err(database_error)
}

fn contradicts_content(connection: &Connection) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT (SELECT COUNT(DISTINCT digest) FROM temp.organization_native_bindings
                   WHERE fingerprint_scheme = ?1) > 1
         OR EXISTS(SELECT 1 FROM temp.organization_command_members AS current
           WHERE current.fingerprint_scheme = ?1 AND current.digest <>
             (SELECT MIN(digest) FROM temp.organization_native_bindings WHERE fingerprint_scheme = ?1))",
            [EXACT_FINGERPRINT_SCHEME],
            |row| row.get(0),
        )
        .map_err(database_error)
}

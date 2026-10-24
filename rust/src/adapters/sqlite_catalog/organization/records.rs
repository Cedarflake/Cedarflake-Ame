use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::{OrganizationGroupKey, stale_selection};

use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, unix_time_ms};
use super::selection::member_query;
use super::{native_recovery, reconciliation};

pub(super) const CURRENT_RECORDS: &str = "SELECT DISTINCT related.record_id
    FROM temp.organization_command_members AS member CROSS JOIN organization_bindings AS binding
      ON member.asset_id = binding.asset_id AND member.source_generation = binding.source_generation
    CROSS JOIN organization_record_groups AS own ON own.record_id = binding.record_id
    CROSS JOIN organization_record_groups AS related ON related.group_id = own.group_id";

pub(super) fn materialize(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<bool, ScanError> {
    prepare_members(connection)?;
    let query = member_query(key)?;
    connection
        .execute(
            &format!(
                "INSERT OR IGNORE INTO temp.organization_command_members
             SELECT l.asset_id, l.source_generation, l.root_id, l.file_size, l.file_identity_scheme,
               l.file_identity_value, l.source_revision_token, f.scheme, f.digest
             FROM {} WHERE {}",
                query.from, query.predicate
            ),
            params_from_iter(query.values),
        )
        .map_err(database_error)?;
    validate_members(connection)?;
    let resolved = reconciliation::resolve_members(connection, key)?;
    let (record_id, merged) = match resolved {
        Some(resolved) => (resolved.record_id, resolved.merged),
        None => (create_record(connection)?, false),
    };
    attach_members(connection, &record_id)?;
    Ok(merged)
}

pub(in crate::adapters::sqlite_catalog) fn publish_generation(
    connection: &Connection,
    source_generation: i64,
) -> Result<(), ScanError> {
    let handled: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_records)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !handled {
        return Ok(());
    }
    native_recovery::publish_generation(connection, source_generation)?;
    let key = connection
        .query_row(
            &format!(
                "SELECT f.digest, f.byte_length FROM {CURRENT_LOCATIONS}
         JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
         WHERE l.source_generation = ?2 LIMIT 1"
            ),
            params![EXACT_FINGERPRINT_SCHEME, source_generation],
            |row| {
                Ok(OrganizationGroupKey::Confirmed {
                    digest: row.get(0)?,
                    byte_length: read_unsigned(row, 1)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(key) = key else {
        return attach_known_source(connection, source_generation);
    };
    prepare_members(connection)?;
    connection
        .execute(
            &format!(
                "INSERT OR IGNORE INTO temp.organization_command_members
         SELECT l.asset_id, l.source_generation, l.root_id, l.file_size, l.file_identity_scheme,
           l.file_identity_value, l.source_revision_token, f.scheme, f.digest
         FROM {CURRENT_LOCATIONS} JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
         WHERE l.source_generation = ?2"
            ),
            params![EXACT_FINGERPRINT_SCHEME, source_generation],
        )
        .map_err(database_error)?;
    validate_members(connection)?;
    if let Some(resolved) = reconciliation::resolve_members(connection, &key)? {
        if resolved.has_content_binding {
            attach_members(connection, &resolved.record_id)?;
        } else {
            // The first handled member can finish hashing after unhandled copies. Bind the
            // already verified group once, including copies published in earlier batches.
            materialize(connection, &key)?;
        }
    }
    Ok(())
}

fn attach_known_source(connection: &Connection, source_generation: i64) -> Result<(), ScanError> {
    connection
        .execute(
            &format!(
                "INSERT OR IGNORE INTO organization_sources(root_id, record_id)
         SELECT l.root_id, binding.record_id FROM {CURRENT_LOCATIONS}
         JOIN organization_bindings AS binding ON binding.asset_id = l.asset_id
           AND binding.source_generation = l.source_generation
         WHERE l.source_generation = ?1 AND binding.byte_length = l.file_size
           AND binding.file_identity_scheme IS l.file_identity_scheme
           AND binding.file_identity_value IS l.file_identity_value
           AND binding.source_revision_token IS l.source_revision_token"
            ),
            [source_generation],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn prepare_members(connection: &Connection) -> Result<(), ScanError> {
    // SQLite holds the membership set; even a very large duplicate group never crosses the bridge
    // or becomes a Rust vector. This table lives only on the command's catalog connection.
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_command_members (
          asset_id TEXT NOT NULL, source_generation INTEGER NOT NULL, root_id TEXT NOT NULL,
          byte_length INTEGER NOT NULL, file_identity_scheme TEXT, file_identity_value TEXT,
          source_revision_token TEXT, fingerprint_scheme TEXT, digest TEXT,
          PRIMARY KEY(asset_id, source_generation, root_id)
        ) WITHOUT ROWID;
        DELETE FROM temp.organization_command_members;",
        )
        .map_err(database_error)
}

pub(super) fn validate_members(connection: &Connection) -> Result<(), ScanError> {
    let incompatible: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM temp.organization_command_members AS member
         CROSS JOIN organization_bindings AS binding
           ON member.asset_id = binding.asset_id AND member.source_generation = binding.source_generation
         WHERE binding.byte_length <> member.byte_length
           OR binding.file_identity_scheme IS NOT member.file_identity_scheme
           OR binding.file_identity_value IS NOT member.file_identity_value
           OR binding.source_revision_token IS NOT member.source_revision_token
           OR (binding.digest IS NOT NULL AND member.digest IS NOT NULL
             AND (binding.fingerprint_scheme <> member.fingerprint_scheme OR binding.digest <> member.digest)))",
        [], |row| row.get(0)
    ).map_err(database_error)?;
    if incompatible {
        return Err(stale_selection());
    }
    Ok(())
}

fn create_record(connection: &Connection) -> Result<String, ScanError> {
    let record_id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_records(record_id, album_id, disposition, favorite,
                   organization_revision, favorite_revision, created_unix_ms, updated_unix_ms)
                 VALUES (?1, NULL, 'pending', 0, 0, 0, ?2, ?2)",
            params![record_id, unix_time_ms()],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO organization_record_groups(record_id, group_id) VALUES (?1, ?1)",
            [&record_id],
        )
        .map_err(database_error)?;
    Ok(record_id)
}

pub(super) fn attach_members(connection: &Connection, record_id: &str) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_bindings(record_id, asset_id, source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest)
         SELECT ?1, asset_id, source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest
         FROM temp.organization_command_members WHERE 1
         GROUP BY asset_id, source_generation
         ON CONFLICT(asset_id, source_generation) DO UPDATE SET
           fingerprint_scheme = COALESCE(excluded.fingerprint_scheme, organization_bindings.fingerprint_scheme),
           digest = COALESCE(excluded.digest, organization_bindings.digest)
         WHERE organization_bindings.digest IS NULL AND excluded.digest IS NOT NULL",
        [record_id]
    ).map_err(database_error)?;
    connection.execute(
        "INSERT OR IGNORE INTO organization_sources(root_id, record_id)
         SELECT member.root_id, binding.record_id FROM temp.organization_command_members AS member
         CROSS JOIN organization_bindings AS binding
           ON binding.asset_id = member.asset_id AND binding.source_generation = member.source_generation", []
    ).map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_records SET unassociated_since_unix_ms = NULL
             WHERE unassociated_since_unix_ms IS NOT NULL AND record_id = ?1",
            [record_id],
        )
        .map_err(database_error)?;
    Ok(())
}

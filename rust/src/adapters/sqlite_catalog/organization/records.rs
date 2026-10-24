use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::domain::ScanError;
use crate::domain::organization::{OrganizationGroupKey, stale_selection};

use super::super::{database_error, unix_time_ms};
use super::selection::member_query;

pub(super) const CURRENT_RECORDS: &str = "SELECT DISTINCT binding.record_id
    FROM organization_bindings AS binding JOIN temp.organization_command_members AS member
      ON member.asset_id = binding.asset_id AND member.source_generation = binding.source_generation";

pub(super) fn materialize(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<(), ScanError> {
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
        .map_err(database_error)?;
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
    let incompatible: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_bindings AS binding
         JOIN temp.organization_command_members AS member
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

    let retained: Option<String> = connection
        .query_row(
            &format!(
                "SELECT record_id FROM organization_records WHERE record_id IN ({CURRENT_RECORDS})
         ORDER BY created_unix_ms, record_id LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let record_id =
        match retained {
            Some(record_id) => record_id,
            None => {
                let record_id: String = connection
                    .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
                    .map_err(database_error)?;
                connection.execute(
                "INSERT INTO organization_records(record_id, album_id, disposition, favorite,
                   organization_revision, favorite_revision, created_unix_ms, updated_unix_ms)
                 VALUES (?1, NULL, 'pending', 0, 0, 0, ?2, ?2)",
                params![record_id, unix_time_ms()]
            ).map_err(database_error)?;
                record_id
            }
        };
    connection.execute(
        "INSERT INTO organization_bindings(record_id, asset_id, source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest)
         SELECT ?1, asset_id, source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest
         FROM temp.organization_command_members WHERE 1
         GROUP BY asset_id, source_generation
         ON CONFLICT(asset_id, source_generation) DO UPDATE SET
           fingerprint_scheme = COALESCE(excluded.fingerprint_scheme, organization_bindings.fingerprint_scheme),
           digest = COALESCE(excluded.digest, organization_bindings.digest)",
        [&record_id]
    ).map_err(database_error)?;
    connection.execute(
        "INSERT OR IGNORE INTO organization_sources(root_id, record_id)
         SELECT member.root_id, binding.record_id FROM temp.organization_command_members AS member
         JOIN organization_bindings AS binding
           ON binding.asset_id = member.asset_id AND binding.source_generation = member.source_generation", []
    ).map_err(database_error)?;
    connection.execute(&format!(
        "UPDATE organization_records SET unassociated_since_unix_ms = NULL WHERE record_id IN ({CURRENT_RECORDS})"
    ), []).map_err(database_error)?;
    Ok(())
}

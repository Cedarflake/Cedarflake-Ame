use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::{database_error, read_unsigned};

pub(super) fn capture(
    connection: &Connection,
    checkpoint: &mut dyn FnMut(u64) -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE organization_restore_current (
           location_id TEXT PRIMARY KEY NOT NULL, byte_length INTEGER NOT NULL,
           file_identity_scheme TEXT, file_identity_value TEXT, source_revision_token TEXT,
           fingerprint_scheme TEXT, digest TEXT, fingerprint_published INTEGER NOT NULL
         ) WITHOUT ROWID;
         CREATE INDEX temp.organization_restore_current_content
           ON organization_restore_current(fingerprint_scheme, digest, byte_length, fingerprint_published);
         CREATE TEMP TABLE organization_restore_native (
           file_identity_scheme TEXT NOT NULL, file_identity_value TEXT NOT NULL,
           source_revision_token TEXT NOT NULL, byte_length INTEGER NOT NULL,
           digest TEXT NOT NULL, copies INTEGER NOT NULL,
           PRIMARY KEY(file_identity_scheme, file_identity_value, source_revision_token, byte_length, digest)
         ) WITHOUT ROWID;
         CREATE TEMP TABLE organization_restore_matched_groups (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;",
    ).map_err(database_error)?;
    let mut after = String::new();
    loop {
        let (last, count): (Option<String>, u64) = connection
            .query_row(
                &format!(
                    "SELECT MAX(location_id), COUNT(*) FROM (
                   SELECT l.location_id FROM {CURRENT_LOCATIONS}
                   WHERE l.location_id > ?1
                   ORDER BY l.location_id LIMIT 256)"
                ),
                [&after],
                |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
            )
            .map_err(database_error)?;
        let Some(last) = last else { break };
        connection.execute(
            &format!(
                "INSERT INTO temp.organization_restore_current
                 SELECT l.location_id, l.file_size, l.file_identity_scheme, l.file_identity_value,
                        l.source_revision_token, f.scheme, f.digest,
                        EXISTS(SELECT 1 FROM gallery_content_fingerprints AS published
                          WHERE published.source_generation = f.source_generation AND published.scheme = f.scheme)
                 FROM {CURRENT_LOCATIONS}
                 LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                 WHERE l.location_id > ?2 AND l.location_id <= ?3"
            ), params![EXACT_FINGERPRINT_SCHEME, after, last],
        ).map_err(database_error)?;
        connection.execute(
            "INSERT INTO temp.organization_restore_native
             SELECT file_identity_scheme, file_identity_value, source_revision_token, byte_length,
                    COALESCE(digest, ''), COUNT(*)
             FROM temp.organization_restore_current
             WHERE location_id > ?1 AND location_id <= ?2
               AND file_identity_scheme = 'windows-file-id-128-v1'
               AND file_identity_value IS NOT NULL
               AND source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
             GROUP BY file_identity_scheme, file_identity_value, source_revision_token, byte_length, digest
             ON CONFLICT DO UPDATE SET copies = copies + excluded.copies",
            params![after, last],
        ).map_err(database_error)?;
        after = last;
        checkpoint(count)?;
    }
    Ok(())
}

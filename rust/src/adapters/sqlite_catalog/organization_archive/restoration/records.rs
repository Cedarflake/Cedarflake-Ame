use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::database_error;

pub(super) fn prepare(connection: &Connection) -> Result<(), ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_restore_new_albums (
           album_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_restore_new_albums;
         INSERT INTO temp.organization_restore_new_albums
         SELECT album_id FROM recovery.archive_album AS source
         WHERE NOT EXISTS(SELECT 1 FROM organization_albums AS current WHERE current.album_id = source.album_id);
         CREATE TEMP TABLE IF NOT EXISTS organization_restore_new_records (
           record_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_restore_new_records;
         INSERT INTO temp.organization_restore_new_records
         SELECT record_id FROM recovery.archive_record AS source
         WHERE NOT EXISTS(SELECT 1 FROM organization_records AS current WHERE current.record_id = source.record_id);",
    ).map_err(database_error)
}

pub(super) fn decisions(
    connection: &Connection,
    revision_base: i64,
    now: i64,
) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_albums
         SELECT album.album_id, album.name, ?1 + album.revision, album.created_unix_ms, album.updated_unix_ms
         FROM temp.organization_restore_new_albums AS selected
         CROSS JOIN recovery.archive_album AS album USING(album_id)", [revision_base],
    ).map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_album_retirements
         SELECT retired.album_id, ?1 + retired.revision, retired.retired_unix_ms
         FROM temp.organization_restore_new_albums AS selected
         CROSS JOIN recovery.archive_retired_album AS retired USING(album_id)",
            [revision_base],
        )
        .map_err(database_error)?;
    connection.execute(
        "INSERT INTO organization_records
         SELECT record.record_id, CASE WHEN retired.album_id IS NULL THEN record.album_id END,
           CASE WHEN retired.album_id IS NULL THEN record.disposition ELSE 'ignored' END, record.favorite,
           COALESCE(retired.revision, ?1 + record.organization_revision), ?1 + record.favorite_revision,
           record.created_unix_ms, record.updated_unix_ms, COALESCE(record.unassociated_since_unix_ms, ?2)
         FROM temp.organization_restore_new_records AS selected
         CROSS JOIN recovery.archive_record AS record USING(record_id)
         LEFT JOIN organization_album_retirements AS retired ON retired.album_id = record.album_id",
        params![revision_base, now],
    ).map_err(database_error)?;
    connection.execute_batch(
        "INSERT INTO organization_record_groups
         SELECT member.record_id, COALESCE(current.group_id, member.group_id)
         FROM temp.organization_restore_new_records AS selected
         CROSS JOIN recovery.archive_group AS member USING(record_id)
         LEFT JOIN organization_record_groups AS current ON current.record_id = member.group_id;
         INSERT INTO organization_restored_fields
         SELECT record.record_id, field.name,
           CASE field.name WHEN 'organization' THEN record.organization_revision ELSE record.favorite_revision END
         FROM temp.organization_restore_new_records AS selected
         CROSS JOIN organization_records AS record USING(record_id)
         CROSS JOIN recovery.archive_record AS source USING(record_id)
         CROSS JOIN (SELECT 'organization' AS name UNION ALL SELECT 'favorite') AS field
         WHERE field.name = 'favorite' OR NOT EXISTS(
           SELECT 1 FROM organization_album_retirements AS retired WHERE retired.album_id = source.album_id);",
    ).map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_conflicts
         SELECT conflict.record_id, conflict.album_id, ?1 + conflict.decision_revision
         FROM temp.organization_restore_new_records AS selected
         CROSS JOIN recovery.archive_conflict AS conflict USING(record_id)
         JOIN organization_record_groups AS member ON member.record_id = conflict.record_id
         WHERE member.record_id = member.group_id",
            [revision_base],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn bindings(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection.execute(
        "INSERT OR IGNORE INTO organization_bindings
         SELECT binding.record_id, identifier.retained_id, generation.local_generation,
           binding.byte_length, binding.file_identity_scheme, binding.file_identity_value,
           binding.source_revision_token, binding.fingerprint_scheme, binding.digest
         FROM recovery.archive_binding AS binding
         JOIN temp.organization_restore_identifiers AS identifier
           ON identifier.kind = 'asset' AND identifier.original_id = binding.asset_id
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?1 AND generation.source_generation = binding.source_generation",
        [archive_id],
    ).map_err(database_error)?;
    connection
        .execute_batch(
            "INSERT OR IGNORE INTO organization_sources
         SELECT COALESCE(root.current_root_id, root.retained_root_id), source.record_id
         FROM recovery.archive_source_owner AS source
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = source.root_id;",
        )
        .map_err(database_error)
}

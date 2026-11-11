use rusqlite::Transaction;

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::{GalleryCollection, GalleryDuplicateMode, GalleryQuery, ScanError};

use super::super::super::exact_duplicates::queries::{
    COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS_BY_GENERATION,
};
use super::super::super::{database_error, sqlite_unsigned};

pub(super) fn validate_query(query: &GalleryQuery) -> Result<(), ScanError> {
    if query.collection != GalleryCollection::Library
        || query.organization_queue.is_some()
        || query.duplicate_mode != GalleryDuplicateMode::Merged
    {
        return Err(ScanError::new(
            "file_operation_selection_invalid",
            "Whole-group deletion requires a merged ordinary gallery selection",
        ));
    }
    Ok(())
}

pub(super) fn expand(transaction: &Transaction<'_>) -> Result<u64, ScanError> {
    transaction
        .execute_batch(
            "CREATE TEMP TABLE file_operation_group_keys (
               digest TEXT NOT NULL, byte_length INTEGER NOT NULL,
               PRIMARY KEY(digest, byte_length)) WITHOUT ROWID;",
        )
        .map_err(database_error)?;
    transaction
        .execute(
            &format!(
                "INSERT INTO temp.file_operation_group_keys
                 SELECT DISTINCT f.digest, f.byte_length
                 FROM temp.file_operation_selection AS selected
                 CROSS JOIN asset_locations AS l INDEXED BY asset_locations_location_id
                   ON l.location_id = selected.location_id
                   AND l.source_generation = selected.source_generation
                 CROSS JOIN content_fingerprints AS f
                   ON f.source_generation = (
                     SELECT f.evidence_source_generation FROM gallery_display_fingerprints AS f
                     WHERE {COMPATIBLE_FINGERPRINT})
                   AND f.scheme = ?1"
            ),
            [EXACT_FINGERPRINT_SCHEME],
        )
        .map_err(database_error)?;
    let members = format!(
        "FROM temp.file_operation_group_keys AS groups
         JOIN gallery_display_fingerprints AS f
           ON f.digest = groups.digest AND f.byte_length = groups.byte_length
           AND f.digest IN (SELECT digest FROM temp.file_operation_group_keys)
         CROSS JOIN {CURRENT_LOCATIONS_BY_GENERATION}"
    );
    let pending: bool = transaction
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 {members} WHERE {COMPATIBLE_FINGERPRINT} AND f.is_pending = 1)"
            ),
            [EXACT_FINGERPRINT_SCHEME],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if pending {
        return Err(ScanError::new(
            "organization_evidence_pending",
            "A selected duplicate group contains files whose content is still being verified",
        ));
    }
    let groups: i64 = transaction
        .query_row(
            &format!(
                "SELECT (SELECT COUNT(*) FROM temp.file_operation_group_keys) + COUNT(*)
                 FROM temp.file_operation_selection AS selected
                 CROSS JOIN asset_locations AS l INDEXED BY asset_locations_location_id
                   ON l.location_id = selected.location_id
                   AND l.source_generation = selected.source_generation
                 WHERE NOT EXISTS(
                   SELECT 1 FROM gallery_display_fingerprints AS f WHERE {COMPATIBLE_FINGERPRINT})"
            ),
            [EXACT_FINGERPRINT_SCHEME],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    // Group badges include current copies in other folders. Freeze those same members once;
    // later preference changes or newly analyzed copies cannot extend this authorization.
    transaction
        .execute(
            &format!(
                "INSERT INTO temp.file_operation_selection
                   (asset_id, location_id, source_generation, root_generation, file_size)
                 SELECT l.asset_id, l.location_id, l.source_generation, state.generation, l.file_size
                 {members}
                 WHERE {COMPATIBLE_FINGERPRINT} AND f.is_pending = 0
                 ORDER BY l.root_id, l.location_id
                 ON CONFLICT(location_id) DO NOTHING"
            ),
            [EXACT_FINGERPRINT_SCHEME],
        )
        .map_err(database_error)?;
    sqlite_unsigned(groups, "selected duplicate groups")
}

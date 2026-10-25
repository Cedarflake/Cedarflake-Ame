use rusqlite::{Connection, OptionalExtension, params};

use crate::adapters::sqlite_catalog::gallery::review::{ReviewRead, candidate};
use crate::adapters::sqlite_catalog::organization::review::{ordering, scope};
use crate::domain::ScanError;

use super::super::super::database_error;

pub(super) fn restore(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    let retained: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_review_bookmark)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if retained {
        return Ok(());
    }
    connection.execute_batch(
        "INSERT INTO organization_review_scopes
         SELECT 'bookmark', scope.collection, scope.album_id,
           COALESCE(root.current_root_id, root.retained_root_id), scope.folder_relative_path,
           scope.include_descendants, scope.search_text, scope.queue
         FROM recovery.archive_review_scope AS scope
         LEFT JOIN temp.organization_restore_roots AS root ON root.original_root_id = scope.root_id
         WHERE EXISTS(SELECT 1 FROM recovery.archive_bookmark)
         ON CONFLICT(slot) DO UPDATE SET collection = excluded.collection, album_id = excluded.album_id,
           root_id = excluded.root_id, folder_relative_path = excluded.folder_relative_path,
           include_descendants = excluded.include_descendants, search_text = excluded.search_text, queue = excluded.queue;",
    ).map_err(database_error)?;
    connection.execute(
        "INSERT INTO organization_review_bookmark
         SELECT 1, 'bookmark', bookmark.record_id, admission.admission_id,
           COALESCE(root.current_root_id, root.retained_root_id), location.retained_id,
           bookmark.after_current, bookmark.updated_unix_ms
         FROM recovery.archive_bookmark AS bookmark
         JOIN recovery.archive_admission AS original ON original.admission_id = bookmark.admission_id
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?1 AND generation.source_generation = original.source_generation
         JOIN organization_recovery_admissions AS admission ON admission.source_generation = generation.local_generation
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = bookmark.root_id
         JOIN temp.organization_restore_identifiers AS location
           ON location.kind = 'location' AND location.original_id = bookmark.location_id", [archive_id],
    ).map_err(database_error)?;
    connection.execute_batch(
        "INSERT INTO organization_review_sort
         SELECT 'bookmark', sort_key, sort_direction FROM recovery.archive_review_scope
         WHERE EXISTS(SELECT 1 FROM organization_review_bookmark)
         ON CONFLICT(scope_slot) DO UPDATE SET sort_key = excluded.sort_key, sort_direction = excluded.sort_direction;
         INSERT INTO organization_review_cursor
         SELECT 1, source.primary_missing, source.primary_text,
           CASE WHEN sort.sort_key = 'library_admission' THEN bookmark.admission_id ELSE source.primary_number END
         FROM recovery.archive_bookmark AS source, organization_review_bookmark AS bookmark
         JOIN organization_review_sort AS sort ON sort.scope_slot = 'bookmark';",
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn refresh(connection: &Connection) -> Result<(), ScanError> {
    let record: Option<String> = connection
        .query_row(
            "SELECT record_id FROM organization_review_bookmark WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(record) = record else { return Ok(()) };
    let mut query = scope::load(connection, "bookmark")?
        .ok_or_else(crate::domain::organization_review::stale_session)?;
    query.organization_queue = None;
    let Some(current) = candidate(connection, &query, ReviewRead::Current(&record))? else {
        return Ok(());
    };
    connection.execute(
        "UPDATE organization_review_bookmark SET admission_id = ?1, root_id = ?2, location_id = ?3 WHERE singleton = 1",
        params![current.boundary.admission_id, current.boundary.root_id, current.boundary.location_id],
    ).map_err(database_error)?;
    ordering::save_cursor(connection, &current.boundary)
}

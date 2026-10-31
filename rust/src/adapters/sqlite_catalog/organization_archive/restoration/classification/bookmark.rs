use rusqlite::{Connection, OptionalExtension, params};

use crate::adapters::sqlite_catalog::classification::review::{position, scope};
use crate::adapters::sqlite_catalog::gallery::review::{ReviewRead, candidate};
use crate::domain::{GallerySortKey, ScanError};

use super::super::super::{database_error, error};

pub(in super::super) fn restore(
    connection: &Connection,
    archive: &str,
    base: Option<i64>,
) -> Result<(), ScanError> {
    if position::load(connection)?.is_none()
        && let Some(base) = base
    {
        import(connection, archive, base)?;
    }
    refresh(connection)
}

fn import(connection: &Connection, archive: &str, base: i64) -> Result<(), ScanError> {
    let payload: Option<String> = connection
        .query_row(
            "SELECT scope FROM recovery.archive_classification_bookmark",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(payload) = payload else {
        return Ok(());
    };
    let mut saved = scope::decode(&payload)?;
    if let Some(root) = &saved.gallery.root_id {
        saved.gallery.root_id = Some(connection.query_row(
            "SELECT COALESCE(current_root_id, retained_root_id) FROM temp.organization_restore_roots WHERE original_root_id = ?1",
            [root], |row| row.get(0),
        ).map_err(database_error)?);
    }
    let inserted = connection.execute(
        "INSERT INTO classification_review_bookmark(singleton, record_id, scope, admission_id, root_id, location_id,
         primary_missing, primary_text, primary_number, after_current, revision)
         SELECT 1, bookmark.record_id, ?1, admission.admission_id,
           COALESCE(root.current_root_id, root.retained_root_id), location.retained_id,
           bookmark.primary_missing, bookmark.primary_text,
           CASE WHEN ?2 THEN admission.admission_id ELSE bookmark.primary_number END,
           bookmark.after_current, ?3 + bookmark.revision
         FROM recovery.archive_classification_bookmark AS bookmark
         JOIN recovery.archive_admission AS original USING(admission_id)
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?4 AND generation.source_generation = original.source_generation
         JOIN organization_recovery_admissions AS admission ON admission.source_generation = generation.local_generation
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = bookmark.root_id
         JOIN temp.organization_restore_identifiers AS location
           ON location.kind = 'location' AND location.original_id = bookmark.location_id",
        params![scope::encode(&saved)?, saved.gallery.sort_key == GallerySortKey::LibraryAdmission, base, archive],
    ).map_err(database_error)?;
    if inserted != 1 {
        return Err(error(
            "classification_bookmark_invalid",
            "The classification bookmark could not be associated with its retained history",
        ));
    }
    Ok(())
}

fn refresh(connection: &Connection) -> Result<(), ScanError> {
    let Some(saved) = position::load(connection)? else {
        return Ok(());
    };
    let Some(current) = candidate(
        connection,
        &saved.scope.gallery,
        ReviewRead::Current(&saved.record_id),
    )?
    else {
        return Ok(());
    };
    let boundary = current.boundary;
    connection.execute(
        "UPDATE classification_review_bookmark SET admission_id = ?1, root_id = ?2, location_id = ?3,
         primary_missing = ?4, primary_text = ?5, primary_number = ?6 WHERE singleton = 1",
        params![boundary.admission_id, boundary.root_id, boundary.location_id, boundary.primary_missing, boundary.primary_text, boundary.primary_number],
    ).map_err(database_error)?;
    Ok(())
}

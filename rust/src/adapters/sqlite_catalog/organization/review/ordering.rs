use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{GalleryQuery, GallerySortDirection, GallerySortKey, ScanError};

use super::super::super::{database_error, gallery::review::ReviewBoundary};

pub(super) fn save_scope(connection: &Connection, query: &GalleryQuery) -> Result<(), ScanError> {
    let key = match query.sort_key {
        GallerySortKey::CaptureTime => "capture_time",
        GallerySortKey::CreatedTime => "created_time",
        GallerySortKey::ModifiedTime => "modified_time",
        GallerySortKey::FileName => "file_name",
        GallerySortKey::LibraryAdmission => "library_admission",
    };
    let direction = match query.order_direction() {
        GallerySortDirection::Ascending => "ascending",
        GallerySortDirection::Descending => "descending",
    };
    connection
        .execute(
            "INSERT INTO organization_review_sort VALUES ('active', ?1, ?2)
         ON CONFLICT(scope_slot) DO UPDATE SET
           sort_key = excluded.sort_key, sort_direction = excluded.sort_direction",
            params![key, direction],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn copy_to_bookmark(connection: &Connection) -> Result<(), ScanError> {
    connection.execute_batch(
        "INSERT INTO organization_review_sort
         SELECT 'bookmark', sort_key, sort_direction FROM organization_review_sort WHERE scope_slot = 'active'
         ON CONFLICT(scope_slot) DO UPDATE SET
           sort_key = excluded.sort_key, sort_direction = excluded.sort_direction;",
    ).map_err(database_error)
}

pub(super) fn load_scope(
    connection: &Connection,
    slot: &str,
    query: &mut GalleryQuery,
) -> Result<(), ScanError> {
    let stored: Option<(String, String)> = connection
        .query_row(
            "SELECT sort_key, sort_direction FROM organization_review_sort WHERE scope_slot = ?1",
            [slot],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    let Some((key, direction)) = stored else {
        return Err(super::super::super::database_error(
            rusqlite::Error::InvalidQuery,
        ));
    };
    query.sort_key = match key.as_str() {
        "capture_time" => GallerySortKey::CaptureTime,
        "created_time" => GallerySortKey::CreatedTime,
        "modified_time" => GallerySortKey::ModifiedTime,
        "file_name" => GallerySortKey::FileName,
        "library_admission" => GallerySortKey::LibraryAdmission,
        _ => return Err(database_error(rusqlite::Error::InvalidQuery)),
    };
    query.sort_direction = match direction.as_str() {
        "ascending" => GallerySortDirection::Ascending,
        "descending" => GallerySortDirection::Descending,
        _ => return Err(database_error(rusqlite::Error::InvalidQuery)),
    };
    Ok(())
}

pub(in super::super::super) fn save_cursor(
    connection: &Connection,
    boundary: &ReviewBoundary,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT INTO organization_review_cursor VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(singleton) DO UPDATE SET
           primary_missing = excluded.primary_missing,
           primary_text = excluded.primary_text, primary_number = excluded.primary_number",
            params![
                boundary.primary_missing,
                boundary.primary_text,
                boundary.primary_number
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::gallery_groups::GalleryGroupFilter;

use super::super::super::database_error;

pub(super) fn save(connection: &Connection, filter: GalleryGroupFilter) -> Result<(), ScanError> {
    let value = match filter {
        GalleryGroupFilter::Unrestricted => "unrestricted",
        GalleryGroupFilter::Exact => "exact",
        GalleryGroupFilter::Similar => "similar",
    };
    connection
        .execute(
            "INSERT INTO organization_review_group_filters VALUES ('active', ?1)
         ON CONFLICT(scope_slot) DO UPDATE SET group_filter = excluded.group_filter",
            params![value],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn copy_to_bookmark(connection: &Connection) -> Result<(), ScanError> {
    connection.execute_batch(
        "DELETE FROM organization_review_group_filters WHERE scope_slot = 'bookmark';
         INSERT INTO organization_review_group_filters
         SELECT 'bookmark', group_filter FROM organization_review_group_filters WHERE scope_slot = 'active';",
    ).map_err(database_error)
}

pub(super) fn load(connection: &Connection, slot: &str) -> Result<GalleryGroupFilter, ScanError> {
    let value: Option<String> = connection
        .query_row(
            "SELECT group_filter FROM organization_review_group_filters WHERE scope_slot = ?1",
            [slot],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    match value.as_deref() {
        None | Some("unrestricted") => Ok(GalleryGroupFilter::Unrestricted),
        Some("exact") => Ok(GalleryGroupFilter::Exact),
        Some("similar") => Ok(GalleryGroupFilter::Similar),
        _ => Err(ScanError::new(
            "catalog_review_group_filter_invalid",
            "Invalid saved group filter",
        )),
    }
}

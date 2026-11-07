use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::gallery_metadata_filter::GalleryMetadataFilter;

use super::super::super::database_error;
use super::super::super::metadata_filter_payload;

pub(super) fn save_scope(
    connection: &Connection,
    filter: &GalleryMetadataFilter,
) -> Result<(), ScanError> {
    filter.validate()?;
    let payload = metadata_filter_payload::encode(filter)?;
    connection.execute(
        "INSERT INTO organization_review_filters(scope_slot, version, payload) VALUES ('active', 2, ?1)
         ON CONFLICT(scope_slot) DO UPDATE SET version = excluded.version, payload = excluded.payload",
        [payload],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn copy_to_bookmark(connection: &Connection) -> Result<(), ScanError> {
    connection.execute_batch(
        "INSERT INTO organization_review_filters
         SELECT 'bookmark', version, payload FROM organization_review_filters WHERE scope_slot = 'active'
         ON CONFLICT(scope_slot) DO UPDATE SET version = excluded.version, payload = excluded.payload;",
    ).map_err(database_error)
}

pub(super) fn load_scope(
    connection: &Connection,
    slot: &str,
) -> Result<GalleryMetadataFilter, ScanError> {
    let (version, payload): (i64, String) = connection
        .query_row(
            "SELECT version, payload FROM organization_review_filters WHERE scope_slot = ?1",
            params![slot],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(database_error)?;
    metadata_filter_payload::decode(&payload, version)
}

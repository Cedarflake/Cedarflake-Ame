use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

const INDEX_NAME: &str = "asset_locations_pending_metadata";
const INDEX_DDL: &str = "CREATE INDEX asset_locations_pending_metadata
    ON asset_locations(root_id, scan_id, relative_path)
    WHERE metadata_engine_id = 'ame-invalidated-media-metadata'";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "index", INDEX_NAME, INDEX_DDL)? {
        return Err(ScanError::new(
            "catalog_metadata_recheck_index_invalid",
            "Pending metadata recovery requires its bounded candidate index",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

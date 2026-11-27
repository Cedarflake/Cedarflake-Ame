use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape;

const NAME: &str = "exact_duplicate_preferred_location";
const INDEX: &str = "CREATE INDEX exact_duplicate_preferred_location
    ON exact_duplicate_decisions(preferred_location_id, preferred_asset_id)
    WHERE preferred_location_id IS NOT NULL";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_shape::schema_object_sql_matches(connection, "index", NAME, INDEX)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_preferred_copy_index_invalid",
        "Preferred-copy reads require their location and asset lookup index",
    ))
}

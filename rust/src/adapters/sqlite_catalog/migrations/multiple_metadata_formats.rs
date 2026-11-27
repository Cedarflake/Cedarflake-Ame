use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape;

const TABLE: &str = "CREATE TABLE organization_review_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version = 2),
    payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 4096 AND json_valid(payload))
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_review_filters",
        TABLE,
    )? {
        return Err(ScanError::new(
            "catalog_review_filter_invalid",
            "Organization scopes require versioned format selections",
        ));
    }
    Ok(())
}

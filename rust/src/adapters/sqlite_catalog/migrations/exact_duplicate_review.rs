use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

// User choices deliberately have no cascading relationship to derived evidence or locations.
const TABLE: &str = "CREATE TABLE exact_duplicate_decisions (
    scheme TEXT NOT NULL,
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
    preferred_location_id TEXT,
    preferred_asset_id TEXT,
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    updated_unix_ms INTEGER NOT NULL,
    CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)),
    PRIMARY KEY(scheme, digest, byte_length)
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "table", "exact_duplicate_decisions", TABLE)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_duplicate_decisions_contract_invalid",
        "Exact duplicate choices require their durable content-keyed table",
    ))
}

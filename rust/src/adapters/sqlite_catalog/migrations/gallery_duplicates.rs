use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

const TABLE: &str = "CREATE TABLE gallery_content_fingerprints (
    source_generation INTEGER NOT NULL,
    scheme TEXT NOT NULL,
    PRIMARY KEY(source_generation, scheme),
    FOREIGN KEY(source_generation, scheme)
      REFERENCES content_fingerprints(source_generation, scheme)
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "table", "gallery_content_fingerprints", TABLE)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_gallery_duplicates_contract_invalid",
        "Gallery grouping requires an atomically published fingerprint projection",
    ))
}

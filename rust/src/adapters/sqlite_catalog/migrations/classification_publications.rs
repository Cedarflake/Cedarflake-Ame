use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "classification_source_publications",
        "CREATE TABLE classification_source_publications (
            publication_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(publication_id > 0),
            root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE,
            root_generation INTEGER NOT NULL CHECK(root_generation > 0),
            scan_id TEXT NOT NULL,
            after_source_generation INTEGER NOT NULL DEFAULT 0 CHECK(after_source_generation >= 0)
        ) STRICT",
    ),
    (
        "index",
        "asset_locations_classification_publication",
        "CREATE INDEX asset_locations_classification_publication
         ON asset_locations(root_id, scan_id, source_generation)",
    ),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_row: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_source_publications'))
         OR EXISTS(SELECT 1 FROM classification_source_publications CROSS JOIN catalog_state
             WHERE after_source_generation >= next_source_generation)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_row {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_publication_invalid",
        "Classification source publication records are inconsistent",
    )
}

#[cfg(test)]
mod tests;

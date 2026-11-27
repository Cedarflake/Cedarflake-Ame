use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const DOCUMENT_COLUMN: &str = "gallery_search_id INTEGER CHECK(gallery_search_id IS NULL OR (
    typeof(gallery_search_id) = 'integer' AND gallery_search_id > 0
))";
const DOCUMENT_INDEX: &str =
    "CREATE UNIQUE INDEX asset_locations_gallery_search ON asset_locations(gallery_search_id)";
const TERMS: &str = "CREATE VIRTUAL TABLE gallery_search_terms USING fts5(
    absolute_path, relative_path, tokenize='trigram', content='', contentless_delete=1,
    detail=none
)";
const INSERT_TRIGGER: &str = "CREATE TRIGGER gallery_search_insert
AFTER INSERT ON asset_locations BEGIN
    UPDATE asset_locations SET gallery_search_id = (
      SELECT COALESCE(MAX(gallery_search_id), 0) + 1 FROM asset_locations
    ) WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
    INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
      SELECT gallery_search_id, NEW.absolute_path, NEW.relative_path FROM asset_locations
      WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
END";
const UPDATE_TRIGGER: &str = "CREATE TRIGGER gallery_search_update
AFTER UPDATE OF absolute_path, relative_path ON asset_locations
WHEN OLD.absolute_path IS NOT NEW.absolute_path OR OLD.relative_path IS NOT NEW.relative_path
BEGIN
    UPDATE gallery_search_terms SET absolute_path = NEW.absolute_path, relative_path = NEW.relative_path
      WHERE rowid = NEW.gallery_search_id;
END";
const DELETE_TRIGGER: &str = "CREATE TRIGGER gallery_search_delete
BEFORE DELETE ON asset_locations BEGIN
    DELETE FROM gallery_search_terms WHERE rowid = OLD.gallery_search_id;
END";
const IDENTITY_TRIGGER: &str = "CREATE TRIGGER gallery_search_identity
BEFORE UPDATE OF gallery_search_id ON asset_locations
WHEN OLD.gallery_search_id IS NOT NULL AND OLD.gallery_search_id IS NOT NEW.gallery_search_id
BEGIN SELECT RAISE(ABORT, 'catalog search identity is immutable'); END";

const OBJECTS: [(&str, &str, &str); 6] = [
    ("index", "asset_locations_gallery_search", DOCUMENT_INDEX),
    ("table", "gallery_search_terms", TERMS),
    ("trigger", "gallery_search_insert", INSERT_TRIGGER),
    ("trigger", "gallery_search_update", UPDATE_TRIGGER),
    ("trigger", "gallery_search_delete", DELETE_TRIGGER),
    ("trigger", "gallery_search_identity", IDENTITY_TRIGGER),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, sql)? {
            return Err(invalid());
        }
    }
    let sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'asset_locations'",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !schema_shape::normalize_schema_sql(&sql)
        .contains(&schema_shape::normalize_schema_sql(DOCUMENT_COLUMN))
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let missing: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_locations WHERE gallery_search_id IS NULL)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if missing {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_search_index_invalid",
        "The catalog text search index requires its canonical identity, table and triggers",
    )
}

#[cfg(test)]
mod tests;

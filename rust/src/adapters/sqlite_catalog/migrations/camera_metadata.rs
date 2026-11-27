use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const MAKE_COLUMN: &str = "camera_make TEXT CHECK(camera_make IS NULL OR (
    typeof(camera_make) = 'text' AND length(CAST(camera_make AS BLOB)) BETWEEN 1 AND 128
))";
const MODEL_COLUMN: &str = "camera_model TEXT CHECK(camera_model IS NULL OR (
    typeof(camera_model) = 'text' AND length(CAST(camera_model AS BLOB)) BETWEEN 1 AND 128
))";

const TABLES: [&str; 3] = [
    "asset_locations",
    "library_change_scan_handoff_items",
    "library_change_catch_up_handoffs",
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for table in TABLES {
        let sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let sql = schema_shape::normalize_schema_sql(&sql);
        if ![MAKE_COLUMN, MODEL_COLUMN]
            .iter()
            .all(|column| sql.contains(&schema_shape::normalize_schema_sql(column)))
        {
            return Err(ScanError::new(
                "catalog_camera_metadata_invalid",
                "The camera metadata columns require their bounded text contract",
            ));
        }
    }
    Ok(())
}

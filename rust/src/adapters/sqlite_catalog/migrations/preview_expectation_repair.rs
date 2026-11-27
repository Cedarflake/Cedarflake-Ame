use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::database_error;
use super::schema_shape::table_columns_match;

pub(super) fn marker_is_complete(connection: &Connection) -> Result<bool, ScanError> {
    let marker_type = connection
        .query_row(
            "SELECT type FROM sqlite_master
             WHERE name = 'library_change_preview_repair_contract'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(marker_type) = marker_type {
        let marker_valid = marker_type == "table"
            && table_columns_match(
                connection,
                "library_change_preview_repair_contract",
                &[
                    ("singleton", "INTEGER", false, 1),
                    ("complete", "INTEGER", true, 0),
                ],
            )?
            && connection
                .query_row(
                    "SELECT singleton = 1 AND complete = 1
                     FROM library_change_preview_repair_contract",
                    [],
                    |row| row.get::<_, bool>(0),
                )
                .optional()
                .map_err(database_error)?
                .unwrap_or(false);
        if !marker_valid {
            return Err(ScanError::new(
                "catalog_change_catch_up_contract_unverifiable",
                "The catalog preview repair marker is malformed",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests;

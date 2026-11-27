use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const OBJECTS: &[(&str, &str)] = &[
    (
        "organization_review_sort",
        "CREATE TABLE organization_review_sort (
           scope_slot TEXT PRIMARY KEY NOT NULL
             REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
           sort_key TEXT NOT NULL CHECK(sort_key IN (
             'capture_time', 'created_time', 'modified_time', 'file_name', 'library_admission')),
           sort_direction TEXT NOT NULL CHECK(sort_direction IN ('ascending', 'descending'))
         )",
    ),
    (
        "organization_review_cursor",
        "CREATE TABLE organization_review_cursor (
           singleton INTEGER PRIMARY KEY CHECK(singleton = 1)
             REFERENCES organization_review_bookmark(singleton) ON DELETE CASCADE,
           primary_missing INTEGER NOT NULL CHECK(primary_missing IN (0, 1)),
           primary_text TEXT NOT NULL,
           primary_number INTEGER NOT NULL
         )",
    ),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, ddl) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, "table", name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let incomplete: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_review_scopes AS scope
           LEFT JOIN organization_review_sort AS sort ON sort.scope_slot = scope.slot WHERE sort.scope_slot IS NULL)
         OR EXISTS(SELECT 1 FROM organization_review_bookmark AS bookmark
           LEFT JOIN organization_review_cursor AS cursor USING(singleton) WHERE cursor.singleton IS NULL)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if incomplete {
        return Err(invalid());
    }
    for (table, _) in OBJECTS {
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_review_order_invalid",
        "Organization ordering requires valid saved scopes and cursors",
    )
}

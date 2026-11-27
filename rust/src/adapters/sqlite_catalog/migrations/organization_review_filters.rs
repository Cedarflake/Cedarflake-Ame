use rusqlite::Connection;

use super::super::metadata_filter_payload;
use crate::domain::ScanError;

use super::database_error;

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let incomplete: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_review_scopes AS scope
           LEFT JOIN organization_review_filters AS filters ON filters.scope_slot = scope.slot
           WHERE filters.scope_slot IS NULL)
         OR EXISTS(SELECT 1 FROM pragma_foreign_key_check('organization_review_filters'))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if incomplete {
        return Err(invalid());
    }
    let mut statement = connection
        .prepare("SELECT version, payload FROM organization_review_filters")
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    for row in rows {
        let (version, payload) = row.map_err(database_error)?;
        metadata_filter_payload::decode(&payload, version)?;
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_review_filter_invalid",
        "Organization scopes require complete, valid metadata filters",
    )
}

use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape;

const TABLE: &str = "CREATE TABLE organization_review_group_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    group_filter TEXT NOT NULL CHECK(group_filter IN ('unrestricted', 'exact', 'similar'))
) STRICT";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_review_group_filters",
        TABLE,
    )? {
        return Err(ScanError::new(
            "catalog_review_group_filter_invalid",
            "The saved organization group filters have an invalid schema",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {}

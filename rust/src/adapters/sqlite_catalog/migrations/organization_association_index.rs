use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

mod schema;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_rows: bool = connection.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM organization_records AS record
           JOIN organization_record_groups AS member USING(record_id)
           WHERE record.unassociated_since_unix_ms IS NOT NULL
             AND NOT EXISTS(SELECT 1 FROM organization_unassociated_records AS indexed
               WHERE indexed.record_id = member.record_id AND indexed.group_id = member.group_id)
         ) OR EXISTS(
           SELECT 1 FROM organization_unassociated_records AS indexed
           LEFT JOIN organization_records AS record USING(record_id)
           LEFT JOIN organization_record_groups AS member USING(record_id)
           WHERE record.unassociated_since_unix_ms IS NULL OR member.group_id IS NOT indexed.group_id
         )",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_rows {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_association_index_invalid",
        "Organization association lookup must match retained records and group ownership",
    )
}

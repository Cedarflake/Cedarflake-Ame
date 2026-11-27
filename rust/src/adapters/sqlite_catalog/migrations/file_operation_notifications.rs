use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::file_operation_journal::invalid;

use super::schema_shape;

const INDEX: &str = "CREATE INDEX file_operation_removal_source ON file_operation_items(
    json_extract(authorization, '$.catalog.location_id'),
    json_extract(authorization, '$.catalog.source_generation'))
    WHERE phase IN ('executing', 'native_completed', 'uncertain')
      AND json_extract(authorization, '$.action') IN ('recycle', 'delete_permanently')";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "index",
        "file_operation_removal_source",
        INDEX,
    )? {
        return Err(invalid("The operation notification index is invalid"));
    }
    Ok(())
}

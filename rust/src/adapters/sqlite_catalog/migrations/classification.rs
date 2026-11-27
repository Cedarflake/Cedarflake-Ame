use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

mod schema;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_content_rows(connection: &Connection) -> Result<(), ScanError> {
    let present: bool = connection
        .query_row(
            "SELECT COUNT(*) = 1 FROM classification_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !present {
        return Err(invalid());
    }
    super::super::classification::validation::catalog(connection)
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_classification_invalid",
        "Classification records require a valid schema and complete relationships",
    )
}

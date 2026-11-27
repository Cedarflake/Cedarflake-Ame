use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const NAME: &str = "organization_admission_clock";
const TABLE: &str = "CREATE TABLE organization_admission_clock (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    restored_prefix INTEGER NOT NULL CHECK (restored_prefix >= 0)
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if schema_shape::schema_object_sql_matches(connection, "table", NAME, TABLE)? {
        return Ok(());
    }
    Err(invalid())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let valid: bool = connection
        .query_row(
            "SELECT COUNT(*) = 1 AND COALESCE(MIN(singleton = 1 AND restored_prefix >= 0
           AND restored_prefix <= (SELECT COALESCE(MAX(value), 0) FROM (
             SELECT seq AS value FROM sqlite_sequence WHERE name = 'organization_admissions'
             UNION ALL SELECT MAX(admission_id) FROM organization_recovery_admissions))), 0)
         FROM organization_admission_clock",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if valid { Ok(()) } else { Err(invalid()) }
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_admission_clock_invalid",
        "The organization admission boundary requires its canonical clock",
    )
}

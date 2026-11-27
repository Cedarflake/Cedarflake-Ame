use rusqlite::Connection;

use super::schema_shape::schema_object_sql_matches;
use crate::domain::ScanError;

const TABLE: &str = "scan_directory_journal_authority";
const DDL: &str = "CREATE TABLE scan_directory_journal_authority (
    scan_id TEXT PRIMARY KEY NOT NULL,
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    journal_id BLOB NOT NULL CHECK(typeof(journal_id) = 'blob' AND length(journal_id) = 8
        AND journal_id != X'0000000000000000'),
    covered_until_usn INTEGER NOT NULL CHECK(covered_until_usn >= 0),
    FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "table", TABLE, DDL)? {
        return Err(ScanError::new(
            "catalog_journal_directory_contract_invalid",
            "Journal directory coverage requires its canonical schema",
        ));
    }
    Ok(())
}

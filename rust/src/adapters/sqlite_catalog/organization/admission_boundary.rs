use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::{database_error, sqlite_integer, sqlite_unsigned};

pub(in crate::adapters::sqlite_catalog) fn load(connection: &Connection) -> Result<u64, ScanError> {
    let boundary: i64 = connection.query_row(
        "SELECT COALESCE(MAX(value), 0) - (SELECT restored_prefix FROM organization_admission_clock)
         FROM (SELECT seq AS value FROM sqlite_sequence WHERE name = 'organization_admissions'
           UNION ALL SELECT MAX(admission_id) FROM organization_recovery_admissions)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    sqlite_unsigned(boundary, "new-arrival boundary")
}

pub(in crate::adapters::sqlite_catalog) fn resolve(
    connection: &Connection,
    boundary: u64,
) -> Result<i64, ScanError> {
    sqlite_integer(boundary, "new-arrival boundary")?
        .checked_add(restored_prefix(connection)?)
        .ok_or_else(capacity_error)
}

pub(in crate::adapters::sqlite_catalog) fn prepend(
    connection: &Connection,
    count: i64,
) -> Result<(), ScanError> {
    let prefix = restored_prefix(connection)?
        .checked_add(count)
        .ok_or_else(capacity_error)?;
    connection
        .execute(
            "UPDATE organization_admission_clock SET restored_prefix = ?1 WHERE singleton = 1",
            [prefix],
        )
        .map_err(database_error)?;
    Ok(())
}

fn restored_prefix(connection: &Connection) -> Result<i64, ScanError> {
    connection
        .query_row(
            "SELECT restored_prefix FROM organization_admission_clock WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

fn capacity_error() -> ScanError {
    ScanError::new(
        "organization_admission_limit",
        "The organization admission boundary has reached its limit",
    )
}

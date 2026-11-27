use rusqlite::Connection;

use crate::domain::ScanError;

use super::organization_reconciliation::{GENERATION_INDEX, ROOT_GENERATION_INDEX};
use super::{database_error, schema_shape};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let retired_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [GENERATION_INDEX.0],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let root_index_matches = schema_shape::schema_object_sql_matches(
        connection,
        "index",
        ROOT_GENERATION_INDEX.0,
        ROOT_GENERATION_INDEX.1,
    )?;
    if retired_exists || !root_index_matches {
        return Err(ScanError::new(
            "catalog_organization_index_contract_invalid",
            "Organization publication requires a root keyset index and reuses the exact source index",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

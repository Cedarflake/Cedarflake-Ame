use rusqlite::Connection;

use crate::domain::ScanError;

use super::database_error;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE name = 'asset_locations_exact_cursor')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        return Err(ScanError::new(
            "catalog_exact_candidate_index_contract_invalid",
            "Exact candidates must reuse the catalog location cursor index",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

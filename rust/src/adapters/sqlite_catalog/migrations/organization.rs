use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape::schema_object_sql_matches};

mod schema;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in schema::OBJECTS {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    let has_state: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_state WHERE singleton = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !has_state {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_contract_invalid",
        "Durable organization requires its versioned records, bindings and operation receipts",
    )
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, table, _) in schema::OBJECTS
        .iter()
        .filter(|(kind, _, _)| *kind == "table")
    {
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    let invalid_revision: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_records AS record, organization_state AS state
           WHERE record.organization_revision > state.revision OR record.favorite_revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operations AS operation, organization_state AS state
           WHERE operation.revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operation_changes AS change, organization_state AS state
           WHERE change.applied_revision > state.revision)", [], |row| row.get(0)
    ).map_err(database_error)?;
    if invalid_revision {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests;

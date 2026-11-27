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
    for (_, table, _) in schema::OBJECTS
        .iter()
        .filter(|(kind, _, _)| *kind == "table")
    {
        let invalid: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if invalid {
            return Err(self::invalid());
        }
    }
    let invalid: bool = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM organization_history_state) <> 1
         OR EXISTS(SELECT 1 FROM organization_history_state AS history, organization_state AS state
           WHERE history.started_revision > state.revision
             OR history.cursor > COALESCE((SELECT MAX(position) FROM organization_history_entries), 0))
         OR (SELECT COUNT(*) FROM organization_history_entries)
             <> COALESCE((SELECT MAX(position) FROM organization_history_entries), 0)
         OR EXISTS(SELECT 1 FROM organization_field_authority AS authority
           JOIN organization_operation_changes AS change USING(operation_id, record_id, field)
           WHERE authority.applied_revision <> change.applied_revision)
         OR EXISTS(SELECT 1 FROM organization_album_authority AS authority
           JOIN organization_album_operation_changes AS change USING(operation_id)
           WHERE authority.applied_revision <> change.applied_revision OR authority.album_id <> change.album_id)
         OR EXISTS(SELECT 1 FROM organization_operation_changes AS change, organization_history_state AS history
           WHERE change.applied_revision > history.started_revision
             AND NOT EXISTS(SELECT 1 FROM organization_field_authority AS authority
               WHERE authority.operation_id = change.operation_id AND authority.record_id = change.record_id
                 AND authority.field = change.field))
         OR EXISTS(SELECT 1 FROM organization_album_operation_changes AS change, organization_history_state AS history
           WHERE change.applied_revision > history.started_revision
             AND NOT EXISTS(SELECT 1 FROM organization_album_authority AS authority
               WHERE authority.operation_id = change.operation_id))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid {
        return Err(self::invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_history_invalid",
        "Organization history must retain its cursor and operation authority",
    )
}

#[cfg(test)]
mod tests;

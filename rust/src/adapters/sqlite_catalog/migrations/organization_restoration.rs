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
    for (kind, name, _) in schema::OBJECTS {
        if *kind != "table" {
            continue;
        }
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    let invalid_ownership: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_recovery_admissions AS history
           WHERE NOT EXISTS(SELECT 1 FROM organization_recovery_admission_sources AS owner
             WHERE owner.source_generation = history.source_generation))
         OR EXISTS(SELECT 1 FROM organization_recovery_admissions AS history
           JOIN organization_admissions AS current USING(admission_id))
         OR EXISTS(SELECT 1 FROM (
           SELECT root_id FROM organization_recovery_admission_sources
           UNION SELECT root_id FROM organization_recovery_duplicate_sources
           UNION SELECT root_id FROM organization_recovery_versions
         ) AS source WHERE NOT EXISTS(SELECT 1 FROM library_roots AS current WHERE current.id = source.root_id)
           AND NOT EXISTS(SELECT 1 FROM organization_recovery_roots AS retained WHERE retained.root_id = source.root_id))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_ownership {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_restoration_invalid",
        "Organization restoration requires valid recovery records and associations",
    )
}

#[cfg(test)]
mod tests;

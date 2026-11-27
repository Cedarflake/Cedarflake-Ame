use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "organization_admissions",
        "CREATE TABLE organization_admissions (
           admission_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(admission_id > 0),
           source_generation INTEGER NOT NULL UNIQUE CHECK(source_generation > 0)
         )",
    ),
    (
        "table",
        "organization_admission_roots",
        "CREATE TABLE organization_admission_roots (
           root_key INTEGER PRIMARY KEY CHECK(root_key > 0),
           root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE
         )",
    ),
    (
        "table",
        "organization_admission_sources",
        "CREATE TABLE organization_admission_sources (
           admission_id INTEGER NOT NULL REFERENCES organization_admissions(admission_id) ON DELETE CASCADE,
           root_key INTEGER NOT NULL REFERENCES organization_admission_roots(root_key) ON DELETE CASCADE,
           PRIMARY KEY(admission_id, root_key)
         ) WITHOUT ROWID",
    ),
    (
        "index",
        "organization_admission_sources_root",
        "CREATE INDEX organization_admission_sources_root
           ON organization_admission_sources(root_key, admission_id)",
    ),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, name, _) in OBJECTS.iter().filter(|(kind, _, _)| *kind == "table") {
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
    let unowned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_admissions AS admitted
             WHERE NOT EXISTS (
               SELECT 1 FROM organization_admission_sources AS owner
               WHERE owner.admission_id = admitted.admission_id
             ))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if unowned {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_admission_invalid",
        "Organization admission requires ordered versions with configured source ownership",
    )
}

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "classification_source_publications",
        "CREATE TABLE classification_source_publications (
            publication_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(publication_id > 0),
            root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE,
            root_generation INTEGER NOT NULL CHECK(root_generation > 0),
            scan_id TEXT NOT NULL,
            after_source_generation INTEGER NOT NULL DEFAULT 0 CHECK(after_source_generation >= 0)
        ) STRICT",
    ),
    (
        "index",
        "asset_locations_classification_publication",
        "CREATE INDEX asset_locations_classification_publication
         ON asset_locations(root_id, scan_id, source_generation)",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 68)?;
    for (_, _, statement) in OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO classification_source_publications(root_id, root_generation, scan_id)
         SELECT root.id, state.generation, root.active_scan_id FROM library_roots AS root
         JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
         WHERE root.active_scan_id IS NOT NULL;
         UPDATE schema_info SET version = 69; PRAGMA user_version = 69;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 69)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_row: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_source_publications'))
         OR EXISTS(SELECT 1 FROM classification_source_publications CROSS JOIN catalog_state
             WHERE after_source_generation >= next_source_generation)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_row {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_publication_invalid",
        "Classification source publication records are inconsistent",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v68_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() != 69 {
        return;
    }
    connection
        .execute_batch(
            "DROP INDEX asset_locations_classification_publication;
         DROP TABLE classification_source_publications;
         UPDATE schema_info SET version = 68; PRAGMA user_version = 68;",
        )
        .unwrap();
}

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "classification_unavailable_sources",
        "CREATE TABLE classification_unavailable_sources (
        failure_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(failure_id > 0),
        work_id INTEGER NOT NULL REFERENCES classification_work_items(work_id) ON DELETE CASCADE,
        root_id TEXT NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
        root_generation INTEGER NOT NULL CHECK(root_generation > 0),
        asset_id TEXT NOT NULL CHECK(length(asset_id) BETWEEN 1 AND 128),
        location_id TEXT NOT NULL CHECK(length(location_id) BETWEEN 1 AND 128),
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        UNIQUE(work_id, source_generation, location_id)
    ) STRICT",
    ),
    (
        "index",
        "classification_unavailable_root",
        "CREATE INDEX classification_unavailable_root
        ON classification_unavailable_sources(root_id, root_generation, failure_id)",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 67)?;
    for (_, _, statement) in OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "UPDATE classification_work_items SET phase = 'queued', ready_unix_ms = 0 WHERE phase = 'waiting';
             UPDATE schema_info SET version = 68; PRAGMA user_version = 68;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 68)
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
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_unavailable_sources'))
         OR EXISTS(SELECT 1 FROM classification_unavailable_sources AS unavailable
             JOIN classification_work_items AS work USING(work_id)
             CROSS JOIN catalog_state
             WHERE unavailable.source_generation >= next_source_generation
                OR (work.kind = 'source' AND unavailable.source_generation <> work.source_generation))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_row {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_availability_invalid",
        "Classification source availability records are inconsistent",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v67_for_test(connection: &Connection) {
    super::classification_publications::downgrade_to_v68_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() != 68 {
        return;
    }
    connection
        .execute_batch(
            "DROP TABLE classification_unavailable_sources;
         UPDATE schema_info SET version = 67; PRAGMA user_version = 67;",
        )
        .unwrap();
}

#[cfg(test)]
mod tests;

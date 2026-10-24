use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "organization_review_scopes",
        "CREATE TABLE organization_review_scopes (
       slot TEXT PRIMARY KEY NOT NULL CHECK(slot IN ('active', 'bookmark')),
       collection TEXT NOT NULL CHECK(collection IN ('library', 'favorites', 'album')),
       album_id TEXT,
       root_id TEXT,
       folder_relative_path TEXT,
       include_descendants INTEGER NOT NULL CHECK(include_descendants IN (0, 1)),
       search_text TEXT NOT NULL CHECK(length(search_text) <= 512),
       queue TEXT NOT NULL CHECK(queue IN ('pending', 'deferred', 'ignored')),
       CHECK((collection = 'album') = (album_id IS NOT NULL))
     )",
    ),
    (
        "table",
        "organization_review_session",
        "CREATE TABLE organization_review_session (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       scope_slot TEXT NOT NULL DEFAULT 'active' CHECK(scope_slot = 'active')
         REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
       session_id TEXT NOT NULL,
       position_sequence INTEGER NOT NULL CHECK(position_sequence >= 0),
       checkpoint_digest TEXT CHECK(length(checkpoint_digest) = 64)
     )",
    ),
    (
        "table",
        "organization_review_bookmark",
        "CREATE TABLE organization_review_bookmark (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       scope_slot TEXT NOT NULL DEFAULT 'bookmark' CHECK(scope_slot = 'bookmark')
         REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
       record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
       admission_id INTEGER NOT NULL CHECK(admission_id > 0),
       root_id TEXT NOT NULL,
       location_id TEXT NOT NULL,
       after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
     )",
    ),
    (
        "table",
        "organization_review_operations",
        "CREATE TABLE organization_review_operations (
       operation_id TEXT PRIMARY KEY NOT NULL
         REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
       session_id TEXT NOT NULL,
       position_sequence INTEGER NOT NULL CHECK(position_sequence > 0),
       committed_sequence INTEGER NOT NULL CHECK(committed_sequence = position_sequence + 1)
     )",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 51)?;
    for (_, _, ddl) in OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 52; PRAGMA user_version = 52;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 52)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, name, _) in OBJECTS {
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
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_review_invalid",
        "Organization continuation requires valid scoped session and lineage records",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v51_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 52 {
        for (kind, name, _) in OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 51; PRAGMA user_version = 51;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

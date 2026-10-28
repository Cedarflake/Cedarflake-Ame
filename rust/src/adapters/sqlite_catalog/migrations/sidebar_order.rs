use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "library_sidebar_root_order",
        "CREATE TABLE library_sidebar_root_order (
           root_id TEXT PRIMARY KEY NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
           position INTEGER NOT NULL CHECK(typeof(position) = 'integer' AND position >= 0)
         )",
    ),
    (
        "index",
        "library_sidebar_root_position",
        "CREATE INDEX library_sidebar_root_position ON library_sidebar_root_order(position, root_id)",
    ),
    (
        "table",
        "organization_sidebar_album_order",
        "CREATE TABLE organization_sidebar_album_order (
           album_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_albums(album_id) ON DELETE CASCADE,
           position INTEGER NOT NULL CHECK(typeof(position) = 'integer' AND position >= 0)
         )",
    ),
    (
        "index",
        "organization_sidebar_album_position",
        "CREATE INDEX organization_sidebar_album_position ON organization_sidebar_album_order(position, album_id)",
    ),
    (
        "trigger",
        "library_sidebar_root_append",
        "CREATE TRIGGER library_sidebar_root_append AFTER INSERT ON library_roots BEGIN
           INSERT INTO library_sidebar_root_order(root_id, position)
             SELECT NEW.id, COALESCE(MAX(position) + 1, 0) FROM library_sidebar_root_order;
         END",
    ),
    (
        "trigger",
        "organization_sidebar_album_append",
        "CREATE TRIGGER organization_sidebar_album_append AFTER INSERT ON organization_albums BEGIN
           INSERT INTO organization_sidebar_album_order(album_id, position)
             SELECT NEW.album_id, COALESCE(MAX(position) + 1, 0) FROM organization_sidebar_album_order;
         END",
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
    current_schema::validate_schema_version(transaction, 57)?;
    for (_, _, ddl) in OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO library_sidebar_root_order
               SELECT id, ROW_NUMBER() OVER(ORDER BY created_unix_ms, id) - 1 FROM library_roots;
             INSERT INTO organization_sidebar_album_order
               SELECT album_id, ROW_NUMBER() OVER(ORDER BY album_id) - 1 FROM organization_albums;
             UPDATE schema_info SET version = 58; PRAGMA user_version = 58;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 58)
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
    let incomplete: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots AS root
               LEFT JOIN library_sidebar_root_order AS ordering ON ordering.root_id = root.id
               WHERE ordering.root_id IS NULL)
             OR EXISTS(SELECT 1 FROM organization_albums AS album
               LEFT JOIN organization_sidebar_album_order AS ordering USING(album_id)
               WHERE ordering.album_id IS NULL)
             OR EXISTS(SELECT 1 FROM pragma_foreign_key_check('library_sidebar_root_order'))
             OR EXISTS(SELECT 1 FROM pragma_foreign_key_check('organization_sidebar_album_order'))
             OR EXISTS(SELECT position FROM library_sidebar_root_order GROUP BY position HAVING COUNT(*) > 1)
             OR EXISTS(SELECT position FROM organization_sidebar_album_order GROUP BY position HAVING COUNT(*) > 1)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if incomplete {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_sidebar_order_invalid",
        "Sidebar ordering requires complete, distinct positions for its items",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v57_for_test(connection: &Connection) {
    super::preferred_copy_index::downgrade_to_v58_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 58 {
        for (kind, name, _) in OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 57; PRAGMA user_version = 57;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

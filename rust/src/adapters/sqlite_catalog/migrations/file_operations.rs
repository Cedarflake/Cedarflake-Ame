use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::invalid;

use super::{current_schema, database_error, schema_shape};

const ITEMS: &str = "CREATE TABLE file_operation_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    authorization TEXT NOT NULL CHECK(length(authorization) <= 524288 AND json_valid(authorization)),
    version INTEGER NOT NULL CHECK(version >= 0),
    phase TEXT NOT NULL CHECK(phase IN ('prepared', 'executing', 'name_occupied', 'copied',
        'removing_source', 'native_completed', 'published', 'failed', 'cancelled', 'uncertain')),
    UNIQUE(batch_id, ordinal)
)";

const EVENTS: &str = "CREATE TABLE file_operation_events (
    item_id INTEGER NOT NULL REFERENCES file_operation_items(id),
    version INTEGER NOT NULL CHECK(version >= 0),
    snapshot TEXT NOT NULL CHECK(length(snapshot) <= 524288 AND json_valid(snapshot)
        AND json_extract(snapshot, '$.version') = version),
    recorded_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(item_id, version)
)";

const PENDING: &str = "CREATE INDEX file_operation_pending ON file_operation_items(id)
    WHERE phase IN ('prepared', 'executing', 'name_occupied', 'copied', 'removing_source', 'native_completed', 'uncertain')";
const IMMUTABLE_ITEMS: &str = "CREATE TRIGGER file_operation_authorization_immutable
    BEFORE UPDATE OF id, batch_id, ordinal, authorization ON file_operation_items
    BEGIN SELECT RAISE(ABORT, 'file operation authorization is immutable'); END";
const IMMUTABLE_EVENTS: &str = "CREATE TRIGGER file_operation_events_immutable
    BEFORE UPDATE ON file_operation_events
    BEGIN SELECT RAISE(ABORT, 'file operation events are immutable'); END";
const RETAIN_EVENTS: &str = "CREATE TRIGGER file_operation_events_retained
    BEFORE DELETE ON file_operation_events
    BEGIN SELECT RAISE(ABORT, 'file operation history is durable'); END";

const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "file_operation_items", ITEMS),
    ("table", "file_operation_events", EVENTS),
    ("index", "file_operation_pending", PENDING),
    (
        "trigger",
        "file_operation_authorization_immutable",
        IMMUTABLE_ITEMS,
    ),
    (
        "trigger",
        "file_operation_events_immutable",
        IMMUTABLE_EVENTS,
    ),
    ("trigger", "file_operation_events_retained", RETAIN_EVENTS),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 78)?;
    for (_, _, sql) in OBJECTS {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 79; PRAGMA user_version = 79;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 79)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, sql)? {
            return Err(invalid(format!(
                "The operation journal schema is invalid: {name}"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    super::super::file_operations::validation::validate_history(connection)
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 79 {
        connection
            .execute_batch(
                "DROP TRIGGER file_operation_events_retained;
             DROP TABLE file_operation_events;
             DROP TABLE file_operation_items;
             UPDATE schema_info SET version = 78; PRAGMA user_version = 78;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape::schema_object_sql_matches};

const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "organization_record_groups", "CREATE TABLE organization_record_groups (
        record_id TEXT PRIMARY KEY NOT NULL,
        group_id TEXT NOT NULL,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(group_id) REFERENCES organization_records(record_id)
    )"),
    ("index", "organization_record_group_members", "CREATE INDEX organization_record_group_members
        ON organization_record_groups(group_id, record_id)"),
    ("table", "organization_album_retirements", "CREATE TABLE organization_album_retirements (
        album_id TEXT PRIMARY KEY NOT NULL,
        revision INTEGER NOT NULL CHECK(revision > 0),
        retired_unix_ms INTEGER NOT NULL CHECK(retired_unix_ms >= 0),
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("table", "organization_album_operation_changes", "CREATE TABLE organization_album_operation_changes (
        operation_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT NOT NULL,
        previous_name TEXT,
        previous_retired INTEGER NOT NULL CHECK(previous_retired IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("table", "organization_operation_conflicts", "CREATE TABLE organization_operation_conflicts (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        album_id TEXT NOT NULL,
        decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0),
        PRIMARY KEY(operation_id, record_id, album_id),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("table", "organization_operation_targets", "CREATE TABLE organization_operation_targets (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE
    )"),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 47)?;
    for (_, _, ddl) in OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction.execute_batch(
        "INSERT INTO organization_record_groups SELECT record_id, record_id FROM organization_records;
         INSERT INTO organization_operation_targets SELECT operation_id, record_id, field FROM organization_operation_changes;
         UPDATE schema_info SET version = 48; PRAGMA user_version = 48;"
    ).map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 48)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, table, _) in OBJECTS.iter().filter(|(kind, _, _)| *kind == "table") {
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
        "SELECT EXISTS(SELECT 1 FROM organization_records AS record
           LEFT JOIN organization_record_groups AS member ON member.record_id = record.record_id
           LEFT JOIN organization_record_groups AS leader ON leader.record_id = member.group_id
           WHERE member.record_id IS NULL OR leader.group_id IS NOT member.group_id)
         OR EXISTS(SELECT 1 FROM organization_album_retirements AS retirement, organization_state AS state
           WHERE retirement.revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_album_operation_changes AS change, organization_state AS state
           WHERE change.applied_revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operation_conflicts AS conflict, organization_state AS state
           WHERE conflict.decision_revision > state.revision)
         OR EXISTS(SELECT 1 FROM organization_operation_changes AS change
           LEFT JOIN organization_operation_targets AS target ON target.operation_id = change.operation_id
             AND target.record_id = change.record_id AND target.field = change.field
           WHERE target.operation_id IS NULL)
         OR EXISTS(SELECT 1 FROM organization_operation_targets AS target
           JOIN organization_records AS record ON record.record_id = target.record_id
           LEFT JOIN organization_operation_changes AS change ON change.operation_id = target.operation_id
             AND change.record_id = target.record_id AND change.field = target.field
           WHERE change.operation_id IS NULL)
         OR EXISTS(SELECT 1 FROM organization_records AS record
           JOIN organization_album_retirements AS retirement ON retirement.album_id = record.album_id)", [], |row| row.get(0)
    ).map_err(database_error)?;
    if invalid {
        return Err(self::invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_transitions_invalid",
        "Organization groups and operation history require complete, non-cyclic durable relationships",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v47_for_test(connection: &Connection) {
    if super::schema_shape::schema_version(connection).unwrap() == 48 {
        for (kind, name, _) in OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 47; PRAGMA user_version = 47;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

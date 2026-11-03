use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::super::{database_error, error};

pub(super) fn apply(connection: &Connection, base: i64) -> Result<(), ScanError> {
    let conflict: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM recovery.archive_classification_operation AS source
         JOIN classification_operations AS current USING(operation_id)
         WHERE source.request_digest <> current.request_digest OR source.operation_kind <> current.operation_kind
           OR source.changed_groups <> current.changed_groups OR source.created_unix_ms <> current.created_unix_ms)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if conflict {
        return Err(error(
            "history_conflict",
            "A backup operation conflicts with retained classification history",
        ));
    }
    connection.execute(
        "INSERT INTO classification_operations(operation_id, request_digest, operation_kind,
         revision, changed_groups, created_unix_ms, restored)
         SELECT operation_id, request_digest, operation_kind, ?1 + revision, changed_groups, created_unix_ms, 1
         FROM recovery.archive_classification_operation WHERE 1
         ON CONFLICT(operation_id) DO NOTHING", [base],
    ).map_err(database_error)?;
    connection.execute_batch(
        "INSERT INTO classification_operation_changes(operation_id, record_id, field, previous_payload,
         previous_revision, previous_restored, applied_revision)
         SELECT change.operation_id, change.record_id, change.field, change.previous_payload,
           CASE WHEN change.previous_revision = 0 THEN 0 ELSE change.previous_revision - source.revision + current.revision END,
           1, current.revision
         FROM recovery.archive_classification_change AS change
         JOIN recovery.archive_classification_operation AS source USING(operation_id)
         JOIN classification_operations AS current USING(operation_id) WHERE 1
         ON CONFLICT(operation_id, record_id, field) DO NOTHING;
         INSERT INTO classification_operation_targets SELECT * FROM recovery.archive_classification_target WHERE 1
         ON CONFLICT(operation_id, record_id, field) DO NOTHING;",
    ).map_err(database_error)
}

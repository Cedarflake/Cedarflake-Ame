use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::{database_error, error};

pub(super) fn apply(connection: &Connection, revision_base: i64) -> Result<(), ScanError> {
    let conflict: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM recovery.archive_operation AS source
         JOIN organization_operations AS current USING(operation_id)
         WHERE source.request_digest <> current.request_digest OR source.operation_kind <> current.operation_kind
           OR source.changed_groups <> current.changed_groups OR source.created_unix_ms <> current.created_unix_ms)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if conflict {
        return Err(error(
            "history_conflict",
            "A backup operation conflicts with retained operation history",
        ));
    }
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_restore_new_operations (
           operation_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_restore_new_operations;
         INSERT INTO temp.organization_restore_new_operations
         SELECT operation_id FROM recovery.archive_operation AS source
         WHERE NOT EXISTS(SELECT 1 FROM organization_operations AS current WHERE current.operation_id = source.operation_id);",
    ).map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_operations
         SELECT operation.operation_id, operation.request_digest, operation.operation_kind,
           ?1 + operation.revision, operation.changed_groups, operation.created_unix_ms
         FROM temp.organization_restore_new_operations AS selected
         CROSS JOIN recovery.archive_operation AS operation USING(operation_id)",
            [revision_base],
        )
        .map_err(database_error)?;
    for query in HISTORY_ROWS {
        connection.execute(query, []).map_err(database_error)?;
    }
    connection.execute_batch(
        "INSERT INTO organization_operation_targets
         SELECT target.operation_id, target.record_id, target.field
         FROM recovery.archive_operation_target AS target WHERE 1
         ON CONFLICT(operation_id, record_id, field) DO NOTHING;
         INSERT INTO organization_review_operations
         SELECT review.operation_id, review.session_id, review.position_sequence, review.committed_sequence
         FROM recovery.archive_review_operation AS review WHERE 1
         ON CONFLICT(operation_id) DO NOTHING;",
    ).map_err(database_error)
}

const HISTORY_ROWS: &[&str] = &[
    "INSERT INTO organization_operation_changes
     SELECT change.operation_id, change.record_id, change.field, change.previous_album_id,
       change.previous_disposition, change.previous_favorite, operation.revision
     FROM recovery.archive_operation_change AS change
     JOIN organization_operations AS operation USING(operation_id) WHERE 1
     ON CONFLICT(operation_id, record_id, field) DO NOTHING",
    "INSERT INTO organization_album_operation_changes
     SELECT change.operation_id, change.album_id, change.previous_name, change.previous_retired,
       operation.revision
     FROM recovery.archive_album_change AS change
     JOIN organization_operations AS operation USING(operation_id) WHERE 1
     ON CONFLICT(operation_id) DO NOTHING",
    "INSERT INTO organization_operation_conflicts
     SELECT conflict.operation_id, conflict.record_id, conflict.album_id,
       CASE WHEN conflict.decision_revision = 0 THEN 0
         ELSE conflict.decision_revision + current.revision - source.revision END
     FROM recovery.archive_operation_conflict AS conflict
     JOIN organization_operations AS current USING(operation_id)
     JOIN recovery.archive_operation AS source USING(operation_id) WHERE 1
     ON CONFLICT(operation_id, record_id, album_id) DO NOTHING",
];

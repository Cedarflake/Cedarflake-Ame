use rusqlite::Connection;

use super::database_error;
use super::schema_shape::schema_object_sql_matches;
use crate::domain::ScanError;

const TABLE: &str = "library_journal_scope_recoveries";
const DDL: &str = "CREATE TABLE library_journal_scope_recoveries (
    parent_change_id INTEGER PRIMARY KEY NOT NULL,
    current_change_id INTEGER NOT NULL UNIQUE,
    previous_change_id INTEGER UNIQUE,
    CHECK(parent_change_id != current_change_id),
    CHECK(previous_change_id IS NULL OR (previous_change_id != parent_change_id
        AND previous_change_id != current_change_id)),
    FOREIGN KEY(parent_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
    FOREIGN KEY(current_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
    FOREIGN KEY(previous_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "table", TABLE, DDL)? {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_row: bool = connection
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM library_journal_scope_recoveries AS recovery
           LEFT JOIN library_change_queue AS parent ON parent.id = recovery.parent_change_id
           LEFT JOIN library_change_queue AS current ON current.id = recovery.current_change_id
           LEFT JOIN library_change_queue AS previous ON previous.id = recovery.previous_change_id
           WHERE parent.id IS NULL OR current.id IS NULL
             OR parent.origin <> 'startup_catch_up' OR parent.scope = 'path'
             OR current.root_id <> parent.root_id
             OR current.root_generation <> parent.root_generation
             OR current.origin <> 'metadata_inventory'
             OR (current.scope = 'subtree' AND current.intent_kind <> 'reconcile')
             OR (current.scope = 'root' AND current.intent_kind <> 'freshness_unknown')
             OR current.scope <> parent.scope OR current.relative_path <> parent.relative_path
             OR current.previous_relative_path IS NOT NULL
             OR NOT EXISTS (SELECT 1 FROM library_change_queue_lanes AS lane
               WHERE lane.change_id = parent.id AND lane.lane = 'p1_journal')
             OR NOT EXISTS (SELECT 1 FROM library_recovery_authorities AS authority
               JOIN library_change_queue_lanes AS lane ON lane.change_id = authority.change_id
               WHERE authority.change_id = current.id AND lane.lane = 'p2_recovery'
                 AND authority.reason = 'journal_reconstruction_failure'
                 AND authority.root_id = parent.root_id
                 AND authority.root_generation = parent.root_generation)
             OR (recovery.previous_change_id IS NOT NULL AND (
               previous.id IS NULL OR parent.previous_relative_path IS NULL
               OR previous.root_id <> parent.root_id
               OR previous.root_generation <> parent.root_generation
               OR previous.origin <> 'metadata_inventory' OR previous.intent_kind <> 'reconcile'
               OR previous.scope <> 'subtree'
               OR previous.relative_path <> parent.previous_relative_path
               OR previous.previous_relative_path IS NOT NULL
               OR NOT EXISTS (SELECT 1 FROM library_recovery_authorities AS authority
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = authority.change_id
                 WHERE authority.change_id = previous.id AND lane.lane = 'p2_recovery'
                   AND authority.reason = 'journal_reconstruction_failure'
                   AND authority.root_id = parent.root_id
                   AND authority.root_generation = parent.root_generation)
               OR current.status <> 'completed'))
             OR (parent.status = 'completed' AND (
               current.status <> 'completed'
               OR (parent.previous_relative_path IS NOT NULL AND
                 (previous.id IS NULL OR previous.status <> 'completed'))))
         )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if invalid_row {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_journal_scope_recovery_invalid",
        "Journal scope recovery requires ordered children bound to the original journal task",
    )
}

#[cfg(test)]
mod tests {}

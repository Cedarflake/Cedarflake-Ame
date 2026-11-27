use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

mod schema;

#[cfg(test)]
mod tests;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_content_rows(connection: &Connection) -> Result<(), ScanError> {
    let state_count: i64 = connection
        .query_row(
            "SELECT count(*) FROM classification_dispatch_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if state_count != 1 {
        return Err(invalid());
    }
    let invalid_work: bool = connection
        .query_row(
            "SELECT EXISTS(
            SELECT 1 FROM classification_work_items AS work
            JOIN classification_runs AS run ON run.run_id = work.created_run_id
            CROSS JOIN classification_dispatch_state AS state
            WHERE work.configuration_revision <> run.configuration_revision
               OR work.lease_id > state.lease_clock OR work.lease_epoch > state.epoch
               OR (work.phase = 'running' AND (
                    work.lease_epoch <> state.epoch OR state.mode <> 'automatic'
                    OR state.session_id IS NULL OR NOT EXISTS(
                        SELECT 1 FROM classification_runs AS active
                        WHERE active.run_id = state.run_id
                          AND active.configuration_revision = work.configuration_revision)))
        ) OR EXISTS(
            SELECT 1 FROM classification_dispatch_state AS state
            JOIN classification_stopped_runs AS stopped USING(run_id)
            WHERE state.mode IN ('automatic', 'paused')
        ) OR EXISTS(
            SELECT 1 FROM classification_dispatch_state AS state
            WHERE state.mode = 'stopped' AND NOT EXISTS(
                SELECT 1 FROM classification_stopped_runs AS stopped WHERE stopped.run_id = state.run_id)
        )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if invalid_work {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_work_invalid",
        "Classification tasks require complete dispatch and lease records",
    )
}

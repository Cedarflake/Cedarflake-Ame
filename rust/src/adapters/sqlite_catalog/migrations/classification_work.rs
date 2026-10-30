use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

mod schema;

#[cfg(test)]
mod tests;

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 65)?;
    for (_, _, statement) in schema::OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO classification_dispatch_state(
             singleton, revision, epoch, mode, source_cursor, lease_clock, fairness_cursor
         ) VALUES (1, 0, 0, 'disabled', 0, 0, 0);
         UPDATE schema_info SET version = 66; PRAGMA user_version = 66;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 66)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in schema::OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
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
    for (kind, name, _) in schema::OBJECTS {
        if *kind != "table" {
            continue;
        }
        let failed: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if failed {
            return Err(invalid());
        }
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

#[cfg(test)]
pub(super) fn downgrade_to_v65_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() != 66 {
        return;
    }
    for (kind, name, _) in schema::OBJECTS.iter().rev() {
        connection
            .execute_batch(&format!("DROP {} {name}", kind.to_uppercase()))
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 65; PRAGMA user_version = 65;")
        .unwrap();
}

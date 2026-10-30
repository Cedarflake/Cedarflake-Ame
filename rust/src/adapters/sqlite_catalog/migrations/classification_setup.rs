use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str)] = &[
    (
        "classification_analysis_setups",
        "CREATE TABLE classification_analysis_setups (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_runs(run_id),
        profile_payload TEXT CHECK(profile_payload IS NULL OR
            length(CAST(profile_payload AS BLOB)) BETWEEN 1 AND 65536)
    ) STRICT, WITHOUT ROWID",
    ),
    (
        "classification_dispatch_admission",
        "CREATE TABLE classification_dispatch_admission (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1)
            REFERENCES classification_dispatch_state(singleton),
        backlog_ceiling INTEGER NOT NULL CHECK(backlog_ceiling >= 0),
        arrival_cursor INTEGER NOT NULL CHECK(arrival_cursor >= backlog_ceiling)
    ) STRICT",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 66)?;
    for (_, statement) in OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "INSERT INTO classification_dispatch_admission(singleton, backlog_ceiling, arrival_cursor)
             SELECT 1, next_source_generation - 1, next_source_generation - 1 FROM catalog_state;
         UPDATE schema_info SET version = 67; PRAGMA user_version = 67;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 67)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, "table", name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let valid: bool = connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM classification_dispatch_admission AS admission
             JOIN classification_dispatch_state AS dispatch USING(singleton)
             CROSS JOIN catalog_state
             WHERE admission.singleton = 1 AND admission.arrival_cursor < next_source_generation
               AND dispatch.source_cursor <= admission.backlog_ceiling
         ) AND NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_analysis_setups'))
           AND NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_dispatch_admission'))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if !valid {
        return Err(invalid());
    }
    super::super::classification::setup::validate_active(connection)
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_setup_invalid",
        "Classification setup or source admission records are inconsistent",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v66_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() != 67 {
        return;
    }
    for (name, _) in OBJECTS.iter().rev() {
        connection
            .execute_batch(&format!("DROP TABLE {name}"))
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 66; PRAGMA user_version = 66;")
        .unwrap();
}

#[cfg(test)]
mod tests;

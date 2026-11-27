use rusqlite::{Connection, TransactionBehavior};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, SQLITE_APPLICATION_ID, database_error};

pub(in crate::adapters::sqlite_catalog) fn prepare_fresh_catalog_auto_vacuum(
    connection: &Connection,
) -> Result<(), ScanError> {
    if user_schema_objects(connection)? == 0 {
        connection
            .execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn initialize(connection: &mut Connection) -> Result<(), ScanError> {
    if !has_schema_info(connection)? {
        prepare_fresh_catalog_auto_vacuum(connection)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if !has_schema_info(&transaction)? {
            if user_schema_objects(&transaction)? != 0 {
                return Err(ScanError::new(
                    "catalog_schema_unsupported",
                    "The existing database does not identify a supported Ame catalog",
                ));
            }
            transaction
                .execute_batch(include_str!("initialization.sql"))
                .map_err(database_error)?;
            super::classification_display::create(&transaction)?;
            transaction
                .execute("UPDATE schema_info SET version = ?1", [SCHEMA_VERSION])
                .map_err(database_error)?;
            transaction
                .pragma_update(None, "application_id", SQLITE_APPLICATION_ID)
                .map_err(database_error)?;
            transaction
                .pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(database_error)?;
            super::current_schema::validate_current_schema_contract(&transaction)?;
        }
        transaction.commit().map_err(database_error)?;
    }
    let versions = connection
        .prepare("SELECT version FROM schema_info LIMIT 2")
        .map_err(database_error)?
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    if versions == [83] {
        super::classification_display::upgrade(connection)?;
    } else if versions != [SCHEMA_VERSION] {
        return Err(ScanError::new(
            "catalog_schema_unsupported",
            format!("Expected catalog schema {SCHEMA_VERSION}, found {versions:?}"),
        ));
    }
    Ok(())
}

fn has_schema_info(connection: &Connection) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema
             WHERE type = 'table' AND name = 'schema_info')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

fn user_schema_objects(connection: &Connection) -> Result<i64, ScanError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}

#[cfg(test)]
mod tests;

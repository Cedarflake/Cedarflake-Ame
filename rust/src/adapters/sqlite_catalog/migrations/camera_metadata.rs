use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const MAKE_COLUMN: &str = "camera_make TEXT CHECK(camera_make IS NULL OR (
    typeof(camera_make) = 'text' AND length(CAST(camera_make AS BLOB)) BETWEEN 1 AND 128
))";
const MODEL_COLUMN: &str = "camera_model TEXT CHECK(camera_model IS NULL OR (
    typeof(camera_model) = 'text' AND length(CAST(camera_model AS BLOB)) BETWEEN 1 AND 128
))";

const TABLES: [&str; 3] = [
    "asset_locations",
    "library_change_scan_handoff_items",
    "library_change_catch_up_handoffs",
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 61)?;
    for table in TABLES {
        for column in [MAKE_COLUMN, MODEL_COLUMN] {
            transaction
                .execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column}"))
                .map_err(database_error)?;
        }
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 62; PRAGMA user_version = 62;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 62)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for table in TABLES {
        let sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let sql = schema_shape::normalize_schema_sql(&sql);
        if ![MAKE_COLUMN, MODEL_COLUMN]
            .iter()
            .all(|column| sql.contains(&schema_shape::normalize_schema_sql(column)))
        {
            return Err(ScanError::new(
                "catalog_camera_metadata_invalid",
                "The camera metadata columns require their bounded text contract",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_to_v61_for_test(connection: &Connection) {
    super::compact_gallery_search::downgrade_to_v62_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 62 {
        for table in TABLES {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE {table} DROP COLUMN camera_model;
                ALTER TABLE {table} DROP COLUMN camera_make;"
                ))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 61; PRAGMA user_version = 61;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

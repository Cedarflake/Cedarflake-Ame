use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::domain::ScanError;

use super::super::metadata_filter_payload;
use super::{current_schema, database_error, schema_shape};

const TABLE: &str = "CREATE TABLE organization_review_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version = 2),
    payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 4096 AND json_valid(payload))
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 76)?;
    transaction
        .execute_batch(
            "ALTER TABLE organization_review_filters RENAME TO organization_review_filters_v1;",
        )
        .map_err(database_error)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    let mut statement = transaction
        .prepare("SELECT scope_slot, payload FROM organization_review_filters_v1")
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    for row in rows {
        let (slot, payload) = row.map_err(database_error)?;
        let filter = metadata_filter_payload::decode(&payload, 1)?;
        transaction
            .execute(
                "INSERT INTO organization_review_filters VALUES (?1, 2, ?2)",
                params![slot, metadata_filter_payload::encode(&filter)?],
            )
            .map_err(database_error)?;
    }
    drop(statement);
    transaction
        .execute_batch(
            "DROP TABLE organization_review_filters_v1;
        UPDATE schema_info SET version = 77; PRAGMA user_version = 77;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 77)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_review_filters",
        TABLE,
    )? {
        return Err(ScanError::new(
            "catalog_review_filter_invalid",
            "Organization scopes require versioned format selections",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() != 77 {
        return;
    }
    let rows: Vec<(String, String)> = connection
        .prepare("SELECT scope_slot, payload FROM organization_review_filters")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    connection
        .execute_batch("DROP TABLE organization_review_filters;")
        .unwrap();
    connection
        .execute_batch(super::organization_review_filters::TABLE)
        .unwrap();
    for (slot, payload) in rows {
        let filter = metadata_filter_payload::decode(&payload, 2).unwrap();
        connection
            .execute(
                "INSERT INTO organization_review_filters VALUES (?1, 1, ?2)",
                params![slot, metadata_filter_payload::encode_legacy(&filter)],
            )
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 76; PRAGMA user_version = 76;")
        .unwrap();
}

#[cfg(test)]
mod tests;

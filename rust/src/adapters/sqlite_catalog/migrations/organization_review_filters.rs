use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::gallery_metadata_filter::GalleryMetadataFilter;

use super::{current_schema, database_error, schema_shape};

const TABLE: &str = "CREATE TABLE organization_review_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version = 1),
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
    current_schema::validate_schema_version(transaction, 63)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    let empty = serde_json::to_string(&GalleryMetadataFilter::default()).map_err(|_| invalid())?;
    transaction.execute(
        "INSERT INTO organization_review_filters SELECT slot, 1, ?1 FROM organization_review_scopes",
        [empty],
    ).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 64; PRAGMA user_version = 64;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 64)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_review_filters",
        TABLE,
    )? {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let incomplete: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_review_scopes AS scope
           LEFT JOIN organization_review_filters AS filters ON filters.scope_slot = scope.slot
           WHERE filters.scope_slot IS NULL)
         OR EXISTS(SELECT 1 FROM pragma_foreign_key_check('organization_review_filters'))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if incomplete {
        return Err(invalid());
    }
    let mut statement = connection
        .prepare("SELECT payload FROM organization_review_filters")
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    for payload in rows {
        let filter: GalleryMetadataFilter =
            serde_json::from_str(&payload.map_err(database_error)?).map_err(|_| invalid())?;
        filter.validate()?;
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_review_filter_invalid",
        "Organization scopes require complete, valid metadata filters",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v63_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 64 {
        connection
            .execute_batch(
                "DROP TABLE organization_review_filters;
            UPDATE schema_info SET version = 63; PRAGMA user_version = 63;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;

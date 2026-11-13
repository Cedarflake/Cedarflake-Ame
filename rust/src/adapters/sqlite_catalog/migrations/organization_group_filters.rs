use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const TABLE: &str = "CREATE TABLE organization_review_group_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    group_filter TEXT NOT NULL CHECK(group_filter IN ('unrestricted', 'exact', 'similar'))
) STRICT";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 82)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 83; PRAGMA user_version = 83;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 83)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_review_group_filters",
        TABLE,
    )? {
        return Err(ScanError::new(
            "catalog_review_group_filter_invalid",
            "The saved organization group filters have an invalid schema",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 83 {
        connection
            .execute_batch(
                "DROP TABLE organization_review_group_filters;
             UPDATE schema_info SET version = 82; PRAGMA user_version = 82;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite_catalog::SqliteCatalog;

    #[test]
    fn upgrade_keeps_default_scope_and_rolls_back_on_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
        downgrade_for_test(&catalog.connection);
        catalog
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_group_upgrade BEFORE UPDATE OF version ON schema_info
             WHEN NEW.version = 83 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
            )
            .unwrap();
        assert!(migrate(&mut catalog.connection).is_err());
        current_schema::validate_schema_version(&catalog.connection, 82).unwrap();
        let created: bool = catalog.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'organization_review_group_filters')",
            [], |row| row.get(0),
        ).unwrap();
        assert!(!created);
        catalog
            .connection
            .execute_batch("DROP TRIGGER reject_group_upgrade")
            .unwrap();
        drop(catalog);
        let catalog = SqliteCatalog::open(path).unwrap();
        current_schema::validate_schema_version(&catalog.connection, 83).unwrap();
        assert_eq!(
            catalog
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM organization_review_group_filters",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

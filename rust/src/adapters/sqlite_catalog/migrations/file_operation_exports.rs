use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::invalid;

use super::{current_schema, database_error, schema_shape};

const BATCHES: &str = "CREATE TABLE file_operation_exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL UNIQUE CHECK(length(batch_id) BETWEEN 1 AND 256),
    authorized_unix_ms INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('copy', 'move')),
    rule TEXT NOT NULL CHECK(rule IN ('direct', 'category', 'album', 'date', 'source')),
    selected_files INTEGER NOT NULL CHECK(selected_files > 0),
    root_path TEXT NOT NULL CHECK(length(root_path) BETWEEN 1 AND 65536),
    identity_scheme TEXT NOT NULL CHECK(length(identity_scheme) BETWEEN 1 AND 256),
    identity_value TEXT NOT NULL CHECK(length(identity_value) BETWEEN 1 AND 4096),
    report_state TEXT NOT NULL DEFAULT 'pending' CHECK(report_state IN ('pending', 'not_needed', 'written', 'failed')),
    report_path TEXT,
    report_error_code TEXT,
    report_error_message TEXT,
    CHECK((report_state = 'written') = (report_path IS NOT NULL)),
    CHECK((report_state = 'failed') = (report_error_code IS NOT NULL AND report_error_message IS NOT NULL))
)";
const SKIPS: &str = "CREATE TABLE file_operation_export_skips (
    batch_id TEXT NOT NULL REFERENCES file_operation_exports(batch_id),
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    source_path TEXT NOT NULL CHECK(length(source_path) BETWEEN 1 AND 65536),
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 4096),
    PRIMARY KEY(batch_id, ordinal)
) WITHOUT ROWID";
const AUTHORITY: &str = "CREATE TRIGGER file_operation_export_authority_immutable
    BEFORE UPDATE OF batch_id, authorized_unix_ms, action, rule, selected_files, root_path, identity_scheme, identity_value
    ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export authority is immutable'); END";
const RETAIN: &str = "CREATE TRIGGER file_operation_exports_retained
    BEFORE DELETE ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export history is durable'); END";
const SKIP_IMMUTABLE: &str = "CREATE TRIGGER file_operation_export_skips_immutable
    BEFORE UPDATE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are immutable'); END";
const SKIP_RETAIN: &str = "CREATE TRIGGER file_operation_export_skips_retained
    BEFORE DELETE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are durable'); END";
const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "file_operation_exports", BATCHES),
    ("table", "file_operation_export_skips", SKIPS),
    (
        "trigger",
        "file_operation_export_authority_immutable",
        AUTHORITY,
    ),
    ("trigger", "file_operation_exports_retained", RETAIN),
    (
        "trigger",
        "file_operation_export_skips_immutable",
        SKIP_IMMUTABLE,
    ),
    (
        "trigger",
        "file_operation_export_skips_retained",
        SKIP_RETAIN,
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 80)?;
    for (_, _, sql) in OBJECTS {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 81; PRAGMA user_version = 81;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 81)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, sql)? {
            return Err(invalid(format!(
                "The export history schema is invalid: {name}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 81 {
        connection
            .execute_batch(
                "DROP TRIGGER file_operation_exports_retained;
            DROP TRIGGER file_operation_export_skips_retained;
            DROP TABLE file_operation_export_skips;
            DROP TABLE file_operation_exports;
            UPDATE schema_info SET version = 80; PRAGMA user_version = 80;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite_catalog::SqliteCatalog;
    use crate::ports::organization::OrganizationRepository;

    #[test]
    fn file_operation_exports_upgrade_rolls_back_and_preserves_existing_history() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
        let album = catalog
            .create_organization_album("album", 0, "旅行", Default::default())
            .unwrap();
        let history = catalog.load_organization_history().unwrap();
        downgrade_for_test(&catalog.connection);
        catalog
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_export_upgrade BEFORE UPDATE OF version ON schema_info
            WHEN NEW.version = 81 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
            )
            .unwrap();
        assert!(migrate(&mut catalog.connection).is_err());
        current_schema::validate_schema_version(&catalog.connection, 80).unwrap();
        let exists: bool = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'file_operation_exports')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
        catalog
            .connection
            .execute_batch("DROP TRIGGER reject_export_upgrade;")
            .unwrap();
        drop(catalog);
        let catalog = SqliteCatalog::open(path).unwrap();
        assert_eq!(
            catalog
                .find_organization_album(&album.album_id)
                .unwrap()
                .unwrap()
                .name,
            "旅行"
        );
        assert_eq!(catalog.load_organization_history().unwrap(), history);
        current_schema::validate_schema_version(&catalog.connection, 81).unwrap();
    }
}

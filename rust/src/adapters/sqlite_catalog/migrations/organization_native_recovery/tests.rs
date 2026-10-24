use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior_catalog() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, 10);
         INSERT INTO organization_record_groups VALUES ('retained', 'retained');
         INSERT INTO organization_bindings VALUES ('retained', 'old-asset', 1, 20,
           'windows-file-id-128-v1', '0000000000000001:00000000000000000000000000000001',
           'windows-file-change-time-100ns-v1:0000000000000001', NULL, NULL);
         UPDATE organization_state SET revision = 1;",
    ).unwrap();
    downgrade_to_v52_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn organization_native_recovery_upgrade_preserves_records_and_revisions() {
    let (directory, mut catalog) = prior_catalog();
    let before: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    let after: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT revision, (SELECT COUNT(*) FROM organization_bindings),
           (SELECT COUNT(*) FROM organization_records WHERE favorite = 1 AND disposition = 'ignored'
             AND unassociated_since_unix_ms = 10) FROM catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(after, (before, 1, 1));
    current_schema::validate_schema_version(&catalog.connection, 53).unwrap();
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn organization_native_recovery_upgrade_rolls_back_index_and_accepts_retry() {
    let (_directory, mut catalog) = prior_catalog();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_version BEFORE UPDATE ON schema_info
         BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 52).unwrap();
    let exists: bool = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [INDEX.0],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!exists);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
}

#[test]
fn organization_native_recovery_schema_rejects_missing_or_altered_lookup() {
    let (_directory, mut catalog) = prior_catalog();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DROP INDEX organization_bindings_native_version")
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_native_recovery_invalid"
    );
    catalog
        .connection
        .execute_batch(
            "CREATE INDEX organization_bindings_native_version ON organization_bindings(asset_id)",
        )
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_native_recovery_invalid"
    );
}

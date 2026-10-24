use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::ports::organization::OrganizationRepository;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog, String) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let album = catalog
        .create_organization_album("create", 0, "Retained")
        .unwrap()
        .album_id;
    downgrade_to_v55_for_test(&catalog.connection);
    (directory, catalog, album)
}

#[test]
fn history_upgrade_preserves_existing_decisions_and_receipts_without_inventing_order() {
    let (directory, mut catalog, album) = prior();
    let before: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(
        catalog
            .find_organization_album(&album)
            .unwrap()
            .unwrap()
            .name,
        "Retained"
    );
    let history = catalog.load_organization_history().unwrap();
    assert_eq!(history.organization_revision, 1);
    assert_eq!(history.undo_operation_id, None);
    assert_eq!(history.redo_operation_id, None);
    let after: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(
        catalog
            .create_organization_album("create", 0, "Retained")
            .unwrap()
            .album_id,
        album
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .undo_organization_operation("legacy-undo", "create", 1)
        .unwrap();
    assert!(catalog.find_organization_album(&album).unwrap().is_none());
}

#[test]
fn failed_history_upgrade_rolls_back_ddl_and_preserves_retry() {
    let (_directory, mut catalog, album) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_history_upgrade BEFORE UPDATE ON schema_info
         WHEN NEW.version = 56 BEGIN SELECT RAISE(ABORT, 'injected upgrade failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        55
    );
    let objects: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'organization_history_state'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(objects, 0);
    assert!(catalog.find_organization_album(&album).unwrap().is_some());
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_history_upgrade;")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn current_history_rejects_missing_authority_broken_cursor_and_missing_trigger() {
    for damage in [
        "DELETE FROM organization_album_authority",
        "UPDATE organization_history_state SET cursor = 2",
        "DROP TRIGGER organization_capture_field_authority",
    ] {
        let (_directory, mut catalog, album) = prior();
        migrate(&mut catalog.connection).unwrap();
        catalog
            .rename_organization_album("rename", 1, &album, "Renamed")
            .unwrap();
        catalog.connection.execute_batch(damage).unwrap();
        assert_eq!(
            current_schema::validate_schema_version(&catalog.connection, 56)
                .unwrap_err()
                .code,
            "catalog_organization_history_invalid"
        );
    }
}

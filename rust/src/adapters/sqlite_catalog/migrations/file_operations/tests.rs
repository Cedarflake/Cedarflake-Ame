use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::domain::organization::OrganizationAlbumIcon;
use crate::ports::organization::OrganizationRepository;

#[test]
fn file_operation_upgrade_preserves_user_decisions_and_rolls_back_partial_schema() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let album = catalog
        .create_organization_album("create", 0, "旅行", OrganizationAlbumIcon::Travel)
        .unwrap();
    let history = catalog.load_organization_history().unwrap();
    downgrade_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_operation_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 79 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 78).unwrap();
    let partial: bool = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'file_operation_items')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!partial);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_operation_upgrade;")
        .unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(path).unwrap();
    let retained = catalog
        .find_organization_album(&album.album_id)
        .unwrap()
        .unwrap();
    assert_eq!(retained.name, "旅行");
    assert_eq!(retained.icon, OrganizationAlbumIcon::Travel);
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        crate::adapters::sqlite_catalog::SCHEMA_VERSION
    );
}

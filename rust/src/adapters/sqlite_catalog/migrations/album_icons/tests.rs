use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::ports::organization::OrganizationRepository;

#[test]
fn album_icon_upgrade_preserves_existing_albums_and_rolls_back_on_failure() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let created = catalog
        .create_organization_album("create", 0, "旅行", OrganizationAlbumIcon::Album)
        .unwrap();
    let history = catalog.load_organization_history().unwrap();
    downgrade_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_icon_upgrade BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 78 BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 77).unwrap();
    let partial_table: bool = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'organization_album_icons')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!partial_table);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_icon_upgrade;")
        .unwrap();
    drop(catalog);

    let catalog = SqliteCatalog::open(path).unwrap();
    let album = catalog
        .find_organization_album(&created.album_id)
        .unwrap()
        .unwrap();
    assert_eq!(album.name, "旅行");
    assert_eq!(album.icon, OrganizationAlbumIcon::Album);
    assert_eq!(catalog.load_organization_history().unwrap(), history);
}

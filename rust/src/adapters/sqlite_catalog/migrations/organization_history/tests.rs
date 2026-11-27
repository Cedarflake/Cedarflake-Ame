use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::adapters::sqlite_catalog::migrations::current_schema;
use crate::ports::organization::OrganizationRepository;

#[test]
fn current_history_rejects_missing_authority_broken_cursor_and_missing_trigger() {
    for damage in [
        "DELETE FROM organization_album_authority",
        "UPDATE organization_history_state SET cursor = cursor + 2",
        "DROP TRIGGER organization_capture_field_authority",
    ] {
        let (_directory, mut catalog, album) = prior();

        catalog
            .rename_organization_album("rename", 1, &album, "Renamed")
            .unwrap();
        catalog.connection.execute_batch(damage).unwrap();
        assert_eq!(
            current_schema::validate_current_schema_contract(&catalog.connection)
                .unwrap_err()
                .code,
            "catalog_organization_history_invalid"
        );
    }
}

fn prior() -> (tempfile::TempDir, SqliteCatalog, String) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let album = catalog
        .create_organization_album("create", 0, "Retained", Default::default())
        .unwrap()
        .album_id;

    (directory, catalog, album)
}

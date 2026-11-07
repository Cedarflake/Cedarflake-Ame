use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::domain::organization::{OrganizationAlbumIcon, OrganizationHistoryDirection};
use crate::ports::organization::OrganizationRepository;

#[test]
fn album_icons_persist_without_entering_or_invalidating_organization_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let created = catalog
        .create_organization_album("create", 0, "旅行", OrganizationAlbumIcon::Nature)
        .unwrap();
    assert_eq!(
        catalog
            .create_organization_album("create", 0, "旅行", OrganizationAlbumIcon::Nature)
            .unwrap(),
        created
    );
    assert_eq!(
        catalog
            .create_organization_album("create", 0, "旅行", OrganizationAlbumIcon::Pets)
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
    let history = catalog.load_organization_history().unwrap();
    let changed = catalog
        .change_organization_album_icon(&created.album_id, OrganizationAlbumIcon::Travel)
        .unwrap();
    assert_eq!(changed.icon, OrganizationAlbumIcon::Travel);
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    assert_eq!(
        catalog
            .change_organization_album_icon(&created.album_id, OrganizationAlbumIcon::Travel)
            .unwrap(),
        changed
    );

    let renamed = catalog
        .rename_organization_album(
            "rename",
            history.organization_revision,
            &created.album_id,
            "旅行照片",
        )
        .unwrap();
    catalog
        .step_organization_history(
            "undo-rename",
            OrganizationHistoryDirection::Undo,
            "rename",
            renamed.operation.organization_revision,
        )
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).unwrap();
    let album = catalog
        .find_organization_album(&created.album_id)
        .unwrap()
        .unwrap();
    assert_eq!(album.name, "旅行");
    assert_eq!(album.icon, OrganizationAlbumIcon::Travel);
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums,
        vec![album]
    );
    let revision = catalog
        .load_organization_history()
        .unwrap()
        .organization_revision;
    catalog
        .delete_organization_album("delete", revision, &created.album_id)
        .unwrap();
    assert_eq!(
        catalog
            .change_organization_album_icon(&created.album_id, OrganizationAlbumIcon::Pets)
            .unwrap_err()
            .code,
        "organization_album_unavailable"
    );
}

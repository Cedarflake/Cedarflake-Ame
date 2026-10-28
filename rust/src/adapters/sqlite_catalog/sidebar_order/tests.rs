use crate::ports::CatalogRepository;
use crate::ports::organization::OrganizationRepository;

use super::*;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    (directory, catalog)
}

fn album(catalog: &SqliteCatalog, id: &str) {
    catalog
        .connection
        .execute(
            "INSERT INTO organization_albums VALUES (?1, ?1, 1, 1, 1)",
            [id],
        )
        .unwrap();
}

fn albums(catalog: &SqliteCatalog) -> Vec<String> {
    catalog
        .load_organization_albums(None)
        .unwrap()
        .albums
        .into_iter()
        .map(|album| album.album_id)
        .collect()
}

fn root(catalog: &mut SqliteCatalog, id: &str) {
    super::super::tests::publish_gallery_fixture(
        catalog,
        &format!("scan-{id}"),
        id,
        &format!("C:\\Fixture\\{id}"),
        &[(id, None, 1)],
    );
}

fn roots(catalog: &SqliteCatalog) -> Vec<String> {
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    super::super::root_views::load_root_views(&transaction)
        .unwrap()
        .into_iter()
        .map(|root| root.root_id)
        .collect()
}

#[test]
fn album_order_survives_rename_append_and_reopen() {
    let (directory, mut catalog) = fixture();
    for id in ["c", "a", "b"] {
        album(&catalog, id);
    }
    assert_eq!(albums(&catalog), ["c", "a", "b"]);
    catalog
        .reorder_sidebar_entry(
            SidebarOrderKind::Albums,
            "b",
            "c",
            SidebarOrderPlacement::Before,
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE organization_albums SET name = 'Renamed' WHERE album_id = 'c'",
            [],
        )
        .unwrap();
    album(&catalog, "new");
    assert_eq!(albums(&catalog), ["b", "c", "a", "new"]);
    drop(catalog);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(albums(&reopened), ["b", "c", "a", "new"]);
}

#[test]
fn album_pages_follow_saved_positions_without_skips_or_duplicates() {
    let (_directory, mut catalog) = fixture();
    let mut expected: Vec<_> = (0..70)
        .rev()
        .map(|index| format!("album-{index:03}"))
        .collect();
    for id in &expected {
        album(&catalog, id);
    }
    catalog
        .reorder_sidebar_entry(
            SidebarOrderKind::Albums,
            "album-000",
            "album-069",
            SidebarOrderPlacement::Before,
        )
        .unwrap();
    expected.rotate_right(1);
    let first = catalog.load_organization_albums(None).unwrap();
    assert_eq!(first.albums.len(), 64);
    assert_eq!(first.total_albums, 70);
    let second = catalog
        .load_organization_albums(first.next_album_id.as_deref())
        .unwrap();
    assert_eq!(second.albums.len(), 6);
    assert!(second.next_album_id.is_none());
    let actual: Vec<_> = first
        .albums
        .into_iter()
        .chain(second.albums)
        .map(|album| album.album_id)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn source_order_is_independent_and_readded_sources_append() {
    let (_directory, mut catalog) = fixture();
    for id in ["c", "a", "b"] {
        album(&catalog, id);
        root(&mut catalog, id);
    }
    catalog
        .reorder_sidebar_entry(
            SidebarOrderKind::SourceFolders,
            "c",
            "b",
            SidebarOrderPlacement::After,
        )
        .unwrap();
    assert_eq!(roots(&catalog), ["a", "b", "c"]);
    assert_eq!(albums(&catalog), ["c", "a", "b"]);
    catalog.unregister_root("a").unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO library_roots VALUES ('a', 'C:\\Readded', NULL, 1)",
            [],
        )
        .unwrap();
    assert_eq!(roots(&catalog), ["b", "c", "a"]);
}

#[test]
fn missing_anchor_and_failed_write_preserve_saved_order() {
    let (_directory, mut catalog) = fixture();
    for id in ["c", "a", "b"] {
        album(&catalog, id);
    }
    let error = catalog
        .reorder_sidebar_entry(
            SidebarOrderKind::Albums,
            "b",
            "missing",
            SidebarOrderPlacement::Before,
        )
        .unwrap_err();
    assert_eq!(error.code, "sidebar_order_entry_unavailable");
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_order BEFORE UPDATE ON organization_sidebar_album_order
         WHEN NEW.album_id = 'b' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .reorder_sidebar_entry(
                SidebarOrderKind::Albums,
                "b",
                "c",
                SidebarOrderPlacement::Before,
            )
            .is_err()
    );
    assert_eq!(albums(&catalog), ["c", "a", "b"]);
}

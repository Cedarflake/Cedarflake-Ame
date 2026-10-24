use crate::domain::organization::{OrganizationAction, OrganizationSelection};
use crate::ports::organization::OrganizationRepository;

use super::*;

fn assign_child(fixture: &Fixture) -> (String, String) {
    let asset_id = fixture.snapshot().assets[0].asset_id.clone();
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let album = catalog
        .create_organization_album("album", 0, "旅行")
        .unwrap()
        .album_id;
    let location = catalog
        .load_active_location_by_asset_id(&asset_id, None)
        .unwrap()
        .unwrap();
    let selections = [OrganizationSelection {
        asset_id: asset_id.clone(),
        location_id: location.location_id,
        source_generation: location.source_generation,
    }];
    let prepared = catalog.prepare_organization_selection(&selections).unwrap();
    catalog
        .change_organization(
            "assign",
            &prepared,
            &OrganizationAction::AssignAlbum {
                album_id: album.clone(),
            },
        )
        .unwrap();
    let prepared = catalog.prepare_organization_selection(&selections).unwrap();
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    (asset_id, album)
}

#[test]
fn complete_parent_import_preserves_album_and_favorite_before_child_retirement() {
    let fixture = Fixture::new();
    let (asset_id, album) = assign_child(&fixture);
    let child_root = fixture.snapshot().roots[0].root_id.clone();
    run_scan_with_storage(
        fixture.parent_request("organized-parent"),
        |_| true,
        fixture.storage.clone(),
    )
    .unwrap();
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.roots.len(), 1);
    assert!(
        snapshot
            .assets
            .iter()
            .any(|asset| asset.asset_id == asset_id)
    );
    let connection = Connection::open(&fixture.storage.catalog_path).unwrap();
    let state: (String, String, bool, String) = connection
        .query_row(
            "SELECT record.album_id, record.disposition, record.favorite, source.root_id
         FROM organization_records AS record JOIN organization_sources AS source USING(record_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        state,
        (
            album,
            "organized".into(),
            true,
            snapshot.roots[0].root_id.clone()
        )
    );
    assert_ne!(state.3, child_root);
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    catalog.unregister_root(&snapshot.roots[0].root_id).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM organization_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    fixture.assert_sources();
}

#[test]
fn failed_parent_retirement_restores_the_childs_organization_ownership() {
    let fixture = Fixture::new();
    let (_, album) = assign_child(&fixture);
    let child_root = fixture.snapshot().roots[0].root_id.clone();
    let connection = Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_organized_child_removal BEFORE DELETE ON library_roots
         BEGIN SELECT RAISE(ABORT, 'injected child retirement failure'); END;",
        )
        .unwrap();
    assert!(
        run_scan_with_storage(
            fixture.parent_request("failed-organized-parent"),
            |_| true,
            fixture.storage.clone()
        )
        .is_err()
    );
    let state: (String, String, bool, String) = connection
        .query_row(
            "SELECT record.album_id, record.disposition, record.favorite, source.root_id
         FROM organization_records AS record JOIN organization_sources AS source USING(record_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(state, (album, "organized".into(), true, child_root));
    SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    fixture.assert_sources();
}

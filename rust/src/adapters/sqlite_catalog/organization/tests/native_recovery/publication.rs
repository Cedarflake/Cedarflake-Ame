use crate::domain::{GalleryCollection, ScanRequest};
use crate::ports::GalleryQueryRepository;

use super::*;

#[test]
fn organization_native_recovery_publishes_all_current_aliases_across_roots() {
    let (_directory, mut catalog) = native_fixture();
    let album = create_album(&mut catalog, "Saved");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    retire_derived_bindings(&catalog);

    let mut alias = catalog.load_active_location("a").unwrap().unwrap();
    let generation = alias.source_generation;
    alias.asset_id = "alias-asset".into();
    alias.location_id = "alias".into();
    alias.root_id = "alias-root".into();
    alias.scan_id = "alias-scan".into();
    alias.relative_path = "renamed.png".into();
    alias.absolute_path = "C:\\Aliases\\renamed.png".into();
    alias.display_path.clone_from(&alias.absolute_path);
    alias.source_generation = 0;
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: alias.scan_id.clone(),
                root_path: "C:\\Aliases".into(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            },
            &alias.root_id,
            "C:\\Aliases",
        )
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test(&alias.scan_id)
        .unwrap();
    catalog
        .stage_location(&alias.scan_id, &alias.root_id, &alias)
        .unwrap();
    catalog.flush_pending_locations().unwrap();
    assert_eq!(active_bindings(&catalog), 0);
    catalog
        .publish_scan(&alias.scan_id, &alias.root_id, 1, 0)
        .unwrap();

    assert_eq!(selected(&catalog, "alias").source_generation, generation);
    assert_eq!(active_bindings(&catalog), 2);
    for location in ["a", "alias"] {
        assert_eq!(
            decision(&catalog, location),
            (Some(album.clone()), "organized".into(), true)
        );
    }
    let snapshot = catalog
        .load_query_snapshot(
            10,
            &GalleryQuery {
                collection: GalleryCollection::Album { album_id: album },
                ..GalleryQuery::default()
            },
            "restored-album",
            None,
        )
        .unwrap();
    assert_eq!(snapshot.timeline.total_items, 1);
    assert_eq!(snapshot.timeline.total_source_items, 2);

    catalog.unregister_root("root-a").unwrap();
    assert_eq!(decision(&catalog, "alias").1, "organized");
    assert!(decision(&catalog, "alias").2);
}

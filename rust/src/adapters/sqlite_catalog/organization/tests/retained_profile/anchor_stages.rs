use crate::domain::GalleryQueryAnchor;

use super::*;

#[test]
#[ignore = "manual retained-catalog anchor profile; requires an isolated populated Favorite snapshot"]
fn profile_retained_organization_anchor_reads() {
    let (directory, path, mut profile) = prepare_copy("organization-anchor-profile.jsonl");
    let mut catalog = SqliteCatalog::open(path).unwrap();
    let query = GalleryQuery {
        collection: GalleryCollection::Favorites,
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    let initial = profile.measure("favorites_without_anchor", || {
        catalog.load_query_snapshot(PAGE_SIZE, &query, "anchor-profile", None)
    });
    assert!(initial.timeline.total_items > 40_000);
    let first = &initial.snapshot.assets[0];
    let next = &initial.snapshot.assets[1];
    let anchor = GalleryQueryAnchor {
        requested_location_id: first.location_id.clone(),
        asset_id: Some(first.asset_id.clone()),
        fallback_ordinal: 0,
    };
    for sample in 0..2 {
        let current = profile.measure(&format!("favorites_retained_anchor_{sample}"), || {
            catalog.load_query_snapshot(PAGE_SIZE, &query, "anchor-profile", Some(&anchor))
        });
        let resolution = current.snapshot.query_anchor_resolution.unwrap();
        assert_eq!(resolution.location_id.as_ref(), Some(&first.location_id));
        assert_eq!(resolution.ordinal, Some(0));
        assert_eq!(current.timeline.total_items, initial.timeline.total_items);
    }
    let selected = selected(&catalog, &first.location_id);
    let prepared = catalog.prepare_organization_selection(&[selected]).unwrap();
    catalog
        .set_organization_favorite("anchor-profile-unfavorite", &prepared, false)
        .unwrap();
    for sample in 0..2 {
        let current = profile.measure(&format!("favorites_removed_anchor_{sample}"), || {
            catalog.load_query_snapshot(PAGE_SIZE, &query, "anchor-profile", Some(&anchor))
        });
        let resolution = current.snapshot.query_anchor_resolution.unwrap();
        assert_eq!(resolution.location_id.as_ref(), Some(&next.location_id));
        assert_eq!(resolution.ordinal, Some(0));
        assert_eq!(resolution.window_start_ordinal, 0);
        assert_eq!(
            current.timeline.total_items + 1,
            initial.timeline.total_items
        );
        assert_eq!(current.snapshot.assets[0].location_id, next.location_id);
        assert!(current.snapshot.assets.len() <= PAGE_SIZE as usize);
    }
    profile.record(json!({
        "complete": true,
        "initial_groups": initial.timeline.total_items,
        "source_media_reads": 0,
    }));
    drop(catalog);
    directory.close().unwrap();
}

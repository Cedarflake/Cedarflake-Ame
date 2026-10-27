use crate::domain::organization_queue::OrganizationQueue;

use super::*;

#[test]
fn collection_groups_resolve_an_unbound_copy_without_inheriting_changed_content() {
    let (_directory, mut catalog, album) = grouped();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let copy = selected(&catalog, "c");
    catalog
        .connection
        .execute(
            "DELETE FROM organization_bindings WHERE asset_id = ?1 AND source_generation = ?2",
            params![
                copy.asset_id,
                i64::try_from(copy.source_generation).unwrap()
            ],
        )
        .unwrap();
    for collection in [album.collection, GalleryCollection::Favorites] {
        let query = GalleryQuery {
            collection,
            root_id: Some("root-c".into()),
            search_text: "c.png".into(),
            sort_key: GallerySortKey::CaptureTime,
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        };
        let snapshot = catalog
            .load_query_snapshot(10, &query, "unbound-copy", None)
            .unwrap();
        assert_eq!(snapshot.snapshot.assets.len(), 1);
        assert_eq!(snapshot.snapshot.assets[0].location_id, "c");
        assert_eq!(snapshot.timeline.total_items, 1);
        assert_eq!(snapshot.timeline.total_source_items, 1);
        catalog
            .connection
            .execute(
                "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'c'",
                [],
            )
            .unwrap();
        let changed = catalog
            .load_query_snapshot(10, &query, "changed-copy", None)
            .unwrap();
        assert!(changed.snapshot.assets.is_empty());
        assert_eq!(changed.timeline.total_items, 0);
        assert_eq!(changed.timeline.total_source_items, 0);
        catalog
            .connection
            .execute(
                "UPDATE asset_locations SET file_size = 20 WHERE location_id = 'c'",
                [],
            )
            .unwrap();
    }
}

#[test]
fn favorite_groups_retain_canonical_aliases_and_independent_queue_membership() {
    let (_directory, mut catalog) = fixture();
    for (operation, id) in [("favorite-a", "a"), ("favorite-c", "c")] {
        let prepared = prepare(&catalog, &[id]);
        catalog
            .set_organization_favorite(operation, &prepared, true)
            .unwrap();
    }
    publish_batch(&mut catalog, &["a", "c"], 'a');
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        2
    );
    let query = GalleryQuery {
        collection: GalleryCollection::Favorites,
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    for (operation, action, queue) in [
        (
            "defer",
            OrganizationAction::Defer,
            OrganizationQueue::Deferred,
        ),
        (
            "ignore",
            OrganizationAction::Ignore,
            OrganizationQueue::Ignored,
        ),
    ] {
        change(&mut catalog, operation, &["c"], action);
        for organization_queue in [None, Some(queue), Some(OrganizationQueue::Pending)] {
            let current = GalleryQuery {
                organization_queue,
                ..query.clone()
            };
            let snapshot = catalog
                .load_query_snapshot(10, &current, "favorite-queue", None)
                .unwrap();
            let expected = u64::from(organization_queue != Some(OrganizationQueue::Pending));
            assert_eq!(snapshot.timeline.total_items, expected);
            assert_eq!(snapshot.timeline.total_source_items, 2 * expected);
            assert_eq!(snapshot.snapshot.assets.len() as u64, expected);
            if expected != 0 {
                assert_eq!(snapshot.snapshot.assets[0].location_id, "a");
            }
        }
    }
}

use crate::domain::organization_queue::OrganizationQueue;

use super::fingerprint_cost::bounded_read;
use super::*;

#[test]
fn empty_decision_scopes_do_not_walk_unrelated_published_locations() {
    let (_directory, mut catalog, _) = grouped();
    let empty_album = create_album(&mut catalog, "Empty");
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 25000
         )
         INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
           absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
           width, height, preview_status, source_generation)
         SELECT current.scan_id, current.asset_id, 'alias-' || value, current.root_id,
           current.absolute_path, current.relative_path, '', current.file_size, 0,
           40, 50, 'pending', current.source_generation
         FROM sequence CROSS JOIN asset_locations AS current
         WHERE current.scan_id = 'scan-a' AND current.location_id = 'a';",
        )
        .unwrap();
    for (collection, queue) in [
        (
            GalleryCollection::Album {
                album_id: empty_album,
            },
            None,
        ),
        (GalleryCollection::Favorites, None),
        (
            GalleryCollection::Library,
            Some(OrganizationQueue::Deferred),
        ),
        (GalleryCollection::Library, Some(OrganizationQueue::Ignored)),
    ] {
        for sort_key in [
            GallerySortKey::CaptureTime,
            GallerySortKey::FileName,
            GallerySortKey::LibraryAdmission,
        ] {
            let query = GalleryQuery {
                collection: collection.clone(),
                organization_queue: queue,
                sort_key,
                new_since_admission: Some(0),
                ..GalleryQuery::default()
            };
            let snapshot = bounded_read(&mut catalog, |catalog| {
                catalog.load_snapshot(10, &query, "empty", None, None, None)
            });
            assert!(snapshot.assets.is_empty());
            let timeline = bounded_read(&mut catalog, |catalog| {
                catalog.load_gallery_timeline(&query, "empty")
            });
            assert_eq!((timeline.total_items, timeline.total_source_items), (0, 0));
            assert!(timeline.buckets.is_empty());
            assert!(timeline.scroll_sections.is_empty());
            let layout = bounded_read(&mut catalog, |catalog| {
                catalog.load_gallery_layout_manifest_chunk(10, &query, "empty", None)
            });
            assert!(layout.location_ids.is_empty());
            assert_eq!(layout.total_items, 0);
        }
    }
}

#[test]
fn decision_presence_refreshes_without_changing_scoped_membership_or_arrival_parameters() {
    let (_directory, mut catalog, _) = grouped();
    let album = create_album(&mut catalog, "Destination");
    let query = GalleryQuery {
        collection: GalleryCollection::Album {
            album_id: album.clone(),
        },
        root_id: Some("root-c".into()),
        search_text: "c.png".into(),
        sort_key: GallerySortKey::LibraryAdmission,
        new_since_admission: Some(0),
        ..GalleryQuery::default()
    };
    assert!(ids(&mut catalog, &query).is_empty());
    assign(&mut catalog, "assign-destination", &["c"], &album);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let favorites = GalleryQuery {
        collection: GalleryCollection::Favorites,
        ..query.clone()
    };
    for scoped in [&query, &favorites] {
        let snapshot = catalog
            .load_query_snapshot(10, scoped, "populated", None)
            .unwrap();
        assert_eq!(snapshot.snapshot.assets.len(), 1);
        assert_eq!(snapshot.snapshot.assets[0].location_id, "c");
        assert_eq!(
            (
                snapshot.timeline.total_items,
                snapshot.timeline.total_source_items
            ),
            (1, 1)
        );
        assert_eq!(snapshot.timeline.scroll_sections.len(), 1);
        assert_eq!(snapshot.timeline.scroll_sections[0].item_count, 1);
    }
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("unfavorite", &prepared, false)
        .unwrap();
    assert!(ids(&mut catalog, &favorites).is_empty());
    assert_eq!(ids(&mut catalog, &query), ["c"]);
}

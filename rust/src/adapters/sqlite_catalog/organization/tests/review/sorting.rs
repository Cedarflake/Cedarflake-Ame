use crate::domain::{GalleryDuplicateMode, GallerySortDirection, GallerySortKey};

use super::*;

#[test]
fn queue_paging_and_continuation_follow_each_gallery_order_across_reopen() {
    for key in [
        GallerySortKey::CaptureTime,
        GallerySortKey::CreatedTime,
        GallerySortKey::ModifiedTime,
        GallerySortKey::FileName,
        GallerySortKey::LibraryAdmission,
    ] {
        for direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            let (directory, mut catalog) = fixture();
            catalog.connection.execute_batch(
                "UPDATE asset_locations SET
                   capture_local_time = CASE location_id WHEN 'a' THEN '2026-01-03T00:00:00' WHEN 'b' THEN '2026-01-01T00:00:00' END,
                   capture_time_source = CASE WHEN location_id IN ('a', 'b') THEN 'exif_original' END,
                   capture_raw_value = CASE location_id WHEN 'a' THEN '2026:01:03 00:00:00' WHEN 'b' THEN '2026:01:01 00:00:00' END,
                   file_local_time = CASE location_id WHEN 'a' THEN '2026-01-01T00:00:00' WHEN 'b' THEN '2026-01-03T00:00:00' END,
                   modified_unix_ms = CASE location_id WHEN 'a' THEN 3 WHEN 'b' THEN 1 ELSE 2 END;",
            ).unwrap();
            let query = GalleryQuery {
                sort_key: key.clone(),
                sort_direction: direction,
                ..pending()
            };
            let ordinary = GalleryQuery {
                organization_queue: None,
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..query.clone()
            };
            let expected = catalog
                .load_snapshot(10, &ordinary, "ordinary", None, None, None)
                .unwrap()
                .assets;
            let ascending = query.order_direction() == &GallerySortDirection::Ascending;
            let names = match (&query.sort_key, ascending) {
                (GallerySortKey::CaptureTime, true) => ["b", "a", "c"],
                (GallerySortKey::CaptureTime, false) => ["a", "b", "c"],
                (GallerySortKey::CreatedTime, true) => ["a", "b", "c"],
                (GallerySortKey::CreatedTime, false) => ["b", "a", "c"],
                (GallerySortKey::ModifiedTime, true) => ["b", "c", "a"],
                (GallerySortKey::ModifiedTime, false) => ["a", "c", "b"],
                (GallerySortKey::FileName, true) => ["a", "b", "c"],
                _ => ["c", "b", "a"],
            };
            assert_eq!(
                expected
                    .iter()
                    .map(|asset| asset.location_id.as_str())
                    .collect::<Vec<_>>(),
                names
            );
            let first = catalog
                .load_snapshot(1, &query, "queue", None, None, None)
                .unwrap();
            assert_eq!(
                first.assets[0].location_id, expected[0].location_id,
                "{query:?}"
            );
            let rest = catalog
                .load_snapshot(10, &query, "queue", first.next_cursor.as_ref(), None, None)
                .unwrap();
            assert_eq!(
                rest.assets
                    .iter()
                    .map(|asset| &asset.location_id)
                    .collect::<Vec<_>>(),
                expected[1..]
                    .iter()
                    .map(|asset| &asset.location_id)
                    .collect::<Vec<_>>(),
                "{query:?}"
            );
            let session = catalog.begin_organization_review(&query).unwrap();
            checkpoint(&mut catalog, &session, 1, &expected[0].location_id);
            change(
                &mut catalog,
                "handle",
                &[&expected[0].location_id],
                OrganizationAction::Ignore,
            );
            assert_eq!(
                target(&catalog).as_deref(),
                Some(expected[1].location_id.as_str()),
                "{query:?}"
            );
            drop(catalog);
            let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
            let restored = catalog.load_organization_continuation(None).unwrap();
            assert_eq!(restored.query.as_ref().unwrap().sort_key, query.sort_key);
            assert_eq!(
                restored.query.as_ref().unwrap().order_direction(),
                query.order_direction()
            );
            assert_eq!(
                restored.target.unwrap().location_id,
                expected[1].location_id
            );
        }
    }
}

#[test]
fn changed_sort_reanchors_the_saved_item_before_choosing_the_next() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "b");
    change(&mut catalog, "handle", &["b"], OrganizationAction::Ignore);
    assert_eq!(target(&catalog).as_deref(), Some("a"));
    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..pending()
    };
    let next = catalog
        .load_organization_continuation(Some(&query))
        .unwrap();
    assert_eq!(next.query.unwrap().sort_key, GallerySortKey::FileName);
    assert_eq!(next.target.unwrap().location_id, "c");
}

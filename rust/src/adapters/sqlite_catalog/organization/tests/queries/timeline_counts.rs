use super::*;

#[test]
fn repeated_representative_ids_do_not_multiply_rows_across_roots() {
    let (_directory, mut catalog) = fixture();
    let album_id = create_album(&mut catalog, "Multiple roots");
    assign(&mut catalog, "assign", &["a", "b", "c"], &album_id);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET location_id = 'a' WHERE root_id = 'root-c'",
            [],
        )
        .unwrap();
    let query = GalleryQuery {
        sort_key: GallerySortKey::CaptureTime,
        ..album_query(&album_id)
    };
    let snapshot = catalog
        .load_query_snapshot(10, &query, "overlap", None)
        .unwrap();
    assert_eq!(snapshot.snapshot.assets.len(), 3);
    assert_eq!(snapshot.timeline.total_items, 3);
    assert_eq!(snapshot.timeline.total_source_items, 3);
    assert_eq!(snapshot.timeline.buckets[0].item_count, 3);
}

#[test]
fn collection_date_buckets_keep_scoped_physical_counts_and_preferred_dates() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET
               file_identity_scheme = 'fixture', file_identity_value = location_id,
               file_local_time = CASE location_id
                 WHEN 'a' THEN '2024-01-02T00:00:00'
                 WHEN 'c' THEN '2025-03-04T00:00:00' END,
               relative_path = 'Folder/' || location_id || '.png',
               parent_relative_path = 'Folder';",
        )
        .unwrap();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album_id = create_album(&mut catalog, "Dated collection");
    assign(&mut catalog, "assign", &["c", "b"], &album_id);
    let album = album_query(&album_id);
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: "c".into(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    let prepared = prepare(&catalog, &["c", "b"]);
    catalog
        .set_organization_favorite("favorite-all", &prepared, true)
        .unwrap();
    for retained in [false, true] {
        if retained {
            evolution::invalidate(&mut catalog, "c");
        }
        for sort_key in [
            GallerySortKey::CaptureTime,
            GallerySortKey::CreatedTime,
            GallerySortKey::ModifiedTime,
        ] {
            for sort_direction in [
                GallerySortDirection::Ascending,
                GallerySortDirection::Descending,
            ] {
                for root_id in [None, Some("root-a"), Some("root-c")] {
                    let ordinary = GalleryQuery {
                        sort_key: sort_key.clone(),
                        sort_direction: sort_direction.clone(),
                        duplicate_mode: GalleryDuplicateMode::Merged,
                        root_id: root_id.map(str::to_owned),
                        folder_relative_path: root_id.map(|_| "Folder".into()),
                        ..GalleryQuery::default()
                    };
                    let expected = catalog
                        .load_gallery_timeline(&ordinary, "ordinary")
                        .unwrap();
                    for collection in [album.collection.clone(), GalleryCollection::Favorites] {
                        let query = GalleryQuery {
                            collection,
                            ..ordinary.clone()
                        };
                        let actual = catalog
                            .load_query_snapshot(1, &query, "collection", None)
                            .unwrap();
                        assert_eq!(actual.timeline.buckets, expected.buckets);
                        assert_eq!(actual.timeline.total_items, expected.total_items);
                        assert_eq!(
                            actual.timeline.total_source_items,
                            expected.total_source_items
                        );
                        assert_eq!(actual.snapshot.assets.len(), 1);
                    }
                }
            }
        }
    }
    publish_batch(&mut catalog, &["c"], 'b');
    let changed = catalog
        .load_gallery_timeline(
            &GalleryQuery {
                sort_key: GallerySortKey::CaptureTime,
                ..album
            },
            "changed-content",
        )
        .unwrap();
    assert_eq!((changed.total_items, changed.total_source_items), (2, 2));
}

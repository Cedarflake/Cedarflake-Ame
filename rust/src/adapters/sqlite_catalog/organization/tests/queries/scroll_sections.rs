use crate::domain::gallery_scroll_section::GalleryScrollSectionKind;
use crate::domain::organization_queue::OrganizationQueue;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

#[test]
fn chronological_queue_buckets_and_layout_match_the_same_ordinary_scope() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_local_time = CASE location_id
           WHEN 'a' THEN '2024-01-02T00:00:00' WHEN 'b' THEN '2025-02-01T00:00:00' END;",
        )
        .unwrap();
    for key in [
        GallerySortKey::CaptureTime,
        GallerySortKey::CreatedTime,
        GallerySortKey::ModifiedTime,
    ] {
        for direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            let ordinary = GalleryQuery {
                sort_key: key.clone(),
                sort_direction: direction,
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..GalleryQuery::default()
            };
            let queue = GalleryQuery {
                organization_queue: Some(OrganizationQueue::Pending),
                ..ordinary.clone()
            };
            let expected = catalog
                .load_query_snapshot(10, &ordinary, "ordinary", None)
                .unwrap();
            let actual = catalog
                .load_query_snapshot(10, &queue, "queue", None)
                .unwrap();
            assert!(!actual.timeline.buckets.is_empty());
            assert_eq!(actual.timeline.buckets, expected.timeline.buckets);
            assert!(actual.timeline.scroll_sections.is_empty());
            let expected_layout = catalog
                .load_gallery_layout_manifest_chunk(10, &ordinary, "ordinary", None)
                .unwrap();
            let actual_layout = catalog
                .load_gallery_layout_manifest_chunk(10, &queue, "queue", None)
                .unwrap();
            assert_eq!(actual_layout.date_groups, expected_layout.date_groups);
            let bucket = &actual.timeline.buckets[0];
            let anchor = crate::domain::GalleryTimeAnchor {
                revision: actual.timeline.revision,
                query_id: "queue".into(),
                month_key: bucket.month_key.clone(),
                item_offset: 0,
            };
            let page = catalog
                .load_snapshot(1, &queue, "queue", None, None, Some(&anchor))
                .unwrap();
            assert_eq!(
                page.assets[0].location_id,
                actual.snapshot.assets[0].location_id
            );
        }
    }
}

#[test]
fn name_sections_follow_natural_order_direction_and_filtered_group_counts() {
    let (_directory, mut catalog) = fixture();
    for (id, name) in [("a", "Apple.png"), ("b", "alpha.png"), ("c", "10.png")] {
        catalog
            .connection
            .execute(
                "UPDATE asset_locations SET natural_name_key = ?1 WHERE location_id = ?2",
                params![crate::adapters::sqlite_catalog::natural_name_key(name), id],
            )
            .unwrap();
    }
    for queue in [None, Some(OrganizationQueue::Pending)] {
        for direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            let query = GalleryQuery {
                organization_queue: queue,
                sort_key: GallerySortKey::FileName,
                sort_direction: direction.clone(),
                ..GalleryQuery::default()
            };
            let result = catalog
                .load_query_snapshot(1, &query, "names", None)
                .unwrap();
            assert!(result.timeline.buckets.is_empty());
            let actual = result
                .timeline
                .scroll_sections
                .iter()
                .map(|s| (s.key.as_str(), s.start_ordinal, s.item_count))
                .collect::<Vec<_>>();
            let expected = if direction == GallerySortDirection::Ascending {
                vec![("0-9", 0, 1), ("A", 1, 2)]
            } else {
                vec![("A", 0, 2), ("0-9", 2, 1)]
            };
            assert_eq!(actual, expected);
            let filtered = GalleryQuery {
                root_id: Some("root-c".into()),
                ..query
            };
            let result = catalog
                .load_query_snapshot(1, &filtered, "filtered", None)
                .unwrap();
            assert_eq!(result.timeline.scroll_sections[0].key, "0-9");
            assert_eq!(result.timeline.total_items, 1);
        }
    }
}

#[test]
fn arrival_range_uses_entry_boundary_and_older_duplicate_group_admission() {
    let (_directory, mut catalog) = fixture();
    let boundary: i64 = catalog.connection.query_row("SELECT admission_id FROM organization_admissions JOIN asset_locations USING(source_generation) WHERE location_id = 'b'", [], |row| row.get(0)).unwrap();
    let query = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        sort_key: GallerySortKey::LibraryAdmission,
        new_since_admission: Some(boundary.try_into().unwrap()),
        ..GalleryQuery::default()
    };
    let result = catalog
        .load_query_snapshot(1, &query, "arrivals", None)
        .unwrap();
    assert_eq!(
        result.timeline.scroll_sections[0].kind,
        GalleryScrollSectionKind::NewArrivals
    );
    assert_eq!(result.timeline.scroll_sections[0].item_count, 1);
    assert_eq!(result.timeline.scroll_sections[1].start_ordinal, 1);
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let merged = catalog
        .load_query_snapshot(1, &query, "merged", None)
        .unwrap();
    assert_eq!(merged.timeline.scroll_sections.len(), 1);
    assert_eq!(
        merged.timeline.scroll_sections[0].kind,
        GalleryScrollSectionKind::Earlier
    );
    assert_eq!(merged.timeline.scroll_sections[0].item_count, 2);
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "new-scan",
        "new-root",
        "E:\\Fixture",
        &[("d", None, 4)],
    );
    let added = catalog
        .load_query_snapshot(1, &query, "added", None)
        .unwrap();
    assert_eq!(added.snapshot.assets[0].location_id, "d");
    assert_eq!(added.timeline.scroll_sections[0].item_count, 1);
    let next_entry = GalleryQuery {
        new_since_admission: Some(catalog.load_organization_admission_boundary().unwrap()),
        ..query
    };
    let entered = catalog
        .load_query_snapshot(1, &next_entry, "entered", None)
        .unwrap();
    assert!(
        entered
            .timeline
            .scroll_sections
            .iter()
            .all(|s| s.kind == GalleryScrollSectionKind::Earlier)
    );
}

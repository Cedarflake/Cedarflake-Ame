use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::domain::GalleryDuplicateMode;

#[test]
fn snapshot_reuses_selection_across_readers_but_observes_publication_and_query_changes() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let ids: Vec<_> = (0..2048).map(|index| format!("location-{index}")).collect();
    let locations: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), None, index as i64))
        .collect();
    publish_gallery_fixture(&mut catalog, "initial", "root", "C:\\Fixture", &locations);
    let session = catalog.validated_session();
    let query = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        organization_queue: Some(crate::domain::organization_queue::OrganizationQueue::Pending),
        ..Default::default()
    };
    let work = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&work);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                counter.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .expect("observe initial query work");
    let first = catalog
        .load_query_snapshot(16, &query, TEST_QUERY_ID, None)
        .expect("first selection");
    let cold_work = work.swap(0, Ordering::Relaxed);
    let mut reader = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("new reader");
    let counter = Arc::clone(&work);
    reader
        .connection
        .progress_handler(
            100,
            Some(move || {
                counter.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .expect("observe reused query work");
    let requested = &first.snapshot.assets[0];
    let anchor = GalleryQueryAnchor {
        requested_location_id: requested.location_id.clone(),
        group_key: None,
        asset_id: Some(requested.asset_id.clone()),
        fallback_ordinal: 0,
    };
    let warm = reader
        .load_query_snapshot(16, &query, TEST_QUERY_ID, Some(&anchor))
        .expect("reused selection");
    let warm_work = work.load(Ordering::Relaxed);
    assert!(warm_work < cold_work, "cold={cold_work}, warm={warm_work}");
    assert_eq!(warm.timeline, first.timeline);
    assert_eq!(warm.organization_counts, first.organization_counts);
    assert_eq!(warm.organization_counts.unwrap().pending, 2048);
    assert_eq!(
        warm.snapshot.query_anchor_resolution.unwrap().location_id,
        Some(requested.location_id.clone()),
    );
    assert_eq!(
        warm.layout_manifest.unwrap()[0].location_ids,
        first.layout_manifest.unwrap()[0].location_ids
    );
    let other_asset = &first.snapshot.assets[1];
    let mismatched = GalleryQueryAnchor {
        asset_id: Some(other_asset.asset_id.clone()),
        ..anchor
    };
    let resolved = reader
        .load_query_snapshot(16, &query, TEST_QUERY_ID, Some(&mismatched))
        .expect("resolve asset identity rather than reuse a different location occupant");
    assert_eq!(
        resolved
            .snapshot
            .query_anchor_resolution
            .unwrap()
            .location_id,
        Some(other_asset.location_id.clone()),
    );

    let other = GalleryQuery {
        root_id: Some("absent".into()),
        ..query.clone()
    };
    assert_eq!(
        reader
            .load_query_snapshot(16, &other, TEST_QUERY_ID, None)
            .expect("different scope")
            .timeline
            .total_items,
        0
    );
    publish_fixture(&mut catalog, "later", "other", "C:\\Other", "new-location");
    let current = reader
        .load_query_snapshot(16, &query, TEST_QUERY_ID, None)
        .expect("publication");
    assert_eq!(current.timeline.total_items, 2049);
    assert_eq!(current.organization_counts.unwrap().pending, 2049);
    assert!(current.timeline.revision > first.timeline.revision);
}

#[test]
fn reused_snapshot_reads_current_geometry_without_a_catalog_revision_change() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "initial",
        "root",
        "C:\\Fixture",
        &[("one", None, 1)],
    );
    let query = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..Default::default()
    };
    let first = catalog
        .load_query_snapshot(16, &query, TEST_QUERY_ID, None)
        .expect("initial geometry");
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET width = 2000, height = 1000 WHERE location_id = 'one'",
            [],
        )
        .expect("preview dimension recovery");
    let next = catalog
        .load_query_snapshot(16, &query, TEST_QUERY_ID, None)
        .expect("current geometry");
    assert_eq!(first.timeline.revision, next.timeline.revision);
    assert_eq!(next.layout_manifest.unwrap()[0].aspect_ratio_milli, [2000]);
    assert_eq!(next.timeline.buckets[0].aspect_ratio_milli_sum, 2000);
    assert_eq!(next.snapshot.assets[0].width, 2000);
}

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn location_anchor_work_excludes_dates_after_its_preceding_range() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_time_window_fixture(&mut catalog);
    let query = GalleryQuery::default();
    let complete = catalog
        .load_snapshot(4096, &query, TEST_QUERY_ID, None, None, None)
        .expect("independent ordered oracle");
    let anchor = &complete.assets[6];
    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(100, Ordering::Relaxed);
                false
            }),
        )
        .expect("query work observer");
    let transaction = catalog.connection.transaction().expect("snapshot");
    let (resolution, cursor) =
        crate::adapters::sqlite_catalog::gallery::resolve_gallery_location_anchor(
            &transaction,
            complete.revision,
            &query,
            TEST_QUERY_ID,
            &anchor.location_id,
            4,
        )
        .expect("resolve location");
    let measured_steps = steps.load(Ordering::Relaxed);
    transaction.commit().expect("commit read");
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire observer");
    assert_eq!(resolution.ordinal, Some(6));
    assert_eq!(resolution.window_start_ordinal, 4);
    assert_eq!(
        cursor.expect("preceding cursor").location_id,
        complete.assets[3].location_id
    );
    assert!(
        measured_steps < 4_000,
        "location-anchor work must exclude the 2000 later records: {measured_steps}"
    );
}

#[test]
fn location_anchor_bounds_preserve_filtered_order_and_missing_dates() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_time_window_fixture(&mut catalog);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET file_local_time = capture_local_time,
                 parent_relative_path = CASE
                   WHEN location_id < 'location-1000' THEN 'selected'
                   ELSE 'selected/child'
                 END",
            [],
        )
        .expect("created-time and folder fixture");
    for sort_key in [GallerySortKey::CaptureTime, GallerySortKey::CreatedTime] {
        for sort_direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            for (folder, descendants, search) in [
                (None, false, ""),
                (Some("selected"), false, ""),
                (Some("selected"), true, "location-00"),
                (Some("selected/child"), false, ""),
            ] {
                let query = GalleryQuery {
                    root_id: Some("time-window-root".to_owned()),
                    folder_relative_path: folder.map(str::to_owned),
                    include_descendants: descendants,
                    search_text: search.to_owned(),
                    sort_key: sort_key.clone(),
                    sort_direction: sort_direction.clone(),
                };
                let complete = catalog
                    .load_snapshot(4096, &query, TEST_QUERY_ID, None, None, None)
                    .expect("ordered filtered oracle");
                let count = complete.assets.len();
                assert!(count > 6, "fixture must exercise a preceding window");
                for ordinal in [0, 6, count / 2, count - 1] {
                    let asset = &complete.assets[ordinal];
                    let page = catalog
                        .load_snapshot_around_location(4, &query, TEST_QUERY_ID, &asset.location_id)
                        .expect("filtered location snapshot");
                    let start = ordinal.saturating_sub(2);
                    let resolution = page.query_anchor_resolution.expect("resolved anchor");
                    assert_eq!(resolution.ordinal, Some(ordinal as u64));
                    assert_eq!(resolution.window_start_ordinal, start as u64);
                    assert_eq!(
                        location_ids(&page.assets),
                        location_ids(&complete.assets[start..(start + 4).min(count)]),
                        "{query:?}, ordinal {ordinal}"
                    );
                }
            }
        }
    }
}

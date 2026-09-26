use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn month_predecessor_work_does_not_scan_unrelated_months() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_time_window_fixture(&mut catalog);
    let query = GalleryQuery::default();
    let timeline = catalog
        .load_gallery_timeline(&query, TEST_QUERY_ID)
        .expect("timeline");
    let anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("2026-09".to_owned()),
        item_offset: 2,
    };
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
        .expect("install query work counter");
    let transaction = catalog.connection.transaction().expect("snapshot");
    let cursor = crate::adapters::sqlite_catalog::gallery::resolve_gallery_anchor_cursor(
        &transaction,
        timeline.revision,
        &query,
        TEST_QUERY_ID,
        &anchor,
    )
    .expect("indexed predecessor");
    let indexed_steps = steps.swap(0, Ordering::Relaxed);

    let old_cursor: String = transaction
        .query_row(
            "SELECT locations.location_id
             FROM library_roots AS roots
             JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
             WHERE CASE
               WHEN COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL
               THEN NULL
               ELSE substr(COALESCE(locations.capture_local_time, locations.file_local_time), 1, 7)
             END = '2026-09'
             ORDER BY (COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL),
                      IFNULL(COALESCE(locations.capture_local_time, locations.file_local_time), '') DESC,
                      locations.modified_unix_ms DESC, locations.root_id, locations.location_id
             LIMIT 1 OFFSET 1",
            [],
            |row| row.get(0),
        )
        .expect("original predicate control");
    let unbounded_steps = steps.load(Ordering::Relaxed);
    transaction.commit().expect("read commit");
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire query work counter");
    assert_eq!(cursor.location_id, old_cursor);
    assert!(indexed_steps < 4_000, "indexed steps: {indexed_steps}");
    assert!(
        unbounded_steps > 20_000,
        "the original predicate must reject the same work bound: {unbounded_steps}"
    );
}

#[test]
fn month_bounds_preserve_december_unknown_dates_filters_and_order() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let locations = [
        ("december-a", Some("9999-12-30T12:00:00"), 2),
        ("december-b", Some("9999-12-30T12:00:00"), 2),
        ("december-c", Some("9999-12-31T12:00:00"), 1),
        ("september-a", Some("2025-09-01T12:00:00"), 4),
        ("september-b", Some("2025-09-01T12:00:00"), 4),
        ("october", Some("2025-10-01T12:00:00"), 3),
        ("unknown-a", None, 5),
        ("unknown-b", None, 5),
    ];
    for (scan, root) in [
        ("bounds-scan-a", "bounds-root-a"),
        ("bounds-scan-b", "bounds-root-b"),
    ] {
        let root_path = format!("C:\\Generated\\{root}");
        publish_gallery_fixture(&mut catalog, scan, root, &root_path, &locations);
    }
    catalog.connection.execute(
        "UPDATE asset_locations
         SET file_local_time = capture_local_time,
             parent_relative_path = CASE WHEN location_id LIKE '%-b' THEN 'other' ELSE 'selected' END",
        [],
    ).expect("created-time and folder fixture");

    for sort_key in [GallerySortKey::CaptureTime, GallerySortKey::CreatedTime] {
        for sort_direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            for (root_id, folder, search) in [
                (None, None, ""),
                (Some("bounds-root-a"), None, ""),
                (Some("bounds-root-b"), Some("selected"), ""),
                (None, None, "december"),
            ] {
                let query = GalleryQuery {
                    root_id: root_id.map(str::to_owned),
                    folder_relative_path: folder.map(str::to_owned),
                    search_text: search.to_owned(),
                    sort_key: sort_key.clone(),
                    sort_direction: sort_direction.clone(),
                    ..GalleryQuery::default()
                };
                let complete = catalog
                    .load_snapshot(160, &query, TEST_QUERY_ID, None, None, None)
                    .expect("independent full query oracle");
                let timeline = catalog
                    .load_gallery_timeline(&query, TEST_QUERY_ID)
                    .expect("timeline");
                let mut start = 0;
                for bucket in &timeline.buckets {
                    let count = usize::try_from(bucket.item_count).expect("fixture count");
                    for offset in 1..count {
                        let anchor = GalleryTimeAnchor {
                            revision: timeline.revision,
                            query_id: TEST_QUERY_ID.to_owned(),
                            month_key: bucket.month_key.clone(),
                            item_offset: offset as u64,
                        };
                        let page = catalog
                            .load_snapshot(3, &query, TEST_QUERY_ID, None, None, Some(&anchor))
                            .expect("anchored window");
                        let ordinal = start + offset;
                        assert_eq!(
                            page.assets
                                .iter()
                                .map(|asset| (&asset.root_id, &asset.location_id))
                                .collect::<Vec<_>>(),
                            complete.assets[ordinal..(ordinal + 3).min(complete.assets.len())]
                                .iter()
                                .map(|asset| (&asset.root_id, &asset.location_id))
                                .collect::<Vec<_>>(),
                            "month bounds must preserve query order and membership: {query:?} {anchor:?}"
                        );
                    }
                    start += count;
                }
            }
        }
    }
}

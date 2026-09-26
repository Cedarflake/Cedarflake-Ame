use super::*;
use crate::domain::GalleryTimeIntent;

#[test]
fn current_time_snapshot_matches_exact_ordinals_and_retains_strict_cursors() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_time_window_fixture(&mut catalog);
    let before = catalog
        .load_gallery_timeline(&GalleryQuery::default(), TEST_QUERY_ID)
        .expect("old timeline");
    publish_fixture(
        &mut catalog,
        "other-scan",
        "other-root",
        "C:\\Other",
        "other",
    );
    let old = GalleryTimeAnchor {
        revision: before.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("2012-03".to_owned()),
        item_offset: 10,
    };
    assert_eq!(
        catalog
            .load_snapshot(
                160,
                &GalleryQuery::default(),
                TEST_QUERY_ID,
                None,
                None,
                Some(&old)
            )
            .expect_err("strict cursor stays stale")
            .code,
        "catalog_cursor_stale"
    );
    for direction in [
        GallerySortDirection::Ascending,
        GallerySortDirection::Descending,
    ] {
        let query = GalleryQuery {
            sort_direction: direction,
            ..GalleryQuery::default()
        };
        let complete = catalog
            .load_snapshot(4096, &query, TEST_QUERY_ID, None, None, None)
            .expect("independent complete oracle");
        for month in [
            Some("2012-03"),
            Some("2011-06"),
            Some("2000-01"),
            Some("2030-01"),
            None,
        ] {
            let result = catalog
                .load_time_snapshot(
                    160,
                    &query,
                    TEST_QUERY_ID,
                    &GalleryTimeIntent {
                        month_key: month.map(str::to_owned),
                        item_offset: 10,
                    },
                )
                .expect("current semantic date read");
            let anchor = result.anchor.expect("nonempty result");
            let start = usize::try_from(result.window_start_ordinal).expect("bounded fixture");
            assert!(result.snapshot.revision > before.revision);
            assert_eq!(anchor.revision, result.timeline.revision);
            assert_eq!(result.snapshot.revision, result.timeline.revision);
            assert_eq!(
                location_ids(&result.snapshot.assets),
                location_ids(&complete.assets[start..(start + 160).min(complete.assets.len())])
            );
            if month == Some("2012-03") {
                assert_eq!(anchor.month_key.as_deref(), Some("2012-03"));
                assert_eq!(anchor.item_offset, 10);
            }
        }
    }
}

#[test]
fn current_time_snapshot_keeps_resolution_and_page_across_concurrent_deletion() {
    let directory = tempdir().expect("temporary catalog");
    let path = directory.path().join("catalog.sqlite3");
    let mut reader = SqliteCatalog::open(path.clone()).expect("reader");
    publish_time_window_fixture(&mut reader);
    let complete = reader
        .load_snapshot(
            4096,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            None,
        )
        .expect("independent pre-deletion order");
    let mut writer = SqliteCatalog::open(path).expect("writer");
    crate::adapters::sqlite_catalog::gallery_time_snapshot::set_after_time_resolution_hook(
        move || {
            writer
                .unregister_root("time-window-root")
                .expect("concurrent root removal");
        },
    );
    let intent = GalleryTimeIntent {
        month_key: Some("2012-03".to_owned()),
        item_offset: 10,
    };
    let result = reader
        .load_time_snapshot(160, &GalleryQuery::default(), TEST_QUERY_ID, &intent)
        .expect("one coherent read transaction");
    assert_eq!(result.timeline.total_items, 2012);
    assert_eq!(result.snapshot.assets.len(), 160);
    assert_eq!(result.window_start_ordinal, 0);
    assert_eq!(result.anchor.as_ref().expect("target").item_offset, 10);
    assert_eq!(
        location_ids(&result.snapshot.assets),
        location_ids(&complete.assets[..160])
    );
    assert_eq!(result.snapshot.revision, result.timeline.revision);
    let after = reader
        .load_time_snapshot(160, &GalleryQuery::default(), TEST_QUERY_ID, &intent)
        .expect("next read observes removal");
    assert!(after.snapshot.revision > result.snapshot.revision);
    assert_eq!(after.timeline.total_items, 0);
    assert!(after.snapshot.assets.is_empty());
    assert!(after.anchor.is_none());
    assert_eq!(after.window_start_ordinal, 0);
}

#[test]
fn current_time_snapshot_preserves_sort_and_month_rejections() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_time_window_fixture(&mut catalog);
    for (sort_key, month, code) in [
        (
            GallerySortKey::FileName,
            Some("2012-03"),
            "catalog_time_anchor_unavailable",
        ),
        (
            GallerySortKey::ModifiedTime,
            None,
            "catalog_time_anchor_invalid",
        ),
        (
            GallerySortKey::CaptureTime,
            Some("2012-99"),
            "catalog_time_anchor_invalid",
        ),
        (
            GallerySortKey::CaptureTime,
            Some("旧日期"),
            "catalog_time_anchor_invalid",
        ),
    ] {
        let query = GalleryQuery {
            sort_key,
            ..GalleryQuery::default()
        };
        let intent = GalleryTimeIntent {
            month_key: month.map(str::to_owned),
            item_offset: 0,
        };
        assert_eq!(
            catalog
                .load_time_snapshot(160, &query, TEST_QUERY_ID, &intent)
                .expect_err("invalid time contract")
                .code,
            code
        );
    }
}

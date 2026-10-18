use crate::adapters::sqlite_catalog::tests::support::{
    TEST_QUERY_ID, load_default_timeline, publish_gallery_dimension_fixture,
    publish_gallery_fixture,
};

use tempfile::tempdir;

use crate::domain::GalleryTimeBucket;

use crate::adapters::sqlite_catalog::*;

fn load_default_layout_manifest_chunk(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    catalog.load_gallery_layout_manifest_chunk(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
    )
}

#[test]
fn gallery_timeline_uses_file_time_when_capture_time_is_missing() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-1",
        "timeline-root-1",
        "C:\\One",
        &[
            ("one-2025-a", Some("2025-08-07T03:04:05.000000000"), 10),
            ("one-2025-b", Some("2025-08-01T03:04:05.000000000"), 20),
            ("one-unknown", None, 30),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-2",
        "timeline-root-2",
        "C:\\Two",
        &[
            ("two-2026", Some("2026-01-02T03:04:05.000000000"), 40),
            ("two-2024", Some("2024-12-31T03:04:05.000000000"), 50),
            ("two-unknown", None, 60),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.revision, 2);
    assert_eq!(timeline.total_items, 6);
    assert_eq!(
        timeline.buckets,
        [
            GalleryTimeBucket {
                month_key: Some("2026-01".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("2025-08".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
            GalleryTimeBucket {
                month_key: Some("2024-12".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("1970-01".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
        ]
    );

    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-3",
        "timeline-root-1",
        "C:\\One",
        &[("one-replacement", Some("2023-07-01T03:04:05.000000000"), 70)],
    );
    let rescanned = load_default_timeline(&mut catalog).expect("timeline after rescan");
    assert_eq!(rescanned.revision, 3);
    assert_eq!(rescanned.total_items, 4);
    assert!(
        rescanned
            .buckets
            .iter()
            .all(|bucket| { bucket.month_key.as_deref() != Some("2025-08") })
    );
}

#[test]
fn gallery_timeline_bounds_aspect_ratio_weight_and_falls_back_for_missing_dimensions() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "timeline-dimensions",
        "timeline-dimensions-root",
        "C:\\Dimensions",
        &[
            (
                "panorama",
                Some("2026-01-02T03:04:05.000000000"),
                10,
                1000,
                100,
            ),
            (
                "portrait",
                Some("2026-01-02T03:04:05.000000000"),
                20,
                100,
                1000,
            ),
            (
                "square",
                Some("2026-01-02T03:04:05.000000000"),
                30,
                500,
                500,
            ),
            ("missing", Some("2026-01-02T03:04:05.000000000"), 40, 0, 0),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.buckets.len(), 1);
    assert_eq!(timeline.buckets[0].item_count, 4);
    assert_eq!(timeline.buckets[0].aspect_ratio_milli_sum, 7_200);
}

#[test]
fn layout_manifest_chunks_preserve_order_ordinals_and_final_geometry_evidence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "layout-manifest-scan",
        "layout-manifest-root",
        "C:\\Layout",
        &[
            ("wide", Some("2026-08-09T12:00:00"), 4_000, 200, 100),
            (
                "unknown-dimensions",
                Some("2026-08-09T11:00:00"),
                3_000,
                0,
                0,
            ),
            (
                "extreme-wide",
                Some("2026-08-08T12:00:00"),
                2_000,
                1_000,
                100,
            ),
            ("extreme-tall", Some("2026-08-07T12:00:00"), 1_000, 10, 100),
        ],
    );

    let first = load_default_layout_manifest_chunk(&mut catalog, 2, None)
        .expect("first layout manifest chunk");
    assert_eq!(first.start_ordinal, 0);
    assert_eq!(first.total_items, 4);
    assert_eq!(first.location_ids, ["wide", "unknown-dimensions"]);
    assert_eq!(first.aspect_ratio_milli, [2_000, 1_000]);
    assert_eq!(first.flags, [LAYOUT_FLAG_DIMENSIONS_KNOWN, 0]);
    assert_eq!(first.date_group_indices, [0, 0]);
    assert_eq!(
        first.date_groups,
        [GalleryLayoutDateGroup {
            date_key: Some("2026-08-09".to_owned()),
        }]
    );

    let cursor = first.next_cursor.expect("next layout cursor");
    assert_eq!(cursor.next_ordinal, 2);
    assert_eq!(cursor.total_items, 4);
    let second = load_default_layout_manifest_chunk(&mut catalog, 2, Some(&cursor))
        .expect("second layout manifest chunk");
    assert_eq!(second.start_ordinal, 2);
    assert_eq!(second.total_items, 4);
    assert_eq!(second.location_ids, ["extreme-wide", "extreme-tall"]);
    assert_eq!(second.aspect_ratio_milli, [5_000, 200]);
    assert_eq!(second.flags, [1, 1]);
    assert_eq!(second.date_group_indices, [0, 1]);
    assert_eq!(
        second.date_groups,
        [
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-08".to_owned()),
            },
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-07".to_owned()),
            },
        ]
    );
    assert!(second.next_cursor.is_none());
}

#[test]
fn layout_manifest_rejects_invalid_limits_and_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-1",
        "layout-stale-root-1",
        "C:\\One",
        &[("first", Some("2026-08-09T12:00:00"), 1_000)],
    );

    let zero_limit = load_default_layout_manifest_chunk(&mut catalog, 0, None)
        .expect_err("zero layout chunk limit must fail");
    assert_eq!(zero_limit.code, "catalog_layout_chunk_limit_invalid");
    let excessive_limit =
        load_default_layout_manifest_chunk(&mut catalog, MAX_LAYOUT_MANIFEST_CHUNK_ITEMS + 1, None)
            .expect_err("excessive layout chunk limit must fail");
    assert_eq!(excessive_limit.code, "catalog_layout_chunk_limit_invalid");

    let first = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("first layout manifest chunk");
    let cursor = first.next_cursor;
    assert!(cursor.is_none(), "one item does not produce a next cursor");

    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-2",
        "layout-stale-root-2",
        "C:\\Two",
        &[("second", Some("2026-08-08T12:00:00"), 900)],
    );
    let page = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("layout page with a cursor");
    let cursor = page.next_cursor.expect("next layout cursor");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-3",
        "layout-stale-root-3",
        "C:\\Three",
        &[("third", Some("2026-08-07T12:00:00"), 800)],
    );

    let stale = load_default_layout_manifest_chunk(&mut catalog, 1, Some(&cursor))
        .expect_err("stale layout cursor must fail");
    assert_eq!(stale.code, "catalog_layout_cursor_stale");
}

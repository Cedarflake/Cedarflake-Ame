use crate::adapters::sqlite_catalog::tests::support::{
    TEST_QUERY_ID, load_default_timeline, publish_gallery_fixture, publish_gallery_query_fixture,
};

use tempfile::tempdir;

use crate::domain::{GallerySortDirection, GallerySortKey};

use crate::adapters::sqlite_catalog::*;

mod fallback_position;

#[test]
fn gallery_location_anchor_resolves_name_order_and_direction() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-scan",
        "anchor-root",
        "C:\\Pictures",
        &[
            ("img10", "img10.png", None, Some(10), 10),
            ("img2", "img2.png", None, Some(20), 20),
            ("img1", "img1.png", None, Some(30), 30),
            ("img3", "img3.png", None, Some(40), 40),
            ("img4", "img4.png", None, Some(50), 50),
        ],
    );
    let ascending = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_location(3, &ascending, "name-ascending", "img3")
        .expect("ascending location anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("img3"));
    assert_eq!(resolution.ordinal, Some(2));
    assert_eq!(resolution.window_start_ordinal, 1);
    assert_eq!(
        page.assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img3", "img4"]
    );

    let descending = GalleryQuery {
        sort_direction: GallerySortDirection::Descending,
        ..ascending
    };
    let page = catalog
        .load_snapshot_around_location(3, &descending, "name-descending", "img3")
        .expect("descending location anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.ordinal, Some(2));
    assert_eq!(resolution.window_start_ordinal, 1);
    assert_eq!(
        page.assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img4", "img3", "img2"]
    );
}

#[test]
fn gallery_asset_anchor_follows_a_renamed_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "asset-anchor-scan",
        "asset-anchor-root",
        "C:\\Pictures",
        &[
            ("before", "Album/before.png", None, Some(10), 10),
            ("neighbor", "Album/neighbor.png", None, Some(20), 20),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET location_id = 'after', relative_path = 'Album/after.png',
                 absolute_path = 'C:\\Pictures\\Album\\after.png'
             WHERE location_id = 'before'",
            [],
        )
        .expect("rename active location while retaining its asset identity");

    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "stable-asset-anchor",
            "before",
            "asset-asset-anchor-scan-before",
            0,
        )
        .expect("stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.requested_location_id, "before");
    assert_eq!(resolution.location_id.as_deref(), Some("after"));
    assert_eq!(resolution.ordinal, Some(0));
    assert_eq!(page.assets[0].asset_id, "asset-asset-anchor-scan-before");
    assert_eq!(page.assets[0].location_id, "after");
}

#[test]
fn gallery_asset_anchor_prefers_the_requested_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "multi-location-scan",
        "multi-location-root",
        "C:\\Pictures",
        &[
            ("first", "Album/first.png", None, Some(10), 10),
            ("second", "Album/second.png", None, Some(20), 20),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET asset_id = 'asset-multi-location-scan-first'
             WHERE location_id = 'second'",
            [],
        )
        .expect("attach second location to the same asset");
    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };

    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "multi-location-anchor",
            "second",
            "asset-multi-location-scan-first",
            1,
        )
        .expect("preferred stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("second"));
    assert_eq!(resolution.ordinal, Some(1));
    let preferred = catalog
        .load_active_location_by_asset_id("asset-multi-location-scan-first", Some("second"))
        .expect("load preferred location")
        .expect("preferred asset location");
    assert_eq!(preferred.location_id, "second");
}

#[test]
fn missing_asset_anchor_falls_back_to_the_nearest_surviving_ordinal() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "missing-anchor-scan",
        "missing-anchor-root",
        "C:\\Pictures",
        &[
            ("first", "Album/first.png", None, Some(10), 10),
            ("middle", "Album/middle.png", None, Some(20), 20),
            ("third", "Album/third.png", None, Some(30), 30),
        ],
    );
    catalog
        .connection
        .execute(
            "DELETE FROM asset_locations WHERE location_id = 'middle'",
            [],
        )
        .expect("remove anchor location");
    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };

    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "missing-asset-anchor",
            "middle",
            "asset-missing-anchor-scan-middle",
            1,
        )
        .expect("fallback stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("third"));
    assert_eq!(resolution.ordinal, Some(1));
    assert_eq!(resolution.window_start_ordinal, 0);
}

#[test]
fn gallery_location_anchor_falls_back_when_filtered_out() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-filter-scan",
        "anchor-filter-root",
        "C:\\Pictures",
        &[
            ("match", "Album/match.png", None, Some(10), 10),
            ("excluded", "Other/excluded.png", None, Some(20), 20),
        ],
    );
    let query = GalleryQuery {
        root_id: Some("anchor-filter-root".to_owned()),
        folder_relative_path: Some("Album".to_owned()),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_location(3, &query, "filtered", "excluded")
        .expect("filtered location fallback");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.requested_location_id, "excluded");
    assert_eq!(resolution.location_id, None);
    assert_eq!(resolution.ordinal, None);
    assert_eq!(resolution.window_start_ordinal, 0);
    assert_eq!(page.assets[0].location_id, "match");
}

#[test]
fn gallery_location_anchor_covers_time_sort_boundaries_missing_values_and_ties() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-time-scan-a",
        "anchor-time-root-a",
        "C:\\TimeA",
        &[
            (
                "low",
                "same.png",
                Some("2024-01-01T00:00:01.000000000"),
                Some(1_000),
                1_000,
            ),
            (
                "tie-a",
                "same.png",
                Some("2024-01-01T00:00:02.000000000"),
                Some(2_000),
                2_000,
            ),
        ],
    );
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-time-scan-b",
        "anchor-time-root-b",
        "C:\\TimeB",
        &[
            (
                "tie-b",
                "same.png",
                Some("2024-01-01T00:00:02.000000000"),
                Some(2_000),
                2_000,
            ),
            (
                "high",
                "same.png",
                Some("2024-01-01T00:00:03.000000000"),
                Some(3_000),
                3_000,
            ),
            ("missing", "same.png", None, None, 4_000),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_local_time = NULL WHERE location_id = 'missing'",
            [],
        )
        .expect("clear derived file time for missing-value fixture");

    for (sort_key, ascending, descending) in [
        (
            GallerySortKey::CaptureTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["high", "tie-a", "tie-b", "low", "missing"],
        ),
        (
            GallerySortKey::CreatedTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["high", "tie-a", "tie-b", "low", "missing"],
        ),
        (
            GallerySortKey::ModifiedTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["missing", "high", "tie-a", "tie-b", "low"],
        ),
    ] {
        for (direction, expected) in [
            (GallerySortDirection::Ascending, ascending),
            (GallerySortDirection::Descending, descending),
        ] {
            let query = GalleryQuery {
                sort_key: sort_key.clone(),
                sort_direction: direction.clone(),
                ..GalleryQuery::default()
            };
            let query_id = format!("anchor-{sort_key:?}-{direction:?}");

            let tied = catalog
                .load_snapshot_around_location(3, &query, &query_id, "tie-b")
                .expect("resolve tied time anchor");
            let tied_resolution = tied.query_anchor_resolution.expect("tie resolution");
            let tied_ordinal = expected
                .iter()
                .position(|location_id| *location_id == "tie-b")
                .expect("tie fixture ordinal");
            let tied_window_start = tied_ordinal.saturating_sub(1);
            assert_eq!(tied_resolution.ordinal, Some(tied_ordinal as u64));
            assert_eq!(
                tied_resolution.window_start_ordinal,
                tied_window_start as u64
            );
            assert_eq!(
                tied.assets
                    .iter()
                    .map(|asset| asset.location_id.as_str())
                    .collect::<Vec<_>>(),
                expected[tied_window_start..tied_window_start + 3]
            );

            let first = catalog
                .load_snapshot_around_location(3, &query, &query_id, expected[0])
                .expect("resolve first time anchor");
            let first_resolution = first.query_anchor_resolution.expect("first resolution");
            assert_eq!(first_resolution.ordinal, Some(0));
            assert_eq!(first_resolution.window_start_ordinal, 0);
            assert_eq!(first.assets[0].location_id, expected[0]);

            let last = catalog
                .load_snapshot_around_location(3, &query, &query_id, expected[4])
                .expect("resolve last time anchor");
            let last_resolution = last.query_anchor_resolution.expect("last resolution");
            assert_eq!(last_resolution.ordinal, Some(4));
            assert_eq!(last_resolution.window_start_ordinal, 3);
            assert_eq!(
                last.assets.last().expect("last window item").location_id,
                expected[4]
            );
        }
    }
}

#[test]
fn gallery_anchor_cursors_begin_at_capture_or_fallback_month_without_gaps() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "anchor-scan",
        "anchor-root",
        "C:\\Pictures",
        &[
            ("newer", Some("2026-02-01T00:00:00.000000000"), 50),
            ("selected-new", Some("2025-08-20T00:00:00.000000000"), 40),
            ("selected-old", Some("2025-08-01T00:00:00.000000000"), 30),
            ("older", Some("2024-01-01T00:00:00.000000000"), 20),
            ("unknown", None, 10),
        ],
    );
    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    let month_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("2025-08".to_owned()),
        item_offset: 0,
    };
    let month_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_anchor),
        )
        .expect("month page");
    assert_eq!(
        month_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-new", "selected-old", "older"]
    );
    assert!(month_page.next_cursor.is_some());
    assert!(month_page.previous_cursor.is_some());

    let month_middle = GalleryTimeAnchor {
        item_offset: 1,
        ..month_anchor.clone()
    };
    let middle_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_middle),
        )
        .expect("month middle page");
    assert_eq!(
        middle_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-old", "older", "unknown"]
    );
    let first_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            middle_page.previous_cursor.as_ref(),
            None,
        )
        .expect("first previous page");
    assert_eq!(first_previous_page.assets[0].location_id, "selected-new");
    assert!(first_previous_page.previous_cursor.is_some());
    assert!(first_previous_page.next_cursor.is_some());

    let oldest_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            first_previous_page.previous_cursor.as_ref(),
            None,
        )
        .expect("oldest previous page");
    assert_eq!(oldest_previous_page.assets[0].location_id, "newer");
    assert!(oldest_previous_page.previous_cursor.is_none());
    assert!(oldest_previous_page.next_cursor.is_some());

    let fallback_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("1970-01".to_owned()),
        item_offset: 0,
    };
    let fallback_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&fallback_anchor),
        )
        .expect("fallback page");
    assert_eq!(fallback_page.assets.len(), 1);
    assert_eq!(fallback_page.assets[0].location_id, "unknown");
    assert!(fallback_page.next_cursor.is_none());
}

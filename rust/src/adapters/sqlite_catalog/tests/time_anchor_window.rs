use super::*;
use crate::domain::GalleryQueryAnchor;
use crate::ports::GalleryQueryRepository;

#[test]
fn time_anchor_and_reverse_windows_match_complete_query_ordinals() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    publish_time_window_fixture(&mut catalog);

    for sort_direction in [
        GallerySortDirection::Ascending,
        GallerySortDirection::Descending,
    ] {
        let query = GalleryQuery {
            sort_direction,
            ..GalleryQuery::default()
        };
        let complete = catalog
            .load_snapshot(4096, &query, TEST_QUERY_ID, None, None, None)
            .expect("complete bounded oracle");
        let timeline = catalog
            .load_gallery_timeline(&query, TEST_QUERY_ID)
            .expect("coherent timeline");
        assert_eq!(complete.assets.len(), 2012);
        assert_eq!(timeline.total_items, 2012);
        let mut preceding = 0;
        for bucket in &timeline.buckets {
            let count = usize::try_from(bucket.item_count).expect("bounded fixture count");
            let mut offsets = vec![0, 1, count.saturating_sub(4), count - 1];
            offsets.sort_unstable();
            offsets.dedup();
            for offset in offsets {
                let anchor = GalleryTimeAnchor {
                    revision: timeline.revision,
                    query_id: TEST_QUERY_ID.to_owned(),
                    month_key: bucket.month_key.clone(),
                    item_offset: offset as u64,
                };
                let window = catalog
                    .load_snapshot(160, &query, TEST_QUERY_ID, None, None, Some(&anchor))
                    .expect("anchored detail window");
                let ordinal = preceding + offset;
                let expected_end = (ordinal + 160).min(complete.assets.len());
                assert_eq!(
                    location_ids(&window.assets),
                    location_ids(&complete.assets[ordinal..expected_end]),
                    "time anchor must start at its timeline ordinal: {anchor:?}",
                );
                let previous = catalog
                    .load_snapshot(
                        500,
                        &query,
                        TEST_QUERY_ID,
                        None,
                        window.previous_cursor.as_ref(),
                        None,
                    )
                    .expect("reverse page");
                assert_eq!(
                    location_ids(&previous.assets),
                    location_ids(&complete.assets[ordinal.saturating_sub(500)..ordinal]),
                    "reverse page must preserve the anchored window's ordinal: {anchor:?}",
                );
            }
            preceding += count;
        }
    }
}

#[test]
fn query_anchor_resolution_matches_the_published_window() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    publish_time_window_fixture(&mut catalog);

    for sort_key in [
        GallerySortKey::CaptureTime,
        GallerySortKey::CreatedTime,
        GallerySortKey::ModifiedTime,
        GallerySortKey::FileName,
    ] {
        for sort_direction in [
            GallerySortDirection::Ascending,
            GallerySortDirection::Descending,
        ] {
            let query = GalleryQuery {
                sort_key: sort_key.clone(),
                sort_direction,
                ..GalleryQuery::default()
            };
            let complete = catalog
                .load_snapshot(4096, &query, TEST_QUERY_ID, None, None, None)
                .expect("complete bounded oracle");
            for ordinal in [0, 11, 250, 1006, 1758, 2008, 2011] {
                let asset = &complete.assets[ordinal];
                let anchor = GalleryQueryAnchor {
                    requested_location_id: asset.location_id.clone(),
                    asset_id: Some(asset.asset_id.clone()),
                    fallback_ordinal: ordinal as u64,
                };
                let result = catalog
                    .load_query_snapshot(500, &query, TEST_QUERY_ID, Some(&anchor))
                    .expect("anchored query snapshot");
                let resolution = result
                    .snapshot
                    .query_anchor_resolution
                    .expect("anchor resolution");
                let window_start = ordinal.saturating_sub(250);
                let window_end = (window_start + 500).min(complete.assets.len());
                assert_eq!(resolution.ordinal, Some(ordinal as u64));
                assert_eq!(resolution.window_start_ordinal, window_start as u64);
                assert_eq!(
                    location_ids(&result.snapshot.assets),
                    location_ids(&complete.assets[window_start..window_end]),
                    "resolved anchor and window must share one ordinal: {query:?}, {ordinal}",
                );
            }
        }
    }
}

#[test]
fn recovered_preview_capture_time_retires_the_old_timeline() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    publish_time_window_fixture(&mut catalog);
    let query = GalleryQuery::default();
    let before = catalog
        .load_query_snapshot(500, &query, TEST_QUERY_ID, None)
        .expect("initial coherent query");
    let bucket = &before.timeline.buckets[0];
    let anchor = GalleryTimeAnchor {
        revision: before.timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: bucket.month_key.clone(),
        item_offset: 1,
    };
    let mut location = before.snapshot.assets[0].clone();
    location.capture_time = Some(CaptureTimeEvidence {
        local_time: "2000-01-02T03:04:05.000000000".to_owned(),
        offset_minutes: None,
        source: CaptureTimeSource::Original,
        raw_value: "2000:01:02 03:04:05".to_owned(),
    });
    catalog
        .update_active_preview(&location, None, None)
        .expect("publish recovered capture metadata");
    let after = catalog
        .load_gallery_timeline(&query, TEST_QUERY_ID)
        .expect("current timeline");
    assert_eq!(after.revision, before.timeline.revision + 1);
    assert_eq!(
        catalog
            .load_snapshot(160, &query, TEST_QUERY_ID, None, None, Some(&anchor))
            .expect_err("old timeline cannot authorize a reordered window")
            .code,
        "catalog_cursor_stale"
    );
    catalog
        .update_active_preview(&location, None, None)
        .expect("repeat compatible preview publication");
    assert_eq!(
        catalog
            .load_gallery_timeline(&query, TEST_QUERY_ID)
            .expect("unchanged timeline")
            .revision,
        after.revision,
        "unchanged capture metadata must not cause another invalidation"
    );
}

fn publish_time_window_fixture(catalog: &mut SqliteCatalog) {
    let identities = (0..2012)
        .map(|index| format!("location-{index:04}"))
        .collect::<Vec<_>>();
    let locations = identities
        .iter()
        .enumerate()
        .map(|(index, identity)| {
            let capture_time = match index {
                0..12 => Some("2026-09-03T12:00:00.000000000"),
                12..1512 => Some("2012-03-04T12:00:00.000000000"),
                1512..2000 => Some("2010-01-02T12:00:00.000000000"),
                _ => None,
            };
            (identity.as_str(), capture_time, (index % 8) as i64)
        })
        .collect::<Vec<_>>();
    publish_gallery_fixture(
        catalog,
        "time-window-scan",
        "time-window-root",
        "C:\\Generated-Time-Window",
        &locations,
    );
}

fn location_ids(assets: &[AssetLocationView]) -> Vec<&str> {
    assets
        .iter()
        .map(|asset| asset.location_id.as_str())
        .collect()
}

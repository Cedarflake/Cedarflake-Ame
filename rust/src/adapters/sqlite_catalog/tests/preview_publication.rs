use super::*;

#[test]
fn preview_dimensions_and_engine_changes_preserve_gallery_revision() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let (mut location, revision) = publish_preview_metadata_fixture(&mut catalog);
    location.width = 320;
    location.height = 240;
    location.metadata_engine_version = "2".to_owned();
    catalog
        .update_active_preview(&location, None, None)
        .expect("publish dimensions and engine evidence");
    let current = catalog
        .load_snapshot(5, &GalleryQuery::default(), TEST_QUERY_ID, None, None, None)
        .expect("current query");
    assert_eq!(current.revision, revision);
    assert_eq!(
        (current.assets[0].width, current.assets[0].height),
        (320, 240)
    );
    assert_eq!(current.assets[0].metadata_engine_version, "2");
}

#[test]
fn preview_recovers_missing_capture_time_but_stale_source_cannot_reorder() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let (mut location, revision) = publish_preview_metadata_fixture(&mut catalog);
    assert!(location.capture_time.is_none());
    location.capture_time = Some(CaptureTimeEvidence {
        local_time: "2012-03-04T09:10:11.000000000".to_owned(),
        offset_minutes: None,
        source: CaptureTimeSource::Original,
        raw_value: "2012:03:04 09:10:11".to_owned(),
    });
    let mut stale = location.clone();
    stale.source_generation += 1;
    assert_eq!(
        catalog
            .update_active_preview(&stale, None, None)
            .expect_err("superseded source must not change metadata")
            .code,
        "active_preview_location_stale"
    );
    let retained = catalog
        .load_snapshot(5, &GalleryQuery::default(), TEST_QUERY_ID, None, None, None)
        .expect("unchanged query after rejected publication");
    assert_eq!(retained.revision, revision);
    assert!(retained.assets[0].capture_time.is_none());
    catalog
        .update_active_preview(&location, None, None)
        .expect("publish current source metadata");
    let current = catalog
        .load_snapshot(5, &GalleryQuery::default(), TEST_QUERY_ID, None, None, None)
        .expect("reordered query");
    assert_eq!(current.revision, revision + 1);
    assert_eq!(current.assets[0].location_id, "known-date");
    assert_eq!(current.assets[1].location_id, "deferred-date");
}

fn publish_preview_metadata_fixture(catalog: &mut SqliteCatalog) -> (AssetLocationView, u64) {
    publish_gallery_query_fixture(
        catalog,
        "preview-metadata-scan",
        "preview-metadata-root",
        "C:\\Generated-Preview-Metadata",
        &[
            (
                "deferred-date",
                "deferred.jpg",
                None,
                Some(1_790_000_000_000),
                1_790_000_000_000,
            ),
            (
                "known-date",
                "known.jpg",
                Some("2020-01-02T03:04:05.000000000"),
                None,
                1,
            ),
        ],
    );
    let snapshot = catalog
        .load_snapshot(5, &GalleryQuery::default(), TEST_QUERY_ID, None, None, None)
        .expect("initial preview metadata fixture");
    assert_eq!(snapshot.assets[0].location_id, "deferred-date");
    (snapshot.assets[0].clone(), snapshot.revision)
}

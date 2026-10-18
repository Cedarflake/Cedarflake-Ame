use crate::adapters::sqlite_catalog::tests::support::{
    TEST_QUERY_ID, exact_preview_request, fixture_request, insert_active_identity_alias,
    publish_fixture, publish_gallery_query_fixture,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

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

#[test]
fn preview_artifact_index_rolls_back_when_active_location_is_stale() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-index-scan",
        "preview-index-root",
        "C:\\PreviewIndexSource",
        "preview-index-location",
    );
    let mut location = catalog
        .load_active_location("preview-index-location")
        .expect("active location query")
        .expect("active location");
    location.modified_unix_ms += 1;
    location.preview_path = "C:\\AmeCache\\preview.jpg".to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "preview-artifact-key".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: location.width,
        height: location.height,
    };

    let error = catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect_err("stale preview publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
}

#[test]
fn preview_publication_adopts_a_null_revision_once_under_the_exact_lease() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-adoption-scan",
        "preview-adoption-root",
        "C:\\PreviewAdoptionSource",
        "preview-adoption-location",
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_revision_token = NULL
             WHERE location_id = 'preview-adoption-location'",
            [],
        )
        .expect("clear legacy revision");
    let mut location = catalog
        .load_active_location("preview-adoption-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    location.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000002".to_owned(),
    });

    catalog
        .update_active_preview(&location, None, Some(&request))
        .expect("adopt source revision");

    let adopted: Option<String> = catalog
        .connection
        .query_row(
            "SELECT source_revision_token FROM asset_locations
             WHERE location_id = 'preview-adoption-location'",
            [],
            |row| row.get(0),
        )
        .expect("adopted revision");
    assert_eq!(
        adopted.as_deref(),
        Some("windows-file-change-time-100ns-v1:0000000000000002")
    );
}

#[test]
fn hardlink_preview_revision_adoption_is_idempotent_for_matching_observations() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "matching-adoption-scan",
        "matching-adoption-root",
        "C:\\MatchingAdoptionSource",
        "matching-adoption-a",
    );
    insert_active_identity_alias(&mut catalog, "matching-adoption-a", "matching-adoption-b");
    let mut first = catalog
        .load_active_location("matching-adoption-a")
        .expect("first location query")
        .expect("first location");
    let mut second = catalog
        .load_active_location("matching-adoption-b")
        .expect("second location query")
        .expect("second location");
    let first_request = exact_preview_request(&first);
    let second_request = exact_preview_request(&second);
    let revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000011".to_owned(),
    };
    first.source_revision = Some(revision.clone());
    second.source_revision = Some(revision);

    catalog
        .update_active_preview(&first, None, Some(&first_request))
        .expect("first hardlink revision adoption");
    catalog
        .update_active_preview(&second, None, Some(&second_request))
        .expect("matching stale-null adoption is idempotent");

    let state = catalog
        .connection
        .query_row(
            "SELECT COUNT(source_revision_token), COUNT(DISTINCT source_revision_token)
             FROM asset_locations
             WHERE file_identity_scheme = 'windows-file-id-128-v1'
               AND file_identity_value = '0000000000000001:00000000000000000000000000000001'",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .expect("matching hardlink revision state");
    assert_eq!(state, (2, 1));
}

#[test]
fn hardlink_preview_revision_adoption_rejects_a_conflicting_observation() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "conflicting-adoption-scan",
        "conflicting-adoption-root",
        "C:\\ConflictingAdoptionSource",
        "conflicting-adoption-a",
    );
    insert_active_identity_alias(
        &mut catalog,
        "conflicting-adoption-a",
        "conflicting-adoption-b",
    );
    let mut first = catalog
        .load_active_location("conflicting-adoption-a")
        .expect("first location query")
        .expect("first location");
    let mut second = catalog
        .load_active_location("conflicting-adoption-b")
        .expect("second location query")
        .expect("second location");
    let first_request = exact_preview_request(&first);
    let second_request = exact_preview_request(&second);
    first.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000011".to_owned(),
    });
    second.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000022".to_owned(),
    });

    catalog
        .update_active_preview(&first, None, Some(&first_request))
        .expect("first hardlink revision adoption");
    let error = catalog
        .update_active_preview(&second, None, Some(&second_request))
        .expect_err("conflicting hardlink revision must be rejected");

    assert_eq!(error.code, "active_preview_location_stale");
    let revisions = catalog
        .connection
        .prepare(
            "SELECT DISTINCT source_revision_token
             FROM asset_locations
             WHERE file_identity_scheme = 'windows-file-id-128-v1'
               AND file_identity_value = '0000000000000001:00000000000000000000000000000001'",
        )
        .expect("prepare hardlink revisions")
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .expect("query hardlink revisions")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect hardlink revisions");
    assert_eq!(
        revisions,
        vec![Some(
            "windows-file-change-time-100ns-v1:0000000000000011".to_owned()
        )]
    );
}

#[test]
fn preview_publication_rejects_a_stale_source_generation() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-generation-scan",
        "preview-generation-root",
        "C:\\PreviewGenerationSource",
        "preview-generation-location",
    );
    let location = catalog
        .load_active_location("preview-generation-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = source_generation + 1
             WHERE location_id = 'preview-generation-location'",
            [],
        )
        .expect("advance source generation");

    let error = catalog
        .update_active_preview(&location, None, Some(&request))
        .expect_err("stale source generation");

    assert_eq!(error.code, "active_preview_location_stale");
}

#[test]
fn unrelated_catalog_revision_does_not_invalidate_an_exact_preview_lease() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-unrelated-scan",
        "preview-unrelated-root",
        "C:\\PreviewUnrelatedSource",
        "preview-unrelated-location",
    );
    let location = catalog
        .load_active_location("preview-unrelated-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    catalog
        .connection
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .expect("advance unrelated catalog revision");

    catalog
        .update_active_preview(&location, None, Some(&request))
        .expect("exact lease remains valid");
}

#[test]
fn preview_publication_rejects_same_timestamp_file_identity_replacement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-original-scan",
        "preview-identity-root",
        "C:\\PreviewIdentitySource",
        "preview-identity-location",
    );
    let mut original = catalog
        .load_active_location("preview-identity-location")
        .expect("original location query")
        .expect("original location");
    let replacement_scan = fixture_request("preview-replacement-scan", "C:\\PreviewIdentitySource");
    let mut replacement = original.clone();
    replacement.absolute_path = "C:\\PreviewIdentitySource\\replacement.png".to_owned();
    replacement.display_path = replacement.absolute_path.clone();
    replacement.relative_path = "replacement.png".to_owned();
    replacement.file_identity = Some(FileIdentityEvidence {
        scheme: "windows-file-id-v1".to_owned(),
        value: "volume:replacement".to_owned(),
    });
    replacement.preview_path.clear();
    replacement.preview_status = PreviewStatus::Pending;
    catalog
        .begin_scan(
            &replacement_scan,
            "preview-identity-root",
            &replacement_scan.root_path,
        )
        .expect("begin replacement scan");
    catalog
        .stage_location(
            &replacement_scan.scan_id,
            "preview-identity-root",
            &replacement,
        )
        .expect("stage replacement");
    catalog
        .publish_scan(&replacement_scan.scan_id, "preview-identity-root", 1, 0)
        .expect("publish replacement scan");
    original.preview_path = "C:\\AmeCache\\stale-preview.jpg".to_owned();
    original.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "stale-preview-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: original.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: original.width,
        height: original.height,
    };

    let error = catalog
        .update_active_preview(&original, Some(&artifact), None)
        .expect_err("stale identity publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
    let active = catalog
        .load_active_location("preview-identity-location")
        .expect("active replacement query")
        .expect("active replacement");
    assert_eq!(active.absolute_path, replacement.absolute_path);
    assert!(matches!(active.preview_status, PreviewStatus::Pending));
}

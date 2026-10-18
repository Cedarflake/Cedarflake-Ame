use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, load_default_snapshot, load_default_timeline, publish_fixture,
    publish_gallery_fixture, publish_gallery_query_fixture,
};
use std::collections::HashSet;

use tempfile::tempdir;

use crate::domain::{GallerySortDirection, GallerySortKey};

use crate::adapters::sqlite_catalog::*;

#[test]
fn capture_time_evidence_round_trips_with_engine_identity() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-capture", "C:\\Pictures");
    catalog
        .begin_scan(&request, "root-capture", "C:\\Pictures")
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove capture fixture first-import handoff");
    catalog
        .stage_location(
            "scan-capture",
            "root-capture",
            &AssetLocationView {
                asset_id: "asset-capture".to_owned(),
                location_id: "location-capture".to_owned(),
                root_id: "root-capture".to_owned(),
                scan_id: "scan-capture".to_owned(),
                absolute_path: "C:\\Pictures\\capture.jpg".to_owned(),
                display_path: "C:\\Pictures\\capture.jpg".to_owned(),
                relative_path: "capture.jpg".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: Some(FileIdentityEvidence {
                    scheme: "windows-file-id-128-v1".to_owned(),
                    value: "0000000000000001:00000000000000000000000000000002".to_owned(),
                }),
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 1,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "kamadak-exif".to_owned(),
                metadata_engine_version: "0.6.1".to_owned(),
                capture_time: Some(CaptureTimeEvidence {
                    local_time: "2025-07-08T09:10:11.123000000".to_owned(),
                    offset_minutes: Some(480),
                    source: CaptureTimeSource::Original,
                    raw_value: "2025:07:08 09:10:11|123|+08:00".to_owned(),
                }),
            },
        )
        .expect("stage location");
    catalog
        .publish_scan("scan-capture", "root-capture", 1, 0)
        .expect("publish scan");

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot");
    let asset = snapshot.assets.first().expect("stored asset");
    let capture = asset.capture_time.as_ref().expect("capture evidence");

    assert_eq!(asset.metadata_engine_id, "kamadak-exif");
    assert_eq!(asset.metadata_engine_version, "0.6.1");
    assert_eq!(
        asset
            .file_identity
            .as_ref()
            .map(|identity| identity.scheme.as_str()),
        Some("windows-file-id-128-v1")
    );
    assert_eq!(capture.local_time, "2025-07-08T09:10:11.123000000");
    assert_eq!(capture.offset_minutes, Some(480));
    assert!(matches!(capture.source, CaptureTimeSource::Original));
}

#[test]
fn keyset_pages_retain_multiple_roots_and_reject_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");

    publish_fixture(&mut catalog, "scan-1", "root-1", "C:\\One", "location-1");
    publish_fixture(&mut catalog, "scan-2", "root-2", "C:\\Two", "location-2");

    let first_asset_id = catalog
        .load_active_location("location-1")
        .expect("existing location")
        .expect("active location")
        .asset_id;
    let first_page = load_default_snapshot(&mut catalog, 1, None).expect("first page");
    let cursor = first_page.next_cursor.clone().expect("next cursor");
    let second_page = load_default_snapshot(&mut catalog, 1, Some(&cursor)).expect("second page");

    assert_eq!(first_asset_id, "asset-scan-1");
    assert_eq!(first_page.revision, 2);
    assert_eq!(first_page.roots.len(), 2);
    assert_eq!(first_page.assets.len(), 1);
    assert_eq!(second_page.assets.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert_ne!(
        first_page.assets[0].location_id,
        second_page.assets[0].location_id,
    );

    publish_fixture(&mut catalog, "scan-3", "root-3", "C:\\Three", "location-3");
    let error = load_default_snapshot(&mut catalog, 1, Some(&cursor))
        .expect_err("published changes invalidate old cursors");
    assert_eq!(error.code, "catalog_cursor_stale");
}

#[test]
fn keyset_walk_returns_each_location_once_across_many_pages() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-many", "C:\\Many");
    catalog
        .begin_scan(&request, "root-many", "C:\\Many")
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove keyset fixture first-import handoff");
    let transaction = catalog
        .connection
        .transaction()
        .expect("fixture transaction");
    for index in 0..1_025 {
        let asset_id = format!("asset-{index:04}");
        let location_id = format!("location-{index:04}");
        let relative_path = format!("{index:04}.png");
        transaction
            .execute(
                "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
                [&asset_id],
            )
            .expect("fixture asset");
        transaction
            .execute(
                "INSERT INTO asset_locations(
                       scan_id, asset_id, location_id, root_id, absolute_path,
                       relative_path, preview_path, file_size, modified_unix_ms,
                       width, height, file_identity_scheme, file_identity_value,
                       source_generation
                     ) VALUES (
                       'scan-many', ?1, ?2, 'root-many', ?3, ?4, ?5, 20, 30, 40, 50,
                       'fixture-identity-v1', ?6, 1
                     )",
                params![
                    asset_id,
                    location_id,
                    format!("C:\\Many\\{relative_path}"),
                    relative_path,
                    format!("C:\\Cache\\{index:04}.jpg"),
                    format!("identity-{index:04}"),
                ],
            )
            .expect("fixture location");
    }
    transaction
        .execute("UPDATE catalog_state SET next_source_generation = 2", [])
        .expect("advance fixture source generation allocator");
    transaction.commit().expect("fixture commit");
    catalog
        .publish_scan("scan-many", "root-many", 1_025, 0)
        .expect("publish fixture");

    let mut cursor = None;
    let mut location_ids = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 128, cursor.as_ref()).expect("keyset page");
        location_ids.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    let unique_ids = location_ids.iter().collect::<HashSet<_>>();
    assert_eq!(location_ids.len(), 1_025);
    assert_eq!(unique_ids.len(), 1_025);
    assert_eq!(
        location_ids.first().map(String::as_str),
        Some("location-0000")
    );
    assert_eq!(
        location_ids.last().map(String::as_str),
        Some("location-1024")
    );
}

#[test]
fn gallery_keyset_orders_capture_and_fallback_times_across_roots() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-1",
        "gallery-root-1",
        "C:\\One",
        &[
            ("known-old", Some("2024-01-02T03:04:05.000000000"), 100),
            ("known-tie-low", Some("2025-01-02T03:04:05.000000000"), 100),
            ("unknown-old", None, 100),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-2",
        "gallery-root-2",
        "C:\\Two",
        &[
            ("known-new", Some("2026-01-02T03:04:05.000000000"), 50),
            ("known-tie-high", Some("2025-01-02T03:04:05.000000000"), 200),
            ("unknown-new", None, 200),
        ],
    );

    let mut cursor = None;
    let mut locations = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 2, cursor.as_ref()).expect("gallery page");
        locations.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(
        locations,
        [
            "known-new",
            "known-tie-high",
            "known-tie-low",
            "known-old",
            "unknown-new",
            "unknown-old",
        ]
    );
}

#[test]
fn gallery_time_fallback_prefers_capture_then_creation_then_modification() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "fallback-scan",
        "fallback-root",
        "C:\\Pictures",
        &[
            (
                "created-fallback",
                "created.png",
                None,
                Some(1_749_988_800_000),
                1_784_116_800_000,
            ),
            (
                "modified-fallback",
                "modified.png",
                None,
                None,
                1_715_774_400_000,
            ),
            (
                "capture",
                "capture.png",
                Some("2023-04-15T12:00:00.000000000"),
                Some(1_784_116_800_000),
                1_784_116_800_000,
            ),
        ],
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("fallback snapshot");
    assert_eq!(
        snapshot
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["created-fallback", "modified-fallback", "capture"]
    );
    assert!(snapshot.assets[0].capture_time.is_none());
    assert!(snapshot.assets[1].capture_time.is_none());

    let timeline = load_default_timeline(&mut catalog).expect("fallback timeline");
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.month_key.as_deref())
            .collect::<Vec<_>>(),
        [Some("2025-06"), Some("2024-05"), Some("2023-04")]
    );
}

#[test]
fn gallery_query_combines_source_folder_search_and_all_sort_modes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-1",
        "query-root-1",
        "C:\\One",
        &[
            (
                "img10",
                "Album/img10.png",
                Some("2026-01-02T03:04:05.000000000"),
                Some(30),
                40,
            ),
            (
                "img2",
                "Album/Sub/img2.png",
                Some("2025-01-02T03:04:05.000000000"),
                Some(10),
                20,
            ),
            ("cat", "Other/Cat.png", None, None, 60),
        ],
    );
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-2",
        "query-root-2",
        "C:\\Two",
        &[(
            "img1-other",
            "Album/img1.png",
            Some("2024-01-02T03:04:05.000000000"),
            Some(5),
            5,
        )],
    );

    let source_query = GalleryQuery {
        root_id: Some("query-root-1".to_owned()),
        ..GalleryQuery::default()
    };
    let source_page = catalog
        .load_snapshot(10, &source_query, "source-query", None, None, None)
        .expect("source query");
    assert_eq!(
        source_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img10", "img2", "cat"]
    );

    let direct_folder_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        include_descendants: false,
        ..source_query.clone()
    };
    let direct_folder_page = catalog
        .load_snapshot(
            10,
            &direct_folder_query,
            "direct-folder-query",
            None,
            None,
            None,
        )
        .expect("direct folder query");
    assert_eq!(direct_folder_page.assets.len(), 1);
    assert_eq!(direct_folder_page.assets[0].location_id, "img10");

    let descendant_search_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        search_text: "IMG".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let descendant_search_page = catalog
        .load_snapshot(
            10,
            &descendant_search_query,
            "descendant-search-query",
            None,
            None,
            None,
        )
        .expect("descendant search query");
    assert_eq!(
        descendant_search_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10"]
    );

    let all_name_query = GalleryQuery {
        search_text: "img".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let first_name_page = catalog
        .load_snapshot(2, &all_name_query, "name-query", None, None, None)
        .expect("first name page");
    let second_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            first_name_page.next_cursor.as_ref(),
            None,
            None,
        )
        .expect("second name page");
    let name_locations = first_name_page
        .assets
        .iter()
        .chain(&second_name_page.assets)
        .map(|asset| asset.location_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(name_locations, ["img1-other", "img2", "img10"]);
    let previous_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            None,
            second_name_page.previous_cursor.as_ref(),
            None,
        )
        .expect("previous name page");
    assert_eq!(
        previous_name_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img1-other", "img2"]
    );
    assert!(previous_name_page.previous_cursor.is_none());

    let created_query = GalleryQuery {
        sort_key: GallerySortKey::CreatedTime,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let created_page = catalog
        .load_snapshot(10, &created_query, "created-query", None, None, None)
        .expect("created query");
    assert_eq!(
        created_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10", "cat"]
    );

    let modified_query = GalleryQuery {
        sort_key: GallerySortKey::ModifiedTime,
        sort_direction: GallerySortDirection::Descending,
        ..source_query
    };
    let modified_page = catalog
        .load_snapshot(10, &modified_query, "modified-query", None, None, None)
        .expect("modified query");
    assert_eq!(
        modified_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["cat", "img10", "img2"]
    );
}

#[test]
fn folder_pages_are_bounded_scoped_and_revision_safe() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "folder-scan",
        "folder-root",
        "C:\\Pictures",
        &[
            ("album", "Album/one.png", None, Some(1), 1),
            ("nested", "Album/Sub/two.png", None, Some(2), 2),
            ("other", "Other/three.png", None, Some(3), 3),
        ],
    );

    let first = catalog
        .load_folder_page("folder-root", "", 1, None)
        .expect("first root folder page");
    assert_eq!(first.folders.len(), 1);
    assert_eq!(first.folders[0].relative_path, "Album");
    assert_eq!(first.folders[0].direct_asset_count, 1);
    assert_eq!(first.folders[0].descendant_asset_count, 2);
    let cursor = first.next_cursor.expect("folder cursor");

    let second = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect("second root folder page");
    assert_eq!(second.folders.len(), 1);
    assert_eq!(second.folders[0].relative_path, "Other");
    assert!(second.next_cursor.is_none());

    let nested = catalog
        .load_folder_page("folder-root", "Album", 10, None)
        .expect("nested folder page");
    assert_eq!(nested.folders.len(), 1);
    assert_eq!(nested.folders[0].relative_path, "Album/Sub");

    let traversal = catalog
        .load_folder_page("folder-root", "../Outside", 10, None)
        .expect_err("folder traversal must be rejected");
    assert_eq!(traversal.code, "catalog_source_scope_invalid");

    publish_fixture(
        &mut catalog,
        "other-scan",
        "other-root",
        "C:\\Other",
        "other-location",
    );
    let replacement = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect("published changes replace the folder window");
    assert_eq!(replacement.folders[0].relative_path, "Album");
    assert_eq!(
        replacement.disposition,
        crate::domain::LibraryFolderPageDisposition::Replace
    );
    assert!(replacement.revision > cursor.revision);
    let wrong_scope = catalog
        .load_folder_page("folder-root", "Album", 1, Some(&cursor))
        .expect_err("a cursor cannot cross folder scopes");
    assert_eq!(wrong_scope.code, "catalog_folder_cursor_stale");
}

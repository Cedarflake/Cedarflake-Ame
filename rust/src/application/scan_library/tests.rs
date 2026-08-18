use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use exif::experimental::Writer;
use exif::{Field, In, Tag, Value};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rusqlite::Connection;
use tempfile::tempdir;

use crate::domain::{GalleryQuery, ScanIssue};

use super::*;

fn load_test_snapshot(storage: &StoragePaths) -> crate::domain::CatalogSnapshot {
    SqliteCatalog::open(storage.catalog_path.clone())
        .expect("test catalog")
        .load_snapshot(
            100,
            &GalleryQuery::default(),
            "test-query",
            None,
            None,
            None,
        )
        .expect("test snapshot")
}

fn orientation_jpeg_fixture(orientation: u16, width: u32, height: u32) -> Vec<u8> {
    let mut exif_writer = Writer::new();
    let orientation_field = Field {
        tag: Tag::Orientation,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![orientation]),
    };
    exif_writer.push_field(&orientation_field);
    let mut exif = Cursor::new(Vec::new());
    exif_writer.write(&mut exif, false).expect("encode EXIF");

    let pixels = vec![96; usize::try_from(width * height * 3).expect("pixel count")];
    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new(&mut jpeg);
    encoder
        .set_exif_metadata(exif.into_inner())
        .expect("set orientation EXIF");
    encoder
        .encode(&pixels, width, height, ExtendedColorType::Rgb8)
        .expect("encode orientation JPEG");
    jpeg
}

#[test]
fn validates_scan_limits() {
    let request = ScanRequest {
        scan_id: "scan-1".to_owned(),
        root_path: "C:\\pictures".to_owned(),
        max_items: Some(0),
        max_entries: Some(100),
        preview_edge: 320,
    };

    let error = validate_request(&request).expect_err("zero limit must be rejected");
    assert_eq!(error.code, "item_limit_invalid");
}

#[test]
fn stable_ids_change_with_namespace() {
    assert_ne!(
        stable_id("library-root-v1", "C:\\pictures"),
        stable_id("asset-location-v1", "C:\\pictures"),
    );
}

#[test]
fn scan_issue_presentation_path_hides_windows_device_prefixes() {
    let issue = user_visible_issue(ScanIssue {
        path: Some(r"\\?\C:\Pictures\broken.png".to_owned()),
        code: "fixture".to_owned(),
        message: "fixture".to_owned(),
    });

    assert_eq!(issue.path.as_deref(), Some(r"C:\Pictures\broken.png"));
}

#[test]
fn discovery_accepts_png_magic_with_wrong_extension() {
    let directory = tempdir().expect("temporary directory");
    let file_path = directory.path().join("image.data");
    fs::write(&file_path, b"\x89PNG\r\n\x1a\nrest").expect("fixture write");
    let discovery =
        FileDiscovery::new(&directory.path().to_string_lossy()).expect("valid discovery root");

    let accepted = discovery
        .entry_paths_in_directory("")
        .expect("directory entries")
        .map(|relative_path| discovery.visit_relative_path(&relative_path))
        .any(|visit| matches!(visit.outcome, FileVisitOutcome::File(_)));

    assert!(accepted);
}

#[test]
fn completed_scan_publishes_metadata_then_materializes_an_external_preview() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("像素.data");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "end-to-end-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed scan");

    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    let started_root_path = events
        .iter()
        .find_map(|event| match event {
            ScanEvent::Started { root_path, .. } => Some(root_path),
            _ => None,
        })
        .expect("started event");
    assert_eq!(started_root_path, &user_visible_path(started_root_path));
    let pending_asset = events
        .iter()
        .find_map(|event| match event {
            ScanEvent::AssetDiscovered { asset, .. } => Some((**asset).clone()),
            _ => None,
        })
        .expect("asset event");
    assert!(pending_asset.preview_path.is_empty());
    assert!(matches!(
        pending_asset.preview_status,
        PreviewStatus::Pending
    ));
    let finalization_progress = events
        .iter()
        .filter_map(|event| match event {
            ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } => Some((*validated_items, *total_items)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(finalization_progress, [(0, 1), (1, 1)]);
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 1,
            issue_count: 0,
            ..
        })
    ));

    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: pending_asset.location_id,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("materialized preview");
    let preview_path = PathBuf::from(&previewed.preview_path);
    assert!(matches!(previewed.preview_status, PreviewStatus::Ready));
    assert!(preview_path.starts_with(&preview_root));
    assert!(!preview_path.starts_with(source.path()));
    assert!(preview_path.is_file());
    assert_eq!(
        fs::read(&source_path).expect("source after preview"),
        original_bytes,
    );

    fs::write(&preview_path, b"truncated cached preview").expect("corrupt cached preview");
    let automatically_repaired = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: previewed.location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("ordinary ready request repairs corrupt cache");
    assert!(matches!(
        automatically_repaired.preview_status,
        PreviewStatus::Ready
    ));
    image::open(&preview_path).expect("automatically repaired preview decodes");
    assert_eq!(
        fs::read(&source_path).expect("source after automatic repair"),
        original_bytes,
    );

    let repaired = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: previewed.location_id,
            preview_edge: 256,
            retry_failed: true,
            protected_location_ids: Vec::new(),
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("repair ready preview");
    assert!(matches!(repaired.preview_status, PreviewStatus::Ready));
    image::open(&preview_path).expect("repaired preview decodes");
    assert_eq!(
        fs::read(&source_path).expect("source after repair"),
        original_bytes,
    );

    let connection = Connection::open(catalog_path).expect("published catalog");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'end-to-end-scan'",
            [],
            |row| row.get(0),
        )
        .expect("scan status");
    let active_scan: String = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan");
    let artifact: (i64, i64, i64, String, i64, String) = connection
        .query_row(
            "SELECT source_file_size, size_bucket, encoded_width,
                    lifecycle_state, byte_size, artifact_path
             FROM preview_artifacts",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("preview artifact evidence");
    assert_eq!(status, "completed");
    assert_eq!(active_scan, "end-to-end-scan");
    assert_eq!(
        artifact.0,
        i64::try_from(original_bytes.len()).expect("source size")
    );
    assert_eq!(artifact.1, 256);
    assert!(artifact.2 > 0);
    assert_eq!(artifact.3, "ready");
    assert!(artifact.4 > 0);
    assert_eq!(PathBuf::from(artifact.5), preview_path);
}

#[test]
fn final_validation_crosses_windows_without_skipping_or_repeating_assets() {
    const ASSET_COUNT: u64 = 257;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let fixture_path = storage.path().join("fixture.png");
    RgbaImage::from_pixel(1, 1, Rgba([24, 48, 96, 255]))
        .save(&fixture_path)
        .expect("fixture image");
    let fixture = fs::read(&fixture_path).expect("fixture bytes");
    for index in 0..ASSET_COUNT {
        fs::write(
            source.path().join(format!("asset-{index:03}.png")),
            &fixture,
        )
        .expect("copy fixture image");
    }
    let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
    let request = ScanRequest {
        scan_id: "windowed-final-validation".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed windowed scan");

    let finalization_progress = events
        .iter()
        .filter_map(|event| match event {
            ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } => Some((*validated_items, *total_items)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        finalization_progress,
        [(0, 257), (128, 257), (256, 257), (257, 257)]
    );
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: ASSET_COUNT,
            issue_count: 0,
            ..
        })
    ));

    let connection = Connection::open(catalog_path).expect("published catalog");
    let published_locations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = 'windowed-final-validation'",
            [],
            |row| row.get(0),
        )
        .expect("published asset count");
    assert_eq!(published_locations, 257);
}

#[test]
fn failed_preview_requires_an_explicit_retry_before_reading_source_again() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("source.png");
    RgbaImage::from_pixel(16, 12, Rgba([40, 80, 120, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 1,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "failed-preview-retry".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("completed scan");
    let location_id = load_test_snapshot(&storage_paths).assets[0]
        .location_id
        .clone();

    let failed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("failed preview state");
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert_eq!(
        failed.preview_issue_code.as_deref(),
        Some("preview_cache_budget_exceeded")
    );

    fs::remove_file(&source_path).expect("remove source after failure");
    let retained = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("retained failure state");
    assert_eq!(
        retained.preview_issue_code.as_deref(),
        Some("preview_cache_budget_exceeded")
    );

    let retried = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id,
            preview_edge: 256,
            retry_failed: true,
            protected_location_ids: Vec::new(),
        },
        storage_paths,
    )
    .expect("retried failure state");
    assert_eq!(
        retried.preview_issue_code.as_deref(),
        Some("source_revalidation_failed")
    );
}

#[test]
fn budget_exhaustion_reclaims_an_unprotected_preview_and_retries_once() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let first_source = source.path().join("first.png");
    let second_source = source.path().join("second.png");
    let pixels = RgbaImage::from_pixel(64, 48, Rgba([80, 120, 200, 255]));
    pixels
        .save_with_format(&first_source, ImageFormat::Png)
        .expect("first source");
    pixels
        .save_with_format(&second_source, ImageFormat::Png)
        .expect("second source");
    let first_source_bytes = fs::read(&first_source).expect("first source bytes");
    let second_source_bytes = fs::read(&second_source).expect("second source bytes");
    let mut storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "preview-reclamation-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("completed scan");
    let snapshot = load_test_snapshot(&storage_paths);
    let first = snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "first.png")
        .expect("first asset");
    let second = snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "second.png")
        .expect("second asset");

    let first_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: first.location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("first preview");
    let first_preview_path = PathBuf::from(&first_preview.preview_path);
    let first_preview_size = first_preview_path
        .metadata()
        .expect("first preview metadata")
        .len();
    storage_paths.preview_budget_bytes = first_preview_size.saturating_add(1);

    let second_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: second.location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: vec![second.location_id.clone()],
        },
        storage_paths.clone(),
    )
    .expect("second preview after reclamation");

    assert!(matches!(
        second_preview.preview_status,
        PreviewStatus::Ready
    ));
    assert!(!first_preview_path.exists());
    let catalog = SqliteCatalog::open(storage_paths.catalog_path).expect("catalog");
    let reclaimed = catalog
        .load_active_location(&first.location_id)
        .expect("reclaimed location query")
        .expect("reclaimed location");
    assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
    assert_eq!((reclaimed.width, reclaimed.height), (64, 48));
    assert_eq!(
        fs::read(first_source).expect("first source after"),
        first_source_bytes
    );
    assert_eq!(
        fs::read(second_source).expect("second source after"),
        second_source_bytes,
    );
}

#[test]
fn unchanged_file_is_reinspected_when_metadata_engine_identity_changes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("plain.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-engine-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET metadata_engine_id = 'old-engine', metadata_engine_version = '1',
                     capture_local_time = '1999-01-01T00:00:00.000000000',
                     capture_offset_minutes = NULL, capture_time_source = 'exif_datetime',
                     capture_raw_value = '1999:01:01 00:00:00||'
                 WHERE scan_id = 'metadata-engine-first'",
            [],
        )
        .expect("replace active evidence with old-engine fixture");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-engine-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("second scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (scan_id, engine_id, engine_version, capture_local_time): (
        String,
        String,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT locations.scan_id, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("active metadata evidence");

    assert_eq!(scan_id, "metadata-engine-second");
    assert_eq!(engine_id, "kamadak-exif");
    assert_eq!(engine_version, "0.6.1+ame-orientation-1");
    assert!(capture_local_time.is_none());
    assert_eq!(
        fs::read(&source_path).expect("source after reinspection"),
        original_bytes,
    );
}

#[test]
fn unchanged_file_reuses_capture_evidence_from_the_active_metadata_engine() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("plain.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-reuse-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET capture_local_time = '1999-01-01T00:00:00.000000000',
                     capture_offset_minutes = NULL, capture_time_source = 'exif_datetime',
                     capture_raw_value = '1999:01:01 00:00:00||'
                 WHERE scan_id = 'metadata-reuse-first'",
            [],
        )
        .expect("install compatible evidence fixture");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-reuse-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("second scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (engine_id, engine_version, capture_local_time): (String, String, Option<String>) =
        connection
            .query_row(
                "SELECT locations.metadata_engine_id,
                            locations.metadata_engine_version, locations.capture_local_time
                     FROM library_roots AS roots
                     JOIN asset_locations AS locations
                       ON locations.scan_id = roots.active_scan_id
                     LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("active metadata evidence");

    assert_eq!(engine_id, "kamadak-exif");
    assert_eq!(engine_version, "0.6.1+ame-orientation-1");
    assert_eq!(
        capture_local_time.as_deref(),
        Some("1999-01-01T00:00:00.000000000")
    );
}

#[test]
fn rescan_repairs_legacy_orientation_dimensions_and_invalidates_old_preview() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("portrait.jpg");
    let original_bytes = orientation_jpeg_fixture(6, 80, 60);
    fs::write(&source_path, &original_bytes).expect("write orientation source");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "orientation-recovery-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first orientation scan");

    fs::create_dir_all(&storage_paths.preview_root).expect("preview root");
    let legacy_preview_path = storage_paths.preview_root.join("legacy-thumbnail-v1.jpg");
    RgbImage::from_pixel(80, 60, Rgb([96, 96, 96]))
        .save(&legacy_preview_path)
        .expect("legacy preview fixture");
    let legacy_preview_text = legacy_preview_path.to_string_lossy().into_owned();
    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET width = 80, height = 60,
                     metadata_engine_version = '0.6.1',
                     preview_path = ?1, preview_status = 'ready'
                 WHERE scan_id = 'orientation-recovery-first'",
            [&legacy_preview_text],
        )
        .expect("install legacy orientation evidence");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "orientation-recovery-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("orientation recovery rescan");

    let recovered = load_test_snapshot(&storage_paths)
        .assets
        .into_iter()
        .next()
        .expect("recovered asset");
    assert_eq!((recovered.width, recovered.height), (60, 80));
    assert_eq!(recovered.metadata_engine_version, "0.6.1+ame-orientation-1");
    assert!(matches!(recovered.preview_status, PreviewStatus::Pending));
    assert!(recovered.preview_path.is_empty());

    let mut catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
    let manifest = catalog
        .load_gallery_layout_manifest_chunk(
            100,
            &GalleryQuery::default(),
            "orientation-recovery-query",
            None,
        )
        .expect("orientation-corrected manifest");
    assert_eq!(manifest.aspect_ratio_milli, [750]);
    drop(catalog);

    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: recovered.location_id,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("materialize recovered preview");
    assert_eq!((previewed.width, previewed.height), (60, 80));
    assert_ne!(PathBuf::from(&previewed.preview_path), legacy_preview_path);
    assert!(crate::adapters::is_current_preview_artifact(
        &previewed.preview_path
    ));
    assert_eq!(
        image::image_dimensions(&previewed.preview_path).expect("preview dimensions"),
        (192, 256)
    );
    assert_eq!(
        fs::read(&source_path).expect("source after recovery"),
        original_bytes,
    );
}

#[cfg(windows)]
#[test]
fn rescans_reconcile_rename_edit_replacement_and_removal_without_stale_rows() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let original_path = source.path().join("original.png");
    let moved_path = source.path().join("moved.png");
    let retained_path = source.path().join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&original_path, ImageFormat::Png)
        .expect("original image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let scan = |scan_id: &str| {
        run_scan_with_storage(
            ScanRequest {
                scan_id: scan_id.to_owned(),
                root_path: source.path().to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            },
            |_| true,
            storage_paths.clone(),
        )
        .expect("completed reconciliation scan");
    };

    scan("reconcile-first");
    let first = load_test_snapshot(&storage_paths);
    assert_eq!(first.assets.len(), 1);
    let first_asset = first.assets[0].clone();
    assert!(first_asset.file_identity.is_some());
    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: first_asset.location_id.clone(),
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("first preview");
    assert!(matches!(previewed.preview_status, PreviewStatus::Ready));

    fs::rename(&original_path, &moved_path).expect("rename image");
    scan("reconcile-renamed");
    let renamed = load_test_snapshot(&storage_paths);
    assert_eq!(renamed.assets.len(), 1);
    let renamed_asset = &renamed.assets[0];
    assert_eq!(renamed_asset.asset_id, first_asset.asset_id);
    assert_eq!(renamed_asset.relative_path, "moved.png");
    assert_eq!(renamed_asset.preview_path, previewed.preview_path);
    assert!(matches!(renamed_asset.preview_status, PreviewStatus::Ready));

    RgbaImage::from_pixel(17, 9, Rgba([10, 20, 30, 255]))
        .save_with_format(&moved_path, ImageFormat::Png)
        .expect("edit image in place");
    scan("reconcile-edited");
    let edited = load_test_snapshot(&storage_paths);
    assert_eq!(edited.assets.len(), 1);
    let edited_asset = &edited.assets[0];
    assert_eq!(edited_asset.asset_id, first_asset.asset_id);
    assert!(edited_asset.preview_path.is_empty());
    assert!(matches!(
        edited_asset.preview_status,
        PreviewStatus::Pending
    ));
    assert_eq!((edited_asset.width, edited_asset.height), (17, 9));

    fs::rename(&moved_path, &retained_path).expect("retain old file identity");
    RgbaImage::from_pixel(3, 2, Rgba([220, 100, 40, 255]))
        .save_with_format(&moved_path, ImageFormat::Png)
        .expect("replacement image");
    scan("reconcile-replaced");
    let replaced = load_test_snapshot(&storage_paths);
    assert_eq!(replaced.assets.len(), 2);
    let replacement = replaced
        .assets
        .iter()
        .find(|asset| asset.relative_path == "moved.png")
        .expect("replacement location");
    let retained = replaced
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("retained location");
    assert_ne!(replacement.asset_id, first_asset.asset_id);
    assert_eq!(retained.asset_id, first_asset.asset_id);

    let replacement_asset_id = replacement.asset_id.clone();
    fs::remove_file(&retained_path).expect("remove retained image");
    scan("reconcile-removed");
    let removed = load_test_snapshot(&storage_paths);
    assert_eq!(removed.assets.len(), 1);
    let remaining = &removed.assets[0];
    assert_eq!(remaining.asset_id, replacement_asset_id);
    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (asset_count, location_count): (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets),
                        (SELECT COUNT(*) FROM asset_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("bounded derived rows");
    assert_eq!(asset_count, 1);
    assert_eq!(location_count, 1);
}

#[test]
fn cancelled_scan_does_not_publish_a_catalog() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("one.png");
    RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "cancelled-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(cancel_scan("cancelled-scan"));
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("cancelled scan");

    assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. })));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'cancelled-scan'",
            [],
            |row| row.get(0),
        )
        .expect("cancelled status");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    let (asset_count, location_count): (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets),
                        (SELECT COUNT(*) FROM asset_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("terminal staged rows");
    assert_eq!(status, "cancelled");
    assert_eq!(active_scan, None);
    assert_eq!(asset_count, 0);
    assert_eq!(location_count, 0);
}

#[test]
fn paused_scan_waits_for_explicit_resume_and_publishes_without_duplicates() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    for index in 0..3 {
        RgbaImage::from_pixel(4, 4, Rgba([30 + index, 60, 90, 255]))
            .save(source.path().join(format!("{index}.png")))
            .expect("fixture image");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "paused-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut paused_events = Vec::new();

    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(pause_scan("paused-scan"));
            }
            paused_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("paused scan");

    assert!(matches!(
        paused_events.last(),
        Some(ScanEvent::Paused {
            visited_entries: 1,
            accepted_items: 1,
            issue_count: 0,
            ..
        })
    ));
    let paused_catalog = SqliteCatalog::open(catalog_path.clone()).expect("paused catalog");
    assert!(
        paused_catalog
            .load_recoverable_scan()
            .expect("interrupted query")
            .is_none()
    );
    let paused = paused_catalog
        .load_paused_scan()
        .expect("paused query")
        .expect("paused task");
    assert_eq!(paused.accepted_items, 1);
    drop(paused_catalog);

    let mut resumed_events = Vec::new();
    run_scan_with_storage(
        request,
        |event| {
            resumed_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("resumed paused scan");

    assert!(matches!(
        resumed_events.last(),
        Some(ScanEvent::Completed {
            asset_count: 3,
            issue_count: 0,
            ..
        })
    ));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let (status, location_count): (String, i64) = connection
        .query_row(
            "SELECT scans.status, COUNT(locations.location_id)
                 FROM scan_runs AS scans
                 LEFT JOIN asset_locations AS locations ON locations.scan_id = scans.id
                 WHERE scans.id = 'paused-scan'
                 GROUP BY scans.status",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("published paused scan");
    assert_eq!(status, "completed");
    assert_eq!(location_count, 3);
}

#[test]
fn corrupt_image_is_isolated_and_scan_completes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    fs::write(source.path().join("broken.jpg"), b"not an image").expect("corrupt fixture");
    let request = ScanRequest {
        scan_id: "corrupt-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed scan with isolated issue");

    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "image_dimensions_failed"
    )));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 1,
            ..
        })
    ));
}

#[cfg(windows)]
#[test]
fn exclusively_locked_image_is_isolated_and_scan_completes() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("locked.png");
    RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]))
        .save(&source_path)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&source_path)
        .expect("exclusive fixture lock");
    let request = ScanRequest {
        scan_id: "locked-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed scan with locked file isolated");

    drop(lock);
    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "image_open_failed"
    )));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 1,
            ..
        })
    ));
}

#[cfg(windows)]
#[test]
fn long_path_image_is_discovered_without_source_changes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let mut nested = source.path().to_path_buf();
    while nested.to_string_lossy().len() < 280 {
        nested.push("long-path-segment-0123456789abcdef");
    }
    fs::create_dir_all(&nested).expect("long fixture directory");
    let source_path = nested.join("pixel.data");
    RgbaImage::from_pixel(4, 4, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("long path fixture image");
    assert!(source_path.to_string_lossy().len() > 260);
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let request = ScanRequest {
        scan_id: "long-path-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed long path scan");

    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 1,
            issue_count: 0,
            ..
        })
    ));
}

#[test]
fn interrupted_deep_scan_resumes_only_the_current_directory_without_duplicates() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let completed_directory = source.path().join("00-completed");
    let interrupted_directory = source.path().join("01-interrupted");
    fs::create_dir_all(&completed_directory).expect("completed fixture directory");
    fs::create_dir_all(&interrupted_directory).expect("interrupted fixture directory");
    for index in 0..2 {
        RgbaImage::from_pixel(2, 2, Rgba([index, 40, 80, 255]))
            .save(completed_directory.join(format!("{index:03}.png")))
            .expect("fixture image");
    }
    for index in 0..128 {
        RgbaImage::from_pixel(2, 2, Rgba([index as u8, 40, 80, 255]))
            .save(interrupted_directory.join(format!("{index:03}.png")))
            .expect("fixture image");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "recoverable-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let first_attempt = catch_unwind(AssertUnwindSafe(|| {
        let mut discovered = 0_u64;
        let _ = run_scan_with_storage(
            request.clone(),
            |event| {
                if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                    discovered += 1;
                    if discovered == 126 {
                        panic!("simulated process interruption");
                    }
                }
                true
            },
            StoragePaths {
                catalog_path: catalog_path.clone(),
                preview_root: preview_root.clone(),
                preview_budget_bytes: 64 * 1024 * 1024,
                settings_path: storage.path().join("settings.sqlite3"),
            },
        );
    }));
    assert!(first_attempt.is_err());

    let recoverable = SqliteCatalog::open(catalog_path.clone())
        .expect("interrupted catalog")
        .load_recoverable_scan()
        .expect("recoverable query")
        .expect("recoverable scan");
    assert_eq!(recoverable.scan_id, request.scan_id);
    assert_eq!(recoverable.visited_entries, 128);
    assert_eq!(recoverable.accepted_items, 126);
    let current_directory: String = Connection::open(catalog_path.clone())
        .expect("checkpoint catalog")
        .query_row(
            "SELECT current_directory_relative_path
                 FROM scan_runs WHERE id = 'recoverable-scan'",
            [],
            |row| row.get(0),
        )
        .expect("current directory frontier");
    assert_eq!(current_directory, "01-interrupted");

    let mut resumed_events = Vec::new();
    run_scan_with_storage(
        request,
        |event| {
            resumed_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("resumed scan");

    assert!(resumed_events.iter().any(|event| matches!(
        event,
        ScanEvent::Progress {
            visited_entries: 128,
            accepted_items: 126,
            ..
        }
    )));
    assert!(matches!(
        resumed_events.last(),
        Some(ScanEvent::Completed {
            asset_count: 130,
            issue_count: 0,
            ..
        })
    ));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let stored_locations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = 'recoverable-scan'",
            [],
            |row| row.get(0),
        )
        .expect("location count");
    assert_eq!(stored_locations, 130);
    assert_eq!(source.path().read_dir().expect("source entries").count(), 2,);
}

#[test]
fn extremely_wide_directory_is_processed_through_bounded_entry_windows() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    for index in 0..1025 {
        fs::write(
            source.path().join(format!("ignored-{index:04}.txt")),
            b"not an image",
        )
        .expect("wide fixture entry");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "wide-directory-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("wide directory scan");

    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 0,
            ..
        })
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Progress {
            visited_entries: 128,
            accepted_items: 0,
            issue_count: 0,
            ..
        }
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let (visited_entries, pending_entries): (i64, i64) = connection
        .query_row(
            "SELECT scans.visited_entries,
                        (SELECT COUNT(*) FROM scan_directory_entries)
                 FROM scan_runs AS scans WHERE scans.id = 'wide-directory-scan'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("wide scan state");
    assert_eq!(visited_entries, 1025);
    assert_eq!(pending_entries, 0);
    assert_eq!(
        source.path().read_dir().expect("source entries").count(),
        1025
    );
}

#[cfg(windows)]
#[test]
#[ignore = "manual synthetic large-library performance acceptance"]
fn synthetic_ten_thousand_file_scan_records_bounded_acceptance_evidence() {
    use std::time::{Duration, Instant};

    const FILE_COUNT: usize = 10_000;
    const CANCEL_AFTER: u64 = 512;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let template_path = source.path().join("template.png");
    RgbaImage::from_pixel(2, 2, Rgba([40, 80, 120, 255]))
        .save_with_format(&template_path, ImageFormat::Png)
        .expect("template image");
    let template_bytes = fs::read(&template_path).expect("template bytes");
    fs::remove_file(&template_path).expect("remove template");
    let fixture_started = Instant::now();
    for index in 0..FILE_COUNT {
        fs::write(
            source.path().join(format!("image-{index:05}.png")),
            &template_bytes,
        )
        .expect("synthetic image");
    }
    let fixture_elapsed = fixture_started.elapsed();
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let request = |scan_id: &str| ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };

    let cold_started = Instant::now();
    run_scan_with_storage(request("benchmark-cold"), |_| true, storage_paths.clone())
        .expect("cold scan");
    let cold_elapsed = cold_started.elapsed();

    let warm_started = Instant::now();
    run_scan_with_storage(request("benchmark-warm"), |_| true, storage_paths.clone())
        .expect("warm scan");
    let warm_elapsed = warm_started.elapsed();

    let mut pause_accepted = 0_u64;
    let mut pause_requested = None;
    run_scan_with_storage(
        request("benchmark-resumed"),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                pause_accepted += 1;
                if pause_accepted == CANCEL_AFTER {
                    pause_requested = Some(Instant::now());
                    assert!(pause_scan("benchmark-resumed"));
                }
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("paused benchmark scan");
    let pause_elapsed = pause_requested.expect("pause request").elapsed();
    let paused_scan = SqliteCatalog::open(storage_paths.catalog_path.clone())
        .expect("paused benchmark catalog")
        .load_paused_scan()
        .expect("paused benchmark state")
        .expect("paused benchmark scan");
    assert_eq!(paused_scan.scan_id, "benchmark-resumed");
    assert_eq!(paused_scan.accepted_items, CANCEL_AFTER);

    let resume_started = Instant::now();
    let mut did_complete_resume = false;
    run_scan_with_storage(
        request("benchmark-resumed"),
        |event| {
            did_complete_resume |= matches!(event, ScanEvent::Completed { .. });
            true
        },
        storage_paths.clone(),
    )
    .expect("resumed benchmark scan");
    let resume_elapsed = resume_started.elapsed();
    assert!(did_complete_resume);

    let mut accepted = 0_u64;
    let mut cancellation_requested = None;
    run_scan_with_storage(
        request("benchmark-cancelled"),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                accepted += 1;
                if accepted == CANCEL_AFTER {
                    cancellation_requested = Some(Instant::now());
                    assert!(cancel_scan("benchmark-cancelled"));
                }
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("cancelled benchmark scan");
    let cancellation_elapsed = cancellation_requested
        .expect("cancellation request")
        .elapsed();

    let connection = Connection::open(&storage_paths.catalog_path).expect("benchmark catalog");
    let (active_locations, all_locations, assets, cancelled_locations): (i64, i64, i64, i64) =
        connection
            .query_row(
                "SELECT
                       (SELECT COUNT(*) FROM library_roots AS roots
                        JOIN asset_locations AS locations
                          ON locations.scan_id = roots.active_scan_id),
                       (SELECT COUNT(*) FROM asset_locations),
                       (SELECT COUNT(*) FROM assets),
                       (SELECT COUNT(*) FROM asset_locations
                        WHERE scan_id = 'benchmark-cancelled')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("benchmark row counts");
    let catalog_bytes = fs::metadata(&storage_paths.catalog_path)
        .expect("catalog metadata")
        .len();

    println!(
        "AME_SYNTHETIC_BENCHMARK files={FILE_COUNT} fixture_ms={} cold_ms={} warm_ms={} \
             pause_ms={} resume_ms={} cancel_ms={} catalog_bytes={catalog_bytes}",
        fixture_elapsed.as_millis(),
        cold_elapsed.as_millis(),
        warm_elapsed.as_millis(),
        pause_elapsed.as_millis(),
        resume_elapsed.as_millis(),
        cancellation_elapsed.as_millis(),
    );

    assert_eq!(active_locations, FILE_COUNT as i64);
    assert_eq!(all_locations, FILE_COUNT as i64);
    assert_eq!(assets, FILE_COUNT as i64);
    assert_eq!(cancelled_locations, 0);
    assert!(cold_elapsed < Duration::from_secs(60));
    assert!(warm_elapsed < Duration::from_secs(60));
    assert!(pause_elapsed < Duration::from_secs(5));
    assert!(resume_elapsed < Duration::from_secs(60));
    assert!(cancellation_elapsed < Duration::from_secs(5));
    assert!(catalog_bytes < 64 * 1024 * 1024);
    assert_eq!(
        fs::read(source.path().join("image-00000.png")).expect("first source bytes"),
        template_bytes,
    );
    assert_eq!(
        fs::read(source.path().join("image-09999.png")).expect("last source bytes"),
        template_bytes,
    );
    assert_eq!(
        source.path().read_dir().expect("source entries").count(),
        FILE_COUNT
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires current explicit user approval for one named source root"]
fn user_authorized_read_only_library_acceptance() {
    use std::time::Instant;

    const CONSENT: &str = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1";
    const SAMPLE_LIMIT: usize = 64;
    const SAMPLE_FILE_LIMIT: u64 = 64 * 1024 * 1024;

    let consent = std::env::var("CEDARFLAKE_AME_ACCEPTANCE_CONSENT")
        .expect("explicit acceptance consent is required");
    assert_eq!(consent, CONSENT, "acceptance consent does not match");
    let root = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_ROOT")
            .expect("an explicit acceptance root is required"),
    )
    .canonicalize()
    .expect("acceptance root must be available");
    let storage_root = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT")
            .expect("an explicit acceptance storage root is required"),
    )
    .canonicalize()
    .expect("acceptance storage root must be available");
    assert!(root.is_dir(), "acceptance root must be a directory");
    assert!(
        storage_root.is_dir(),
        "acceptance storage must be a directory"
    );
    assert!(
        !acceptance_paths_overlap(&root, &storage_root),
        "acceptance storage must remain outside the source root"
    );
    let report_path = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_REPORT")
            .expect("an explicit acceptance report path is required"),
    );
    assert_eq!(
        report_path
            .parent()
            .expect("acceptance report parent")
            .canonicalize()
            .expect("acceptance report parent must be available"),
        storage_root,
        "acceptance report must remain directly inside acceptance storage"
    );

    let scan_id = std::env::var("CEDARFLAKE_AME_ACCEPTANCE_SCAN_ID")
        .expect("an explicit acceptance scan ID is required");
    assert!(!scan_id.trim().is_empty(), "acceptance scan ID is empty");
    let cancel_after = acceptance_optional_count("CEDARFLAKE_AME_ACCEPTANCE_CANCEL_AFTER");
    let pause_after = acceptance_optional_count("CEDARFLAKE_AME_ACCEPTANCE_PAUSE_AFTER");
    assert!(
        cancel_after.is_none() || pause_after.is_none(),
        "cancel and pause injection cannot be combined"
    );
    let storage = StoragePaths {
        catalog_path: storage_root.join("catalog").join("ame.sqlite3"),
        preview_root: storage_root.join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage_root.join("settings").join("storage.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: scan_id.clone(),
        root_path: root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    acceptance_record(
        &report_path,
        &format!(
            "AME_REAL_LIBRARY_BEGIN scan_id={scan_id:?} root={:?}",
            root.to_string_lossy()
        ),
    );

    let mut accepted_events = 0_u64;
    let mut next_progress_report = 10_000_u64;
    let mut terminal = None;
    let mut pause_requested = None;
    let mut cancel_requested = None;
    let mut samples = Vec::new();
    let mut sample_error = None;
    let scan_started = Instant::now();
    run_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::Progress {
                visited_entries, ..
            } = &event
                && cancel_after.is_some_and(|limit| *visited_entries >= limit)
                && cancel_requested.is_none()
            {
                cancel_requested = Some(Instant::now());
                assert!(
                    cancel_scan(&scan_id),
                    "acceptance cancellation request was rejected"
                );
            }
            match event {
                ScanEvent::Progress {
                    visited_entries,
                    accepted_items,
                    issue_count,
                    ..
                } if visited_entries >= next_progress_report => {
                    let line = format!(
                        "AME_ACCEPTANCE_PROGRESS visited={visited_entries} \
                             accepted={accepted_items} issues={issue_count}"
                    );
                    println!("{line}");
                    acceptance_record(&report_path, &line);
                    while next_progress_report <= visited_entries {
                        next_progress_report = next_progress_report.saturating_add(10_000);
                    }
                }
                ScanEvent::AssetDiscovered { asset, .. } => {
                    accepted_events += 1;
                    if samples.len() < SAMPLE_LIMIT
                        && asset.file_size <= SAMPLE_FILE_LIMIT
                        && acceptance_should_sample(&asset.relative_path, accepted_events)
                    {
                        let path = PathBuf::from(&asset.absolute_path);
                        match acceptance_hash_file(&path) {
                            Ok(hash) => samples.push((path, hash)),
                            Err(error) => {
                                sample_error = Some(format!(
                                    "Could not hash {} before scan completion: {error}",
                                    asset.absolute_path
                                ));
                            }
                        }
                    }
                    if pause_after.is_some_and(|limit| accepted_events == limit) {
                        pause_requested = Some(Instant::now());
                        assert!(
                            pause_scan(&scan_id),
                            "acceptance pause request was rejected"
                        );
                    }
                }
                ScanEvent::Completed { was_limited, .. } => {
                    assert!(
                        !was_limited,
                        "acceptance scan must not publish a limited result"
                    );
                    terminal = Some("completed");
                }
                ScanEvent::Cancelled { .. } => terminal = Some("cancelled"),
                ScanEvent::Paused { .. } => terminal = Some("paused"),
                ScanEvent::Stale { .. } => terminal = Some("stale"),
                _ => {}
            }
            true
        },
        storage.clone(),
    )
    .expect("read-only acceptance scan");
    let first_pass_elapsed = scan_started.elapsed();
    let pause_response_ms = pause_requested.map(|started| started.elapsed().as_millis());
    let cancel_response_ms = cancel_requested.map(|started| started.elapsed().as_millis());

    let mut resume_elapsed_ms = None;
    if pause_after.is_some() {
        assert!(pause_requested.is_some(), "pause threshold was not reached");
        assert_eq!(terminal, Some("paused"));
        let resume_started = Instant::now();
        let mut next_resume_progress_report = 10_000_u64;
        run_scan_with_storage(
            request,
            |event| {
                match event {
                    ScanEvent::Progress {
                        visited_entries,
                        accepted_items,
                        issue_count,
                        ..
                    } if visited_entries >= next_resume_progress_report => {
                        let line = format!(
                            "AME_ACCEPTANCE_RESUME_PROGRESS visited={visited_entries} \
                                 accepted={accepted_items} issues={issue_count}"
                        );
                        println!("{line}");
                        acceptance_record(&report_path, &line);
                        while next_resume_progress_report <= visited_entries {
                            next_resume_progress_report =
                                next_resume_progress_report.saturating_add(10_000);
                        }
                    }
                    ScanEvent::Completed { was_limited, .. } => {
                        assert!(
                            !was_limited,
                            "resumed acceptance scan must not publish a limited result"
                        );
                        terminal = Some("completed");
                    }
                    ScanEvent::Stale { .. } => terminal = Some("stale"),
                    _ => {}
                }
                true
            },
            storage.clone(),
        )
        .expect("resumed read-only acceptance scan");
        resume_elapsed_ms = Some(resume_started.elapsed().as_millis());
    } else if cancel_after.is_some() {
        assert!(
            cancel_requested.is_some(),
            "cancel threshold was not reached"
        );
        assert_eq!(terminal, Some("cancelled"));
    } else {
        assert_eq!(terminal, Some("completed"));
    }

    if let Some(error) = sample_error {
        panic!("{error}");
    }
    if accepted_events > 0 {
        assert!(
            !samples.is_empty(),
            "accepted source files did not produce an integrity sample"
        );
    }
    for (path, expected_hash) in &samples {
        assert_eq!(
            acceptance_hash_file(path).expect("post-scan sample hash"),
            *expected_hash,
            "source bytes changed during the acceptance scan: {}",
            path.display()
        );
    }

    let connection = Connection::open(&storage.catalog_path).expect("acceptance catalog");
    let (status, visited_entries, accepted_items, issue_count, scan_locations, is_active): (
        String,
        i64,
        i64,
        i64,
        i64,
        bool,
    ) = connection
        .query_row(
            "SELECT runs.status, runs.visited_entries, runs.accepted_items,
                        runs.issue_count,
                        (SELECT COUNT(*) FROM asset_locations WHERE scan_id = runs.id),
                        EXISTS(
                          SELECT 1 FROM library_roots WHERE active_scan_id = runs.id
                        )
                 FROM scan_runs AS runs WHERE runs.id = ?1",
            [&scan_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("acceptance scan metrics");
    if status == "completed" {
        assert!(is_active, "completed acceptance scan is not active");
        assert_eq!(scan_locations, accepted_items);
    } else if status == "cancelled" {
        assert!(!is_active, "cancelled acceptance scan was published");
        assert_eq!(scan_locations, 0, "cancelled scan left staged locations");
    }
    let (active_roots, active_locations_total): (i64, i64) = connection
        .query_row(
            "SELECT
                   (SELECT COUNT(*) FROM library_roots WHERE active_scan_id IS NOT NULL),
                   (SELECT COUNT(*) FROM library_roots AS roots
                    JOIN asset_locations AS locations
                      ON locations.scan_id = roots.active_scan_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("active acceptance catalog metrics");
    let issue_codes = {
        let mut statement = connection
            .prepare(
                "SELECT code, COUNT(*) FROM scan_issues
                     WHERE scan_id = ?1 GROUP BY code ORDER BY code",
            )
            .expect("acceptance issue-code statement");
        let rows = statement
            .query_map([&scan_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("acceptance issue-code query");
        let codes = rows
            .map(|row| {
                let (code, count) = row.expect("acceptance issue-code row");
                format!("{code}:{count}")
            })
            .collect::<Vec<_>>();
        if codes.is_empty() {
            "none".to_owned()
        } else {
            codes.join(",")
        }
    };
    let issue_evidence = {
        let mut statement = connection
            .prepare(
                "SELECT code, COUNT(*), MIN(message), MIN(COALESCE(path, ''))
                     FROM scan_issues WHERE scan_id = ?1
                     GROUP BY code ORDER BY code",
            )
            .expect("acceptance issue-evidence statement");
        statement
            .query_map([&scan_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .expect("acceptance issue-evidence query")
            .map(|row| row.expect("acceptance issue-evidence row"))
            .collect::<Vec<_>>()
    };
    let catalog_bytes = acceptance_catalog_bytes(&storage.catalog_path);
    let elapsed_ms = first_pass_elapsed.as_millis() + resume_elapsed_ms.unwrap_or_default();
    let throughput = if elapsed_ms == 0 {
        0.0
    } else {
        accepted_items as f64 * 1_000.0 / elapsed_ms as f64
    };

    let report = format!(
        "AME_REAL_LIBRARY_ACCEPTANCE status={status} scan_id={scan_id:?} root={:?} \
             visited={visited_entries} accepted={accepted_items} issues={issue_count} \
             scan_locations={scan_locations} is_active={is_active} \
             active_roots={active_roots} active_locations_total={active_locations_total} \
             issue_codes={issue_codes} \
             elapsed_ms={elapsed_ms} throughput_items_per_second={throughput:.2} \
             pause_response_ms={pause_response_ms:?} resume_ms={resume_elapsed_ms:?} \
             cancel_response_ms={cancel_response_ms:?} catalog_bytes={catalog_bytes} \
             source_hash_samples={}",
        root.to_string_lossy(),
        samples.len(),
    );
    println!("{report}");
    acceptance_record(&report_path, &report);
    for (code, count, message, sample_path) in issue_evidence {
        let evidence = format!(
            "AME_REAL_LIBRARY_ISSUE code={code:?} count={count} \
                 message={message:?} sample_path={sample_path:?}"
        );
        println!("{evidence}");
        acceptance_record(&report_path, &evidence);
    }
}

#[cfg(windows)]
fn acceptance_optional_count(name: &str) -> Option<u64> {
    std::env::var(name).ok().map(|value| {
        let value = value
            .parse::<u64>()
            .expect("acceptance count must be an integer");
        assert!(value > 0, "acceptance count must be positive");
        value
    })
}

#[cfg(windows)]
fn acceptance_paths_overlap(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    let left = normalize(left);
    let right = normalize(right);
    left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\"))
}

#[cfg(windows)]
fn acceptance_should_sample(relative_path: &str, accepted_items: u64) -> bool {
    if accepted_items == 1 {
        return true;
    }
    let hash = blake3::hash(relative_path.as_bytes());
    u16::from_le_bytes([hash.as_bytes()[0], hash.as_bytes()[1]]).is_multiple_of(1024)
}

#[cfg(windows)]
fn acceptance_hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

#[cfg(windows)]
fn acceptance_catalog_bytes(path: &Path) -> u64 {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return 0;
    };
    let Some(parent) = path.parent() else {
        return 0;
    };
    ["", "-wal", "-shm"]
        .iter()
        .filter_map(|suffix| fs::metadata(parent.join(format!("{file_name}{suffix}"))).ok())
        .map(|metadata| metadata.len())
        .sum()
}

#[cfg(windows)]
fn acceptance_record(path: &Path, line: &str) {
    use std::io::Write;

    let mut report = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("acceptance report");
    writeln!(report, "{line}").expect("acceptance report write");
    report.flush().expect("acceptance report flush");
}

#[test]
fn missing_checkpoint_position_marks_recovery_stale() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(2, 2, Rgba([30, 60, 90, 255]))
        .save(source.path().join("present.png"))
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "missing-checkpoint-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let discovery = FileDiscovery::new(&request.root_path).expect("discovery");
    let canonical_root = discovery.canonical_root().expect("canonical root");
    let root_path = canonical_root.to_string_lossy().into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin scan");
    catalog
        .checkpoint_scan(
            &request.scan_id,
            &crate::domain::ScanCheckpoint {
                last_visited_relative_path: Some("removed.png".to_owned()),
                visited_entries: 2,
                accepted_items: 1,
                issue_count: 0,
                requires_previous_snapshot: false,
            },
        )
        .expect("checkpoint");
    drop(catalog);

    let mut events = Vec::new();
    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("stale recovery");

    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "scan_checkpoint_unavailable"
    )));
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    let status: String = Connection::open(catalog_path)
        .expect("catalog database")
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'missing-checkpoint-scan'",
            [],
            |row| row.get(0),
        )
        .expect("scan status");
    assert_eq!(status, "stale");
}

#[test]
fn source_change_marks_scan_stale_instead_of_publishing() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("changing.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "stale-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                fs::write(&source_path, b"changed by another process")
                    .expect("external source change");
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("stale scan");

    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_changed_during_scan"
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'stale-scan'",
            [],
            |row| row.get(0),
        )
        .expect("stale status");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    assert_eq!(status, "stale");
    assert_eq!(active_scan, None);
}

#[test]
fn missing_source_marks_scan_stale_instead_of_publishing() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("missing.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "missing-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                fs::remove_file(&source_path).expect("external fixture deletion");
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("missing source scan");

    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_revalidation_failed"
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    assert_eq!(active_scan, None);
}

#[test]
fn corrupt_rescan_preserves_the_last_trustworthy_published_location() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "trustworthy-initial-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let before_asset = before.assets.first().expect("published asset").clone();
    let corrupt_bytes = b"not a decodable image";
    fs::write(&source_path, corrupt_bytes).expect("controlled corruption");
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "trustworthy-corrupt-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("corrupt rescan remains recoverable");

    let after = load_test_snapshot(&storage_paths);
    let after_asset = after.assets.first().expect("retained asset");
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
    assert_eq!(after_asset.asset_id, before_asset.asset_id);
    assert_eq!(after_asset.width, before_asset.width);
    assert_eq!(after_asset.height, before_asset.height);
    assert_eq!(
        fs::read(&source_path).expect("corrupt source bytes"),
        corrupt_bytes
    );
}

#[cfg(windows)]
#[test]
fn authoritative_full_scan_with_new_placeholder_remains_stale_without_advancing_audit() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(source.path().join("published.png"))
        .expect("published fixture");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "placeholder-full-scan-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let before_audit: Option<i64> = connection
        .query_row(
            "SELECT last_consistency_audit_unix_ms FROM library_change_root_state LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("initial audit time");
    drop(connection);
    let placeholder_path = source.path().join("new-placeholder.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 100, 120, 255]))
        .save(&placeholder_path)
        .expect("placeholder fixture");
    set_scan_fixture_offline_attribute(&placeholder_path, true);
    let mut events = Vec::new();

    run_scan_with_storage_owned(
        ScanRequest {
            scan_id: "sync-recovery-placeholder-full-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
        true,
    )
    .expect("placeholder full scan remains recoverable");
    set_scan_fixture_offline_attribute(&placeholder_path, false);

    let after = load_test_snapshot(&storage_paths);
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let (status, owner, after_audit): (String, String, Option<i64>) = connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, state.last_consistency_audit_unix_ms
             FROM scan_runs AS scans
             JOIN library_change_root_state AS state ON state.root_id = scans.root_id
             WHERE scans.id = 'sync-recovery-placeholder-full-scan'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("authoritative scan state");
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert_eq!(status, "stale");
    assert_eq!(owner, "authoritative_recovery");
    assert_eq!(after_audit, before_audit);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
}

#[cfg(windows)]
#[test]
fn migrated_v17_placeholder_preserves_the_normalized_legacy_location() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    let source_path = album.join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-normalization-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let before_asset = before.assets.first().expect("published asset");
    let old_location_id = before_asset.location_id.clone();
    let old_asset_id = before_asset.asset_id.clone();
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP INDEX asset_locations_root_relative;
             DROP INDEX library_change_queue_catch_up_peer;
             DROP INDEX scan_runs_one_active_root;
             ALTER TABLE library_change_queue DROP COLUMN authoritative_scan_id;
             ALTER TABLE library_change_root_state DROP COLUMN last_consistency_audit_unix_ms;
             ALTER TABLE scan_runs DROP COLUMN requires_previous_snapshot;
             ALTER TABLE scan_runs DROP COLUMN root_generation_at_start;
             ALTER TABLE scan_runs DROP COLUMN change_queue_high_watermark;
             ALTER TABLE scan_runs DROP COLUMN scan_owner;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_ownership_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN authoritative_recovery_complete;",
        )
        .expect("restore v17 table shape");
    connection
        .execute(
            "UPDATE preview_artifact_locations
             SET location_id = 'legacy-v17-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy preview owner");
    connection
        .execute(
            "UPDATE asset_locations
             SET relative_path = 'album\\retained.png',
                 location_id = 'legacy-v17-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy location identity");
    connection
        .execute_batch(
            "ALTER TABLE library_change_queue_contract
               DROP COLUMN change_catch_up_complete;
             DROP TABLE library_change_catch_up_state;
             UPDATE schema_info SET version = 17;",
        )
        .expect("restore v17 version");
    drop(connection);
    set_scan_fixture_offline_attribute(&source_path, true);
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-normalization-placeholder-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("migrated placeholder rescan remains recoverable");
    set_scan_fixture_offline_attribute(&source_path, false);

    let after = load_test_snapshot(&storage_paths);
    let retained = after.assets.first().expect("retained legacy location");
    let version: i64 = Connection::open(&storage_paths.catalog_path)
        .expect("migrated catalog")
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert_eq!(version, 19);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
    assert_eq!(retained.location_id, "legacy-v17-location");
    assert_eq!(retained.relative_path, "album/retained.png");
    assert_eq!(retained.asset_id, old_asset_id);
}

#[cfg(windows)]
#[test]
fn migrated_v17_healthy_file_preserves_legacy_location_without_identity_evidence() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    let source_path = album.join("healthy.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-healthy-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let before_asset = before.assets.first().expect("published asset");
    let old_location_id = before_asset.location_id.clone();
    let old_asset_id = before_asset.asset_id.clone();
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP INDEX asset_locations_root_relative;
             DROP INDEX library_change_queue_catch_up_peer;
             DROP INDEX scan_runs_one_active_root;
             ALTER TABLE library_change_queue DROP COLUMN authoritative_scan_id;
             ALTER TABLE library_change_root_state DROP COLUMN last_consistency_audit_unix_ms;
             ALTER TABLE scan_runs DROP COLUMN requires_previous_snapshot;
             ALTER TABLE scan_runs DROP COLUMN root_generation_at_start;
             ALTER TABLE scan_runs DROP COLUMN change_queue_high_watermark;
             ALTER TABLE scan_runs DROP COLUMN scan_owner;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_ownership_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN authoritative_recovery_complete;",
        )
        .expect("restore v17 table shape");
    connection
        .execute(
            "UPDATE preview_artifact_locations
             SET location_id = 'legacy-v17-healthy-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy preview owner");
    connection
        .execute(
            "UPDATE asset_locations
             SET relative_path = 'album\\healthy.png',
                 location_id = 'legacy-v17-healthy-location',
                 file_identity_scheme = NULL,
                 file_identity_value = NULL
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy location without identity evidence");
    connection
        .execute_batch(
            "ALTER TABLE library_change_queue_contract
               DROP COLUMN change_catch_up_complete;
             DROP TABLE library_change_catch_up_state;
             UPDATE schema_info SET version = 17;",
        )
        .expect("restore v17 version");
    drop(connection);
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-healthy-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("healthy migrated rescan");

    let after = load_test_snapshot(&storage_paths);
    let retained = after.assets.first().expect("retained legacy location");
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert_eq!(after.assets.len(), 1);
    assert_eq!(retained.location_id, "legacy-v17-healthy-location");
    assert_eq!(retained.relative_path, "album/healthy.png");
    assert_eq!(retained.asset_id, old_asset_id);
    assert!(!fs::read(&source_path).expect("source bytes").is_empty());
}

#[cfg(windows)]
fn set_scan_fixture_offline_attribute(path: &std::path::Path, is_offline: bool) {
    let status = std::process::Command::new("attrib.exe")
        .arg(if is_offline { "+O" } else { "-O" })
        .arg(path)
        .status()
        .expect("attrib executable");
    assert!(status.success());
}

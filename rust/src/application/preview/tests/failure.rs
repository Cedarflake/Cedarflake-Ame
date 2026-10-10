use image::GenericImageView;

use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants, truncated_pixels};

use super::*;

fn legacy_ready_fixture(
    suffix: &str,
) -> (
    PreviewFixture,
    Arc<LocalPreviewStore>,
    AssetLocationView,
    Vec<u8>,
) {
    let valid = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 32, 24).expect("valid JPEG");
    let bytes = truncated_pixels(MediaFixtureFormat::Jpeg, &valid);
    let fixture = preview_fixture_with_bytes(suffix, true, &bytes);
    let store = Arc::new(
        LocalPreviewStore::new(
            fixture.storage.preview_root.clone(),
            fixture.storage.preview_budget_bytes,
        )
        .expect("store"),
    );
    let context =
        load_preview_catalog_context(&fixture.storage, &fixture.request).expect("catalog context");
    let location = context.location;
    let (root, _, identity) = context.stored_source_root.expect("source authority");
    let opened = open_preview_source(
        &ExpectedFileState {
            absolute_path: location.absolute_path.clone(),
            file_size: location.file_size,
            modified_unix_ms: location.modified_unix_ms,
            file_identity: location.file_identity.clone(),
            source_revision: location.source_revision.clone(),
        },
        Path::new(&root),
        identity.as_ref(),
    )
    .expect("guarded source");
    let file = DiscoveredFile {
        source_root_path: root,
        absolute_path: location.absolute_path,
        relative_path: location.relative_path,
        file_size: location.file_size,
        created_unix_ms: location.created_unix_ms,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity,
        source_revision: opened.source_revision.clone(),
        source_generation: location.source_generation,
        issues: Vec::new(),
    };
    let legacy = crate::adapters::seed_legacy_jpeg_preview(&store, &file, &opened.file, 256);
    drop(opened);
    let pixel = image::open(&legacy.path)
        .expect("legacy output decodes")
        .get_pixel(128, 96);
    assert!(
        pixel.0[..3]
            .iter()
            .all(|channel| channel.abs_diff(128) <= 1),
        "the previous decoder fills missing entropy with gray"
    );
    let ready =
        materialize_preview_with_store(fixture.request.clone(), fixture.storage.clone(), &store)
            .expect("ordinary demand reuses legacy cache");
    assert!(matches!(ready.preview_status, PreviewStatus::Ready));
    assert_eq!(ready.preview_path, legacy.path);
    assert_eq!(
        fs::read(&fixture.source_path).expect("unchanged corrupt source"),
        bytes
    );
    (fixture, store, ready, bytes)
}

fn assert_retained_ready(fixture: &PreviewFixture, initial: &AssetLocationView, old_bytes: &[u8]) {
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let retained = catalog
        .load_active_location(&initial.location_id)
        .expect("load location")
        .expect("active location");
    assert!(matches!(retained.preview_status, PreviewStatus::Ready));
    assert_eq!(retained.preview_path, initial.preview_path);
    assert_eq!(retained.source_generation, initial.source_generation);
    assert_eq!(retained.source_revision, initial.source_revision);
    assert_eq!(
        fs::read(&initial.preview_path).expect("retained bytes"),
        old_bytes
    );
    assert_unique_ready_preview_owner(
        &fixture.storage.catalog_path,
        &initial.location_id,
        &initial.preview_path,
    );
}

#[test]
fn explicit_retry_of_same_version_corrupt_source_retires_legacy_ready_ownership() {
    let _lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("lifecycle lock");
    let (fixture, store, initial, source_bytes) = legacy_ready_fixture("retire-corrupt-ready");
    let old_bytes = fs::read(&initial.preview_path).expect("legacy bytes");
    let failed = materialize_preview_with_store(
        preview_request_for(&initial, 256, true),
        fixture.storage.clone(),
        &store,
    )
    .expect("publish confirmed corruption");
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert!(failed.preview_path.is_empty());
    assert_eq!(
        failed.preview_issue_code.as_deref(),
        Some("image_decode_failed")
    );
    assert_eq!(failed.source_generation, initial.source_generation);
    assert_eq!(failed.source_revision, initial.source_revision);
    let retained = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("catalog")
        .load_active_location(&initial.location_id)
        .expect("stored failure")
        .expect("active");
    assert!(matches!(retained.preview_status, PreviewStatus::Failed));
    assert!(retained.preview_path.is_empty());
    assert_eq!(preview_owner_count(&fixture.storage.catalog_path), 0);
    let lifecycle: String = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("connection")
        .query_row(
            "SELECT lifecycle_state FROM preview_artifacts WHERE artifact_path = ?1",
            [&initial.preview_path],
            |row| row.get(0),
        )
        .expect("stale artifact");
    assert_eq!(lifecycle, "stale");
    assert_eq!(
        fs::read(&initial.preview_path).expect("bytes await owned reclamation"),
        old_bytes
    );
    assert_eq!(
        store.used_bytes(),
        u64::try_from(old_bytes.len()).expect("artifact length")
    );
    let repeated = materialize_preview_with_store(
        preview_request_for(&failed, 256, false),
        fixture.storage.clone(),
        &store,
    )
    .expect("ordinary demand keeps failure");
    assert!(matches!(repeated.preview_status, PreviewStatus::Failed));
    assert!(repeated.preview_path.is_empty());
    assert_eq!(
        fs::read(&fixture.source_path).expect("unchanged source"),
        source_bytes
    );
}

#[test]
fn storage_pressure_and_atomic_replacement_failure_keep_healthy_ready() {
    let _lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("lifecycle lock");
    let fixture = preview_fixture("retain-ready-on-transient");
    let store = LocalPreviewStore::new(
        fixture.storage.preview_root.clone(),
        fixture.storage.preview_budget_bytes,
    )
    .expect("store");
    let initial =
        materialize_preview_with_store(fixture.request.clone(), fixture.storage.clone(), &store)
            .expect("ready");
    let source_bytes = fs::read(&fixture.source_path).expect("source");
    let old_bytes = fs::read(&initial.preview_path).expect("ready bytes");
    crate::adapters::fail_next_atomic_replace_for_test(Path::new(&initial.preview_path));
    let error = materialize_preview_with_store(
        preview_request_for(&initial, 256, true),
        fixture.storage.clone(),
        &store,
    )
    .expect_err("controlled installation failure");
    assert_eq!(error.code, "preview_publish_failed");
    assert_retained_ready(&fixture, &initial, &old_bytes);
    let constrained =
        LocalPreviewStore::new(fixture.storage.preview_root.clone(), 1).expect("constrained store");
    let mut storage = fixture.storage.clone();
    storage.preview_budget_bytes = 1;
    let error = materialize_preview_with_store(
        preview_request_for(&initial, 256, true),
        storage,
        &constrained,
    )
    .expect_err("capacity failure");
    assert_eq!(error.code, "preview_cache_budget_exceeded");
    assert_retained_ready(&fixture, &initial, &old_bytes);
    assert_eq!(
        constrained.used_bytes(),
        u64::try_from(old_bytes.len()).expect("artifact length")
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("source after failures"),
        source_bytes
    );
}

fn publish_replaced_source(
    storage: &StoragePaths,
    request: &PreviewRequest,
    source_path: &Path,
) -> AssetLocationView {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 48, 64).expect("replacement image");
    fs::write(source_path, bytes).expect("replace disposable source");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
    let mut replacement = catalog
        .load_active_location(&request.location_id)
        .expect("load old location")
        .expect("location");
    let root = canonical_source_root_path(source_path.parent().expect("source root"))
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let scan = ScanRequest {
        scan_id: format!("{}-new-version", request.expected_scan_id),
        root_path: root.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    begin_preview_fixture_scan(&mut catalog, &scan, &request.expected_root_id, &root, true);
    replacement.scan_id = scan.scan_id.clone();
    replacement.file_size = source_path.metadata().expect("replacement metadata").len();
    replacement.modified_unix_ms = source_path_modified_unix_ms(source_path);
    replacement.source_revision = None;
    replacement.source_generation = 0;
    replacement.width = 48;
    replacement.height = 64;
    replacement.preview_status = PreviewStatus::Pending;
    replacement.preview_path.clear();
    replacement.preview_issue_code = None;
    replacement.preview_issue_message = None;
    catalog
        .stage_location(&scan.scan_id, &request.expected_root_id, &replacement)
        .expect("stage replacement");
    catalog
        .publish_scan(&scan.scan_id, &request.expected_root_id, 1, 0)
        .expect("publish replacement");
    catalog
        .load_active_location(&request.location_id)
        .expect("new active")
        .expect("replacement location")
}

#[test]
fn corruption_result_superseded_by_new_source_cannot_downgrade_new_ready() {
    let _lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("lifecycle lock");
    let (fixture, store, initial, _) = legacy_ready_fixture("supersede-corrupt-ready");
    let new_ready = Arc::new(Mutex::new(None));
    let delivered = Arc::clone(&new_ready);
    let hook_storage = fixture.storage.clone();
    let hook_request = preview_request_for(&initial, 256, true);
    let hook_source = fixture.source_path.clone();
    let hook_store = Arc::clone(&store);
    install_preview_test_hook(
        &AFTER_PREVIEW_REVALIDATION_HOOKS,
        &initial.location_id,
        move || {
            let replacement = publish_replaced_source(&hook_storage, &hook_request, &hook_source);
            let ready = materialize_preview_with_store(
                preview_request_for(&replacement, 256, false),
                hook_storage,
                &hook_store,
            )
            .expect("new source ready");
            assert!(matches!(ready.preview_status, PreviewStatus::Ready));
            let bytes = fs::read(&ready.preview_path).expect("new artifact bytes");
            *delivered.lock().expect("new result") = Some((ready, bytes));
        },
    );
    let error = materialize_preview_with_store(
        preview_request_for(&initial, 256, true),
        fixture.storage.clone(),
        &store,
    )
    .expect_err("old corrupt source superseded");
    assert_eq!(error.code, "preview_request_superseded");
    let (ready, bytes) = new_ready
        .lock()
        .expect("new result")
        .take()
        .expect("replacement executed");
    assert_ne!(ready.scan_id, initial.scan_id);
    assert_ne!(ready.source_revision, initial.source_revision);
    assert_retained_ready(&fixture, &ready, &bytes);
    assert_eq!((ready.width, ready.height), (48, 64));
    let source =
        image::load_from_memory(&fs::read(&fixture.source_path).expect("new source bytes"))
            .expect("new source pixels")
            .to_rgb8();
    let preview = image::open(&ready.preview_path)
        .expect("new preview pixels")
        .to_rgb8();
    let expected = source.get_pixel(source.width() / 4, source.height() / 4);
    let actual = preview.get_pixel(preview.width() / 4, preview.height() / 4);
    assert!(
        expected
            .0
            .iter()
            .zip(actual.0.iter())
            .all(|(expected, actual)| expected.abs_diff(*actual) <= 8)
    );
}

use super::*;
use crate::domain::{AssetLocationView, PreviewArtifact, PreviewStatus, ScanRequest};
use crate::ports::StorageSettingsRepository;
use tempfile::tempdir;

mod activation_tests;

#[test]
fn overlap_detection_is_case_insensitive_and_bidirectional() {
    assert!(paths_overlap(
        Path::new("E:\\ExampleLibrary"),
        Path::new("e:\\examplelibrary\\.ame-cache"),
    ));
    assert!(paths_overlap(
        Path::new("E:\\ExampleLibrary\\nested"),
        Path::new("E:\\ExampleLibrary"),
    ));
    assert!(!paths_overlap(
        Path::new("E:\\ExampleLibrary"),
        Path::new("E:\\AmeCache"),
    ));
}

#[test]
fn resolved_comparison_collapses_existing_path_aliases() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("source");
    let nested = source.join("nested");
    let previews = source.join("previews");
    fs::create_dir_all(&nested).expect("nested source");
    fs::create_dir_all(&previews).expect("previews");
    let aliased_previews = nested.join("..").join("previews");

    assert!(resolved_paths_same(&aliased_previews, &previews).expect("resolved comparison"));
    assert!(resolved_paths_overlap(&source, &aliased_previews).expect("resolved overlap"));
}

#[test]
fn resolved_comparison_normalizes_nonexistent_parent_segments() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("source");
    let previews = source.join("previews");
    fs::create_dir_all(&source).expect("source");
    let disguised_previews = directory
        .path()
        .join("not-created")
        .join("..")
        .join("source")
        .join("previews");

    assert!(
        resolved_paths_same(&disguised_previews, &previews)
            .expect("normalized resolved comparison")
    );
    assert!(
        resolved_paths_overlap(&source, &disguised_previews).expect("normalized resolved overlap")
    );
}

#[test]
fn equivalent_storage_candidate_preserves_the_active_path_identity() {
    let directory = tempdir().expect("temporary directory");
    let active = directory
        .path()
        .join("previews")
        .join(PREVIEW_CACHE_VERSION);
    let nested = directory.path().join("nested");
    fs::create_dir_all(&active).expect("active preview root");
    fs::create_dir_all(&nested).expect("alias parent");
    let candidate = nested
        .join("..")
        .join("previews")
        .join(PREVIEW_CACHE_VERSION);

    assert_eq!(
        preserve_active_path_if_equivalent(candidate, &active).expect("equivalent storage path"),
        active
    );
}

#[test]
fn source_root_cannot_contain_active_storage() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("source");
    let preview_root = source.join("previews");
    fs::create_dir_all(&preview_root).expect("preview root");
    let storage = StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root,
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };

    let error = validate_source_root_storage_paths(&source, &storage)
        .expect_err("overlapping source must be rejected");

    assert_eq!(error.code, "source_root_overlaps_storage");
}

#[test]
fn source_root_cannot_contain_a_configured_preview_target() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("source");
    let active_preview_root = directory.path().join("active-previews");
    let storage = StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: active_preview_root.clone(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let configured = StorageConfiguration {
        catalog_path: storage.catalog_path.to_string_lossy().into_owned(),
        preview_root: source
            .join("pending-previews")
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
    };
    fs::create_dir_all(&source).expect("source root");
    let mut settings =
        SqliteStorageSettings::open(storage.settings_path.clone()).expect("storage settings");
    settings
        .save(&configured, Some(&active_preview_root.to_string_lossy()))
        .expect("save configured preview target");

    let error = validate_source_root_storage_paths(&source, &storage)
        .expect_err("configured preview overlap must be rejected");

    assert_eq!(error.code, "source_root_overlaps_storage");
}

#[test]
fn preview_budget_has_explicit_supported_bounds() {
    assert!(validate_budget(MIN_PREVIEW_BUDGET_BYTES).is_ok());
    assert!(validate_budget(MAX_PREVIEW_BUDGET_BYTES).is_ok());
    assert_eq!(
        validate_budget(MIN_PREVIEW_BUDGET_BYTES - 1)
            .expect_err("small budget")
            .code,
        "preview_budget_invalid"
    );
}

#[test]
fn saved_configuration_is_pending_until_the_process_restarts() {
    let storage = tempdir().expect("storage");
    let active = StoragePaths {
        catalog_path: storage.path().join("active").join("ame.sqlite3"),
        preview_root: storage.path().join("active-previews"),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let initial = load_configured_storage(&active).expect("initial settings");
    assert_eq!(initial.catalog_path, active.catalog_path.to_string_lossy());

    let configured = StorageConfiguration {
        catalog_path: initial.catalog_path,
        preview_root: storage
            .path()
            .join("next-previews")
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes: 8 * 1024 * 1024 * 1024,
    };
    SqliteStorageSettings::open(active.settings_path.clone())
        .expect("settings")
        .save(&configured, Some(&active.preview_root.to_string_lossy()))
        .expect("save configuration");

    let status = storage_status(&active, &configured).expect("status");

    assert!(status.requires_restart);
    assert_eq!(
        status.active_preview_root,
        active.preview_root.to_string_lossy()
    );
    assert_eq!(status.configured_preview_root, configured.preview_root);
}

#[test]
fn successful_preview_root_activation_retires_the_previous_root() {
    let storage = tempdir().expect("storage");
    let settings_path = storage.path().join("settings.sqlite3");
    let old_root = storage.path().join("old-previews");
    let target_root = storage.path().join("target-previews");
    let defaults = StorageConfiguration {
        catalog_path: storage
            .path()
            .join("catalog.sqlite3")
            .to_string_lossy()
            .into_owned(),
        preview_root: old_root.to_string_lossy().into_owned(),
        preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
    };
    let configured = StorageConfiguration {
        preview_root: target_root.to_string_lossy().into_owned(),
        ..defaults.clone()
    };
    let mut settings = SqliteStorageSettings::open(settings_path.clone()).expect("settings");
    settings
        .load_or_initialize(&defaults)
        .expect("initialize settings");
    settings
        .save(&configured, Some(&defaults.preview_root))
        .expect("save pending target");
    drop(settings);

    let active_root = activate_configured_preview_root_with(
        &settings_path,
        Path::new(&configured.catalog_path),
        &configured,
        |_, _| Ok(()),
    )
    .expect("activate target");

    assert_eq!(active_root, target_root);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
    assert!(
        settings
            .load_pending_preview_roots()
            .expect("pending roots")
            .is_empty()
    );
    assert_eq!(
        settings
            .load_retired_preview_roots()
            .expect("retired roots"),
        vec![defaults.preview_root]
    );
}

#[test]
fn failed_preview_root_activation_keeps_the_previous_root_pending() {
    let storage = tempdir().expect("storage");
    let settings_path = storage.path().join("settings.sqlite3");
    let old_root = storage.path().join("old-previews");
    let target_root = storage.path().join("target-previews");
    let defaults = StorageConfiguration {
        catalog_path: storage
            .path()
            .join("catalog.sqlite3")
            .to_string_lossy()
            .into_owned(),
        preview_root: old_root.to_string_lossy().into_owned(),
        preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
    };
    let configured = StorageConfiguration {
        preview_root: target_root.to_string_lossy().into_owned(),
        ..defaults.clone()
    };
    let mut settings = SqliteStorageSettings::open(settings_path.clone()).expect("settings");
    settings
        .load_or_initialize(&defaults)
        .expect("initialize settings");
    settings
        .save(&configured, Some(&defaults.preview_root))
        .expect("save pending target");
    drop(settings);

    let active_root = activate_configured_preview_root_with(
        &settings_path,
        Path::new(&configured.catalog_path),
        &configured,
        |path, _| {
            if path == target_root {
                Err(ScanError::new(
                    "preview_cache_unavailable",
                    "target unavailable",
                ))
            } else {
                Ok(())
            }
        },
    )
    .expect("fall back to previous root");

    assert_eq!(active_root, old_root);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
    assert_eq!(
        settings
            .load_pending_preview_roots()
            .expect("pending roots"),
        vec![defaults.preview_root]
    );
    assert!(
        settings
            .load_retired_preview_roots()
            .expect("retired roots")
            .is_empty()
    );
}

#[test]
fn overlapping_preview_root_activation_keeps_the_previous_root() {
    let storage = tempdir().expect("storage");
    let source_root = storage.path().join("source");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let old_root = storage.path().join("old-previews");
    let target_root = source_root.join("target-previews");
    publish_storage_fixture(&catalog_path, &source_root, &old_root);
    let configured =
        save_pending_preview_target(&settings_path, &catalog_path, &old_root, &target_root);
    let mut initialized = Vec::new();

    let active_root = activate_configured_preview_root_with(
        &settings_path,
        &catalog_path,
        &configured,
        |path, _| {
            initialized.push(path.to_path_buf());
            Ok(())
        },
    )
    .expect("fall back from overlapping target");

    assert_eq!(active_root, old_root);
    assert_eq!(initialized.as_slice(), std::slice::from_ref(&old_root));
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
    assert_eq!(
        settings
            .load_pending_preview_roots()
            .expect("pending roots"),
        vec![old_root.to_string_lossy().into_owned()]
    );
}

#[test]
fn settings_activation_failure_keeps_the_previous_preview_root() {
    let storage = tempdir().expect("storage");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let old_root = storage.path().join("old-previews");
    let target_root = storage.path().join("target-previews");
    let configured =
        save_pending_preview_target(&settings_path, &catalog_path, &old_root, &target_root);
    rusqlite::Connection::open(&settings_path)
        .expect("settings trigger connection")
        .execute_batch(
            "CREATE TRIGGER fail_preview_root_activation
             BEFORE UPDATE OF state ON preview_root_ownership
             BEGIN SELECT RAISE(ABORT, 'activation failure'); END;",
        )
        .expect("activation failure trigger");

    let active_root = activate_configured_preview_root_with(
        &settings_path,
        &catalog_path,
        &configured,
        |_, _| Ok(()),
    )
    .expect("fall back after settings activation failure");

    assert_eq!(active_root, old_root);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
    assert_eq!(
        settings
            .load_pending_preview_roots()
            .expect("pending roots"),
        vec![old_root.to_string_lossy().into_owned()]
    );
}

#[test]
fn catalog_reset_failure_restores_the_previous_preview_root() {
    let storage = tempdir().expect("storage");
    let source_root = storage.path().join("source");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let old_root = storage.path().join("old-previews");
    let target_root = storage.path().join("target-previews");
    publish_storage_fixture(&catalog_path, &source_root, &old_root);
    let configured =
        save_pending_preview_target(&settings_path, &catalog_path, &old_root, &target_root);
    rusqlite::Connection::open(&catalog_path)
        .expect("catalog trigger connection")
        .execute_batch(
            "CREATE TRIGGER fail_preview_reset
             BEFORE UPDATE OF preview_path ON asset_locations
             BEGIN SELECT RAISE(ABORT, 'reset failure'); END;",
        )
        .expect("preview reset failure trigger");

    let active_root = activate_configured_preview_root_with(
        &settings_path,
        &catalog_path,
        &configured,
        |_, _| Ok(()),
    )
    .expect("restore previous root after reset failure");

    assert_eq!(active_root, old_root);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
    assert_eq!(
        settings
            .load_pending_preview_roots()
            .expect("pending roots"),
        vec![old_root.to_string_lossy().into_owned()]
    );
    let catalog = SqliteCatalog::open(catalog_path).expect("catalog");
    let location = catalog
        .load_active_location("storage-location")
        .expect("active location")
        .expect("stored location");
    assert!(matches!(location.preview_status, PreviewStatus::Ready));
    assert!(Path::new(&location.preview_path).starts_with(&old_root));
}

#[test]
fn storage_status_separates_access_and_display_paths() {
    let storage = tempdir().expect("storage");
    let active = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let configured = StorageConfiguration {
        catalog_path: r"\\?\C:\AmeData\ame.sqlite3".to_owned(),
        preview_root: r"\\?\C:\AmeCache\previews".to_owned(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
    };

    let status = storage_status(&active, &configured).expect("status");

    assert_eq!(status.configured_catalog_path, configured.catalog_path);
    assert_eq!(
        status.configured_catalog_display_path,
        r"C:\AmeData\ame.sqlite3",
    );
    assert_eq!(
        status.configured_preview_display_path,
        r"C:\AmeCache\previews",
    );
}

#[test]
fn storage_status_counts_current_and_legacy_preview_bytes() {
    let storage = tempdir().expect("storage");
    let preview_root = storage.path().join("previews");
    fs::create_dir_all(&preview_root).expect("preview root");
    let current = preview_root.join(format!("{PREVIEW_CACHE_VERSION}-{}.jpg", "a".repeat(64)));
    let legacy = preview_root.join(format!("{}.jpg", "b".repeat(64)));
    let foreign = preview_root.join("keep.jpg");
    fs::write(current, vec![1_u8; 7]).expect("current preview");
    fs::write(legacy, vec![2_u8; 13]).expect("legacy preview");
    fs::write(foreign, vec![3_u8; 17]).expect("foreign file");
    let active = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: preview_root.clone(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let configured = StorageConfiguration {
        catalog_path: active.catalog_path.to_string_lossy().into_owned(),
        preview_root: preview_root.to_string_lossy().into_owned(),
        preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
    };

    let status = storage_status(&active, &configured).expect("storage status");

    assert_eq!(status.preview_used_bytes, 20);
}

fn save_pending_preview_target(
    settings_path: &Path,
    catalog_path: &Path,
    old_root: &Path,
    target_root: &Path,
) -> StorageConfiguration {
    let defaults = StorageConfiguration {
        catalog_path: catalog_path.to_string_lossy().into_owned(),
        preview_root: old_root.to_string_lossy().into_owned(),
        preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
    };
    let configured = StorageConfiguration {
        preview_root: target_root.to_string_lossy().into_owned(),
        ..defaults.clone()
    };
    let mut settings =
        SqliteStorageSettings::open(settings_path.to_path_buf()).expect("storage settings");
    settings
        .load_or_initialize(&defaults)
        .expect("initialize storage settings");
    settings
        .save(&configured, Some(&defaults.preview_root))
        .expect("save pending preview target");
    configured
}

fn publish_storage_fixture(catalog_path: &Path, source_root: &Path, preview_root: &Path) {
    fs::create_dir_all(source_root).expect("source root");
    fs::create_dir_all(preview_root).expect("preview root");
    let preview_path = preview_root.join("preview.jpg");
    fs::write(&preview_path, b"preview").expect("preview artifact");
    let mut catalog = SqliteCatalog::open(catalog_path.to_path_buf()).expect("catalog");
    let request = ScanRequest {
        scan_id: "storage-scan".to_owned(),
        root_path: source_root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    catalog
        .begin_scan(&request, "storage-root", &request.root_path)
        .expect("begin storage scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove storage fixture first-import handoff");
    catalog
        .stage_location(
            &request.scan_id,
            "storage-root",
            &AssetLocationView {
                asset_id: "storage-asset".to_owned(),
                location_id: "storage-location".to_owned(),
                root_id: "storage-root".to_owned(),
                scan_id: request.scan_id.clone(),
                absolute_path: source_root.join("one.png").to_string_lossy().into_owned(),
                display_path: "source\\one.png".to_owned(),
                relative_path: "one.png".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                source_revision: Some(crate::domain::SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 0,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("stage storage location");
    catalog
        .publish_scan(&request.scan_id, "storage-root", 1, 0)
        .expect("publish storage scan");
    let mut location = catalog
        .load_active_location("storage-location")
        .expect("active storage location query")
        .expect("active storage location");
    location.preview_path = preview_path.to_string_lossy().into_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "storage-preview-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 512,
        path: preview_path.to_string_lossy().into_owned(),
        byte_size: fs::metadata(&preview_path).expect("preview metadata").len(),
        encoded_width: 40,
        encoded_height: 50,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect("publish storage preview artifact");
}

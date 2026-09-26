use std::path::PathBuf;

use tempfile::{TempDir, tempdir};

use super::*;
use crate::application::scan_library::run_scan_with_storage;
use crate::domain::{GalleryQuery, ScanEvent, ScanRequest, StorageConfiguration};

#[path = "source_admission_tests/namespace_tests.rs"]
mod namespace_tests;

#[test]
fn retired_cleanup_rejects_overlapping_source_registration_until_deletion_finishes() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    let mut rejected = false;
    let mut cleaned = false;
    clear_preview_scope(
        "retired-registration".to_owned(),
        |event| {
            if matches!(event, PreviewCleanupEvent::Started { .. }) {
                let mut scan_events = Vec::new();
                let error = run_scan_with_storage(
                    fixture.request("overlapping", &fixture.retired_root),
                    |event| {
                        scan_events.push(event);
                        true
                    },
                    fixture.paths.clone(),
                )
                .expect_err("cleanup cannot admit its files as a new source");
                assert_eq!(error.code, "source_root_cleanup_active");
                assert!(scan_events.is_empty());
                assert_eq!(fixture.catalog_counts(), (0, 0));
                assert_eq!(
                    fs::read(&fixture.artifact).expect("not yet removed"),
                    fixture.bytes
                );
                rejected = true;
            }
            cleaned |= matches!(
                event,
                PreviewCleanupEvent::Completed {
                    removed_files: 1,
                    issue_count: 0,
                    ..
                }
            );
            true
        },
        fixture.scope(),
    )
    .expect("authorized derived cleanup");
    assert!(rejected && cleaned);
    assert!(!fixture.artifact.exists());
    assert_eq!(fixture.catalog_counts(), (0, 0));
    fixture.import("after-cleanup", &fixture.retired_root, 0);
    assert_eq!(fixture.catalog_counts(), (1, 0));
}

#[test]
fn retired_cleanup_allows_nonoverlapping_source_import_while_reserved() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    let other_root = fixture.directory.path().join("other-source");
    fs::create_dir(&other_root).expect("other source");
    let source = other_root.join("image.jpg");
    fs::write(&source, &fixture.bytes).expect("other source image");
    let mut imported = false;
    clear_preview_scope(
        "unrelated-import".to_owned(),
        |event| {
            if matches!(event, PreviewCleanupEvent::Started { .. }) {
                fixture.import("nonoverlapping", &other_root, 1);
                imported = true;
            }
            true
        },
        fixture.scope(),
    )
    .expect("independent cleanup");
    assert!(imported);
    assert_eq!(fixture.catalog_counts(), (1, 1));
    assert_eq!(
        fs::read(source).expect("unrelated source preserved"),
        fixture.bytes
    );
    assert!(!fixture.artifact.exists());
}

#[test]
fn published_source_prevents_retired_cleanup_without_deleting_media() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    fixture.import("registered-first", &fixture.retired_root, 1);
    let mut events = Vec::new();
    let error = clear_preview_scope(
        "registered-source".to_owned(),
        |event| {
            events.push(event);
            true
        },
        fixture.scope(),
    )
    .expect_err("registered media is not cleanup storage");
    assert_eq!(error.code, "preview_root_overlaps_source");
    assert!(events.is_empty());
    assert_eq!(fixture.catalog_counts(), (1, 1));
    assert_eq!(
        fs::read(&fixture.artifact).expect("source preserved"),
        fixture.bytes
    );
}

#[test]
fn registration_before_commit_prevents_cleanup_without_waiting_for_the_commit() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    super::super::storage::catalog_admission::with_scan_start(
        &fixture.paths,
        &fixture.retired_root,
        || {
            assert_eq!(fixture.catalog_counts(), (0, 0));
            let mut events = Vec::new();
            let error = clear_preview_scope(
                "before-source-commit".to_owned(),
                |event| {
                    events.push(event);
                    true
                },
                fixture.scope(),
            )
            .expect_err("registration retains the future source before its first commit");
            assert_eq!(error.code, "preview_cleanup_source_registration_active");
            assert!(events.is_empty());
            assert_eq!(
                fs::read(&fixture.artifact).expect("no deletion"),
                fixture.bytes
            );
            Ok(())
        },
    )
    .expect("admitted source boundary");
    fixture.import("after-registration", &fixture.retired_root, 1);
    assert_eq!(fixture.catalog_counts(), (1, 1));
}

#[test]
fn cancelled_or_detached_cleanup_releases_source_reservation_without_deletion() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    for cancel in [false, true] {
        let fixture = Fixture::new();
        let mut started = false;
        let mut cancelled = false;
        clear_preview_scope(
            "released-cleanup".to_owned(),
            |event| {
                if matches!(event, PreviewCleanupEvent::Started { .. }) {
                    started = true;
                    if cancel {
                        assert!(cancel_preview_cleanup("released-cleanup"));
                    } else {
                        return false;
                    }
                }
                cancelled |= matches!(
                    event,
                    PreviewCleanupEvent::Cancelled {
                        removed_files: 0,
                        ..
                    }
                );
                true
            },
            fixture.scope(),
        )
        .expect("cleanup cancellation");
        assert!(started);
        assert_eq!(cancelled, cancel);
        fixture.import("after-release", &fixture.retired_root, 1);
        assert_eq!(fixture.catalog_counts(), (1, 1));
        assert_eq!(
            fs::read(&fixture.artifact).expect("source preserved"),
            fixture.bytes
        );
    }
}

struct Fixture {
    directory: TempDir,
    retired_root: PathBuf,
    artifact: PathBuf,
    bytes: Vec<u8>,
    paths: StoragePaths,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("isolated storage");
        let retired_root = directory.path().join("retired");
        let active_root = directory.path().join("active");
        fs::create_dir(&retired_root).expect("retired root");
        fs::create_dir(&active_root).expect("active root");
        let artifact = retired_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.jpg",
            "d".repeat(64)
        ));
        image::RgbImage::from_pixel(4, 4, image::Rgb([20, 40, 60]))
            .save(&artifact)
            .expect("real JPEG");
        let bytes = fs::read(&artifact).expect("source bytes");
        let paths = StoragePaths {
            catalog_path: directory.path().join("catalog.sqlite3"),
            preview_root: active_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let initial = StorageConfiguration {
            catalog_path: paths.catalog_path.to_string_lossy().into_owned(),
            preview_root: retired_root.to_string_lossy().into_owned(),
            preview_budget_bytes: paths.preview_budget_bytes,
        };
        let active = StorageConfiguration {
            preview_root: paths.preview_root.to_string_lossy().into_owned(),
            ..initial.clone()
        };
        let mut settings =
            SqliteStorageSettings::open(paths.settings_path.clone()).expect("settings");
        settings
            .load_or_initialize(&initial)
            .expect("initial configuration");
        settings
            .save(&active, Some(&initial.preview_root))
            .expect("pending switch");
        settings
            .activate_preview_root(&active.preview_root)
            .expect("retired ownership");
        drop(settings);
        drop(SqliteCatalog::open(paths.catalog_path.clone()).expect("empty catalog"));
        Self {
            directory,
            retired_root,
            artifact,
            bytes,
            paths,
        }
    }

    fn scope(&self) -> CleanupScope {
        CleanupScope::Retired {
            preview_root: self.retired_root.clone(),
            catalog_path: self.paths.catalog_path.clone(),
            settings_path: self.paths.settings_path.clone(),
            stored_preview_root: self.retired_root.to_string_lossy().into_owned(),
        }
    }

    fn request(&self, id: &str, root: &Path) -> ScanRequest {
        ScanRequest {
            scan_id: id.to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        }
    }

    fn import(&self, id: &str, root: &Path, expected_assets: u64) {
        let mut completed = false;
        run_scan_with_storage(
            self.request(id, root),
            |event| {
                if let ScanEvent::Completed { asset_count, .. } = event {
                    assert_eq!(asset_count, expected_assets);
                    completed = true;
                }
                true
            },
            self.paths.clone(),
        )
        .expect("production import");
        assert!(completed);
    }

    fn catalog_counts(&self) -> (usize, usize) {
        let mut catalog = SqliteCatalog::open(self.paths.catalog_path.clone()).expect("catalog");
        let snapshot = catalog
            .load_snapshot(
                10,
                &GalleryQuery::default(),
                "source-cleanup-admission",
                None,
                None,
                None,
            )
            .expect("source membership");
        (snapshot.roots.len(), snapshot.assets.len())
    }
}

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{ScanEvent, ScanRequest};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};

use super::*;

type BeforeRemove = Option<(PathBuf, Box<dyn FnOnce()>)>;

thread_local! {
    static BEFORE_REMOVE: RefCell<BeforeRemove> = RefCell::new(None);
    static AFTER_OBSERVATION: RefCell<BeforeRemove> = RefCell::new(None);
}

pub(super) fn after_namespace_observation(path: &Path) {
    let hook = AFTER_OBSERVATION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|(expected, _)| expected == path) {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}

#[test]
fn missing_cache_root_recovery_repairs_the_index_without_admitting_a_later_directory() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    for appears_later in [false, true] {
        let directory = tempfile::tempdir().expect("isolated missing-root fixture");
        let root = directory.path().join("absent-cache");
        let artifact = root.join(format!(
            "{}-{}.jpg",
            crate::adapters::PREVIEW_CACHE_VERSION,
            "d".repeat(64)
        ));
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog.sqlite3"),
            preview_root: root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        super::tests::publish_artifact(&mut catalog, "missing-root", &artifact, 7);
        drop(catalog);
        let bytes =
            encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 16, 16).expect("generated later file");
        if appears_later {
            let hook_root = root.clone();
            let hook_artifact = artifact.clone();
            let hook_bytes = bytes.clone();
            AFTER_OBSERVATION.with(|slot| {
                *slot.borrow_mut() = Some((
                    root.clone(),
                    Box::new(move || {
                        fs::create_dir(&hook_root).expect("later directory");
                        fs::write(hook_artifact, hook_bytes).expect("later source-shaped file");
                    }),
                ))
            });
        }
        let result = run_preview_recovery_for_test(&storage);
        AFTER_OBSERVATION.with(|slot| slot.borrow_mut().take());
        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("reopened catalog");
        let location = catalog
            .load_active_location("recovery-missing-root-location")
            .expect("location query")
            .expect("location");
        assert_eq!((location.width, location.height), (4032, 3024));
        let snapshot = preview_recovery_snapshot();
        assert_eq!(snapshot.inspected_files, 0);
        assert_eq!(snapshot.removed_files, 0);
        assert_eq!(snapshot.corrected_sizes, 0);
        if appears_later {
            assert_eq!(
                result.expect_err("later root is not admitted").code,
                "preview_recovery_namespace_changed"
            );
            assert!(matches!(
                location.preview_status,
                crate::domain::PreviewStatus::Ready
            ));
            assert_eq!(fs::read(&artifact).expect("later file preserved"), bytes);
            assert_eq!(fs::read_dir(&root).expect("later root entries").count(), 1);
            assert_eq!(snapshot.missing_artifacts, 0);
        } else {
            result.expect("missing-root background index repair");
            assert!(matches!(
                location.preview_status,
                crate::domain::PreviewStatus::Pending
            ));
            assert!(location.preview_path.is_empty());
            assert_eq!(snapshot.missing_artifacts, 1);
            assert_eq!(snapshot.phase, PreviewRecoveryPhase::Completed);
            assert!(!root.exists(), "recovery does not create a cache root");
        }
    }
}

pub(super) fn before_remove(path: &Path) {
    let hook = BEFORE_REMOVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|(expected, _)| expected == path) {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}

#[test]
fn startup_recovery_never_deletes_a_source_replacing_its_cache_namespace() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let directory = tempfile::tempdir().expect("isolated fixture");
    let cache = directory.path().join("cache");
    let displaced = directory.path().join("displaced-cache");
    let source_root = directory.path().join("source");
    fs::create_dir(&cache).expect("derived cache directory");
    fs::create_dir(&source_root).expect("generated source directory");
    let leaf = format!(
        "{}-{}.jpg",
        crate::adapters::PREVIEW_CACHE_VERSION,
        "a".repeat(64)
    );
    let orphan = cache.join(&leaf);
    let source = source_root.join(&leaf);
    let source_bytes =
        encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 32, 16).expect("generated source JPEG");
    fs::write(&orphan, [0_u8; 64]).expect("unreferenced derived fixture");
    fs::write(&source, &source_bytes).expect("source fixture");
    let storage = StoragePaths {
        catalog_path: directory.path().join("catalog.sqlite3"),
        preview_root: cache.clone(),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings.sqlite3"),
    };
    let mut imported = false;
    crate::application::scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "recovery-namespace-source".to_owned(),
            root_path: source_root.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if let ScanEvent::Completed { asset_count, .. } = event {
                assert_eq!(asset_count, 1);
                imported = true;
            }
            true
        },
        storage.clone(),
    )
    .expect("production source import");
    assert!(imported);
    assert_eq!(
        fs::read(&source).expect("import preserved source"),
        source_bytes
    );

    let attempted = Arc::new(AtomicBool::new(false));
    let replaced = Arc::new(AtomicBool::new(false));
    let hook_attempted = Arc::clone(&attempted);
    let hook_replaced = Arc::clone(&replaced);
    let hook_cache = cache.clone();
    let hook_source = source_root.clone();
    let hook_displaced = displaced.clone();
    BEFORE_REMOVE.with(|slot| {
        *slot.borrow_mut() = Some((
            orphan.clone(),
            Box::new(move || {
                hook_attempted.store(true, Ordering::Release);
                match fs::rename(&hook_cache, &hook_displaced) {
                    Ok(()) => {
                        fs::rename(&hook_source, &hook_cache)
                            .expect("replace cache using only disposable source fixtures");
                        hook_replaced.store(true, Ordering::Release);
                    }
                    Err(error) => assert_eq!(
                        error.raw_os_error(),
                        Some(32),
                        "only a held Windows sharing guard may exclude the interleaving",
                    ),
                }
            }),
        ));
    });
    let result = run_preview_recovery_for_test(&storage);
    BEFORE_REMOVE.with(|slot| slot.borrow_mut().take());
    assert!(
        attempted.load(Ordering::Acquire),
        "actual deletion boundary: {result:?}"
    );
    let replaced = replaced.load(Ordering::Acquire);
    let remaining_source = if replaced { cache.join(&leaf) } else { source };
    eprintln!(
        "recovery namespace: replacement_admitted={replaced}, source_survives={}",
        remaining_source.exists()
    );
    assert_eq!(
        fs::read(&remaining_source).expect("recovery cannot delete an externally moved source"),
        source_bytes,
    );
    assert_eq!(
        fs::read_dir(remaining_source.parent().expect("source directory"))
            .expect("source entries")
            .count(),
        1,
    );
    if !replaced {
        result.expect("original derived cache recovery succeeds");
        assert!(!orphan.exists());
        fs::rename(&cache, &displaced).expect("recovery releases its namespace lifetime");
    }
}

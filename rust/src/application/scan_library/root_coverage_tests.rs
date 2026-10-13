use std::fs;
use std::path::Path;

use image::{Rgb, RgbImage};
use rusqlite::{Connection, OpenFlags};
use tempfile::tempdir;

use crate::adapters::{
    reset_source_content_open_instrumentation, reset_source_enumeration_instrumentation,
    source_content_open_count, source_entry_read_count,
};
use crate::domain::{CatalogSnapshot, GalleryQuery};

use super::*;

#[test]
fn already_covered_child_does_not_create_another_root_or_scan() {
    let source = tempdir().expect("generated source");
    let derived = tempdir().expect("isolated catalog and previews");
    let child = source.path().join("旅行");
    fs::create_dir(&child).expect("child directory");
    let mut original_bytes = Vec::new();
    for (path, color) in [
        (source.path().join("root.png"), Rgb([10, 90, 170])),
        (child.join("child.png"), Rgb([170, 90, 10])),
    ] {
        RgbImage::from_pixel(8, 6, color)
            .save(&path)
            .expect("generated image");
        original_bytes.push((path.clone(), fs::read(path).expect("original bytes")));
    }
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(request("parent", source.path()), |_| true, storage.clone())
        .expect("import parent");
    let before = snapshot(&storage);
    assert_eq!(before.roots.len(), 1);
    assert_eq!(before.assets.len(), 2);

    let mut events = Vec::new();
    let parent_path = before.roots[0].path.clone();
    let child_path = FileDiscovery::new(&child.to_string_lossy())
        .expect("child identity")
        .canonical_root()
        .expect("canonical child")
        .to_string_lossy()
        .into_owned();
    reset_source_enumeration_instrumentation(&parent_path);
    reset_source_enumeration_instrumentation(&child_path);
    reset_source_content_open_instrumentation(&parent_path);
    reset_source_content_open_instrumentation(&child_path);
    run_scan_with_storage(
        request("covered-child", &child),
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("select an already covered child");
    let after = snapshot(&storage);
    assert_eq!(source_entry_read_count(&parent_path), 0);
    assert_eq!(source_entry_read_count(&child_path), 0);
    assert_eq!(source_content_open_count(&parent_path), 0);
    assert_eq!(source_content_open_count(&child_path), 0);
    for (path, bytes) in original_bytes {
        assert_eq!(fs::read(path).expect("source remains readable"), bytes);
    }
    eprintln!(
        "Parent then child: roots={}, gallery_items={}",
        after.roots.len(),
        after.assets.len()
    );
    assert_eq!(after.roots.len(), 1, "covered child must reuse its parent");
    assert_eq!(after.assets.len(), 2, "a physical image must appear once");
    assert_eq!(after.roots[0].root_id, before.roots[0].root_id);
    assert_eq!(
        after.roots[0].active_scan_id,
        before.roots[0].active_scan_id
    );
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, ScanEvent::Started { .. }))
    );
    assert!(matches!(events.as_slice(), [ScanEvent::AlreadyCovered {
        scan_id, root_id, relative_path,
    }] if scan_id == "covered-child" && root_id == &before.roots[0].root_id && relative_path == "旅行"));
    SqliteCatalog::open(storage.catalog_path.clone()).expect("full reopen");
    let scan_count: i64 =
        Connection::open_with_flags(&storage.catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read scan history")
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
            .expect("scan count");
    assert_eq!(scan_count, 1, "covered selection must not begin a scan");
}

#[test]
fn parent_import_absorbs_registered_child_after_complete_publication() {
    let source = tempdir().expect("generated source");
    let derived = tempdir().expect("isolated catalog and previews");
    let child = source.path().join("旅行");
    fs::create_dir(&child).expect("child directory");
    let mut original_bytes = Vec::new();
    for (path, color) in [
        (source.path().join("root.png"), Rgb([10, 90, 170])),
        (child.join("child.png"), Rgb([170, 90, 10])),
    ] {
        RgbImage::from_pixel(8, 6, color)
            .save(&path)
            .expect("generated image");
        original_bytes.push((path.clone(), fs::read(path).expect("original bytes")));
    }
    let storage = storage(derived.path());
    run_scan_with_storage(request("child", &child), |_| true, storage.clone())
        .expect("import child");
    let before = snapshot(&storage);
    assert_eq!(before.roots.len(), 1);
    assert_eq!(before.assets.len(), 1);
    let retained_asset_id = before.assets[0].asset_id.clone();

    run_scan_with_storage(request("parent", source.path()), |_| true, storage.clone())
        .expect("import parent");
    let after = snapshot(&storage);
    for (path, bytes) in original_bytes {
        assert_eq!(fs::read(path).expect("source remains readable"), bytes);
    }
    eprintln!(
        "Child then parent: roots={}, gallery_items={}",
        after.roots.len(),
        after.assets.len()
    );
    assert_eq!(
        after.roots.len(),
        1,
        "the complete parent subsumes its child"
    );
    assert_eq!(after.assets.len(), 2, "one physical path must appear once");
    assert_eq!(after.roots[0].active_scan_id.as_deref(), Some("parent"));
    let retained = after
        .assets
        .iter()
        .find(|asset| asset.relative_path == "旅行/child.png")
        .expect("child image remains inside the parent");
    assert_eq!(retained.asset_id, retained_asset_id);
    SqliteCatalog::open(storage.catalog_path).expect("full reopen after consolidation");
}

#[test]
fn exact_parent_selection_still_updates_the_registered_root() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let storage = storage(derived.path());
    run_scan_with_storage(request("initial", source.path()), |_| true, storage.clone())
        .expect("initial import");
    let before = snapshot(&storage);
    let mut events = Vec::new();
    run_scan_with_storage(
        request("update", source.path()),
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("normal update");
    let after = snapshot(&storage);
    assert_eq!(after.roots.len(), 1);
    assert_eq!(after.roots[0].root_id, before.roots[0].root_id);
    assert_eq!(after.roots[0].active_scan_id.as_deref(), Some("update"));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ScanEvent::Started { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ScanEvent::AlreadyCovered { .. }))
    );
}

#[test]
fn sibling_with_shared_text_prefix_is_a_separate_source() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let parent = source.path().join("photos");
    let sibling = source.path().join("photos-more");
    fs::create_dir(&parent).expect("parent");
    fs::create_dir(&sibling).expect("sibling");
    let storage = storage(derived.path());
    run_scan_with_storage(request("initial", &parent), |_| true, storage.clone())
        .expect("initial import");
    run_scan_with_storage(request("sibling", &sibling), |_| true, storage.clone())
        .expect("independent import");
    assert_eq!(snapshot(&storage).roots.len(), 2);
}

#[test]
fn canonical_child_selection_preserves_nested_folder_scope() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let child = source.path().join("Photos").join("旅行");
    fs::create_dir_all(&child).expect("nested child");
    let storage = storage(derived.path());
    run_scan_with_storage(request("initial", source.path()), |_| true, storage.clone())
        .expect("initial import");
    let alternate = child.to_string_lossy().to_uppercase();
    let mut events = Vec::new();
    run_scan_with_storage(
        request("child", Path::new(&alternate)),
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("canonical child selection");
    assert!(matches!(events.as_slice(), [ScanEvent::AlreadyCovered {
        relative_path, ..
    }] if relative_path == "Photos/旅行"));
    assert_eq!(snapshot(&storage).roots.len(), 1);
}

#[test]
fn a_replaced_parent_cannot_claim_coverage_from_its_old_publication() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let parent = source.path().join("photos");
    let child = parent.join("child");
    fs::create_dir_all(&child).expect("source");
    let storage = storage(derived.path());
    run_scan_with_storage(request("initial", &parent), |_| true, storage.clone())
        .expect("initial import");
    let before = snapshot(&storage);
    fs::rename(&parent, source.path().join("previous")).expect("move generated parent");
    fs::create_dir_all(&child).expect("replacement with different identity");
    let result = run_scan_with_storage(request("child", &child), |_| true, storage.clone());
    assert!(
        result.is_err(),
        "a stale parent identity must reject coverage"
    );
    let after = snapshot(&storage);
    assert_eq!(after.roots.len(), 1);
    assert_eq!(
        after.roots[0].active_scan_id,
        before.roots[0].active_scan_id
    );
}

#[test]
fn coverage_of_a_paused_parent_does_not_publish_or_replace_its_checkpoint() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let child = source.path().join("child");
    fs::create_dir(&child).expect("child");
    let storage = storage(derived.path());
    run_scan_with_storage(
        request("paused-parent", source.path()),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                assert!(pause_scan("paused-parent"));
            }
            true
        },
        storage.clone(),
    )
    .expect("pause first import");
    let before = snapshot(&storage);
    assert_eq!(before.roots.len(), 1);
    assert!(before.roots[0].active_scan_id.is_none());
    let mut events = Vec::new();
    run_scan_with_storage(
        request("covered-paused-child", &child),
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("select configured folder");
    let after = snapshot(&storage);
    assert_eq!(after.roots.len(), 1);
    assert!(after.roots[0].active_scan_id.is_none());
    assert!(
        events
            .iter()
            .all(|event| matches!(event, ScanEvent::AlreadyCovered { .. }))
    );
    let scan: (String, String) =
        Connection::open_with_flags(&storage.catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("catalog")
            .query_row("SELECT id, status FROM scan_runs", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("retained checkpoint");
    assert_eq!(scan, ("paused-parent".to_owned(), "paused".to_owned()));
}

#[test]
fn controls_before_coverage_never_admit_a_scan_or_a_pause_checkpoint() {
    let source = tempdir().expect("source");
    let derived = tempdir().expect("derived");
    let child = source.path().join("child");
    fs::create_dir(&child).expect("child");
    let storage = storage(derived.path());
    run_scan_with_storage(request("initial", source.path()), |_| true, storage.clone())
        .expect("import parent");
    for command in [CONTROL_CANCEL, CONTROL_SUSPEND, CONTROL_PAUSE] {
        let scan_id = format!("covered-control-{command}");
        let mut events = Vec::new();
        run_scan_with_storage_reason(
            request(&scan_id, &child),
            |event| {
                events.push(event);
                true
            },
            || {
                let accepted = match command {
                    CONTROL_CANCEL => cancel_scan(&scan_id),
                    CONTROL_SUSPEND => suspend_scan(&scan_id),
                    CONTROL_PAUSE => pause_scan(&scan_id),
                    _ => unreachable!("fixed control roster"),
                };
                assert!(accepted);
                Ok(storage.clone())
            },
            FullScanReason::ExplicitUserRequest,
        )
        .expect("controlled selection");
        assert_eq!(events.len(), 1);
        if command == CONTROL_PAUSE {
            assert!(matches!(events[0], ScanEvent::AlreadyCovered { .. }));
        } else {
            assert!(matches!(events[0], ScanEvent::Cancelled { .. }));
        }
        assert_eq!(
            snapshot(&storage).roots[0].active_scan_id.as_deref(),
            Some("initial")
        );
    }
}

fn storage(derived: &Path) -> StoragePaths {
    StoragePaths {
        catalog_path: derived.join("catalog.sqlite3"),
        preview_root: derived.join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: derived.join("settings.sqlite3"),
    }
}

fn request(scan_id: &str, root: &Path) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    }
}

fn snapshot(storage: &StoragePaths) -> CatalogSnapshot {
    SqliteCatalog::open(storage.catalog_path.clone())
        .expect("catalog")
        .load_snapshot(100, &GalleryQuery::default(), "coverage", None, None, None)
        .expect("gallery")
}

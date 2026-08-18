use std::fs;
use std::path::Path;

use image::{ImageFormat, Rgba, RgbaImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::SqliteCatalog;
use crate::application::StoragePaths;
use crate::application::scan_library::run_scan_with_storage;
use crate::domain::{
    IncrementalCatalogRoot, LibraryChangeIntent, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    ScanRequest,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

use super::*;

#[test]
fn recovery_policy_rejects_unbounded_or_zero_limits() {
    assert!(AuthoritativeRecoveryPolicy::default().is_valid());
    assert!(
        !AuthoritativeRecoveryPolicy {
            max_scope_entries: MAX_AUTHORITATIVE_ENTRIES + 1,
            ..AuthoritativeRecoveryPolicy::default()
        }
        .is_valid()
    );
    assert!(
        !AuthoritativeRecoveryPolicy {
            max_scope_paths: 0,
            ..AuthoritativeRecoveryPolicy::default()
        }
        .is_valid()
    );
}

#[test]
fn bounded_subtree_reconciles_addition_and_removal_at_one_revision() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    write_png(&album.join("removed.png"), [10, 20, 30, 255]);
    write_png(&source.path().join("outside.png"), [40, 50, 60, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-subtree-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let outside_before = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "outside.png")
        .expect("outside location")
        .expect("published outside location");
    let source_bytes = fs::read(source.path().join("outside.png")).expect("outside bytes");

    fs::remove_file(album.join("removed.png")).expect("remove controlled fixture");
    write_png(&album.join("new.png"), [70, 80, 90, 255]);
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        2_000,
    );
    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        2_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("bounded subtree recovery");

    let refreshed = only_root(&catalog);
    let outside_after = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "outside.png")
        .expect("outside location after recovery")
        .expect("outside remains published");
    assert_eq!(report.incremental.completed_count, 1);
    assert_eq!(report.incremental.applied_mutation_count, 2);
    assert!(report.full_scan.is_none());
    assert_eq!(refreshed.catalog_revision, root.catalog_revision + 1);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/removed.png")
            .expect("removed path query")
            .is_none()
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/new.png")
            .expect("new path query")
            .is_some()
    );
    assert_eq!(outside_after.asset_id, outside_before.asset_id);
    assert_eq!(
        fs::read(source.path().join("outside.png")).expect("outside after recovery"),
        source_bytes
    );
}

#[test]
fn oversized_authoritative_scope_defers_without_publishing_and_requests_full_scan() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("one.png"), [10, 20, 30, 255]);
    write_png(&source.path().join("two.png"), [40, 50, 60, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-overflow-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    enqueue_intent(&mut catalog, root_gap_intent(&root), 3_000);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_000,
        immediate_queue_policy(),
        AuthoritativeRecoveryPolicy {
            max_scope_entries: 1,
            max_scope_paths: 1,
            audit_interval_millis: 1_000,
        },
    )
    .expect("overflow escalation");
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &root.root_id,
            root.root_generation,
            3_000,
            immediate_queue_policy(),
        )
        .expect("queue metrics");

    assert_eq!(report.incremental.deferred_count, 1);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(report.incremental.catalog_revision, root.catalog_revision);
    assert_eq!(
        report
            .full_scan
            .as_ref()
            .map(|request| request.root_id.as_str()),
        Some(root.root_id.as_str())
    );
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
}

#[cfg(windows)]
#[test]
fn directory_rename_preserves_asset_identity_across_the_authoritative_batch() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let old_directory = source.path().join("old");
    fs::create_dir(&old_directory).expect("old directory");
    write_png(&old_directory.join("photo.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-rename-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let prior = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "old/photo.png")
        .expect("old path")
        .expect("published old path");

    fs::rename(&old_directory, source.path().join("new")).expect("rename controlled directory");
    enqueue_intent(
        &mut catalog,
        subtree_intent(
            &root,
            LibraryChangeIntentKind::RenameCandidate,
            "new",
            Some("old"),
        ),
        4_000,
    );
    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        4_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("directory rename recovery");
    let renamed = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "new/photo.png")
        .expect("new path")
        .expect("published new path");

    assert_eq!(report.incremental.completed_count, 1);
    assert_eq!(report.incremental.applied_mutation_count, 2);
    assert_eq!(renamed.asset_id, prior.asset_id);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "old/photo.png")
            .expect("old path after rename")
            .is_none()
    );
}

fn fixture_storage(directory: &TempDir) -> StoragePaths {
    StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings.sqlite3"),
    }
}

fn publish_initial_scan(source: &TempDir, storage: StoragePaths, scan_id: &str) {
    run_scan_with_storage(
        ScanRequest {
            scan_id: scan_id.to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage,
    )
    .expect("initial scan");
}

fn only_root(catalog: &SqliteCatalog) -> IncrementalCatalogRoot {
    let roots = catalog
        .load_incremental_catalog_roots()
        .expect("catalog roots");
    assert_eq!(roots.len(), 1);
    roots.into_iter().next().expect("one root")
}

fn write_png(path: &Path, color: [u8; 4]) {
    RgbaImage::from_pixel(8, 6, Rgba(color))
        .save_with_format(path, ImageFormat::Png)
        .expect("fixture image");
}

fn enqueue_intent(catalog: &mut SqliteCatalog, intent: LibraryChangeIntent, now_unix_ms: i64) {
    let report = catalog
        .enqueue_library_change_intents(&[intent], now_unix_ms, immediate_queue_policy())
        .expect("enqueue authoritative work");
    assert_eq!(report.inserted_count, 1);
}

fn subtree_intent(
    root: &IncrementalCatalogRoot,
    kind: LibraryChangeIntentKind,
    relative_path: &str,
    previous_relative_path: Option<&str>,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        kind,
        scope: LibraryChangeScope::Subtree,
        relative_path: relative_path.to_owned(),
        previous_relative_path: previous_relative_path.map(str::to_owned),
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: 1_000,
        most_recent_observed_unix_ms: 1_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    }
}

fn root_gap_intent(root: &IncrementalCatalogRoot) -> LibraryChangeIntent {
    LibraryChangeIntent {
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        ..subtree_intent(root, LibraryChangeIntentKind::Reconcile, "", None)
    }
}

fn immediate_queue_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 32,
        max_lease_batch: 16,
        lease_duration_millis: 1_000,
        max_attempts: 4,
        retry_initial_delay_millis: 10,
        retry_maximum_delay_millis: 100,
        terminal_retention_millis: 60_000,
        cleanup_batch: 16,
    }
}

fn fixture_recovery_policy() -> AuthoritativeRecoveryPolicy {
    AuthoritativeRecoveryPolicy {
        max_scope_entries: 64,
        max_scope_paths: 32,
        audit_interval_millis: 1_000,
    }
}

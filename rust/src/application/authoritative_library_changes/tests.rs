use std::fs;
use std::path::Path;

use image::{ImageFormat, Rgba, RgbaImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::SqliteCatalog;
use crate::application::StoragePaths;
use crate::application::metadata_inventory::{
    leased_change_requires_metadata_inventory, process_leased_metadata_inventory_change,
};
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
fn oversized_authoritative_scope_retries_for_metadata_inventory_without_publishing() {
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
        },
    )
    .expect("bounded metadata retry");
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &root.root_id,
            root.root_generation,
            3_000,
            immediate_queue_policy(),
        )
        .expect("queue metrics");

    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(report.incremental.catalog_revision, root.catalog_revision);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
}

#[test]
fn oversized_subtree_continues_with_pageable_inventory_without_starting_a_scan() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    write_png(&album.join("removed.png"), [10, 20, 30, 255]);
    write_png(&album.join("kept.png"), [40, 50, 60, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-pageable-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let active_scan_id = root.active_scan_id.clone();
    fs::remove_file(album.join("removed.png")).expect("remove fixture");
    write_png(&album.join("added.png"), [70, 80, 90, 255]);
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        3_000,
    );

    let bounded = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_000,
        immediate_queue_policy(),
        AuthoritativeRecoveryPolicy {
            max_scope_entries: 1,
            max_scope_paths: 1,
        },
    )
    .expect("bounded scope requests pageable inventory");
    let leased = catalog
        .lease_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            3_010,
            immediate_queue_policy(),
        )
        .expect("lease pageable retry")
        .expect("pageable retry");
    assert_eq!(bounded.incremental.retried_count, 1);
    assert!(leased_change_requires_metadata_inventory(&leased));
    assert_eq!(
        leased
            .change
            .last_failure
            .as_ref()
            .map(|failure| failure.code.as_str()),
        Some("metadata_inventory_required")
    );

    let mut current_lease = Some(leased);
    let mut inventory = None;
    let mut completed_candidates = 0_u32;
    for _ in 0..12 {
        let leased = current_lease.take().unwrap_or_else(|| {
            catalog
                .lease_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    immediate_queue_policy(),
                )
                .expect("lease inventory continuation")
                .expect("inventory continuation")
        });
        let current = process_leased_metadata_inventory_change(
            &mut catalog,
            &root,
            &leased,
            3_010,
            1,
            immediate_queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("pageable subtree inventory");
        completed_candidates = completed_candidates.saturating_add(
            crate::application::process_ready_library_changes(
                &mut catalog,
                &root.root_id,
                root.root_generation,
                3_010,
                immediate_queue_policy(),
            )
            .expect("publish pageable inventory candidates")
            .completed_count,
        );
        let is_complete = current.inventory.is_complete;
        inventory = Some(current);
        if is_complete {
            break;
        }
    }
    let inventory = inventory.expect("inventory recovery report");
    let refreshed = only_root(&catalog);

    assert!(inventory.inventory.is_complete);
    assert_eq!(completed_candidates, 2);
    assert_eq!(refreshed.active_scan_id, active_scan_id);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/removed.png")
            .expect("removed path")
            .is_none()
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/added.png")
            .expect("added path")
            .is_some()
    );
}

#[test]
fn cancelled_background_recovery_leaves_authoritative_work_pending() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("pending.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-cancellation-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    enqueue_intent(&mut catalog, root_gap_intent(&root), 3_500);
    let cancellation = AtomicBool::new(true);

    let report = process_ready_authoritative_library_change_cancellable(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_500,
        immediate_queue_policy(),
        fixture_recovery_policy(),
        &cancellation,
    )
    .expect("cancelled recovery");
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &root.root_id,
            root.root_generation,
            3_500,
            immediate_queue_policy(),
        )
        .expect("cancelled queue metrics");

    assert_eq!(report, AuthoritativeLibraryChangeReport::default());
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.leased_count, 0);
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

#[cfg(windows)]
#[test]
fn new_cloud_placeholder_retries_authoritative_recovery_without_recording_an_audit() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-empty-placeholder-scan");
    let placeholder = source.path().join("online-only.png");
    fs::write(&placeholder, b"must not be hydrated").expect("placeholder fixture");
    set_offline_attribute(&placeholder, true);
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let prior_audit = root.last_consistency_audit_unix_ms;
    enqueue_intent(&mut catalog, root_gap_intent(&root), 5_000);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        5_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("placeholder recovery remains retryable");
    set_offline_attribute(&placeholder, false);
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &root.root_id,
            root.root_generation,
            5_000,
            immediate_queue_policy(),
        )
        .expect("placeholder queue metrics");

    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(report.incremental.completed_count, 0);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(
        only_root(&catalog).last_consistency_audit_unix_ms,
        prior_audit
    );
}

#[cfg(windows)]
#[test]
fn existing_cloud_placeholder_preserves_the_last_trustworthy_location() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let placeholder = source.path().join("retained.png");
    write_png(&placeholder, [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-retained-placeholder-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let prior = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "retained.png")
        .expect("prior location")
        .expect("published prior location");
    set_offline_attribute(&placeholder, true);
    enqueue_intent(&mut catalog, root_gap_intent(&root), 6_000);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        6_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("existing placeholder recovery remains retryable");
    set_offline_attribute(&placeholder, false);
    let retained = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "retained.png")
        .expect("retained location")
        .expect("last trustworthy location");

    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
    assert_eq!(retained.location_id, prior.location_id);
    assert_eq!(retained.asset_id, prior.asset_id);
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

#[cfg(windows)]
fn set_offline_attribute(path: &Path, is_offline: bool) {
    let status = std::process::Command::new("attrib.exe")
        .arg(if is_offline { "+O" } else { "-O" })
        .arg(path)
        .status()
        .expect("attrib executable");
    assert!(status.success());
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
    }
}

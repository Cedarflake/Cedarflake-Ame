use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::{FileDiscovery, LocalMetadataInventory, SqliteCatalog};
use crate::application::StoragePaths;
use crate::application::process_ready_library_changes;
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPage, MetadataInventoryPlaceholderState, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, MetadataInventoryScope, MetadataInventoryStartRequest, ScanError,
    ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue,
    MetadataInventoryRepository, MetadataInventorySource,
};

use super::{
    leased_change_requires_metadata_inventory, metadata_inventory_run_id,
    process_leased_metadata_inventory_change, retry_terminalization, run_metadata_inventory,
};

#[test]
fn closed_process_metadata_changes_converge_through_bounded_inventory_pages() {
    let mut fixture = InventoryFixture::new(&[
        "keep.png",
        "modify.png",
        "delete.png",
        "rename.png",
        "album/moved.png",
    ]);
    write_png(
        &fixture.source.path().join("modify.png"),
        4,
        3,
        [90, 80, 70],
    );
    fs::remove_file(fixture.source.path().join("delete.png")).expect("delete fixture");
    fs::rename(
        fixture.source.path().join("rename.png"),
        fixture.source.path().join("renamed.png"),
    )
    .expect("rename fixture");
    fs::rename(
        fixture.source.path().join("album"),
        fixture.source.path().join("moved-album"),
    )
    .expect("move directory fixture");
    write_png(
        &fixture.source.path().join("新增图片.png"),
        3,
        2,
        [20, 40, 60],
    );
    let long_name = format!("{}.png", "长路径".repeat(30));
    write_png(&fixture.source.path().join(&long_name), 2, 3, [60, 40, 20]);

    let report = fixture.run_inventory(2, &AtomicBool::new(false));

    assert!(report.is_complete);
    assert!(report.staged_entry_count >= 7);
    assert!(report.candidate_count >= 6);
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process inventory candidates");
    assert!(incremental.completed_count >= 6);
    assert!(fixture.location("keep.png").is_some());
    assert_eq!(
        fixture
            .location("modify.png")
            .map(|location| (location.width, location.height)),
        Some((4, 3))
    );
    assert!(fixture.location("delete.png").is_none());
    assert!(fixture.location("rename.png").is_none());
    assert!(fixture.location("renamed.png").is_some());
    assert!(fixture.location("album/moved.png").is_none());
    assert!(fixture.location("moved-album/moved.png").is_some());
    assert!(fixture.location("新增图片.png").is_some());
    assert!(fixture.location(&long_name).is_some());
}

#[test]
fn cancelled_inventory_keeps_absence_unproven_and_catalog_unchanged() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::remove_file(fixture.source.path().join("retained.png")).expect("delete fixture");
    let cancellation = AtomicBool::new(true);

    let report = fixture.run_inventory(1, &cancellation);

    assert!(report.is_cancelled);
    assert!(!report.is_complete);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("inventory-1")
        .expect("load inventory")
        .expect("inventory run");
    assert_eq!(run.status, MetadataInventoryRunStatus::Cancelled);
    assert!(!run.enumeration_complete);
    assert!(!run.absence_authority);
    assert!(fixture.location("retained.png").is_some());
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count + metrics.retry_wait_count, 0);
}

#[test]
fn placeholder_evidence_is_staged_and_enqueued_without_media_inspection() {
    let mut fixture = InventoryFixture::new(&[]);
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-only.bin".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(123),
                modified_unix_ms: 1_500,
                file_identity: None,
                placeholder_state: MetadataInventoryPlaceholderState::Offline,
                is_reparse_point: false,
            }],
            cursor: Some("cloud-only.bin".to_owned()),
            is_complete: true,
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only placeholder inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 1);
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 1);
}

#[test]
fn unchanged_reparse_cloud_placeholder_preserves_location_without_retry() {
    let mut fixture = InventoryFixture::new(&["cloud-only.png"]);
    let prior = fixture
        .location("cloud-only.png")
        .expect("published placeholder location");
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-only.png".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(prior.file_size),
                modified_unix_ms: prior.modified_unix_ms,
                file_identity: None,
                placeholder_state: MetadataInventoryPlaceholderState::Offline,
                is_reparse_point: true,
            }],
            cursor: Some("cloud-only.png".to_owned()),
            is_complete: true,
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only reparse placeholder inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 0);
    assert_eq!(report.absence_candidate_count, 0);
    assert!(fixture.location("cloud-only.png").is_some());
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count + metrics.retry_wait_count, 0);
}

#[test]
fn hydrated_cloud_files_reparse_matches_the_existing_location() {
    let mut fixture = InventoryFixture::new(&["cloud-local.png"]);
    let prior = fixture
        .location("cloud-local.png")
        .expect("published local Cloud Files location");
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-local.png".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(prior.file_size),
                modified_unix_ms: prior.modified_unix_ms,
                file_identity: prior.file_identity.clone(),
                placeholder_state: MetadataInventoryPlaceholderState::Available,
                is_reparse_point: true,
            }],
            cursor: Some("cloud-local.png".to_owned()),
            is_complete: true,
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only hydrated Cloud Files inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 0);
    assert_eq!(report.absence_candidate_count, 0);
    assert!(fixture.location("cloud-local.png").is_some());
}

#[test]
fn source_failure_terminates_the_durable_inventory_run() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    let mut source = FailingInventorySource;

    let error = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect_err("inventory source failure");
    let run = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load failed inventory")
        .expect("failed inventory run");

    assert_eq!(error.code, "metadata_inventory_fixture_failure");
    assert_eq!(run.status, MetadataInventoryRunStatus::Failed);
    assert_eq!(
        run.last_issue_code.as_deref(),
        Some("metadata_inventory_fixture_failure")
    );
    assert!(!run.absence_authority);
}

#[test]
fn terminalization_retries_one_transient_catalog_contention_failure() {
    let mut attempts = 0;

    retry_terminalization(|| {
        attempts += 1;
        if attempts == 1 {
            Err(ScanError::new(
                "catalog_database_busy",
                "fixture writer still owns the catalog",
            ))
        } else {
            Ok(())
        }
    })
    .expect("second terminalization attempt");

    assert_eq!(attempts, 2);
}

#[test]
fn newer_epoch_supersedes_an_orphaned_active_run_and_cleanup_stays_bounded() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&first)
        .expect("begin first inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &first.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("orphaned.txt")],
                cursor: Some("orphaned.txt".to_owned()),
                is_complete: false,
            },
            2_100,
        )
        .expect("stage orphaned entry");
    let second = MetadataInventoryRunRequest {
        run_id: "inventory-2".to_owned(),
        epoch: 2,
        started_unix_ms: 3_000,
        ..first.clone()
    };

    fixture
        .catalog
        .begin_metadata_inventory(&second)
        .expect("newer epoch");

    let superseded = fixture
        .catalog
        .load_metadata_inventory_run(&first.run_id)
        .expect("load first run")
        .expect("superseded run");
    assert_eq!(superseded.status, MetadataInventoryRunStatus::Superseded);
    let cleanup = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(4_000, 1, 1)
        .expect("bounded cleanup");
    assert_eq!(cleanup.removed_entry_count, 1);
    assert_eq!(cleanup.removed_run_count, 1);
    assert!(!cleanup.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&first.run_id)
            .expect("load cleaned run")
            .is_none()
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&second.run_id)
            .expect("load active run")
            .expect("active run")
            .status,
        MetadataInventoryRunStatus::Running
    );
}

#[test]
fn next_epoch_is_allocated_atomically_and_does_not_depend_on_wall_clock_order() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "inventory-next-1".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 5_000,
        })
        .expect("begin first allocated epoch");
    let second = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "inventory-next-2".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: MetadataInventoryScope::Subtree {
                relative_path: "album".to_owned(),
            },
            started_unix_ms: 4_000,
        })
        .expect("begin second allocated epoch");

    assert_eq!(first.request.epoch, 1);
    assert_eq!(second.request.epoch, 2);
    assert_eq!(second.request.started_unix_ms, 4_000);
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&first.request.run_id)
            .expect("load first allocated epoch")
            .expect("first allocated epoch")
            .status,
        MetadataInventoryRunStatus::Superseded
    );
}

#[test]
fn startup_gap_uses_inventory_and_completes_its_authoritative_lease() {
    let mut fixture = InventoryFixture::new(&["removed.png"]);
    fs::remove_file(fixture.source.path().join("removed.png")).expect("remove fixture");
    write_png(&fixture.source.path().join("added.png"), 3, 2, [40, 50, 60]);
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..queue_policy()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue startup authority");
    let mut report = None;
    let mut completed_candidates = 0_u32;
    for _ in 0..8 {
        let leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                2_000,
                policy,
            )
            .expect("lease startup authority")
            .expect("startup authority");
        assert!(leased_change_requires_metadata_inventory(&leased));
        let root = fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load root")
            .expect("root");
        let current = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &leased,
            2_000,
            1,
            policy,
            &AtomicBool::new(false),
        )
        .expect("process startup inventory page");
        completed_candidates = completed_candidates.saturating_add(
            process_ready_library_changes(
                &mut fixture.catalog,
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                2_000,
                policy,
            )
            .expect("publish inventory candidates")
            .completed_count,
        );
        let is_complete = current.inventory.is_complete;
        report = Some(current);
        if is_complete {
            break;
        }
    }
    let report = report.expect("inventory recovery report");

    assert!(report.inventory.is_complete);
    assert_eq!(report.inventory.staged_entry_count, 1);
    assert_eq!(report.inventory.candidate_count, 2);
    assert_eq!(report.incremental.leased_count, 1);
    assert_eq!(completed_candidates, 2);
    assert!(fixture.location("removed.png").is_none());
    assert!(fixture.location("added.png").is_some());
}

#[test]
fn newer_gap_supersedes_an_incomplete_inventory_epoch_and_its_staged_candidates() {
    let mut fixture = InventoryFixture::new(&[]);
    write_png(&fixture.source.path().join("first.png"), 2, 2, [10, 20, 30]);
    write_png(
        &fixture.source.path().join("second.png"),
        2,
        2,
        [40, 50, 60],
    );
    let policy = queue_policy();
    let first_intent = LibraryChangeIntent {
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: 2_000,
        most_recent_observed_unix_ms: 2_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    };
    fixture
        .catalog
        .enqueue_library_change_intents(std::slice::from_ref(&first_intent), 2_000, policy)
        .expect("enqueue first epoch");
    let first_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease first epoch")
        .expect("first epoch");
    let first_run_id = metadata_inventory_run_id(&first_lease);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let first_report = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &first_lease,
        2_000,
        1,
        policy,
        &AtomicBool::new(false),
    )
    .expect("start first epoch");
    assert!(!first_report.inventory.is_complete);

    let newer_intent = LibraryChangeIntent {
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: 3_000,
        most_recent_observed_unix_ms: 3_000,
        first_sequence: 2,
        most_recent_sequence: 2,
        ..first_intent
    };
    fixture
        .catalog
        .enqueue_library_change_intents(&[newer_intent], 3_000, policy)
        .expect("enqueue newer gap");
    let second_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            3_000,
            policy,
        )
        .expect("lease newer epoch")
        .expect("newer epoch");
    let second_run_id = metadata_inventory_run_id(&second_lease);
    process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &second_lease,
        3_000,
        1,
        policy,
        &AtomicBool::new(false),
    )
    .expect("start newer epoch");

    let first_run = fixture
        .catalog
        .load_metadata_inventory_run(&first_run_id)
        .expect("load first run")
        .expect("first run");
    let second_run = fixture
        .catalog
        .load_metadata_inventory_run(&second_run_id)
        .expect("load second run")
        .expect("second run");
    assert_eq!(first_run.status, MetadataInventoryRunStatus::Superseded);
    assert_eq!(second_run.request.epoch, first_run.request.epoch + 1);
}

#[test]
fn inventory_failure_exhausts_durable_retry_without_starting_a_full_scan() {
    let mut fixture = InventoryFixture::new(&[]);
    fs::remove_dir(fixture.source.path()).expect("remove source fixture");
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 2,
        retry_initial_delay_millis: 1,
        retry_maximum_delay_millis: 1,
        ..queue_policy()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue failing inventory");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    for now_unix_ms in [2_000, 2_001] {
        let leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms,
                policy,
            )
            .expect("lease failing inventory")
            .expect("failing inventory authority");
        let report = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &leased,
            now_unix_ms,
            16,
            policy,
            &AtomicBool::new(false),
        )
        .expect("record inventory failure");
        assert_eq!(report.incremental.retried_count, 1);
    }
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_001,
            policy,
        )
        .expect("load exhausted inventory metrics");

    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(metrics.ready_count, 0);
    assert!(
        fixture
            .catalog
            .load_recoverable_scan()
            .expect("recoverable scan")
            .is_none()
    );
}

#[test]
fn cancelled_automatic_inventory_defers_authority_and_preserves_absence() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::remove_file(fixture.source.path().join("retained.png")).expect("remove fixture");
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue cancellable inventory");
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease cancellable inventory")
        .expect("cancellable inventory");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");

    let report = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &leased,
        2_000,
        1,
        policy,
        &AtomicBool::new(true),
    )
    .expect("cancel inventory");
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("load cancelled inventory metrics");

    assert!(report.inventory.is_cancelled);
    assert_eq!(report.incremental.deferred_count, 1);
    assert_eq!(metrics.pending_count, 1);
    assert!(fixture.location("retained.png").is_some());
}

#[test]
fn terminal_inventory_cleanup_deletes_staging_in_bounded_batches() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &request.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![
                    metadata_entry("a.txt"),
                    metadata_entry("b.txt"),
                    metadata_entry("c.txt"),
                ],
                cursor: Some("c.txt".to_owned()),
                is_complete: false,
            },
            2_100,
        )
        .expect("stage entries");
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_200,
        )
        .expect("terminate inventory");

    let first = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 2, 1)
        .expect("first cleanup batch");
    assert_eq!(first.removed_entry_count, 2);
    assert_eq!(first.removed_run_count, 0);
    assert!(first.has_more);
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone())
        .expect("reopen partially cleaned terminal inventory");
    let second = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 2, 1)
        .expect("second cleanup batch");
    assert_eq!(second.removed_entry_count, 1);
    assert_eq!(second.removed_run_count, 0);
    assert!(!second.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load retained summary")
            .is_some()
    );
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen cleaned catalog");
    let expired = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(3_000, 2, 1)
        .expect("expired summary cleanup");
    assert_eq!(expired.removed_entry_count, 0);
    assert_eq!(expired.removed_run_count, 1);
    assert!(!expired.has_more);
}

#[cfg(windows)]
#[test]
fn one_previous_path_is_not_reused_for_multiple_hard_link_candidates() {
    let mut fixture = InventoryFixture::new(&["old.png"]);
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new-a.png"),
    )
    .expect("rename fixture");
    fs::hard_link(
        fixture.source.path().join("new-a.png"),
        fixture.source.path().join("new-b.png"),
    )
    .expect("hard link fixture");

    let report = fixture.run_inventory(16, &AtomicBool::new(false));
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process hard link candidates");
    let first = fixture.location("new-a.png").expect("first hard link");
    let second = fixture.location("new-b.png").expect("second hard link");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 2);
    assert_eq!(incremental.completed_count, 2);
    assert!(fixture.location("old.png").is_none());
    assert_eq!(first.asset_id, second.asset_id);
}

struct FixedInventorySource {
    page: Option<MetadataInventoryPage>,
}

impl MetadataInventorySource for FixedInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        self.page.take().ok_or_else(|| {
            ScanError::new(
                "fixed_inventory_exhausted",
                "The fixed inventory page was already consumed",
            )
        })
    }
}

struct FailingInventorySource;

impl MetadataInventorySource for FailingInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        Err(ScanError::new(
            "metadata_inventory_fixture_failure",
            "The metadata inventory fixture failed",
        ))
    }
}

struct InventoryFixture {
    source: TempDir,
    _storage: TempDir,
    catalog: SqliteCatalog,
    root_id: String,
    root_path: String,
}

impl InventoryFixture {
    fn new(relative_paths: &[&str]) -> Self {
        let source = tempdir().expect("source directory");
        for relative_path in relative_paths {
            let path = source.path().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            write_png(&path, 2, 2, [10, 20, 30]);
        }
        let storage = tempdir().expect("storage directory");
        let storage_paths = StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let root_path = FileDiscovery::new(&source.path().to_string_lossy())
            .expect("source discovery")
            .canonical_root()
            .expect("canonical root")
            .to_string_lossy()
            .into_owned();
        let root_id = stable_id("library-root-v1", &root_path);
        let scan_id = stable_id("metadata-inventory-initial-scan-v1", &root_path);
        run_scan_with_storage(
            ScanRequest {
                scan_id,
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage_paths.clone(),
        )
        .expect("initial scan");
        let catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
        Self {
            source,
            _storage: storage,
            catalog,
            root_id,
            root_path,
        }
    }

    fn request(&self) -> MetadataInventoryRunRequest {
        MetadataInventoryRunRequest {
            run_id: "inventory-1".to_owned(),
            root_id: self.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 2_000,
        }
    }

    fn run_inventory(
        &mut self,
        page_limit: u32,
        cancellation: &AtomicBool,
    ) -> crate::domain::MetadataInventoryReport {
        let request = self.request();
        let mut source = LocalMetadataInventory::new(&self.root_path, &request.scope)
            .expect("local metadata inventory");
        run_metadata_inventory(
            &mut self.catalog,
            &mut source,
            &request,
            2_000,
            page_limit,
            queue_policy(),
            cancellation,
        )
        .expect("run metadata inventory")
    }

    fn location(&self, relative_path: &str) -> Option<crate::domain::AssetLocationView> {
        self.catalog
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("load location")
    }
}

fn queue_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 128,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn metadata_entry(relative_path: &str) -> MetadataInventoryEntry {
    MetadataInventoryEntry {
        relative_path: relative_path.to_owned(),
        kind: MetadataInventoryEntryKind::File,
        file_size: Some(1),
        modified_unix_ms: 1,
        file_identity: None,
        placeholder_state: MetadataInventoryPlaceholderState::Available,
        is_reparse_point: false,
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("PNG parent");
    }
    RgbImage::from_pixel(width, height, Rgb(color))
        .save(path)
        .expect("write PNG fixture");
}

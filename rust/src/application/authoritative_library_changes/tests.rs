use std::fs;
use std::path::Path;

use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

use crate::adapters::{
    FileDiscovery, FileVisitOutcome, PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, SqliteCatalog,
};
#[cfg(windows)]
use crate::adapters::{reset_source_content_open_instrumentation, source_content_open_count};
use crate::application::StoragePaths;
use crate::application::metadata_inventory::{
    MetadataInventoryRecoveryExecution, leased_change_requires_metadata_inventory,
    process_leased_metadata_inventory_change_with_retained_source,
};
use crate::application::scan_library::run_scan_with_storage;
use crate::domain::{
    IncrementalCatalogRoot, JournalFileReference, JournalIdentifier, JournalUsn,
    LibraryChangeIntent, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    PersistentJournalBaselineClosingBoundary, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalVolumeIdentity, PreviewArtifact,
    PreviewRequest, PreviewStatus, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue,
    PersistentJournalRepository,
};

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

#[cfg(windows)]
#[test]
fn bounded_root_recovery_without_v29_publication_proof_reads_nothing_and_retries() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("existing.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "root-proof-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    remove_publication_namespace_proof(&paths.catalog_path, &root.root_id);
    write_png(
        &source.path().join("must-not-publish.png"),
        [40, 50, 60, 255],
    );
    enqueue_intent(&mut catalog, root_gap_intent(&root), 1_500);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        1_500,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("missing proof remains retryable");

    assert_eq!(report.incremental.completed_count, 0);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "must-not-publish.png")
            .expect("new location query")
            .is_none()
    );
}

#[cfg(windows)]
#[test]
fn bounded_subtree_recovery_without_v29_publication_proof_publishes_nothing() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    write_png(&album.join("existing.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "subtree-proof-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    remove_publication_namespace_proof(&paths.catalog_path, &root.root_id);
    write_png(&album.join("must-not-publish.png"), [40, 50, 60, 255]);
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        1_600,
    );

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        1_600,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("missing subtree proof remains retryable");

    assert_eq!(report.incremental.completed_count, 0);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(
                &root.root_id,
                "album/must-not-publish.png",
            )
            .expect("new subtree location query")
            .is_none()
    );
}

#[cfg(windows)]
#[test]
fn bounded_p2_authoritative_recovery_without_v29_proof_publishes_nothing() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("existing.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "p2-proof-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    remove_publication_namespace_proof(&paths.catalog_path, &root.root_id);
    write_png(
        &source.path().join("must-not-publish.png"),
        [40, 50, 60, 255],
    );
    let mut p2 = root_gap_intent(&root);
    p2.origin = LibraryChangeOrigin::ConsistencyAudit;
    enqueue_intent(&mut catalog, p2, 1_700);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        1_700,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("missing P2 proof remains retryable");

    assert_eq!(report.incremental.completed_count, 0);
    assert_eq!(report.incremental.applied_mutation_count, 0);
    assert_eq!(report.incremental.retried_count, 1);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "must-not-publish.png")
            .expect("new P2 location query")
            .is_none()
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
fn root_live_gap_waits_for_durable_journal_range_without_publishing() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("one.png"), [10, 20, 30, 255]);
    write_png(&source.path().join("two.png"), [40, 50, 60, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "initial-overflow-scan");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    seed_current_journal_authority(&mut catalog, &root, 2_900);
    enqueue_intent(&mut catalog, root_gap_intent(&root), 3_000);

    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("durable journal ownership retry");
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
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(only_root(&catalog).catalog_revision, root.catalog_revision);
    let connection = Connection::open(&paths.catalog_path).expect("live-gap evidence catalog");
    let evidence: (
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
    ) = connection
        .query_row(
            "SELECT gap.origin, lane.lane, gap.intent_kind, gap.scope, gap.status,
                    claim.consumer_kind,
                    (SELECT COUNT(*) FROM library_change_queue_lanes
                     WHERE lane = 'p2_recovery'),
                    (SELECT COUNT(*) FROM library_recovery_authorities
                     WHERE reason = 'watcher_uncovered_gap'),
                    (SELECT COUNT(*) FROM scan_runs)
             FROM library_change_queue AS gap
             JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
             JOIN library_live_gap_recovery_claims AS claim
               ON claim.gap_change_id = gap.id
             WHERE gap.root_id = ?1 AND gap.origin = 'live_notification'
               AND gap.intent_kind = 'freshness_unknown'
               AND gap.scope = 'root' AND gap.relative_path = ''",
            [&root.root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .expect("durable pending journal claim");
    assert_eq!(
        evidence,
        (
            "live_notification".to_owned(),
            "p0_live".to_owned(),
            "freshness_unknown".to_owned(),
            "root".to_owned(),
            "retry_wait".to_owned(),
            "pending_journal".to_owned(),
            0,
            0,
            1,
        )
    );
}

#[cfg(windows)]
#[test]
fn capacity_degraded_gap_after_restart_invalidates_an_equal_metadata_preview() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("same.bmp");
    write_bmp(&source_path, [10, 20, 30]);
    let original_modified = fs::metadata(&source_path)
        .expect("original source metadata")
        .modified()
        .expect("original modified time");
    let original_bytes = fs::read(&source_path).expect("original source bytes");
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "capacity-dirty-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    publish_ready_preview(&mut catalog, &storage, &root.root_id, "same.bmp");
    let prior = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "same.bmp")
        .expect("prior location")
        .expect("published prior location");
    assert!(matches!(prior.preview_status, PreviewStatus::Ready));
    assert!(!prior.preview_path.is_empty());

    write_bmp(&source_path, [30, 20, 10]);
    fs::OpenOptions::new()
        .write(true)
        .open(&source_path)
        .expect("open replacement for timestamp restore")
        .set_times(fs::FileTimes::new().set_modified(original_modified))
        .expect("restore replacement modified time");
    let replacement_bytes = fs::read(&source_path).expect("replacement source bytes");
    assert_ne!(replacement_bytes, original_bytes);
    assert_eq!(replacement_bytes.len(), original_bytes.len());
    let replacement = match FileDiscovery::new(&root.root_path)
        .expect("replacement discovery")
        .visit_relative_path("same.bmp")
        .outcome
    {
        FileVisitOutcome::File(file) => file,
        _ => panic!("replacement must remain a supported local image"),
    };
    assert_eq!(replacement.file_size, prior.file_size);
    assert_eq!(replacement.modified_unix_ms, prior.modified_unix_ms);
    assert_eq!(replacement.file_identity, prior.file_identity);
    assert_ne!(replacement.source_revision, prior.source_revision);

    let replacement_revision = replacement
        .source_revision
        .as_ref()
        .expect("replacement ChangeTime evidence");
    let replacement_revision_token = format!(
        "{}:{}",
        replacement_revision.scheme, replacement_revision.value
    );
    assert_eq!(
        Connection::open(&paths.catalog_path)
            .expect("open adversarial evidence fixture")
            .execute(
                "UPDATE asset_locations
                 SET source_revision_token = ?1
                 WHERE root_id = ?2 AND relative_path = 'same.bmp'",
                rusqlite::params![replacement_revision_token, &root.root_id],
            )
            .expect("align catalog evidence with the replacement"),
        1
    );
    let apparently_unchanged = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "same.bmp")
        .expect("apparently unchanged location")
        .expect("apparently unchanged location remains published");
    assert_eq!(apparently_unchanged.file_size, replacement.file_size);
    assert_eq!(
        apparently_unchanged.modified_unix_ms,
        replacement.modified_unix_ms
    );
    assert_eq!(
        apparently_unchanged.file_identity,
        replacement.file_identity
    );
    assert_eq!(
        apparently_unchanged.source_revision,
        replacement.source_revision
    );
    assert_eq!(
        apparently_unchanged.source_generation,
        prior.source_generation
    );
    assert_eq!(apparently_unchanged.preview_path, prior.preview_path);
    reset_source_content_open_instrumentation(&root.root_path);

    let capacity_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_queue_policy()
    };
    let mut first = subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "same.bmp", None);
    first.scope = LibraryChangeScope::Path;
    let mut second = subtree_intent(
        &root,
        LibraryChangeIntentKind::Reconcile,
        "overflow.bmp",
        None,
    );
    second.scope = LibraryChangeScope::Path;
    second.first_sequence = 2;
    second.most_recent_sequence = 2;
    let enqueue = catalog
        .enqueue_library_change_intents(&[first, second], 3_000, capacity_policy)
        .expect("degrade precise dirty paths to one bounded root gap");
    assert!(enqueue.capacity_degraded);
    assert!(enqueue.freshness_unknown_enqueued);
    drop(catalog);

    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("reopen catalog");
    let promoted = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_010,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("promote the recovered live gap to metadata inventory");
    assert_eq!(promoted.incremental.retried_count, 1);

    let mut retained_source = None;
    let mut completed_candidates = 0_u32;
    let mut inventory_complete = false;
    for now_unix_ms in 3_020..3_032 {
        let leased = catalog
            .lease_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                now_unix_ms,
                immediate_queue_policy(),
            )
            .expect("lease recovery continuation")
            .expect("metadata inventory recovery continuation");
        assert!(leased_change_requires_metadata_inventory(&leased));
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut catalog,
            &root,
            &leased,
            MetadataInventoryRecoveryExecution::without_progress(
                now_unix_ms,
                32,
                immediate_queue_policy(),
                &AtomicBool::new(false),
            ),
            retained_source.take(),
        )
        .expect("process bounded metadata inventory page");
        retained_source = page.retained_source;
        completed_candidates = completed_candidates.saturating_add(
            crate::application::process_ready_library_changes(
                &mut catalog,
                &root.root_id,
                root.root_generation,
                now_unix_ms,
                immediate_queue_policy(),
            )
            .expect("publish conservative dirty candidate")
            .completed_count,
        );
        if page.report.inventory.is_complete {
            inventory_complete = true;
            break;
        }
    }
    assert!(inventory_complete);
    assert_eq!(completed_candidates, 1);
    let refreshed = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "same.bmp")
        .expect("refreshed location")
        .expect("refreshed location remains published");
    assert_eq!(refreshed.asset_id, prior.asset_id);
    assert_eq!(refreshed.source_revision, replacement.source_revision);
    assert!(refreshed.source_generation > prior.source_generation);
    assert!(matches!(refreshed.preview_status, PreviewStatus::Pending));
    assert!(refreshed.preview_path.is_empty());
    assert_eq!(source_content_open_count(&root.root_path), 0);
    let preview_evidence: (i64, String) = Connection::open(&paths.catalog_path)
        .expect("open preview invalidation evidence")
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM preview_artifact_locations
                WHERE artifact_key = 'capacity-dirty-ready-preview'),
               (SELECT lifecycle_state FROM preview_artifacts
                WHERE artifact_key = 'capacity-dirty-ready-preview')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load preview invalidation evidence");
    assert_eq!(preview_evidence, (0, "stale".to_owned()));
    assert_eq!(
        fs::read(&source_path).expect("replacement source after recovery"),
        replacement_bytes
    );
}

#[cfg(windows)]
#[test]
fn root_live_gap_capacity_deferral_survives_the_terminal_retry_budget() {
    let source = tempdir().expect("disposable source");
    let storage = tempdir().expect("storage directory");
    write_png(&source.path().join("existing.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "live-gap-capacity-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let active_scan_id = root.active_scan_id.clone();
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::LiveOnly,
            continuity: PersistentJournalContinuityState::LiveOnly,
            failure: None,
            updated_unix_ms: 3_900,
        })
        .expect("live-only journal capability");
    let capacity_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 1,
        ..immediate_queue_policy()
    };
    let mut journal = subtree_intent(
        &root,
        LibraryChangeIntentKind::Reconcile,
        "journal-owned.jpg",
        None,
    );
    journal.origin = LibraryChangeOrigin::StartupCatchUp;
    journal.scope = LibraryChangeScope::Path;
    catalog
        .enqueue_library_change_intents(&[journal], 4_000, capacity_policy)
        .expect("fill P1 allowance");
    catalog
        .enqueue_library_change_intents(&[root_gap_intent(&root)], 4_001, capacity_policy)
        .expect("use reserved P0 admission");

    for capacity_attempt in 0..=capacity_policy.max_attempts {
        let observed_unix_ms = 4_001 + i64::from(capacity_attempt) * 100;
        let leased = catalog
            .lease_live_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                observed_unix_ms,
                capacity_policy,
            )
            .expect("lease P0 capacity gap")
            .expect("capacity deferral must not exhaust the P0 gap");
        let report = process_leased_authoritative_library_change_cancellable(
            &mut catalog,
            &root,
            &leased,
            observed_unix_ms,
            capacity_policy,
            fixture_recovery_policy(),
            &AtomicBool::new(false),
        )
        .expect("capacity backpressure remains retryable");
        assert_eq!(
            report.incremental.retried_count, 1,
            "capacity authoritative report: {report:?}"
        );
        assert_eq!(report.incremental.applied_mutation_count, 0);
    }
    assert_eq!(only_root(&catalog).active_scan_id, active_scan_id);

    let connection = Connection::open(&paths.catalog_path).expect("capacity evidence catalog");
    type CapacityRollbackEvidence = (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        i64,
        i64,
        i64,
        i64,
    );
    let evidence: CapacityRollbackEvidence = connection
        .query_row(
            "SELECT gap.status, gap.origin, lane.lane, gap.intent_kind, gap.scope,
                    gap.last_failure_code, gap.superseded_by_change_id,
                    gap.next_retry_unix_ms,
                    (SELECT COUNT(*) FROM library_change_queue_lanes
                     WHERE lane = 'p2_recovery'),
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims),
                    (SELECT COUNT(*) FROM library_recovery_authorities
                     WHERE reason = 'watcher_uncovered_gap'),
                    (SELECT COUNT(*) FROM scan_runs)
             FROM library_change_queue AS gap
             JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
             WHERE gap.root_id = ?1 AND gap.origin = 'live_notification'
               AND gap.intent_kind = 'freshness_unknown'
               AND gap.scope = 'root' AND gap.relative_path = ''",
            [&root.root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .expect("capacity rollback evidence");
    assert_eq!(
        evidence,
        (
            "retry_wait".to_owned(),
            "live_notification".to_owned(),
            "p0_live".to_owned(),
            "freshness_unknown".to_owned(),
            "root".to_owned(),
            "live_gap_p2_capacity_deferred".to_owned(),
            None,
            Some(4_411),
            0,
            0,
            0,
            1,
        )
    );
    let gap_state: (i64, i64) = connection
        .query_row(
            "SELECT attempt_count, lease_generation
             FROM library_change_queue
             WHERE root_id = ?1 AND last_failure_code = 'live_gap_p2_capacity_deferred'",
            [&root.root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("capacity deferral budget evidence");
    assert_eq!(gap_state, (0, i64::from(capacity_policy.max_attempts + 1)));
    drop(connection);

    let crashed_lease = catalog
        .lease_live_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            4_501,
            capacity_policy,
        )
        .expect("lease capacity gap before simulated crash")
        .expect("capacity gap before simulated crash");
    drop(catalog);

    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("reopen catalog");
    assert!(
        catalog
            .lease_live_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                crashed_lease.lease_expires_unix_ms,
                capacity_policy,
            )
            .expect("recover expired capacity lease")
            .is_none(),
        "expired capacity work must re-enter bounded deferral before leasing"
    );
    let resumed_unix_ms = crashed_lease.lease_expires_unix_ms + 10;
    let resumed = catalog
        .lease_live_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            resumed_unix_ms,
            capacity_policy,
        )
        .expect("lease recovered capacity gap")
        .expect("recovered capacity gap");
    process_leased_authoritative_library_change_cancellable(
        &mut catalog,
        &root,
        &resumed,
        resumed_unix_ms,
        capacity_policy,
        fixture_recovery_policy(),
        &AtomicBool::new(false),
    )
    .expect("recovered capacity gap remains deferred");

    let released_unix_ms = resumed_unix_ms + 1;
    let journal = catalog
        .lease_path_library_changes_in_lane(
            &root.root_id,
            root.root_generation,
            crate::domain::LibraryChangeLane::Journal,
            released_unix_ms,
            capacity_policy,
        )
        .expect("lease capacity owner")
        .pop()
        .expect("capacity owner");
    assert_eq!(
        catalog
            .complete_library_change(
                journal.change.id,
                journal.lease_generation,
                root.catalog_revision,
                released_unix_ms,
            )
            .expect("release P1 capacity"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let promoted = catalog
        .lease_live_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            released_unix_ms,
            capacity_policy,
        )
        .expect("lease capacity-woken gap")
        .expect("capacity-woken gap");
    process_leased_authoritative_library_change_cancellable(
        &mut catalog,
        &root,
        &promoted,
        released_unix_ms,
        capacity_policy,
        fixture_recovery_policy(),
        &AtomicBool::new(false),
    )
    .expect("promote capacity-woken gap");

    let ownership: (String, String, String, String, i64, i64) =
        Connection::open(&paths.catalog_path)
            .expect("open capacity release evidence")
            .query_row(
                "SELECT gap.status, claim.consumer_kind, recovery.status, lane.lane,
                        (SELECT COUNT(*) FROM library_recovery_authorities AS authority
                         WHERE authority.change_id = recovery.id
                           AND authority.reason = 'watcher_uncovered_gap'
                           AND authority.retired_unix_ms IS NULL),
                        (SELECT COUNT(*) FROM scan_runs)
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
                 JOIN library_change_queue AS recovery ON recovery.id = claim.recovery_change_id
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = recovery.id
                 WHERE gap.root_id = ?1",
                [&root.root_id],
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
            .expect("capacity release ownership evidence");
    assert_eq!(
        ownership,
        (
            "superseded".to_owned(),
            "metadata_inventory_control".to_owned(),
            "pending".to_owned(),
            "p2_recovery".to_owned(),
            1,
            1,
        )
    );
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
    seed_current_journal_authority(&mut catalog, &root, 2_900);
    let active_scan_id = root.active_scan_id.clone();
    let kept_before = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "album/kept.png")
        .expect("load kept location before recovery")
        .expect("kept location before recovery");
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
    let mut retained_source = None;
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
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut catalog,
            &root,
            &leased,
            MetadataInventoryRecoveryExecution::without_progress(
                3_010,
                1,
                immediate_queue_policy(),
                &AtomicBool::new(false),
            ),
            retained_source.take(),
        )
        .expect("pageable subtree inventory");
        retained_source = page.retained_source;
        let current = page.report;
        if current.inventory.awaiting_closing_boundary {
            let baseline = catalog
                .load_persistent_journal_baselines()
                .expect("load pageable recovery baseline")
                .into_iter()
                .find(|baseline| baseline.root_id == root.root_id)
                .expect("pageable recovery baseline");
            catalog
                .capture_persistent_journal_baseline_closing_boundary(
                    &PersistentJournalBaselineClosingBoundary {
                        change_id: baseline.change_id,
                        volume: baseline.volume,
                        root_file_reference: baseline.root_file_reference,
                        journal_id: baseline.journal_id,
                        closing_next_usn: baseline.opening_next_usn,
                        protocol_version: baseline.protocol_version,
                        captured_unix_ms: 3_010,
                    },
                )
                .expect("close pageable recovery baseline");
        }
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
    assert_eq!(completed_candidates, 3);
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
    let kept_after = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "album/kept.png")
        .expect("load kept location after recovery")
        .expect("kept location after recovery");
    assert_eq!(kept_after.asset_id, kept_before.asset_id);
    assert_eq!(kept_after.file_identity, kept_before.file_identity);
    assert_eq!(kept_after.source_revision, kept_before.source_revision);
    assert!(kept_after.source_generation > kept_before.source_generation);
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
fn new_cloud_placeholder_root_gap_uses_p2_without_hydrating_or_recording_an_audit() {
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
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.pending_count, 1);
    let connection = Connection::open(&paths.catalog_path).expect("placeholder evidence catalog");
    let recovery: (String, String, String, String, String, i64) = connection
        .query_row(
            "SELECT gap.status, gap.origin, claim.consumer_kind,
                    recovery.origin, recovery_lane.lane,
                    (SELECT COUNT(*) FROM scan_runs)
             FROM library_change_queue AS gap
             JOIN library_live_gap_recovery_claims AS claim
               ON claim.gap_change_id = gap.id
             JOIN library_change_queue AS recovery
               ON recovery.id = claim.recovery_change_id
             JOIN library_change_queue_lanes AS recovery_lane
               ON recovery_lane.change_id = recovery.id
             WHERE gap.root_id = ?1 AND gap.origin = 'live_notification'
               AND gap.intent_kind = 'freshness_unknown'
               AND gap.scope = 'root' AND gap.relative_path = ''",
            [&root.root_id],
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
        .expect("placeholder P2 ownership");
    assert_eq!(
        recovery,
        (
            "superseded".to_owned(),
            "live_notification".to_owned(),
            "metadata_inventory_control".to_owned(),
            "metadata_inventory".to_owned(),
            "p2_recovery".to_owned(),
            1,
        )
    );
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

#[cfg(windows)]
fn remove_publication_namespace_proof(catalog_path: &Path, root_id: &str) {
    let connection = Connection::open(catalog_path).expect("open proof fixture catalog");
    let removed = connection
        .execute(
            "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
            [root_id],
        )
        .expect("remove publication namespace proof");
    assert_eq!(removed, 1);
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
fn write_bmp(path: &Path, color: [u8; 3]) {
    RgbaImage::from_pixel(8, 6, Rgba([color[0], color[1], color[2], 255]))
        .save_with_format(path, ImageFormat::Bmp)
        .expect("fixture BMP");
}

#[cfg(windows)]
fn publish_ready_preview(
    catalog: &mut SqliteCatalog,
    storage: &TempDir,
    root_id: &str,
    relative_path: &str,
) {
    let mut location = catalog
        .load_incremental_location_by_relative_path(root_id, relative_path)
        .expect("preview location")
        .expect("published preview location");
    let request = PreviewRequest {
        location_id: location.location_id.clone(),
        expected_root_id: location.root_id.clone(),
        expected_scan_id: location.scan_id.clone(),
        expected_source_revision: location.source_revision.clone(),
        expected_source_generation: location.source_generation,
        preview_edge: 256,
        retry_failed: false,
        protected_location_ids: Vec::new(),
    };
    let artifact_path = storage.path().join("capacity-dirty-ready-preview.jpg");
    RgbImage::from_pixel(
        location.width.max(1),
        location.height.max(1),
        Rgb([4, 8, 15]),
    )
    .save_with_format(&artifact_path, ImageFormat::Jpeg)
    .expect("write ready preview artifact");
    location.preview_path = artifact_path.to_string_lossy().into_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "capacity-dirty-ready-preview".to_owned(),
        algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
        algorithm_version: PREVIEW_ALGORITHM_VERSION,
        orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: fs::metadata(&artifact_path)
            .expect("ready preview metadata")
            .len(),
        encoded_width: location.width.max(1),
        encoded_height: location.height.max(1),
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), Some(&request))
        .expect("publish ready preview");
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

fn seed_current_journal_authority(
    catalog: &mut SqliteCatalog,
    root: &IncrementalCatalogRoot,
    updated_unix_ms: i64,
) {
    let checkpoint = PersistentJournalCheckpoint {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "authoritative-test-volume".to_owned(),
            volume_serial: 41,
        },
        root_file_reference: JournalFileReference::V3([7; 16]),
        journal_id: JournalIdentifier::new(83).expect("journal ID"),
        next_unread_usn: JournalUsn::new(1_024).expect("opening USN"),
        captured_exclusive_end: JournalUsn::new(1_024).expect("opening end"),
        covered_catalog_revision: root.catalog_revision,
        protocol_version: 5,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::Current,
        failure: None,
        updated_unix_ms,
    };
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: checkpoint.root_id.clone(),
            root_generation: checkpoint.root_generation,
            protocol_version: checkpoint.protocol_version,
            contract_version: checkpoint.contract_version,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms,
        })
        .expect("current journal authority");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&checkpoint)
        .expect("current journal checkpoint");
}

fn fixture_recovery_policy() -> AuthoritativeRecoveryPolicy {
    AuthoritativeRecoveryPolicy {
        max_scope_entries: 64,
        max_scope_paths: 32,
    }
}

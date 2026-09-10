use std::fs;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use image::{ImageFormat, Rgb, RgbImage};
use rusqlite::Connection;
use tempfile::{TempDir, tempdir, tempdir_in};

use crate::adapters::{
    FileDiscovery, FileVisitOutcome, LocalMediaInspector, PublicationGuardedFileDiscovery,
    SqliteCatalog, configured_root_open_count, remove_persistent_journal_v22_contract_for_test,
    reset_configured_root_open_instrumentation, reset_source_enumeration_instrumentation,
    source_directory_open_count, source_entry_read_count,
};
#[cfg(windows)]
use crate::adapters::{reset_source_content_open_instrumentation, source_content_open_count};
use crate::application::metadata_inventory::{
    MetadataInventoryRecoveryExecution, process_leased_metadata_inventory_change,
    process_leased_metadata_inventory_change_with_retained_source,
};
use crate::application::persistent_journal_continuity::{
    PersistentJournalBrokerRoot, SessionBackedPersistentJournalVolumeReader,
    catch_up_persistent_journal_volume,
};
use crate::application::{
    AuthoritativeRecoveryPolicy, process_ready_authoritative_library_change_cancellable,
    process_ready_library_changes_in_lane,
    process_ready_metadata_inventory_recovery_candidates_cancellable,
};
#[cfg(windows)]
use crate::domain::PreviewRequest;
use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaPublication, DerivedEvidenceDisposition,
    FileIdentityEvidence, JournalFileReference, JournalIdentifier, JournalUsn,
    LibraryChangeCatchUpEvidence, LibraryChangeCatchUpQueueBatch, LibraryChangeEnqueueReport,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLane, LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    PersistentJournalBaselineClosingBoundary, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalCrossRootLineage,
    PersistentJournalEnrollmentBatch, PersistentJournalLineageState,
    PersistentJournalPendingRename, PersistentJournalRangeState, PersistentJournalSourceRange,
    PersistentJournalVolumeBatch, PersistentJournalVolumeIdentity, PersistentJournalVolumePage,
    PreviewArtifact, PreviewStatus, ScanError, ScanRequest, TerminalMediaEvidence,
    persistent_journal_batch_id, persistent_journal_pending_rename_id,
};

#[cfg(windows)]
mod preparation_rebase;
mod revision_races;
#[cfg(windows)]
mod terminal_media;

use revision_races::{RevisionRacingCatalog, prepare_revision_races};

#[test]
fn p1_revision_rebase_reprepares_and_revalidates_the_latest_file_state() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let target = fixture.source.path().join("created.png");
    write_png(&target, 2, 2, [10, 20, 30]);
    let mut change = intent(&fixture.root_id, "created.png", None, 1);
    change.origin = LibraryChangeOrigin::StartupCatchUp;
    fixture.enqueue(&[change]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let CatalogFixture {
        source,
        _storage,
        catalog,
        root_id,
        ..
    } = fixture;
    let mut repository = RevisionRacingCatalog::new(catalog, races);
    repository.rewrite_on_first_publication = Some(target);

    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("rebase P1 change");

    assert_eq!(repository.publication_attempts, 2);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    let location = repository
        .catalog
        .load_incremental_location_by_relative_path(&root_id, "created.png")
        .expect("load refreshed location")
        .expect("refreshed location");
    assert_eq!((location.width, location.height), (4, 3));
    assert!(source.path().join("created.png").is_file());
    drop(repository);
    drop(_storage);
}

#[test]
fn p0_keeps_the_root_namespace_guard_through_the_sqlite_delta_commit() {
    let parent = tempdir().expect("controlled ancestor");
    let source = tempdir_in(parent.path()).expect("source root");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("created.png"),
        2,
        2,
        [10, 20, 30],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "created.png", None, 1)]);
    let moved_root = fixture.source.path().with_file_name("held-p0-root");
    let CatalogFixture {
        source,
        _storage,
        catalog,
        root_id,
        ..
    } = fixture;
    let mut repository = RevisionRacingCatalog::new(catalog, Vec::new());
    repository.guarded_rename_during_publication =
        Some((source.path().to_path_buf(), moved_root.clone()));

    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        2_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("publish guarded P0 delta");

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&root_id, "created.png")
            .expect("load P0 location")
            .is_some()
    );
    fs::rename(source.path(), &moved_root).expect("root rename after publication guard drop");
    fs::rename(&moved_root, source.path()).expect("restore source root");
    drop(repository);
    drop(_storage);
    drop(source);
    drop(parent);
}

#[test]
fn p1_paired_rename_keeps_every_ancestor_guarded_through_commit() {
    let parent = tempdir().expect("controlled ancestor");
    let source = tempdir_in(parent.path()).expect("source root");
    write_png(&source.path().join("old.png"), 2, 2, [30, 20, 10]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("same-volume source rename");
    let mut change = intent(&fixture.root_id, "new.png", Some("old.png"), 1);
    change.origin = LibraryChangeOrigin::StartupCatchUp;
    fixture.enqueue(&[change]);
    let moved_ancestor = parent.path().with_file_name("held-p1-ancestor");
    let CatalogFixture {
        source,
        _storage,
        catalog,
        root_id,
        ..
    } = fixture;
    let mut repository = RevisionRacingCatalog::new(catalog, Vec::new());
    repository.guarded_rename_during_publication =
        Some((parent.path().to_path_buf(), moved_ancestor.clone()));

    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("publish guarded paired P1 rename");

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&root_id, "old.png")
            .expect("load old P1 location")
            .is_none()
    );
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&root_id, "new.png")
            .expect("load new P1 location")
            .is_some()
    );
    fs::rename(parent.path(), &moved_ancestor)
        .expect("ancestor rename after publication guard drop");
    fs::rename(&moved_ancestor, parent.path()).expect("restore controlled ancestor");
    drop(repository);
    drop(_storage);
    drop(source);
    drop(parent);
}

#[test]
fn p1_revision_rebase_is_bounded_under_sustained_catalog_churn() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("created.png"),
        2,
        2,
        [10, 20, 30],
    );
    let mut change = intent(&fixture.root_id, "created.png", None, 1);
    change.origin = LibraryChangeOrigin::StartupCatchUp;
    fixture.enqueue(&[change]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 3);
    let CatalogFixture {
        source: _source,
        _storage,
        catalog,
        root_id,
        ..
    } = fixture;
    let mut repository = RevisionRacingCatalog::new(catalog, races);

    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("bound P1 revision churn");

    assert_eq!(repository.publication_attempts, 3);
    assert_eq!(report.completed_count, 0);
    assert_eq!(report.retried_count, 1);
    let metrics = repository
        .load_library_change_root_queue_metrics(
            &root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("load bounded churn metrics");
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.leased_count, 0);
    drop(repository);
    drop(_storage);
    drop(_source);
}

#[test]
fn p1_revision_rebase_cancellation_returns_the_owned_lease() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("created.png"),
        2,
        2,
        [10, 20, 30],
    );
    let mut change = intent(&fixture.root_id, "created.png", None, 1);
    change.origin = LibraryChangeOrigin::StartupCatchUp;
    fixture.enqueue(&[change]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let CatalogFixture {
        source: _source,
        _storage,
        catalog,
        root_id,
        ..
    } = fixture;
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut repository = RevisionRacingCatalog::new(catalog, races);
    repository.cancel_after_first_publication = Some(Arc::clone(&cancelled));

    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        cancelled.as_ref(),
    )
    .expect("cancel P1 revision rebase");

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(report.deferred_count, 1);
    let metrics = repository
        .load_library_change_root_queue_metrics(
            &root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("load cancelled rebase metrics");
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.leased_count, 0);
    drop(repository);
    drop(_storage);
    drop(_source);
}

#[test]
fn cancelled_p2_batch_returns_all_leases_in_one_write_transaction() {
    let fixture = seed_catalog(tempdir().expect("source directory"), &[]);
    let CatalogFixture {
        source: _source,
        _storage,
        mut catalog,
        root_id,
        ..
    } = fixture;
    let queue_policy = LibraryChangeQueuePolicy {
        max_lease_batch: 64,
        ..LibraryChangeQueuePolicy::default()
    };
    let changes = (0..64)
        .map(|index| {
            let mut change = intent(&root_id, &format!("cancelled-{index}.png"), None, index + 1);
            change.origin = LibraryChangeOrigin::MetadataInventory;
            change
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&changes, 1_000, queue_policy)
        .expect("enqueue P2 batch");
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut repository = RevisionRacingCatalog::new(catalog, Vec::new());
    repository.cancel_after_leasing = Some(Arc::clone(&cancelled));
    let epoch = repository.catalog.completed_write_epoch();
    let report = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Recovery,
        2_000,
        queue_policy,
        &cancelled,
    )
    .expect("cancel the leased recovery batch");
    assert_eq!(report.leased_count, 64);
    assert_eq!(report.deferred_count, 64);
    assert_eq!(report.completed_count, 0);
    assert_eq!(
        repository.catalog.completed_write_epoch() - epoch,
        2,
        "one lease transaction and one atomic return, independent of batch cardinality"
    );
    let metrics = repository
        .catalog
        .load_library_change_root_queue_metrics(
            &root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy,
        )
        .expect("returned queue metrics");
    assert_eq!(metrics.pending_count, 64);
    assert_eq!(metrics.leased_count, 0);
    cancelled.store(false, Ordering::Release);
    repository.cancel_after_leasing = None;
    let resumed = process_ready_library_changes_in_lane_cancellable(
        &mut repository,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Recovery,
        2_000,
        queue_policy,
        &cancelled,
    )
    .expect("resume the same recovery batch");
    assert_eq!(resumed.completed_count, 64);
    drop(repository);
    drop(_storage);
    drop(_source);
}
use crate::journal_broker::target_translation_recovery_fixture;
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector,
    PersistentJournalRepository,
};

#[cfg(windows)]
#[test]
fn empty_path_queue_opens_no_source_root() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    reset_configured_root_open_instrumentation(&fixture.root_path);
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let report = fixture.process();

    assert_eq!(report.leased_count, 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, true), 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, false), 0);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
}

#[cfg(windows)]
#[test]
fn missing_v29_proof_retries_leased_work_before_any_source_root_open() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    fixture.enqueue(&[intent(&fixture.root_id, "created.png", None, 1)]);
    let connection = Connection::open(fixture.catalog.catalog_path()).expect("proof fixture");
    connection
        .execute(
            "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
            [&fixture.root_id],
        )
        .expect("remove v29 publication proof");
    drop(connection);
    reset_configured_root_open_instrumentation(&fixture.root_path);
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let report = fixture.process();

    assert_eq!(report.leased_count, 1);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, true), 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, false), 0);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
}

#[cfg(windows)]
#[test]
fn replacement_root_uses_only_the_proof_guard_and_enumerates_nothing() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    fixture.enqueue(&[intent(&fixture.root_id, "replacement.png", None, 1)]);
    let original = fixture.source.path().to_path_buf();
    let retained = original.with_extension("incremental-proof-root");
    fs::rename(&original, &retained).expect("retain proven root");
    fs::create_dir(&original).expect("create replacement root");
    write_png(&original.join("replacement.png"), 2, 2, [10, 20, 30]);
    reset_configured_root_open_instrumentation(&fixture.root_path);
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let report = fixture.process();

    fs::remove_dir_all(&original).expect("remove replacement root");
    fs::rename(&retained, &original).expect("restore proven root");
    assert_eq!(report.leased_count, 1);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, true), 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, false), 1);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    assert!(fixture.location("replacement.png").is_none());
}

use super::{
    PreparationCatalog, prepare_change, process_ready_library_changes,
    process_ready_library_changes_in_lane_cancellable, stable_id, stable_location_id,
    user_visible_path,
};

#[cfg(windows)]
#[test]
fn live_equal_length_rewrite_with_restored_mtime_supersedes_the_ready_preview() {
    assert_equal_length_rewrite_supersedes_ready_preview(
        LibraryChangeOrigin::LiveNotification,
        "live-equal-length-preview",
    );
}

#[cfg(windows)]
#[test]
fn startup_catch_up_equal_length_rewrite_with_restored_mtime_supersedes_the_ready_preview() {
    assert_equal_length_rewrite_supersedes_ready_preview(
        LibraryChangeOrigin::StartupCatchUp,
        "catch-up-equal-length-preview",
    );
}

#[cfg(windows)]
#[test]
fn metadata_inventory_equal_evidence_invalidates_preview_without_opening_content() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("dirty.png"), 4, 3, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["dirty.png"]);
    let ready = publish_ready_preview(&mut fixture, "dirty.png", "metadata-dirty-ready-preview");
    let revision = fixture.revision();
    reset_source_content_open_instrumentation(&fixture.root_path);
    let mut change = intent(&fixture.root_id, "dirty.png", None, 1);
    change.origin = LibraryChangeOrigin::MetadataInventory;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    let after = fixture.location("dirty.png").expect("dirty location");
    assert_eq!(after.asset_id, ready.location.asset_id);
    assert_eq!(after.file_size, ready.location.file_size);
    assert_eq!(after.modified_unix_ms, ready.location.modified_unix_ms);
    assert_eq!(after.file_identity, ready.location.file_identity);
    assert_eq!(after.source_revision, ready.location.source_revision);
    assert_ne!(after.source_generation, ready.location.source_generation);
    assert_eq!(
        (after.width, after.height),
        (ready.location.width, ready.location.height)
    );
    assert_eq!(
        after.metadata_engine_id,
        super::super::INVALIDATED_MEDIA_METADATA_ENGINE_ID
    );
    assert_eq!(
        after.metadata_engine_version,
        super::super::INVALIDATED_MEDIA_METADATA_ENGINE_VERSION
    );
    assert!(after.capture_time.is_none());
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert!(after.preview_issue_code.is_none());
    assert!(after.preview_issue_message.is_none());
    assert_ready_preview_superseded(&mut fixture, &ready);
}

#[cfg(windows)]
#[test]
fn metadata_inventory_equal_wrong_extension_skips_magic_content_read() {
    let source = tempdir().expect("source directory");
    write_png_with_format(&source.path().join("illustration.data"), 5, 3, [31, 41, 59]);
    let mut fixture = seed_catalog(source, &["illustration.data"]);
    let before = fixture
        .location("illustration.data")
        .expect("wrong-extension location");
    reset_source_content_open_instrumentation(&fixture.root_path);
    let mut change = intent(&fixture.root_id, "illustration.data", None, 1);
    change.origin = LibraryChangeOrigin::MetadataInventory;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    let after = fixture
        .location("illustration.data")
        .expect("invalidated wrong-extension location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.source_generation, before.source_generation);
    assert_eq!((after.width, after.height), (before.width, before.height));
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
}

#[cfg(windows)]
#[test]
fn metadata_inventory_changed_evidence_falls_back_to_full_inspection() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("changed.png"), 2, 2, [26, 53, 82]);
    let mut fixture = seed_catalog(source, &["changed.png"]);
    let before = fixture.location("changed.png").expect("prior location");
    write_png(
        &fixture.source.path().join("changed.png"),
        7,
        4,
        [97, 93, 29],
    );
    reset_source_content_open_instrumentation(&fixture.root_path);
    let mut change = intent(&fixture.root_id, "changed.png", None, 1);
    change.origin = LibraryChangeOrigin::MetadataInventory;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(source_content_open_count(&fixture.root_path), 1);
    let after = fixture.location("changed.png").expect("changed location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.source_generation, before.source_generation);
    assert_eq!((after.width, after.height), (7, 4));
}

#[cfg(windows)]
#[test]
fn metadata_inventory_engine_upgrade_falls_back_to_full_inspection() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("upgrade.png"), 3, 2, [38, 46, 54]);
    let mut fixture =
        seed_catalog_with_metadata(source, &["upgrade.png"], Some(("legacy-metadata", "0")));
    reset_source_content_open_instrumentation(&fixture.root_path);
    let mut change = intent(&fixture.root_id, "upgrade.png", None, 1);
    change.origin = LibraryChangeOrigin::MetadataInventory;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(source_content_open_count(&fixture.root_path), 1);
    let after = fixture.location("upgrade.png").expect("upgraded location");
    assert_ne!(after.metadata_engine_id, "legacy-metadata");
    assert_ne!(after.metadata_engine_version, "0");
    assert_eq!((after.width, after.height), (3, 2));
}

#[cfg(windows)]
#[test]
fn metadata_inventory_equal_hard_link_invalidates_every_owner_without_content_read() {
    let source = tempdir().expect("source directory");
    let primary_path = source.path().join("primary.png");
    let alias_path = source.path().join("alias.png");
    write_png(&primary_path, 3, 2, [50, 100, 150]);
    fs::hard_link(&primary_path, &alias_path).expect("create source hard link");
    let mut fixture = seed_catalog(source, &["primary.png", "alias.png"]);
    let primary_ready = publish_ready_preview_at_edge(
        &mut fixture,
        "primary.png",
        "metadata-hard-link-primary-preview",
        256,
    );
    let alias_ready = publish_ready_preview_at_edge(
        &mut fixture,
        "alias.png",
        "metadata-hard-link-alias-preview",
        512,
    );
    assert_eq!(
        preview_owner_count(&fixture.catalog, &primary_ready.artifact.artifact_key),
        1
    );
    assert_eq!(
        preview_owner_count(&fixture.catalog, &alias_ready.artifact.artifact_key),
        1
    );
    reset_source_content_open_instrumentation(&fixture.root_path);
    let mut change = intent(&fixture.root_id, "primary.png", None, 1);
    change.origin = LibraryChangeOrigin::MetadataInventory;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    let primary = fixture.location("primary.png").expect("updated primary");
    let alias = fixture.location("alias.png").expect("updated alias");
    assert_eq!(primary.source_generation, alias.source_generation);
    assert_ne!(
        primary.source_generation,
        primary_ready.location.source_generation
    );
    for location in [&primary, &alias] {
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
        assert!(location.preview_issue_code.is_none());
        assert!(location.preview_issue_message.is_none());
    }
    assert_ready_preview_superseded(&mut fixture, &primary_ready);
    assert_ready_preview_superseded(&mut fixture, &alias_ready);
}

#[test]
fn created_file_is_added_in_one_incremental_revision() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let revision = fixture.revision();
    write_png(
        &fixture.source.path().join("created.png"),
        3,
        2,
        [40, 50, 60],
    );
    let source_bytes = fs::read(fixture.source.path().join("created.png")).expect("source bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "created.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let stored = fixture.location("created.png").expect("created location");
    assert_eq!((stored.width, stored.height), (3, 2));
    assert_eq!(
        fs::read(fixture.source.path().join("created.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[test]
fn cancelled_journal_drain_yields_before_leasing_or_reading_source_files() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("cancelled.png"),
        3,
        2,
        [41, 51, 61],
    );
    fixture.enqueue(&[catch_up_intent(&fixture.root_id, "cancelled.png", 1)]);
    let cancelled = AtomicBool::new(true);

    let stopped = process_ready_library_changes_in_lane_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        &cancelled,
    )
    .expect("cancel journal drain at its first boundary");

    assert_eq!(stopped.leased_count, 0);
    assert_eq!(stopped.completed_count, 0);
    assert!(fixture.location("cancelled.png").is_none());
    cancelled.store(false, Ordering::Release);
    let resumed = process_ready_library_changes_in_lane_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_001,
        policy(),
        &cancelled,
    )
    .expect("resume the same durable journal work");
    assert_eq!(resumed.completed_count, 1);
    assert!(fixture.location("cancelled.png").is_some());
}

#[test]
fn related_valid_files_publish_together_at_one_catalog_revision() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let revision = fixture.revision();
    write_png(&fixture.source.path().join("first.png"), 2, 3, [61, 62, 63]);
    write_png(
        &fixture.source.path().join("second.png"),
        4,
        2,
        [64, 65, 66],
    );
    fixture.enqueue(&[
        intent(&fixture.root_id, "first.png", None, 1),
        intent(&fixture.root_id, "second.png", None, 2),
    ]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 2);
    assert_eq!(report.applied_mutation_count, 2);
    assert_eq!(report.catalog_revision, revision + 1);
    assert_eq!(fixture.revision(), revision + 1);
    assert!(fixture.location("first.png").is_some());
    assert!(fixture.location("second.png").is_some());
}

#[test]
fn modified_file_retains_asset_identity_and_invalidates_derived_preview_state() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("modified.png"), 2, 2, [70, 80, 90]);
    let mut fixture = seed_catalog(source, &["modified.png"]);
    let before = fixture.location("modified.png").expect("original location");
    let revision = fixture.revision();
    write_png(
        &fixture.source.path().join("modified.png"),
        5,
        3,
        [90, 80, 70],
    );
    let source_bytes = fs::read(fixture.source.path().join("modified.png")).expect("source bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "modified.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let after = fixture.location("modified.png").expect("modified location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_eq!((after.width, after.height), (5, 3));
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert_eq!(
        fs::read(fixture.source.path().join("modified.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[test]
fn unchanged_source_is_reinspected_when_metadata_engine_identity_changes() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("metadata.png"), 3, 2, [91, 92, 93]);
    let source_bytes = fs::read(source.path().join("metadata.png")).expect("source bytes");
    let mut fixture =
        seed_catalog_with_metadata(source, &["metadata.png"], Some(("legacy-metadata", "0")));
    let before = fixture.location("metadata.png").expect("original location");
    let revision = fixture.revision();
    fixture.enqueue(&[intent(&fixture.root_id, "metadata.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let after = fixture.location("metadata.png").expect("updated location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.metadata_engine_id, "legacy-metadata");
    assert_ne!(after.metadata_engine_version, "0");
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert_eq!(
        fs::read(fixture.source.path().join("metadata.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[cfg(windows)]
#[test]
fn paired_rename_preserves_the_asset_and_moves_the_location() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 4, 3, [100, 110, 120]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let before = fixture.location("old.png").expect("original location");
    let revision = fixture.revision();
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source fixture");
    let source_bytes = fs::read(fixture.source.path().join("new.png")).expect("renamed bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("old.png").is_none());
    let after = fixture.location("new.png").expect("renamed location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.location_id, before.location_id);
    assert_eq!(after.file_identity, before.file_identity);
    assert_eq!(
        fs::read(fixture.source.path().join("new.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[cfg(windows)]
#[test]
fn paired_rename_followed_by_removal_drops_the_obsolete_old_location() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 4, 3, [121, 122, 123]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let revision = fixture.revision();
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source fixture");
    fs::remove_file(fixture.source.path().join("new.png")).expect("remove renamed fixture");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("old.png").is_none());
    assert!(fixture.location("new.png").is_none());
}

#[cfg(windows)]
#[test]
fn delete_recreate_at_the_same_path_supersedes_the_ready_preview_source() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("replace.png"), 2, 2, [130, 140, 150]);
    let mut fixture = seed_catalog(source, &["replace.png"]);
    let ready = publish_ready_preview(&mut fixture, "replace.png", "delete-recreate-ready-preview");
    fs::remove_file(fixture.source.path().join("replace.png")).expect("remove original fixture");
    write_png(
        &fixture.source.path().join("replace.png"),
        6,
        2,
        [150, 140, 130],
    );
    let replacement = fixture.discovered("replace.png");
    let replacement_bytes =
        fs::read(fixture.source.path().join("replace.png")).expect("replacement bytes");
    assert_ne!(replacement.file_identity, ready.location.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "replace.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    let after = fixture
        .location("replace.png")
        .expect("replacement location");
    assert_ne!(after.asset_id, ready.location.asset_id);
    assert_eq!(after.file_identity, replacement.file_identity);
    assert_eq!(after.source_revision, replacement.source_revision);
    assert_ne!(after.source_generation, ready.location.source_generation);
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert_ready_preview_superseded(&mut fixture, &ready);
    assert_eq!(
        fs::read(fixture.source.path().join("replace.png")).expect("replacement bytes after delta"),
        replacement_bytes
    );
}

#[cfg(windows)]
#[test]
fn atomic_replace_at_the_same_path_supersedes_the_ready_preview_source() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("replace.png");
    let replacement_path = source.path().join("replacement-staging.png");
    write_png(&source_path, 2, 2, [13, 21, 34]);
    let mut fixture = seed_catalog(source, &["replace.png"]);
    let ready = publish_ready_preview(&mut fixture, "replace.png", "atomic-replace-preview");
    write_png(&replacement_path, 6, 3, [55, 89, 144]);
    let replacement_identity = FileDiscovery::new(&fixture.root_path)
        .expect("replacement discovery")
        .visit_relative_path("replacement-staging.png");
    let FileVisitOutcome::File(replacement) = replacement_identity.outcome else {
        panic!("expected replacement source file");
    };
    assert_ne!(replacement.file_identity, ready.location.file_identity);

    crate::adapters::replace_file_atomically_for_test(&replacement_path, &source_path)
        .expect("atomically replace source fixture");
    assert!(!replacement_path.exists());
    let replaced = fixture.discovered("replace.png");
    let replaced_bytes = fs::read(&source_path).expect("atomically replaced source bytes");
    assert_eq!(replaced.file_identity, replacement.file_identity);
    assert_ne!(replaced.file_identity, ready.location.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "replace.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    let after = fixture
        .location("replace.png")
        .expect("replacement location");
    assert_ne!(after.asset_id, ready.location.asset_id);
    assert_eq!(after.file_identity, replaced.file_identity);
    assert_eq!(after.source_revision, replaced.source_revision);
    assert_ne!(after.source_generation, ready.location.source_generation);
    assert_eq!((after.width, after.height), (6, 3));
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert_ready_preview_superseded(&mut fixture, &ready);
    assert_eq!(
        fs::read(&source_path).expect("source bytes after atomic replacement delta"),
        replaced_bytes,
    );
}

#[cfg(windows)]
#[test]
fn hard_link_write_fans_out_source_generation_and_invalidates_every_ready_owner() {
    let source = tempdir().expect("source directory");
    let primary_path = source.path().join("primary.png");
    let alias_path = source.path().join("alias.png");
    write_png(&primary_path, 3, 2, [21, 34, 55]);
    fs::hard_link(&primary_path, &alias_path).expect("create source hard link");
    let mut fixture = seed_catalog(source, &["primary.png", "alias.png"]);
    let primary_ready =
        publish_ready_preview(&mut fixture, "primary.png", "shared-hard-link-preview");
    let alias_ready = publish_ready_preview(&mut fixture, "alias.png", "shared-hard-link-preview");
    assert_eq!(
        primary_ready.location.file_identity,
        alias_ready.location.file_identity
    );
    assert_eq!(
        primary_ready.location.source_generation,
        alias_ready.location.source_generation
    );
    assert_eq!(
        preview_owner_count(&fixture.catalog, &primary_ready.artifact.artifact_key),
        2
    );

    write_png(&primary_path, 5, 4, [89, 144, 233]);
    let rewritten = fixture.discovered("primary.png");
    let rewritten_bytes = fs::read(&primary_path).expect("rewritten hard-link bytes");
    assert_eq!(
        fs::read(&alias_path).expect("rewritten alias bytes"),
        rewritten_bytes
    );
    fixture.enqueue(&[intent(&fixture.root_id, "primary.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    let primary = fixture.location("primary.png").expect("updated primary");
    let alias = fixture.location("alias.png").expect("updated alias");
    assert_eq!(primary.file_identity, primary_ready.location.file_identity);
    assert_eq!(alias.file_identity, primary_ready.location.file_identity);
    assert_eq!(primary.source_revision, rewritten.source_revision);
    assert_eq!(alias.source_revision, rewritten.source_revision);
    assert_eq!(primary.source_generation, alias.source_generation);
    assert_ne!(
        primary.source_generation,
        primary_ready.location.source_generation
    );
    assert_eq!((primary.width, primary.height), (5, 4));
    assert_eq!((alias.width, alias.height), (5, 4));
    for location in [&primary, &alias] {
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
    }
    assert_ready_preview_superseded(&mut fixture, &primary_ready);
    assert_ready_preview_superseded(&mut fixture, &alias_ready);
    assert_eq!(
        fs::read(&primary_path).expect("hard-link bytes after delta"),
        rewritten_bytes
    );
}

#[cfg(windows)]
#[test]
fn ready_preview_survives_no_stale_publication_across_zero_truncated_and_valid_rewrites() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("recover.png");
    write_png(&source_path, 4, 3, [8, 13, 21]);
    let mut fixture = seed_catalog(source, &["recover.png"]);
    let ready = publish_ready_preview(&mut fixture, "recover.png", "zero-truncated-ready-preview");

    fs::write(&source_path, b"").expect("truncate source to zero bytes");
    let zero_source = fixture.discovered("recover.png");
    assert_eq!(zero_source.file_identity, ready.location.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "recover.png", None, 1)]);
    let zero_report = fixture.process();

    assert_eq!(zero_report.completed_count, 1, "{zero_report:?}");
    assert_eq!(zero_report.applied_mutation_count, 1);
    let zero = fixture
        .location("recover.png")
        .expect("zero-byte terminal state");
    assert_eq!(zero.asset_id, ready.location.asset_id);
    assert_eq!(zero.file_identity, ready.location.file_identity);
    assert_eq!(zero.source_revision, zero_source.source_revision);
    assert_ne!(zero.source_generation, ready.location.source_generation);
    assert!(matches!(zero.preview_status, PreviewStatus::Failed));
    assert!(zero.preview_path.is_empty());
    assert!(zero.preview_issue_code.is_some());
    assert!(
        fs::read(&source_path)
            .expect("zero bytes after delta")
            .is_empty()
    );

    fs::write(&source_path, b"\x89PNG\r\n\x1a\ntruncated").expect("write truncated PNG source");
    let truncated_source = fixture.discovered("recover.png");
    let truncated_bytes = fs::read(&source_path).expect("truncated source bytes");
    assert_eq!(truncated_source.file_identity, ready.location.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "recover.png", None, 2)]);
    let truncated_report = fixture.process();

    assert_eq!(truncated_report.completed_count, 1, "{truncated_report:?}");
    assert_eq!(truncated_report.applied_mutation_count, 1);
    let truncated = fixture
        .location("recover.png")
        .expect("truncated terminal state");
    assert_eq!(truncated.asset_id, ready.location.asset_id);
    assert_eq!(truncated.file_identity, ready.location.file_identity);
    assert_eq!(truncated.source_revision, truncated_source.source_revision);
    assert_ne!(truncated.source_generation, zero.source_generation);
    assert!(matches!(truncated.preview_status, PreviewStatus::Failed));
    assert!(truncated.preview_path.is_empty());
    assert!(truncated.preview_issue_code.is_some());
    assert_eq!(
        fs::read(&source_path).expect("truncated bytes after delta"),
        truncated_bytes
    );

    write_png(&source_path, 6, 5, [34, 55, 89]);
    let recovered_source = fixture.discovered("recover.png");
    let recovered_bytes = fs::read(&source_path).expect("recovered source bytes");
    assert_eq!(recovered_source.file_identity, ready.location.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "recover.png", None, 3)]);
    let recovered_report = fixture.process();

    assert_eq!(recovered_report.completed_count, 1, "{recovered_report:?}");
    assert_eq!(recovered_report.applied_mutation_count, 1);
    let recovered = fixture.location("recover.png").expect("recovered location");
    assert_eq!(recovered.asset_id, ready.location.asset_id);
    assert_eq!(recovered.file_identity, ready.location.file_identity);
    assert_eq!(recovered.source_revision, recovered_source.source_revision);
    assert_ne!(recovered.source_generation, truncated.source_generation);
    assert_eq!((recovered.width, recovered.height), (6, 5));
    assert!(matches!(recovered.preview_status, PreviewStatus::Pending));
    assert!(recovered.preview_path.is_empty());
    assert!(recovered.preview_issue_code.is_none());
    assert_ready_preview_superseded(&mut fixture, &ready);
    assert_eq!(
        fs::read(&source_path).expect("recovered bytes after delta"),
        recovered_bytes
    );
}

#[cfg(windows)]
#[test]
fn disappeared_ready_source_reappears_without_accepting_its_old_preview_request() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("reappear.png");
    write_png(&source_path, 2, 3, [144, 89, 55]);
    let mut fixture = seed_catalog(source, &["reappear.png"]);
    let ready = publish_ready_preview(
        &mut fixture,
        "reappear.png",
        "disappear-reappear-ready-preview",
    );

    fs::remove_file(&source_path).expect("remove ready source");
    fixture.enqueue(&[intent(&fixture.root_id, "reappear.png", None, 1)]);
    let removal = fixture.process();

    assert_eq!(removal.completed_count, 1, "{removal:?}");
    assert_eq!(removal.applied_mutation_count, 1);
    assert!(fixture.location("reappear.png").is_none());
    assert_eq!(
        preview_owner_count(&fixture.catalog, &ready.artifact.artifact_key),
        0
    );

    write_png(&source_path, 7, 4, [233, 144, 89]);
    let reappeared_source = fixture.discovered("reappear.png");
    let reappeared_bytes = fs::read(&source_path).expect("reappeared source bytes");
    assert_ne!(
        reappeared_source.file_identity,
        ready.location.file_identity
    );
    fixture.enqueue(&[intent(&fixture.root_id, "reappear.png", None, 2)]);
    let reappearance = fixture.process();

    assert_eq!(reappearance.completed_count, 1, "{reappearance:?}");
    assert_eq!(reappearance.applied_mutation_count, 1);
    let reappeared = fixture
        .location("reappear.png")
        .expect("reappeared location");
    assert_ne!(reappeared.asset_id, ready.location.asset_id);
    assert_eq!(reappeared.file_identity, reappeared_source.file_identity);
    assert_eq!(
        reappeared.source_revision,
        reappeared_source.source_revision
    );
    assert_ne!(
        reappeared.source_generation,
        ready.location.source_generation
    );
    assert_eq!((reappeared.width, reappeared.height), (7, 4));
    assert!(matches!(reappeared.preview_status, PreviewStatus::Pending));
    assert!(reappeared.preview_path.is_empty());
    assert_ready_preview_superseded(&mut fixture, &ready);
    assert_eq!(
        fs::read(&source_path).expect("reappeared bytes after delta"),
        reappeared_bytes
    );
}

#[cfg(windows)]
#[test]
fn dirty_rename_invalidates_preview_generation_and_removes_the_previous_path() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 2, 2, [31, 41, 59]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let before = fixture.location("old.png").expect("original location");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source fixture");
    fixture.enqueue(&[
        intent(&fixture.root_id, "new.png", Some("old.png"), 1),
        intent(&fixture.root_id, "new.png", None, 2),
    ]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert!(fixture.location("old.png").is_none());
    let after = fixture.location("new.png").expect("renamed location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_eq!(after.file_identity, before.file_identity);
    assert_ne!(after.source_generation, before.source_generation);
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
}

#[test]
fn authoritative_absence_removes_the_location_from_the_current_projection() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("removed.png"), 2, 3, [160, 170, 180]);
    let mut fixture = seed_catalog(source, &["removed.png"]);
    let revision = fixture.revision();
    fs::remove_file(fixture.source.path().join("removed.png")).expect("remove source fixture");
    fixture.enqueue(&[intent(&fixture.root_id, "removed.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("removed.png").is_none());
}

#[test]
fn malformed_image_completes_once_without_blocking_a_valid_sibling() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("valid.png"),
        3,
        4,
        [190, 200, 210],
    );
    fs::write(fixture.source.path().join("broken.jpg"), b"not a jpeg")
        .expect("malformed image fixture");
    let valid_bytes = fs::read(fixture.source.path().join("valid.png")).expect("valid bytes");
    let broken_bytes = fs::read(fixture.source.path().join("broken.jpg")).expect("broken bytes");
    fixture.enqueue(&[
        intent(&fixture.root_id, "valid.png", None, 1),
        intent(&fixture.root_id, "broken.jpg", None, 2),
    ]);

    let report = fixture.process();

    assert_eq!(report.leased_count, 2);
    assert_eq!(report.completed_count, 2);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 2);
    assert!(fixture.location("valid.png").is_some());
    let failed = fixture
        .location("broken.jpg")
        .expect("terminal image placeholder");
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert_eq!((failed.width, failed.height), (0, 0));
    assert!(failed.preview_path.is_empty());
    assert_eq!(
        failed.preview_issue_code.as_deref(),
        Some("image_format_unsupported")
    );
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.completed_count, 2);
    assert_eq!(metrics.retry_wait_count, 0);
    let evidence = fixture
        .catalog
        .load_terminal_media_evidence_by_relative_paths(
            &fixture.root_id,
            &["broken.jpg".to_owned()],
        )
        .expect("load terminal media evidence");
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].issue.code, "image_format_unsupported");
    assert_eq!(
        fs::read(fixture.source.path().join("valid.png")).expect("valid bytes after delta"),
        valid_bytes
    );
    assert_eq!(
        fs::read(fixture.source.path().join("broken.jpg")).expect("broken bytes after delta"),
        broken_bytes
    );

    write_png(
        &fixture.source.path().join("broken.jpg"),
        2,
        3,
        [220, 221, 222],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "broken.jpg", None, 3)]);

    let recovered = fixture.process();

    assert_eq!(recovered.completed_count, 1);
    assert_eq!(recovered.applied_mutation_count, 1);
    let repaired = fixture.location("broken.jpg").expect("repaired image");
    assert_eq!(repaired.asset_id, failed.asset_id);
    assert_ne!(repaired.source_generation, failed.source_generation);
    assert_eq!((repaired.width, repaired.height), (2, 3));
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(
                &fixture.root_id,
                &["broken.jpg".to_owned()],
            )
            .expect("reload terminal media evidence")
            .is_empty()
    );
}

#[cfg(windows)]
#[test]
fn valid_image_can_become_corrupt_and_recover_at_the_same_path() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("mutable.png"), 4, 3, [30, 40, 50]);
    let mut fixture = seed_catalog(source, &["mutable.png"]);
    let original = fixture.location("mutable.png").expect("original image");

    fs::write(
        fixture.source.path().join("mutable.png"),
        b"corrupt image bytes",
    )
    .expect("corrupt source in place");
    let corrupt_source = fixture.discovered("mutable.png");
    assert_eq!(corrupt_source.file_identity, original.file_identity);
    assert_ne!(corrupt_source.source_revision, original.source_revision);
    let corrupt_bytes =
        fs::read(fixture.source.path().join("mutable.png")).expect("corrupt source bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "mutable.png", None, 1)]);

    let failed_report = fixture.process();

    assert_eq!(failed_report.completed_count, 1);
    assert_eq!(failed_report.applied_mutation_count, 1);
    let failed = fixture
        .location("mutable.png")
        .expect("failed image placeholder");
    assert_eq!(failed.asset_id, original.asset_id);
    assert_eq!(failed.file_identity, original.file_identity);
    assert_ne!(failed.source_generation, original.source_generation);
    assert_eq!(failed.source_revision, corrupt_source.source_revision);
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert_eq!((failed.width, failed.height), (0, 0));
    assert_eq!(
        fs::read(fixture.source.path().join("mutable.png")).expect("corrupt bytes after update"),
        corrupt_bytes
    );

    write_png(
        &fixture.source.path().join("mutable.png"),
        5,
        2,
        [60, 70, 80],
    );
    let repaired_source = fixture.discovered("mutable.png");
    let repaired_bytes =
        fs::read(fixture.source.path().join("mutable.png")).expect("repaired source bytes");
    assert_eq!(repaired_source.file_identity, original.file_identity);
    assert_ne!(
        repaired_source.source_revision,
        corrupt_source.source_revision
    );
    fixture.enqueue(&[intent(&fixture.root_id, "mutable.png", None, 2)]);

    let repaired_report = fixture.process();

    assert_eq!(repaired_report.completed_count, 1);
    assert_eq!(repaired_report.applied_mutation_count, 1);
    let repaired = fixture.location("mutable.png").expect("repaired image");
    assert_eq!(repaired.asset_id, original.asset_id);
    assert_eq!(repaired.file_identity, original.file_identity);
    assert_ne!(repaired.source_generation, failed.source_generation);
    assert_eq!(repaired.source_revision, repaired_source.source_revision);
    assert_eq!((repaired.width, repaired.height), (5, 2));
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert!(repaired.preview_issue_code.is_none());
    assert!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(
                &fixture.root_id,
                &["mutable.png".to_owned()],
            )
            .expect("repaired terminal evidence")
            .is_empty()
    );
    assert_eq!(
        fs::read(fixture.source.path().join("mutable.png")).expect("repaired bytes after update"),
        repaired_bytes
    );
}

#[cfg(windows)]
#[test]
fn locked_wrong_extension_retries_without_removing_trustworthy_catalog_state() {
    let source = tempdir().expect("source directory");
    write_png_with_format(&source.path().join("image.data"), 2, 2, [31, 32, 33]);
    let mut fixture = seed_catalog(source, &["image.data"]);
    let before = fixture.location("image.data").expect("original location");
    write_png_with_format(
        &fixture.source.path().join("image.data"),
        4,
        3,
        [41, 42, 43],
    );
    let _exclusive_lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(fixture.source.path().join("image.data"))
        .expect("exclusive fixture lock");
    fixture.enqueue(&[intent(&fixture.root_id, "image.data", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 0);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.applied_mutation_count, 0);
    let retained = fixture
        .location("image.data")
        .expect("last trustworthy location");
    assert_eq!(retained.location_id, before.location_id);
    assert_eq!(retained.asset_id, before.asset_id);
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.retry_wait_count, 1);
}

#[test]
fn pending_live_work_publishes_into_active_and_running_scan() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("waiting.png"),
        3,
        3,
        [211, 212, 213],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "waiting.png", None, 1)]);
    let replacement_scan = ScanRequest {
        scan_id: "replacement-scan".to_owned(),
        root_path: fixture.root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    fixture
        .catalog
        .begin_scan(&replacement_scan, &fixture.root_id, &fixture.root_path)
        .expect("begin replacement scan");

    let report = process_ready_library_changes_in_lane(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        2_000,
        policy(),
    )
    .expect("process live changes through running scan");

    assert_eq!(report.leased_count, 1);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 1);
    let active = fixture
        .location("waiting.png")
        .expect("live location published to active scan");
    let identity = active
        .file_identity
        .as_ref()
        .expect("live location stable identity");
    let staged = fixture
        .catalog
        .load_scan_location_by_file_identity(&replacement_scan.scan_id, identity)
        .expect("load mirrored replacement location")
        .expect("live location mirrored to replacement scan");
    assert_eq!(staged.relative_path, "waiting.png");
    assert_eq!(staged.source_generation, active.source_generation);
    assert_eq!(staged.source_revision, active.source_revision);
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.completed_count, 1);

    fixture
        .catalog
        .publish_scan(&replacement_scan.scan_id, &fixture.root_id, 1, 0)
        .expect("publish replacement scan after live queue drains");
    let published = fixture
        .location("waiting.png")
        .expect("live location survives replacement publication");
    assert_eq!(published.source_generation, active.source_generation);
    assert_eq!(published.source_revision, active.source_revision);
}

#[test]
fn authoritative_work_remains_pending_without_consuming_the_incremental_retry_budget() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let strict_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_attempts: 1,
        ..LibraryChangeQueuePolicy::default()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                ..intent(&fixture.root_id, "placeholder.png", None, 1)
            }],
            1_000,
            strict_policy,
        )
        .expect("enqueue authoritative work");

    let report = fixture.process_with_policy(strict_policy);

    assert_eq!(report.leased_count, 0);
    assert_eq!(report.retried_count, 0);
    let leased = fixture
        .catalog
        .lease_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            strict_policy,
        )
        .expect("authoritative worker can lease pending work");
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.attempt_count, 1);
}

#[cfg(windows)]
#[test]
fn unavailable_root_is_leased_then_durably_retried_without_source_inspection() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let strict_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_attempts: 1,
        ..LibraryChangeQueuePolicy::default()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[intent(&fixture.root_id, "waiting.png", None, 1)],
            1_000,
            strict_policy,
        )
        .expect("enqueue path work");
    fs::remove_dir_all(fixture.source.path()).expect("make root unavailable");
    reset_configured_root_open_instrumentation(&fixture.root_path);
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let report = fixture.process_with_policy(strict_policy);

    assert_eq!(report.leased_count, 1);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, true), 0);
    assert_eq!(configured_root_open_count(&fixture.root_path, false), 0);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            strict_policy,
        )
        .expect("load unavailable-root retry metrics");
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("root_publication_namespace_unavailable")
    );
}

#[cfg(windows)]
#[test]
fn identity_backfill_preserves_asset_continuity_for_a_later_rename() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("legacy.png"), 3, 3, [11, 12, 13]);
    let mut fixture = seed_catalog_without_identity(source, &["legacy.png"]);
    let original = fixture.location("legacy.png").expect("legacy location");
    assert!(original.file_identity.is_none());
    fixture.enqueue(&[intent(&fixture.root_id, "legacy.png", None, 1)]);

    let backfill = fixture.process();

    assert_eq!(backfill.applied_mutation_count, 1);
    let identified = fixture.location("legacy.png").expect("identified location");
    assert_eq!(identified.asset_id, original.asset_id);
    assert!(identified.file_identity.is_some());
    fs::rename(
        fixture.source.path().join("legacy.png"),
        fixture.source.path().join("moved.png"),
    )
    .expect("rename identified file");
    fixture.enqueue(&[intent(&fixture.root_id, "moved.png", Some("legacy.png"), 2)]);

    fixture.process();

    let moved = fixture.location("moved.png").expect("moved location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, identified.file_identity);
}

#[cfg(windows)]
#[test]
fn migrated_v17_location_is_preserved_while_unproven_namespace_blocks_backfill() {
    let source = tempdir().expect("source directory");
    fs::create_dir(source.path().join("album")).expect("album directory");
    write_png(
        &source.path().join("album").join("legacy.png"),
        3,
        3,
        [14, 15, 16],
    );
    let fixture = seed_catalog_without_identity(source, &["album/legacy.png"]);
    let original = fixture
        .location("album/legacy.png")
        .expect("legacy location");
    let legacy_location_id = stable_location_id(&fixture.root_id, r"album\legacy.png");
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    let CatalogFixture {
        source,
        _storage,
        catalog,
        root_id,
        root_path,
    } = fixture;
    drop(catalog);
    let connection = Connection::open(&catalog_path).expect("migration fixture catalog");
    remove_persistent_journal_v22_contract_for_test(&connection);
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP INDEX asset_locations_root_relative;
             DROP TABLE library_change_scan_handoff_items;
             DROP TABLE library_change_scan_handoff_lineage;
             DROP TABLE library_change_scan_handoff_batches;
             DROP TABLE library_change_queue_catch_up_lineage;
             DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             DROP TABLE scan_run_catch_up_lineage;
             DROP TABLE library_change_catch_up_handoffs;
             DROP INDEX scan_runs_one_active_root;
             ALTER TABLE library_change_queue DROP COLUMN authoritative_scan_id;
             ALTER TABLE library_change_root_state DROP COLUMN last_consistency_audit_unix_ms;
             ALTER TABLE scan_runs DROP COLUMN requires_previous_snapshot;
             ALTER TABLE scan_runs DROP COLUMN root_generation_at_start;
             ALTER TABLE scan_runs DROP COLUMN change_queue_high_watermark;
             ALTER TABLE scan_runs DROP COLUMN scan_owner;
             ALTER TABLE library_change_queue_contract DROP COLUMN scan_ownership_complete;
             ALTER TABLE library_change_queue_contract DROP COLUMN authoritative_recovery_complete;",
        )
        .expect("restore v17 table shape");
    let updated = connection
        .execute(
            "UPDATE asset_locations
             SET relative_path = 'album\\legacy.png', location_id = ?1
             WHERE root_id = ?2 AND relative_path = ?3",
            [
                legacy_location_id.as_str(),
                root_id.as_str(),
                "album/legacy.png",
            ],
        )
        .expect("restore the v17 path and location identity");
    assert_eq!(updated, 1);
    connection
        .execute_batch(
            "ALTER TABLE library_change_queue_contract DROP COLUMN scan_handoff_batch_complete;
             ALTER TABLE library_change_queue_contract DROP COLUMN scan_catch_up_lineage_complete;
             ALTER TABLE library_change_queue_contract DROP COLUMN change_catch_up_complete;
             DROP TABLE library_change_catch_up_state;
             UPDATE schema_info SET version = 17;",
        )
        .expect("restore v17 version");
    drop(connection);
    let catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate v17 catalog");
    let mut fixture = CatalogFixture {
        source,
        _storage,
        catalog,
        root_id,
        root_path,
    };
    fixture.enqueue(&[intent(&fixture.root_id, "album/legacy.png", None, 1)]);

    let backfill = fixture.process();

    assert_eq!(backfill.applied_mutation_count, 0);
    assert_eq!(backfill.retried_count, 1);
    let identified = fixture
        .location("album/legacy.png")
        .expect("identified location");
    assert_eq!(identified.asset_id, original.asset_id);
    assert_eq!(identified.location_id, legacy_location_id);
    assert!(identified.file_identity.is_none());
    let connection = Connection::open(catalog_path).expect("verified migration fixture catalog");
    let (schema_version, location_count, asset_count, proof_count, failure_code): (
        i64,
        i64,
        i64,
        i64,
        String,
    ) = connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT COUNT(*) FROM asset_locations
                 WHERE root_id = ?1 AND relative_path = 'album/legacy.png'),
               (SELECT asset_count FROM scan_runs WHERE id = 'baseline-scan'),
               (SELECT COUNT(*) FROM library_root_publication_namespaces WHERE root_id = ?1),
               (SELECT last_failure_code FROM library_change_queue
                WHERE root_id = ?1 AND relative_path = 'album/legacy.png')",
            [&fixture.root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("load post-backfill counts");
    assert_eq!(schema_version, 32);
    assert_eq!(location_count, 1);
    assert_eq!(asset_count, 1);
    assert_eq!(proof_count, 0);
    assert_eq!(failure_code, "root_publication_namespace_unproven");
}

#[cfg(windows)]
#[test]
fn paired_rename_reconciles_a_replacement_at_the_previous_path() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 3, 3, [21, 22, 23]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let original = fixture.location("old.png").expect("original location");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename original");
    write_png(&fixture.source.path().join("old.png"), 5, 2, [31, 32, 33]);
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 2);
    let replacement = fixture.location("old.png").expect("replacement location");
    let moved = fixture.location("new.png").expect("moved location");
    assert_ne!(replacement.asset_id, original.asset_id);
    assert_eq!(moved.asset_id, original.asset_id);
    assert_ne!(replacement.file_identity, moved.file_identity);
}

#[cfg(windows)]
#[test]
fn case_only_rename_removes_the_obsolete_catalog_spelling() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("Photo.png"), 3, 3, [34, 35, 36]);
    let mut fixture = seed_catalog(source, &["Photo.png"]);
    let original = fixture.location("Photo.png").expect("original location");
    fs::rename(
        fixture.source.path().join("Photo.png"),
        fixture.source.path().join("rename-intermediate.png"),
    )
    .expect("rename through intermediate");
    fs::rename(
        fixture.source.path().join("rename-intermediate.png"),
        fixture.source.path().join("photo.png"),
    )
    .expect("apply case-only spelling");
    fixture.enqueue(&[intent(&fixture.root_id, "photo.png", Some("Photo.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.applied_mutation_count, 1);
    assert!(fixture.location("Photo.png").is_none());
    let renamed = fixture.location("photo.png").expect("renamed location");
    assert_eq!(renamed.asset_id, original.asset_id);
    assert_eq!(renamed.file_identity, original.file_identity);
}

#[cfg(windows)]
#[test]
fn metadata_engine_mismatch_invalidates_a_rename_mutation_contract() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 3, 2, [41, 42, 43]);
    let mut fixture =
        seed_catalog_with_metadata(source, &["old.png"], Some(("legacy-metadata", "0")));
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);
    let leased = fixture
        .catalog
        .lease_path_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("lease rename")
        .pop()
        .expect("rename lease");
    let expected_identity = FileDiscovery::new(&fixture.root_path)
        .expect("file discovery")
        .metadata_inventory_root_identity()
        .expect("root identity")
        .expect("stable root identity");
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &fixture.root_path,
        &expected_identity,
    )
    .expect("publication-guarded discovery");
    let inspector = LocalMediaInspector::new();

    let prepared = prepare_change(
        &mut PreparationCatalog::new(&fixture.catalog),
        &discovery,
        &inspector,
        &leased,
    )
    .expect("prepare rename");
    let rename = prepared
        .mutations
        .iter()
        .find(|mutation| mutation.upsert_location.is_some())
        .expect("rename mutation");

    assert_eq!(
        rename.evidence_disposition,
        DerivedEvidenceDisposition::InvalidateDerived
    );
    let location = rename.upsert_location.as_ref().expect("renamed location");
    assert!(matches!(location.preview_status, PreviewStatus::Pending));
    assert_ne!(location.metadata_engine_id, "legacy-metadata");
}

#[cfg(windows)]
#[test]
fn compatible_rename_invalidates_failed_preview_source_context() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 2, 3, [51, 52, 53]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let mut failed = fixture.location("old.png").expect("original location");
    failed.preview_status = PreviewStatus::Failed;
    failed.preview_issue_code = Some("preview_decode_failed".to_owned());
    failed.preview_issue_message = Some("fixture failure".to_owned());
    fixture
        .catalog
        .update_active_preview(&failed, None, None)
        .expect("record failed preview");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    fixture.process();

    let renamed = fixture.location("new.png").expect("renamed location");
    assert_eq!(renamed.asset_id, failed.asset_id);
    assert!(matches!(renamed.preview_status, PreviewStatus::Pending));
    assert!(renamed.preview_path.is_empty());
    assert!(renamed.preview_issue_code.is_none());
    assert!(renamed.preview_issue_message.is_none());
    assert_ne!(renamed.source_generation, failed.source_generation);
}

#[cfg(windows)]
#[test]
fn source_first_cross_root_move_preserves_asset_and_invalidates_preview_context() {
    assert_cross_root_move_preserves_continuity(
        "a-source",
        "z-destination",
        true,
        false,
        false,
        false,
    );
}

#[cfg(windows)]
#[test]
fn destination_first_cross_root_move_preserves_asset_and_invalidates_preview_context() {
    assert_cross_root_move_preserves_continuity(
        "z-source",
        "a-destination",
        false,
        false,
        false,
        false,
    );
}

#[cfg(windows)]
#[test]
fn persistent_journal_source_first_move_reaches_final_identity_reconciliation() {
    assert_persistent_journal_cross_root_move_preserves_continuity(true);
}

#[cfg(windows)]
#[test]
fn persistent_journal_destination_first_move_reaches_final_identity_reconciliation() {
    assert_persistent_journal_cross_root_move_preserves_continuity(false);
}

#[cfg(windows)]
#[test]
fn target_translation_recovery_source_first_preserves_final_identity() {
    assert_target_translation_recovery_preserves_continuity(true);
}

#[cfg(windows)]
#[test]
fn target_translation_recovery_destination_first_preserves_final_identity() {
    assert_target_translation_recovery_preserves_continuity(false);
}

#[cfg(windows)]
#[test]
fn persistent_journal_range_stays_catching_up_through_retry_until_terminal() {
    let source = tempdir().expect("persistent journal retry source");
    write_png_with_format(&source.path().join("locked.data"), 2, 2, [31, 32, 33]);
    let mut fixture = seed_catalog(source, &["locked.data"]);
    write_png_with_format(
        &fixture.source.path().join("locked.data"),
        4,
        3,
        [41, 42, 43],
    );
    fixture
        .catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::CatchingUp,
            failure: None,
            updated_unix_ms: 500,
        })
        .expect("support retry journal root");
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: "journal-retry-volume".to_owned(),
        volume_serial: 91,
    };
    fixture
        .catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume.clone(),
            root_file_reference: JournalFileReference::V3([9; 16]),
            journal_id: JournalIdentifier::new(88).expect("journal"),
            next_unread_usn: JournalUsn::new(10).expect("baseline"),
            captured_exclusive_end: JournalUsn::new(10).expect("baseline end"),
            covered_catalog_revision: 0,
            protocol_version: 5,
            contract_version: 1,
            continuity: PersistentJournalContinuityState::CatchingUp,
            failure: None,
            updated_unix_ms: 500,
        })
        .expect("seed retry journal checkpoint");
    let mut enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "retry-range".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume.clone(),
            journal_id: JournalIdentifier::new(88).expect("journal"),
            requested_start_usn: JournalUsn::new(10).expect("start"),
            requested_end_usn: JournalUsn::new(20).expect("end"),
            covered_until_usn: JournalUsn::new(20).expect("covered"),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 1_000,
            checkpointed_unix_ms: None,
        },
        intents: vec![catch_up_intent(&fixture.root_id, "locked.data", 1)],
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    enrollment.range.batch_id = persistent_journal_batch_id(&enrollment);
    fixture
        .catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment,
                    checkpoint: PersistentJournalCheckpoint {
                        root_id: fixture.root_id.clone(),
                        root_generation: LibraryRootGeneration::initial(),
                        volume,
                        root_file_reference: JournalFileReference::V3([9; 16]),
                        journal_id: JournalIdentifier::new(88).expect("journal"),
                        next_unread_usn: JournalUsn::new(20).expect("covered"),
                        captured_exclusive_end: JournalUsn::new(20).expect("end"),
                        covered_catalog_revision: 0,
                        protocol_version: 5,
                        contract_version: 1,
                        continuity: PersistentJournalContinuityState::CatchingUp,
                        failure: None,
                        updated_unix_ms: 1_000,
                    },
                }],
            },
            1_000,
            policy(),
        )
        .expect("publish retry journal page");

    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(fixture.source.path().join("locked.data"))
        .expect("exclusive retry lock");
    let failed = fixture.process();
    assert_eq!(failed.retried_count, 1);
    assert_eq!(
        fixture
            .catalog
            .load_persistent_journal_checkpoint(&fixture.root_id, LibraryRootGeneration::initial(),)
            .expect("load retry checkpoint")
            .expect("retry checkpoint")
            .continuity,
        PersistentJournalContinuityState::CatchingUp
    );

    drop(lock);
    let completed = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        10_000,
        policy(),
    )
    .expect("retry persistent journal work");
    assert_eq!(completed.completed_count, 1);
    assert_eq!(
        fixture
            .catalog
            .load_persistent_journal_checkpoint(&fixture.root_id, LibraryRootGeneration::initial(),)
            .expect("load terminal checkpoint")
            .expect("terminal checkpoint")
            .continuity,
        PersistentJournalContinuityState::Current
    );
}

#[cfg(windows)]
#[test]
fn newer_watermark_keeps_an_older_cross_root_handoff_visible() {
    assert_cross_root_move_preserves_continuity(
        "a-source",
        "z-destination",
        true,
        true,
        false,
        false,
    );
}

#[cfg(windows)]
#[test]
fn preview_cleanup_downgrades_a_bounded_handoff_before_destination_adoption() {
    assert_cross_root_move_preserves_continuity(
        "a-source",
        "z-destination",
        true,
        false,
        true,
        false,
    );
}

#[cfg(windows)]
#[test]
fn prerelease_preview_repair_does_not_bypass_an_unproven_namespace() {
    assert_cross_root_move_preserves_continuity(
        "a-source",
        "z-destination",
        true,
        false,
        false,
        true,
    );
}

#[cfg(windows)]
#[test]
fn unrelated_cross_root_removals_do_not_block_each_other() {
    let storage = tempdir().expect("cross-root removal storage");
    let first_path = storage.path().join("first");
    let second_path = storage.path().join("second");
    fs::create_dir_all(&first_path).expect("first root");
    fs::create_dir_all(&second_path).expect("second root");
    write_png(&first_path.join("first.png"), 2, 2, [81, 82, 83]);
    write_png(&second_path.join("second.png"), 2, 2, [84, 85, 86]);
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("removal catalog");
    seed_root(
        &mut catalog,
        "first-root",
        "first-scan",
        &first_path,
        &["first.png"],
    );
    seed_root(
        &mut catalog,
        "second-root",
        "second-scan",
        &second_path,
        &["second.png"],
    );
    fs::remove_file(first_path.join("first.png")).expect("remove first fixture");
    fs::remove_file(second_path.join("second.png")).expect("remove second fixture");
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume:journal:901".to_owned(),
    };
    for (root_id, relative_path, sequence) in [
        ("first-root", "first.png", 1),
        ("second-root", "second.png", 2),
    ] {
        catalog
            .enqueue_library_change_intents_with_catch_up(
                &[catch_up_intent(root_id, relative_path, sequence)],
                &evidence,
                1_000,
                policy(),
            )
            .expect("enqueue unrelated removal");
    }

    for root_id in ["first-root", "second-root"] {
        let report = process_ready_library_changes(
            &mut catalog,
            root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("publish unrelated removal");
        assert_eq!(report.completed_count, 1);
        assert_eq!(report.deferred_count, 0);
    }
}

#[cfg(windows)]
#[test]
fn bidirectional_authoritative_moves_preserve_both_assets_without_a_dependency_cycle() {
    let storage = tempdir().expect("authoritative handoff storage");
    let first_path = storage.path().join("first");
    let second_path = storage.path().join("second");
    fs::create_dir_all(&first_path).expect("first root");
    fs::create_dir_all(&second_path).expect("second root");
    write_png(&first_path.join("first.png"), 2, 2, [91, 92, 93]);
    write_png(&second_path.join("second.png"), 2, 2, [94, 95, 96]);
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("handoff catalog");
    seed_root(
        &mut catalog,
        "first-root",
        "first-authoritative-scan",
        &first_path,
        &["first.png"],
    );
    seed_root(
        &mut catalog,
        "second-root",
        "second-authoritative-scan",
        &second_path,
        &["second.png"],
    );
    let mut first = catalog
        .load_incremental_location_by_relative_path("first-root", "first.png")
        .expect("load first prior")
        .expect("first prior");
    let mut second = catalog
        .load_incremental_location_by_relative_path("second-root", "second.png")
        .expect("load second prior")
        .expect("second prior");
    for location in [&mut first, &mut second] {
        location.preview_status = PreviewStatus::Failed;
        location.preview_issue_code = Some("preview_decode_failed".to_owned());
        location.preview_issue_message = Some("retained authoritative evidence".to_owned());
        catalog
            .update_active_preview(location, None, None)
            .expect("record authoritative preview evidence");
    }
    let first_temporary = storage.path().join("first-moving.png");
    fs::rename(first_path.join("first.png"), &first_temporary).expect("stage first move");
    fs::rename(
        second_path.join("second.png"),
        first_path.join("from-second.png"),
    )
    .expect("move second into first root");
    fs::rename(&first_temporary, second_path.join("from-first.png"))
        .expect("move first into second root");
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume:journal:902".to_owned(),
    };
    for (root_id, sequence) in [("first-root", 1), ("second-root", 2)] {
        catalog
            .enqueue_library_change_intents_with_catch_up(
                &[catch_up_root_intent(root_id, sequence)],
                &evidence,
                1_000,
                policy(),
            )
            .expect("enqueue authoritative handoff");
    }
    let cancelled = AtomicBool::new(false);
    for (root_id, now_unix_ms) in [("first-root", 2_000), ("second-root", 2_100)] {
        let report = process_ready_authoritative_library_change_cancellable(
            &mut catalog,
            root_id,
            LibraryRootGeneration::initial(),
            now_unix_ms,
            policy(),
            AuthoritativeRecoveryPolicy::default(),
            &cancelled,
        )
        .expect("publish authoritative handoff");
        assert_eq!(report.incremental.completed_count, 1);
    }
    let moved_second = catalog
        .load_incremental_location_by_relative_path("first-root", "from-second.png")
        .expect("load moved second")
        .expect("moved second");
    let moved_first = catalog
        .load_incremental_location_by_relative_path("second-root", "from-first.png")
        .expect("load moved first")
        .expect("moved first");
    assert_eq!(moved_second.asset_id, second.asset_id);
    assert_eq!(moved_first.asset_id, first.asset_id);
    for moved in [&moved_second, &moved_first] {
        assert!(matches!(moved.preview_status, PreviewStatus::Pending));
        assert!(moved.preview_path.is_empty());
        assert!(moved.preview_issue_code.is_none());
        assert!(moved.preview_issue_message.is_none());
    }
    assert_ne!(moved_second.source_generation, second.source_generation);
    assert_ne!(moved_first.source_generation, first.source_generation);
}

#[cfg(windows)]
fn assert_cross_root_move_preserves_continuity(
    source_root_id: &str,
    destination_root_id: &str,
    source_first: bool,
    supersede_destination_watermark: bool,
    cleanup_after_source: bool,
    repair_stale_handoff_preview: bool,
) {
    let storage = tempdir().expect("cross-root storage");
    let source_path = storage.path().join("source");
    let destination_path = storage.path().join("destination");
    fs::create_dir_all(&source_path).expect("source root");
    fs::create_dir_all(&destination_path).expect("destination root");
    write_png(&source_path.join("old.png"), 3, 2, [71, 72, 73]);
    let catalog_path = storage.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("cross-root catalog");
    seed_root(
        &mut catalog,
        source_root_id,
        "cross-root-source-scan",
        &source_path,
        &["old.png"],
    );
    seed_root(
        &mut catalog,
        destination_root_id,
        "cross-root-destination-scan",
        &destination_path,
        &[],
    );
    let mut original = catalog
        .load_incremental_location_by_relative_path(source_root_id, "old.png")
        .expect("load source location")
        .expect("source location");
    if cleanup_after_source || repair_stale_handoff_preview {
        original.preview_path = storage
            .path()
            .join("preview-before-cleanup.jpg")
            .to_string_lossy()
            .into_owned();
        original.preview_status = PreviewStatus::Ready;
        let artifact = PreviewArtifact {
            artifact_key: "cross-root-preview-before-cleanup".to_owned(),
            algorithm_id: "preview".to_owned(),
            algorithm_version: 1,
            orientation_contract: "orientation".to_owned(),
            size_bucket: 256,
            path: original.preview_path.clone(),
            byte_size: 128,
            encoded_width: original.width,
            encoded_height: original.height,
            width: original.width,
            height: original.height,
        };
        catalog
            .update_active_preview(&original, Some(&artifact), None)
            .expect("record ready preview before cleanup");
    } else {
        original.preview_status = PreviewStatus::Failed;
        original.preview_issue_code = Some("preview_decode_failed".to_owned());
        original.preview_issue_message = Some("retained cross-root evidence".to_owned());
        catalog
            .update_active_preview(&original, None, None)
            .expect("record retained preview evidence");
    }
    fs::rename(
        source_path.join("old.png"),
        destination_path.join("new.png"),
    )
    .expect("move fixture across roots");
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume:journal:900".to_owned(),
    };
    catalog
        .enqueue_library_change_catch_up_batches(
            &[
                LibraryChangeCatchUpQueueBatch {
                    intents: vec![catch_up_intent(source_root_id, "old.png", 1)],
                    evidence: Some(evidence.clone()),
                },
                LibraryChangeCatchUpQueueBatch {
                    intents: vec![catch_up_intent(destination_root_id, "new.png", 2)],
                    evidence: Some(evidence),
                },
            ],
            1_000,
            policy(),
        )
        .expect("atomically enqueue both cross-root plans");

    if source_first {
        let source = process_ready_library_changes(
            &mut catalog,
            source_root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("publish source snapshot before destination handoff");
        assert_eq!(source.completed_count, 1);
        assert!(
            catalog
                .load_incremental_location_by_relative_path(source_root_id, "old.png")
                .expect("load snapshotted source")
                .is_none()
        );
        if cleanup_after_source {
            catalog
                .reset_all_previews_for_cleanup()
                .expect("reset handoff preview before destination adoption");
        } else if repair_stale_handoff_preview {
            drop(catalog);
            let connection = Connection::open(&catalog_path).expect("prerelease preview catalog");
            assert_eq!(
                connection
                    .execute(
                        "UPDATE preview_artifacts SET lifecycle_state = 'stale'
                         WHERE artifact_key = 'cross-root-preview-before-cleanup'",
                        [],
                    )
                    .expect("restore prerelease stale handoff preview"),
                1
            );
            remove_persistent_journal_v22_contract_for_test(&connection);
            connection
                .execute_batch(
                    "DROP TABLE library_change_preview_repair_contract;
                     DROP TABLE library_metadata_inventory_entries;
                     DROP TABLE library_metadata_inventory_runs;
                     DROP TABLE library_metadata_inventory_contract;
                     DROP TABLE library_terminal_media_evidence;
                     DROP TABLE library_terminal_media_evidence_contract;
                     UPDATE schema_info SET version = 19;",
                )
                .expect("restore prerelease preview repair marker");
            drop(connection);
            catalog = SqliteCatalog::open(catalog_path.clone())
                .expect("repair prerelease stale handoff preview");
        }
    }

    if supersede_destination_watermark {
        let newer_evidence = LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: "volume:journal:1200".to_owned(),
        };
        catalog
            .enqueue_library_change_intents_with_catch_up(
                &[catch_up_intent(destination_root_id, "new.png", 3)],
                &newer_evidence,
                2_050,
                policy(),
            )
            .expect("coalesce newer destination watermark");
    }

    let destination = process_ready_library_changes(
        &mut catalog,
        destination_root_id,
        LibraryRootGeneration::initial(),
        2_100,
        policy(),
    )
    .expect("publish destination handoff");
    if repair_stale_handoff_preview {
        assert_eq!(destination.completed_count, 0);
        assert_eq!(destination.retried_count, 1);
        assert!(
            catalog
                .load_incremental_location_by_relative_path(destination_root_id, "new.png")
                .expect("load blocked destination location")
                .is_none()
        );
        drop(catalog);
        let connection = Connection::open(&catalog_path).expect("verify guarded preview repair");
        let (proof_count, ready_handoff_count, failure_code): (i64, i64, String) = connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_root_publication_namespaces
                    WHERE root_id = ?1),
                   ((SELECT COUNT(*) FROM library_change_catch_up_handoffs
                     WHERE preview_status = 'ready')
                    + (SELECT COUNT(*) FROM library_change_scan_handoff_items
                       WHERE preview_status = 'ready')),
                   (SELECT last_failure_code FROM library_change_queue
                    WHERE root_id = ?1 AND relative_path = 'new.png')",
                [destination_root_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load guarded preview repair evidence");
        assert_eq!(proof_count, 0);
        assert_eq!(ready_handoff_count, 0);
        assert_eq!(failure_code, "root_publication_namespace_unproven");
        return;
    }
    assert_eq!(destination.completed_count, 1);
    let moved = catalog
        .load_incremental_location_by_relative_path(destination_root_id, "new.png")
        .expect("load destination location")
        .expect("destination location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, original.file_identity);
    assert!(matches!(moved.preview_status, PreviewStatus::Pending));
    assert!(moved.preview_path.is_empty());
    assert!(moved.preview_issue_code.is_none());
    assert!(moved.preview_issue_message.is_none());
    assert_ne!(moved.source_generation, original.source_generation);

    if !source_first {
        let source = process_ready_library_changes(
            &mut catalog,
            source_root_id,
            LibraryRootGeneration::initial(),
            2_200,
            policy(),
        )
        .expect("complete source removal after destination handoff");
        assert_eq!(source.completed_count, 1);
    }
    assert!(
        catalog
            .load_incremental_location_by_relative_path(source_root_id, "old.png")
            .expect("load removed source")
            .is_none()
    );
    assert_eq!(
        catalog
            .load_incremental_location_by_relative_path(destination_root_id, "new.png")
            .expect("load retained destination")
            .expect("retained destination")
            .asset_id,
        original.asset_id
    );
}

#[cfg(windows)]
fn assert_target_translation_recovery_preserves_continuity(source_first: bool) {
    let storage = tempdir().expect("target translation recovery storage");
    let source_path = storage.path().join("service-source");
    let destination_path = storage.path().join("service-destination");
    fs::create_dir_all(&source_path).expect("service source root");
    fs::create_dir_all(&destination_path).expect("service destination root");
    write_png(&source_path.join("old.png"), 3, 2, [111, 112, 113]);
    let catalog_path = storage.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("service catalog");
    seed_root(
        &mut catalog,
        "journal-source-root",
        "service-source-scan",
        &source_path,
        &["old.png"],
    );
    seed_root(
        &mut catalog,
        "journal-destination-root",
        "service-destination-scan",
        &destination_path,
        &[],
    );
    let mut original = catalog
        .load_incremental_location_by_relative_path("journal-source-root", "old.png")
        .expect("load service source")
        .expect("service source location");
    original.preview_status = PreviewStatus::Failed;
    original.preview_issue_code = Some("preview_decode_failed".to_owned());
    original.preview_issue_message = Some("service target recovery evidence".to_owned());
    catalog
        .update_active_preview(&original, None, None)
        .expect("record service preview evidence");
    for root_id in ["journal-source-root", "journal-destination-root"] {
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 5,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::CatchingUp,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("support service journal root");
    }
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: "journal-volume-guid".to_owned(),
        volume_serial: 55,
    };
    let baseline_checkpoint = |root_id: &str| PersistentJournalCheckpoint {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: volume.clone(),
        root_file_reference: JournalFileReference::V3([7; 16]),
        journal_id: JournalIdentifier::new(77).expect("journal"),
        next_unread_usn: JournalUsn::new(10).expect("baseline"),
        captured_exclusive_end: JournalUsn::new(10).expect("baseline end"),
        covered_catalog_revision: 0,
        protocol_version: 5,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: 500,
    };
    let mut checkpoints = [
        baseline_checkpoint("journal-source-root"),
        baseline_checkpoint("journal-destination-root"),
    ];
    for checkpoint in &checkpoints {
        catalog
            .seed_persistent_journal_checkpoint_for_test(checkpoint)
            .expect("seed service journal checkpoint");
    }
    fs::rename(
        source_path.join("old.png"),
        destination_path.join("new.png"),
    )
    .expect("move service fixture");

    let broker = target_translation_recovery_fixture();
    let reader = SessionBackedPersistentJournalVolumeReader::new(
        Arc::clone(&broker.journal),
        broker.caller.clone(),
        broker
            .roots
            .iter()
            .enumerate()
            .map(|(index, authorization)| PersistentJournalBrokerRoot {
                authorization: authorization.clone(),
                client_root_handle: u64::try_from(index + 1).expect("root handle"),
                volume_serial: volume.volume_serial,
            })
            .collect(),
    )
    .expect("construct production session reader");
    let cancelled = AtomicBool::new(false);
    let first = catch_up_persistent_journal_volume(
        &mut catalog,
        &reader,
        &checkpoints,
        1_000,
        policy(),
        &cancelled,
    )
    .expect("publish target-failure safe prefix");
    assert_eq!(first.enrolled_root_count, 1);
    assert_eq!(first.advanced_checkpoint_count, 1);
    assert_eq!(first.failed_root_count, 1);
    assert_eq!(broker.physical_reads.load(Ordering::Acquire), 1);
    let carries = catalog
        .load_persistent_journal_pending_renames(
            &volume,
            JournalIdentifier::new(77).expect("journal"),
        )
        .expect("load synthesized OLD carry");
    let [carry] = carries.as_slice() else {
        panic!("expected one synthesized OLD carry")
    };
    assert_eq!(carry.old_usn, JournalUsn::new(15).expect("OLD"));
    assert_eq!(carry.previous_relative_path, "old.png");
    assert_eq!(carry.file_reference, JournalFileReference::V3([5; 16]));
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(
                "journal-source-root",
                LibraryRootGeneration::initial(),
            )
            .expect("source checkpoint")
            .expect("source checkpoint row")
            .next_unread_usn,
        JournalUsn::new(18).expect("NEW barrier")
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(
                "journal-destination-root",
                LibraryRootGeneration::initial(),
            )
            .expect("target checkpoint")
            .expect("target checkpoint row")
            .next_unread_usn,
        JournalUsn::new(10).expect("last trustworthy target checkpoint")
    );
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen synthesized carry");
    let destination_root = catalog
        .load_incremental_catalog_root("journal-destination-root")
        .expect("load recovery destination root")
        .expect("recovery destination root");
    let destination_baseline = catalog
        .load_persistent_journal_baselines()
        .expect("load recovery baselines")
        .into_iter()
        .find(|baseline| baseline.root_id == destination_root.root_id)
        .expect("destination recovery baseline");
    let first_inventory = catalog
        .lease_metadata_inventory_recovery(
            &destination_root.root_id,
            destination_root.root_generation,
            1_100,
            policy(),
        )
        .expect("lease destination recovery inventory")
        .expect("destination recovery inventory");
    let mut control = Some(first_inventory);
    let mut retained_source = None;
    let mut enumeration = None;
    for offset in 0..8_i64 {
        let observed_unix_ms = 1_100 + offset;
        let lease = control.take().unwrap_or_else(|| {
            catalog
                .lease_metadata_inventory_recovery(
                    &destination_root.root_id,
                    destination_root.root_generation,
                    observed_unix_ms,
                    policy(),
                )
                .expect("lease destination enumeration continuation")
                .expect("destination enumeration continuation")
        });
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut catalog,
            &destination_root,
            &lease,
            MetadataInventoryRecoveryExecution::without_progress(
                observed_unix_ms,
                4_096,
                policy(),
                &cancelled,
            ),
            retained_source.take(),
        )
        .expect("enumerate destination recovery baseline");
        retained_source = page.retained_source;
        if page.report.inventory.staged_entry_count > 0 {
            enumeration = Some(page.report);
            break;
        }
    }
    let enumeration = enumeration.expect("destination enumeration stages its source entry");
    assert_eq!(enumeration.inventory.staged_entry_count, 1);
    assert!(!enumeration.inventory.is_complete);
    catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &PersistentJournalBaselineClosingBoundary {
                change_id: destination_baseline.change_id,
                volume: destination_baseline.volume.clone(),
                root_file_reference: destination_baseline.root_file_reference.clone(),
                journal_id: destination_baseline.journal_id,
                closing_next_usn: JournalUsn::new(20).expect("recovery closing boundary"),
                protocol_version: destination_baseline.protocol_version,
                captured_unix_ms: 1_101,
            },
        )
        .expect("close destination recovery baseline");
    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(destination_baseline.change_id)
            .expect("destination closing replay coverage"),
        "the target must replay its pre-inventory gap before absence authority"
    );
    broker.recovered.store(true, Ordering::Release);
    for (index, root_id) in ["journal-source-root", "journal-destination-root"]
        .into_iter()
        .enumerate()
    {
        checkpoints[index] = catalog
            .load_persistent_journal_checkpoint(root_id, LibraryRootGeneration::initial())
            .expect("load recovery checkpoint")
            .expect("recovery checkpoint row");
    }
    let recovered = catch_up_persistent_journal_volume(
        &mut catalog,
        &reader,
        &checkpoints,
        1_102,
        policy(),
        &cancelled,
    )
    .expect("publish recovered NEW handoff");
    assert_eq!(
        recovered.enrolled_root_count, 2,
        "recovered report: {recovered:?}"
    );
    assert_eq!(recovered.advanced_checkpoint_count, 2);
    assert_eq!(recovered.failed_root_count, 0);
    assert_eq!(broker.physical_reads.load(Ordering::Acquire), 2);
    assert!(
        catalog
            .load_persistent_journal_pending_renames(
                &volume,
                JournalIdentifier::new(77).expect("journal"),
            )
            .expect("load consumed carry set")
            .is_empty()
    );
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen recovered handoff");
    let roots = if source_first {
        ["journal-source-root", "journal-destination-root"]
    } else {
        ["journal-destination-root", "journal-source-root"]
    };
    for (index, root_id) in roots.into_iter().enumerate() {
        let report = process_ready_library_changes(
            &mut catalog,
            root_id,
            LibraryRootGeneration::initial(),
            2_000 + i64::try_from(index).expect("bounded index"),
            policy(),
        )
        .expect("process recovered cross-root move");
        assert_eq!(report.completed_count, 1);
    }
    assert!(
        catalog
            .persistent_journal_baseline_closing_is_covered(destination_baseline.change_id)
            .expect("terminal destination closing replay coverage")
    );
    let mut inventory_completed = false;
    for offset in 0..8_i64 {
        let now_unix_ms = 2_100 + offset;
        let candidates = process_ready_metadata_inventory_recovery_candidates_cancellable(
            &mut catalog,
            &destination_root.root_id,
            destination_root.root_generation,
            now_unix_ms,
            policy(),
            &cancelled,
        )
        .expect("drain destination recovery candidates");
        if let Some(control) = catalog
            .lease_metadata_inventory_recovery(
                &destination_root.root_id,
                destination_root.root_generation,
                now_unix_ms,
                policy(),
            )
            .expect("lease destination recovery continuation")
        {
            let continuation = process_leased_metadata_inventory_change(
                &mut catalog,
                &destination_root,
                &control,
                now_unix_ms,
                4_096,
                policy(),
                &cancelled,
            )
            .expect("continue destination recovery baseline");
            inventory_completed |= continuation.incremental.completed_count == 1;
        }
        if inventory_completed {
            assert_eq!(candidates.retried_count, 0);
            break;
        }
    }
    assert!(
        inventory_completed,
        "destination P2 recovery must become Current"
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(
                &destination_root.root_id,
                destination_root.root_generation,
            )
            .expect("load destination recovery checkpoint")
            .expect("destination recovery checkpoint")
            .continuity,
        PersistentJournalContinuityState::Current
    );
    drop(catalog);

    let catalog = SqliteCatalog::open(catalog_path).expect("reopen completed recovery handoff");
    let moved = catalog
        .load_incremental_location_by_relative_path("journal-destination-root", "new.png")
        .expect("load recovered destination")
        .expect("recovered destination location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, original.file_identity);
    assert!(matches!(moved.preview_status, PreviewStatus::Pending));
    assert!(moved.preview_path.is_empty());
    assert!(moved.preview_issue_code.is_none());
    assert!(moved.preview_issue_message.is_none());
    assert_ne!(moved.source_generation, original.source_generation);
    assert!(
        catalog
            .load_incremental_location_by_relative_path("journal-source-root", "old.png")
            .expect("load removed service source")
            .is_none()
    );
    for root_id in ["journal-source-root", "journal-destination-root"] {
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint(root_id, LibraryRootGeneration::initial())
                .expect("load terminal service checkpoint")
                .expect("terminal service checkpoint")
                .continuity,
            PersistentJournalContinuityState::Current
        );
    }
}

fn assert_persistent_journal_cross_root_move_preserves_continuity(source_first: bool) {
    let storage = tempdir().expect("persistent journal cross-root storage");
    let source_path = storage.path().join("journal-source");
    let destination_path = storage.path().join("journal-destination");
    fs::create_dir_all(&source_path).expect("journal source root");
    fs::create_dir_all(&destination_path).expect("journal destination root");
    write_png(&source_path.join("old.png"), 3, 2, [101, 102, 103]);
    let catalog_path = storage.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("journal catalog");
    seed_root(
        &mut catalog,
        "journal-source-root",
        "journal-source-scan",
        &source_path,
        &["old.png"],
    );
    seed_root(
        &mut catalog,
        "journal-destination-root",
        "journal-destination-scan",
        &destination_path,
        &[],
    );
    let mut original = catalog
        .load_incremental_location_by_relative_path("journal-source-root", "old.png")
        .expect("load journal source")
        .expect("journal source location");
    original.preview_status = PreviewStatus::Failed;
    original.preview_issue_code = Some("preview_decode_failed".to_owned());
    original.preview_issue_message = Some("persistent journal retained evidence".to_owned());
    catalog
        .update_active_preview(&original, None, None)
        .expect("record journal preview evidence");
    for root_id in ["journal-source-root", "journal-destination-root"] {
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: 5,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::CatchingUp,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("support journal root");
    }
    fs::rename(
        source_path.join("old.png"),
        destination_path.join("new.png"),
    )
    .expect("move persistent journal fixture");
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: "journal-volume-guid".to_owned(),
        volume_serial: 55,
    };
    for root_id in ["journal-source-root", "journal-destination-root"] {
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                volume: volume.clone(),
                root_file_reference: JournalFileReference::V3([7; 16]),
                journal_id: JournalIdentifier::new(77).expect("journal"),
                next_unread_usn: JournalUsn::new(10).expect("baseline"),
                captured_exclusive_end: JournalUsn::new(10).expect("baseline end"),
                covered_catalog_revision: 0,
                protocol_version: 5,
                contract_version: 1,
                continuity: PersistentJournalContinuityState::CatchingUp,
                failure: None,
                updated_unix_ms: 500,
            })
            .expect("seed journal checkpoint");
    }
    let lineage = |owner_source_range_id: &str| PersistentJournalCrossRootLineage {
        lineage_id: "journal-cross-root-lineage".to_owned(),
        owner_source_range_id: owner_source_range_id.to_owned(),
        volume: volume.clone(),
        journal_id: JournalIdentifier::new(77).expect("journal"),
        file_reference: JournalFileReference::V3([5; 16]),
        old_usn: None,
        new_usn: None,
        previous_carry_id: None,
        previous_root_id: "journal-source-root".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "old.png".to_owned(),
        current_root_id: "journal-destination-root".to_owned(),
        current_root_generation: LibraryRootGeneration::initial(),
        current_relative_path: "new.png".to_owned(),
        state: PersistentJournalLineageState::Pending,
    };
    let checkpoint = |root_id: &str| PersistentJournalCheckpoint {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: volume.clone(),
        root_file_reference: JournalFileReference::V3([7; 16]),
        journal_id: JournalIdentifier::new(77).expect("journal"),
        next_unread_usn: JournalUsn::new(20).expect("covered"),
        captured_exclusive_end: JournalUsn::new(20).expect("end"),
        covered_catalog_revision: 0,
        protocol_version: 5,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: 1_000,
    };
    let mut safe_prefix = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "journal-safe-prefix".to_owned(),
            root_id: "journal-source-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume.clone(),
            journal_id: JournalIdentifier::new(77).expect("journal"),
            requested_start_usn: JournalUsn::new(10).expect("prefix start"),
            requested_end_usn: JournalUsn::new(20).expect("prefix end"),
            covered_until_usn: JournalUsn::new(12).expect("safe prefix boundary"),
            is_complete: false,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 750,
            checkpointed_unix_ms: None,
        },
        intents: Vec::new(),
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    safe_prefix.range.batch_id = persistent_journal_batch_id(&safe_prefix);
    let mut prefix_checkpoint = checkpoint("journal-source-root");
    prefix_checkpoint.next_unread_usn = JournalUsn::new(12).expect("safe prefix boundary");
    prefix_checkpoint.updated_unix_ms = 750;
    catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment: safe_prefix,
                    checkpoint: prefix_checkpoint,
                }],
            },
            750,
            policy(),
        )
        .expect("publish safe non-rename prefix before endpoint recovery");
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(
                "journal-source-root",
                LibraryRootGeneration::initial(),
            )
            .expect("load safe-prefix checkpoint")
            .expect("safe-prefix checkpoint")
            .next_unread_usn,
        JournalUsn::new(12).expect("safe prefix boundary")
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(
                "journal-destination-root",
                LibraryRootGeneration::initial(),
            )
            .expect("load failed sibling checkpoint")
            .expect("failed sibling checkpoint")
            .next_unread_usn,
        JournalUsn::new(10).expect("failed sibling remains at baseline")
    );
    let mut carry = PersistentJournalPendingRename {
        carry_id: String::new(),
        source_range_id: "journal-source-range".to_owned(),
        volume: volume.clone(),
        journal_id: JournalIdentifier::new(77).expect("journal"),
        file_reference: JournalFileReference::V3([5; 16]),
        old_usn: JournalUsn::new(15).expect("OLD USN"),
        previous_root_id: "journal-source-root".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "old.png".to_owned(),
        is_directory: false,
        enrolled_unix_ms: 1_000,
    };
    carry.carry_id = persistent_journal_pending_rename_id(&carry);
    let mut source_enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: carry.source_range_id.clone(),
            root_id: "journal-source-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume.clone(),
            journal_id: JournalIdentifier::new(77).expect("journal"),
            requested_start_usn: JournalUsn::new(12).expect("start"),
            requested_end_usn: JournalUsn::new(20).expect("end"),
            covered_until_usn: JournalUsn::new(20).expect("covered"),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 1_000,
            checkpointed_unix_ms: None,
        },
        intents: vec![catch_up_intent("journal-source-root", "old.png", 1)],
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: vec![carry],
        consumed_pending_rename_ids: Vec::new(),
    };
    let source_range_id = persistent_journal_batch_id(&source_enrollment);
    source_enrollment
        .range
        .batch_id
        .clone_from(&source_range_id);
    source_enrollment.pending_renames[0]
        .source_range_id
        .clone_from(&source_range_id);
    catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment: source_enrollment,
                    checkpoint: checkpoint("journal-source-root"),
                }],
            },
            1_000,
            policy(),
        )
        .expect("publish journal OLD carry");
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path).expect("reopen journal OLD carry");
    let carries = catalog
        .load_persistent_journal_pending_renames(
            &volume,
            JournalIdentifier::new(77).expect("journal"),
        )
        .expect("load reopened journal OLD carry");
    let [durable_carry] = carries.as_slice() else {
        panic!("expected one durable journal OLD carry");
    };
    assert_eq!(durable_carry.source_range_id, source_range_id);
    let carried_lineage = |owner_source_range_id: &str| {
        let mut item = lineage(owner_source_range_id);
        item.old_usn = Some(durable_carry.old_usn);
        item.new_usn = Some(JournalUsn::new(18).expect("NEW USN"));
        item.previous_carry_id = Some(durable_carry.carry_id.clone());
        item
    };

    let mut destination_enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "journal-destination-range".to_owned(),
            root_id: "journal-destination-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume.clone(),
            journal_id: JournalIdentifier::new(77).expect("journal"),
            requested_start_usn: JournalUsn::new(10).expect("start"),
            requested_end_usn: JournalUsn::new(20).expect("end"),
            covered_until_usn: JournalUsn::new(20).expect("covered"),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 1_000,
            checkpointed_unix_ms: None,
        },
        intents: vec![catch_up_intent("journal-destination-root", "new.png", 2)],
        cross_root_lineage: vec![carried_lineage("journal-destination-range")],
        carried_cross_root_lineage: vec![carried_lineage(&source_range_id)],
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: vec![durable_carry.carry_id.clone()],
    };
    let destination_range_id = persistent_journal_batch_id(&destination_enrollment);
    destination_enrollment
        .range
        .batch_id
        .clone_from(&destination_range_id);
    destination_enrollment.cross_root_lineage[0]
        .owner_source_range_id
        .clone_from(&destination_range_id);
    catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment: destination_enrollment,
                    checkpoint: checkpoint("journal-destination-root"),
                }],
            },
            1_000,
            policy(),
        )
        .expect("publish carried journal cross-root handoff");
    assert!(
        catalog
            .load_persistent_journal_pending_renames(
                &volume,
                JournalIdentifier::new(77).expect("journal"),
            )
            .expect("load consumed journal carries")
            .is_empty()
    );
    for root_id in ["journal-source-root", "journal-destination-root"] {
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint(root_id, LibraryRootGeneration::initial())
                .expect("load pending journal checkpoint")
                .expect("pending checkpoint")
                .continuity,
            PersistentJournalContinuityState::CatchingUp
        );
    }
    let roots = if source_first {
        ["journal-source-root", "journal-destination-root"]
    } else {
        ["journal-destination-root", "journal-source-root"]
    };
    for (index, root_id) in roots.into_iter().enumerate() {
        let report = process_ready_library_changes(
            &mut catalog,
            root_id,
            LibraryRootGeneration::initial(),
            2_000 + i64::try_from(index).expect("bounded index"),
            policy(),
        )
        .expect("process journal cross-root move");
        assert_eq!(report.completed_count, 1);
        if index == 0 {
            for pending_root in ["journal-source-root", "journal-destination-root"] {
                assert_eq!(
                    catalog
                        .load_persistent_journal_checkpoint(
                            pending_root,
                            LibraryRootGeneration::initial(),
                        )
                        .expect("load half-terminal journal checkpoint")
                        .expect("half-terminal checkpoint")
                        .continuity,
                    PersistentJournalContinuityState::CatchingUp
                );
            }
        }
    }
    let moved = catalog
        .load_incremental_location_by_relative_path("journal-destination-root", "new.png")
        .expect("load journal destination")
        .expect("journal destination location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, original.file_identity);
    assert!(matches!(moved.preview_status, PreviewStatus::Pending));
    assert!(moved.preview_path.is_empty());
    assert!(moved.preview_issue_code.is_none());
    assert!(moved.preview_issue_message.is_none());
    assert_ne!(moved.source_generation, original.source_generation);
    assert!(
        catalog
            .load_incremental_location_by_relative_path("journal-source-root", "old.png")
            .expect("load removed journal source")
            .is_none()
    );
    for root_id in ["journal-source-root", "journal-destination-root"] {
        assert_eq!(
            catalog
                .load_persistent_journal_checkpoint(root_id, LibraryRootGeneration::initial())
                .expect("load terminal journal checkpoint")
                .expect("terminal checkpoint")
                .continuity,
            PersistentJournalContinuityState::Current
        );
    }
}

struct CatalogFixture {
    catalog: SqliteCatalog,
    source: TempDir,
    _storage: TempDir,
    root_id: String,
    root_path: String,
}

impl CatalogFixture {
    fn enqueue(&mut self, intents: &[LibraryChangeIntent]) {
        self.catalog
            .enqueue_library_change_intents(intents, 1_000, policy())
            .expect("enqueue changes");
    }

    fn process(&mut self) -> crate::domain::IncrementalLibraryChangeReport {
        self.process_with_policy(policy())
    }

    fn process_with_policy(
        &mut self,
        policy: LibraryChangeQueuePolicy,
    ) -> crate::domain::IncrementalLibraryChangeReport {
        process_ready_library_changes(
            &mut self.catalog,
            &self.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("process changes")
    }

    fn revision(&self) -> u64 {
        self.catalog
            .load_incremental_catalog_root(&self.root_id)
            .expect("load root")
            .expect("catalog root")
            .catalog_revision
    }

    fn location(&self, relative_path: &str) -> Option<AssetLocationView> {
        self.catalog
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("load location")
    }

    fn discovered(&self, relative_path: &str) -> crate::domain::DiscoveredFile {
        let discovery = FileDiscovery::new(&self.root_path).expect("file discovery");
        match discovery.visit_relative_path(relative_path).outcome {
            FileVisitOutcome::File(file) => file,
            _ => panic!("expected discovered file"),
        }
    }
}

#[cfg(windows)]
struct ReadyPreviewFixture {
    location: AssetLocationView,
    request: PreviewRequest,
    artifact: PreviewArtifact,
}

#[cfg(windows)]
fn publish_ready_preview(
    fixture: &mut CatalogFixture,
    relative_path: &str,
    artifact_key: &str,
) -> ReadyPreviewFixture {
    publish_ready_preview_at_edge(fixture, relative_path, artifact_key, 256)
}

#[cfg(windows)]
fn publish_ready_preview_at_edge(
    fixture: &mut CatalogFixture,
    relative_path: &str,
    artifact_key: &str,
    preview_edge: u32,
) -> ReadyPreviewFixture {
    let mut location = fixture.location(relative_path).expect("preview location");
    assert!(location.source_revision.is_some());
    assert_ne!(location.source_generation, 0);
    let request = PreviewRequest {
        location_id: location.location_id.clone(),
        expected_root_id: location.root_id.clone(),
        expected_scan_id: location.scan_id.clone(),
        expected_source_revision: location.source_revision.clone(),
        expected_source_generation: location.source_generation,
        preview_edge,
        retry_failed: false,
        protected_location_ids: Vec::new(),
    };
    let artifact_path = fixture._storage.path().join(format!("{artifact_key}.jpg"));
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
        artifact_key: artifact_key.to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: preview_edge,
        path: location.preview_path.clone(),
        byte_size: fs::metadata(&artifact_path)
            .expect("ready preview metadata")
            .len(),
        encoded_width: location.width.max(1),
        encoded_height: location.height.max(1),
        width: location.width,
        height: location.height,
    };
    fixture
        .catalog
        .update_active_preview(&location, Some(&artifact), Some(&request))
        .expect("publish ready preview");
    ReadyPreviewFixture {
        location,
        request,
        artifact,
    }
}

#[cfg(windows)]
fn assert_ready_preview_superseded(fixture: &mut CatalogFixture, ready: &ReadyPreviewFixture) {
    assert_eq!(
        preview_owner_count(&fixture.catalog, &ready.artifact.artifact_key),
        0
    );
    assert_eq!(
        preview_lifecycle_state(&fixture.catalog, &ready.artifact.artifact_key),
        "stale"
    );
    let error = fixture
        .catalog
        .update_active_preview(&ready.location, Some(&ready.artifact), Some(&ready.request))
        .expect_err("stale preview request must not publish");
    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(
        preview_owner_count(&fixture.catalog, &ready.artifact.artifact_key),
        0
    );
    assert_eq!(
        preview_lifecycle_state(&fixture.catalog, &ready.artifact.artifact_key),
        "stale"
    );
}

#[cfg(windows)]
fn preview_owner_count(catalog: &SqliteCatalog, artifact_key: &str) -> i64 {
    Connection::open(catalog.catalog_path())
        .expect("open preview owner query")
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview owner count")
}

#[cfg(windows)]
fn preview_lifecycle_state(catalog: &SqliteCatalog, artifact_key: &str) -> String {
    Connection::open(catalog.catalog_path())
        .expect("open preview lifecycle query")
        .query_row(
            "SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview lifecycle state")
}

#[cfg(windows)]
fn assert_equal_length_rewrite_supersedes_ready_preview(
    origin: LibraryChangeOrigin,
    artifact_key: &str,
) {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("same.bmp");
    write_bmp(&source_path, 4, 3, [10, 20, 30]);
    let original_modified = fs::metadata(&source_path)
        .expect("original source metadata")
        .modified()
        .expect("original modified time");
    let original_bytes = fs::read(&source_path).expect("original source bytes");
    let mut fixture = seed_catalog(source, &["same.bmp"]);
    let ready = publish_ready_preview(&mut fixture, "same.bmp", artifact_key);
    let revision = fixture.revision();

    write_bmp(&source_path, 4, 3, [30, 20, 10]);
    fs::OpenOptions::new()
        .write(true)
        .open(&source_path)
        .expect("open rewritten source for timestamp restore")
        .set_times(fs::FileTimes::new().set_modified(original_modified))
        .expect("restore source modified time");
    let rewritten_bytes = fs::read(&source_path).expect("rewritten source bytes");
    assert_ne!(rewritten_bytes, original_bytes);
    assert_eq!(rewritten_bytes.len(), original_bytes.len());
    let rewritten = fixture.discovered("same.bmp");
    assert_eq!(rewritten.file_size, ready.location.file_size);
    assert_eq!(rewritten.modified_unix_ms, ready.location.modified_unix_ms);
    assert_eq!(rewritten.file_identity, ready.location.file_identity);
    assert_ne!(rewritten.source_revision, ready.location.source_revision);
    let mut change = intent(&fixture.root_id, "same.bmp", None, 1);
    change.origin = origin;
    fixture.enqueue(&[change]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert_eq!(fixture.revision(), revision + 1);
    let after = fixture.location("same.bmp").expect("rewritten location");
    assert_eq!(after.asset_id, ready.location.asset_id);
    assert_eq!(after.file_identity, ready.location.file_identity);
    assert_eq!(after.file_size, ready.location.file_size);
    assert_eq!(after.modified_unix_ms, ready.location.modified_unix_ms);
    assert_eq!(after.source_revision, rewritten.source_revision);
    assert_ne!(after.source_generation, ready.location.source_generation);
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert_ready_preview_superseded(&mut fixture, &ready);
    assert_eq!(
        fs::read(&source_path).expect("rewritten bytes after delta"),
        rewritten_bytes
    );
}

fn seed_root(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    scan_id: &str,
    root_path: &Path,
    relative_paths: &[&str],
) {
    let root_input = root_path.to_string_lossy().into_owned();
    let discovery = FileDiscovery::new(&root_input).expect("root discovery");
    let canonical_root = discovery
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let request = ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: canonical_root.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("root identity query")
        .expect("root stable identity");
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            &canonical_root,
            &publication_identity,
        )
        .expect("begin root scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(scan_id)
        .expect("prove root fixture first-import handoff");
    let inspector = LocalMediaInspector::new();
    for relative_path in relative_paths {
        let file = match discovery.visit_relative_path(relative_path).outcome {
            FileVisitOutcome::File(file) => file,
            _ => panic!("expected root fixture file"),
        };
        let inspection = inspector.inspect(&file).expect("inspect root fixture");
        let location = AssetLocationView {
            asset_id: stable_id("test-asset-v1", relative_path),
            location_id: stable_location_id(root_id, relative_path),
            root_id: root_id.to_owned(),
            scan_id: scan_id.to_owned(),
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path,
            preview_path: String::new(),
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: file.file_identity,
            source_revision: file.source_revision,
            source_generation: 0,
            width: inspection.width,
            height: inspection.height,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: inspection.metadata.engine_id,
            metadata_engine_version: inspection.metadata.engine_version,
            capture_time: inspection.metadata.capture_time,
        };
        catalog
            .stage_location(scan_id, root_id, &location)
            .expect("stage root fixture");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(relative_paths.len()).expect("root fixture count"),
            0,
        )
        .expect("publish root scan");
}

fn seed_catalog(source: TempDir, relative_paths: &[&str]) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, None, false)
}

fn seed_catalog_with_metadata(
    source: TempDir,
    relative_paths: &[&str],
    metadata_override: Option<(&str, &str)>,
) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, metadata_override, false)
}

fn seed_catalog_without_identity(source: TempDir, relative_paths: &[&str]) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, None, true)
}

fn seed_catalog_with_options(
    source: TempDir,
    relative_paths: &[&str],
    metadata_override: Option<(&str, &str)>,
    clear_file_identity: bool,
) -> CatalogFixture {
    let root_input = source.path().to_string_lossy().into_owned();
    let discovery = FileDiscovery::new(&root_input).expect("file discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let storage = tempdir().expect("storage directory");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("open catalog");
    let request = ScanRequest {
        scan_id: "baseline-scan".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("baseline root identity query")
        .expect("baseline root stable identity");
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            &root_id,
            &root_path,
            &publication_identity,
        )
        .expect("begin baseline scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove baseline fixture first-import handoff");
    let inspector = LocalMediaInspector::new();
    for relative_path in relative_paths {
        let file = match discovery.visit_relative_path(relative_path).outcome {
            FileVisitOutcome::File(file) => file,
            _ => panic!("expected baseline file"),
        };
        let inspection = inspector.inspect(&file).expect("inspect baseline image");
        let (metadata_engine_id, metadata_engine_version) = metadata_override
            .map(|(engine_id, engine_version)| (engine_id.to_owned(), engine_version.to_owned()))
            .unwrap_or((
                inspection.metadata.engine_id,
                inspection.metadata.engine_version,
            ));
        let location = AssetLocationView {
            asset_id: stable_id("test-asset-v1", relative_path),
            location_id: stable_location_id(&root_id, relative_path),
            root_id: root_id.clone(),
            scan_id: request.scan_id.clone(),
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path,
            preview_path: String::new(),
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: (!clear_file_identity)
                .then_some(file.file_identity)
                .flatten(),
            source_revision: file.source_revision,
            source_generation: 0,
            width: inspection.width,
            height: inspection.height,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id,
            metadata_engine_version,
            capture_time: inspection.metadata.capture_time,
        };
        catalog
            .stage_location(&request.scan_id, &root_id, &location)
            .expect("stage baseline location");
    }
    catalog
        .publish_scan(
            &request.scan_id,
            &root_id,
            u64::try_from(relative_paths.len()).expect("baseline count"),
            0,
        )
        .expect("publish baseline scan");
    CatalogFixture {
        source,
        _storage: storage,
        catalog,
        root_id,
        root_path,
    }
}

fn intent(
    root_id: &str,
    relative_path: &str,
    previous_relative_path: Option<&str>,
    sequence: u64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind: if previous_relative_path.is_some() {
            LibraryChangeIntentKind::RenameCandidate
        } else {
            LibraryChangeIntentKind::Reconcile
        },
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: previous_relative_path.map(str::to_owned),
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: 1_000,
        most_recent_observed_unix_ms: 1_000,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: 1,
    }
}

fn catch_up_intent(root_id: &str, relative_path: &str, sequence: u64) -> LibraryChangeIntent {
    LibraryChangeIntent {
        origin: LibraryChangeOrigin::StartupCatchUp,
        ..intent(root_id, relative_path, None, sequence)
    }
}

fn catch_up_root_intent(root_id: &str, sequence: u64) -> LibraryChangeIntent {
    LibraryChangeIntent {
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        origin: LibraryChangeOrigin::StartupCatchUp,
        ..intent(root_id, "root-gap", None, sequence)
    }
}

fn policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    let image = RgbImage::from_pixel(width, height, Rgb(color));
    image.save(path).expect("write PNG fixture");
}

#[cfg(windows)]
fn write_bmp(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    RgbImage::from_pixel(width, height, Rgb(color))
        .save_with_format(path, ImageFormat::Bmp)
        .expect("write BMP fixture");
}

fn write_png_with_format(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("PNG parent");
    }
    RgbImage::from_pixel(width, height, Rgb(color))
        .save_with_format(path, ImageFormat::Png)
        .expect("write PNG fixture");
}

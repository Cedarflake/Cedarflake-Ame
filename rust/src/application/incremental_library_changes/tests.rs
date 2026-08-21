use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use image::{Rgb, RgbImage};
use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

use crate::adapters::{FileDiscovery, FileVisitOutcome, LocalMediaInspector, SqliteCatalog};
use crate::application::{
    AuthoritativeRecoveryPolicy, process_ready_authoritative_library_change_cancellable,
};
use crate::domain::{
    AssetLocationView, DerivedEvidenceDisposition, LibraryChangeCatchUpEvidence,
    LibraryChangeCatchUpQueueBatch, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    PreviewArtifact, PreviewStatus, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector,
};

use super::{
    prepare_change, process_ready_library_changes, stable_id, stable_location_id, user_visible_path,
};

#[test]
fn unchanged_path_completes_without_incrementing_the_catalog_revision() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("same.png"), 2, 2, [10, 20, 30]);
    let original_bytes = fs::read(source.path().join("same.png")).expect("source bytes");
    let mut fixture = seed_catalog(source, &["same.png"]);
    let revision = fixture.revision();
    fixture.enqueue(&[intent(&fixture.root_id, "same.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 0);
    assert_eq!(report.catalog_revision, revision);
    assert_eq!(fixture.revision(), revision);
    assert_eq!(
        fs::read(fixture.source.path().join("same.png")).expect("source bytes after delta"),
        original_bytes
    );
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
fn same_path_replacement_creates_a_new_asset_identity() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("replace.png"), 2, 2, [130, 140, 150]);
    let mut fixture = seed_catalog(source, &["replace.png"]);
    let before = fixture.location("replace.png").expect("original location");
    fs::remove_file(fixture.source.path().join("replace.png")).expect("remove original fixture");
    write_png(
        &fixture.source.path().join("replace.png"),
        6,
        2,
        [150, 140, 130],
    );
    let replacement = fixture.discovered("replace.png");
    assert_ne!(replacement.file_identity, before.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "replace.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    let after = fixture
        .location("replace.png")
        .expect("replacement location");
    assert_ne!(after.asset_id, before.asset_id);
    assert_eq!(after.file_identity, replacement.file_identity);
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
fn one_unreadable_image_retries_without_blocking_a_valid_sibling() {
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
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert!(fixture.location("valid.png").is_some());
    assert!(fixture.location("broken.jpg").is_none());
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.completed_count, 1);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(
        fs::read(fixture.source.path().join("valid.png")).expect("valid bytes after delta"),
        valid_bytes
    );
    assert_eq!(
        fs::read(fixture.source.path().join("broken.jpg")).expect("broken bytes after delta"),
        broken_bytes
    );
}

#[test]
fn pending_work_is_claimed_by_a_running_full_scan_without_entering_the_path_worker() {
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

    let report = fixture.process();

    assert_eq!(report.leased_count, 0);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 0);
    assert!(fixture.location("waiting.png").is_none());
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.retry_wait_count, 0);
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

#[test]
fn unavailable_root_does_not_consume_a_path_retry_attempt() {
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
        .expect("path work remains leasable");
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.attempt_count, 1);
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
fn identity_backfill_preserves_a_migrated_v17_location_identifier() {
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

    assert_eq!(backfill.applied_mutation_count, 1);
    let identified = fixture
        .location("album/legacy.png")
        .expect("identified location");
    assert_eq!(identified.asset_id, original.asset_id);
    assert_eq!(identified.location_id, legacy_location_id);
    assert!(identified.file_identity.is_some());
    let connection = Connection::open(catalog_path).expect("verified migration fixture catalog");
    let (schema_version, location_count, asset_count): (i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT COUNT(*) FROM asset_locations
                WHERE root_id = ?1 AND relative_path = 'album/legacy.png'),
               (SELECT asset_count FROM scan_runs WHERE id = 'baseline-scan')",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load post-backfill counts");
    assert_eq!(schema_version, 20);
    assert_eq!(location_count, 1);
    assert_eq!(asset_count, 1);
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
    let discovery = FileDiscovery::new(&fixture.root_path).expect("file discovery");
    let inspector = LocalMediaInspector::new();

    let prepared =
        prepare_change(&fixture.catalog, &discovery, &inspector, &leased).expect("prepare rename");
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
fn compatible_rename_preserves_failed_preview_evidence() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 2, 3, [51, 52, 53]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let mut failed = fixture.location("old.png").expect("original location");
    failed.preview_status = PreviewStatus::Failed;
    failed.preview_issue_code = Some("preview_decode_failed".to_owned());
    failed.preview_issue_message = Some("fixture failure".to_owned());
    fixture
        .catalog
        .update_active_preview(&failed, None)
        .expect("record failed preview");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    fixture.process();

    let renamed = fixture.location("new.png").expect("renamed location");
    assert!(matches!(renamed.preview_status, PreviewStatus::Failed));
    assert_eq!(
        renamed.preview_issue_code.as_deref(),
        Some("preview_decode_failed")
    );
    assert_eq!(
        renamed.preview_issue_message.as_deref(),
        Some("fixture failure")
    );
}

#[cfg(windows)]
#[test]
fn source_first_cross_root_move_preserves_asset_and_preview_continuity() {
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
fn destination_first_cross_root_move_preserves_asset_and_preview_continuity() {
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
fn prerelease_stale_preview_is_downgraded_before_bounded_handoff_adoption() {
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
            .update_active_preview(location, None)
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
    assert!(matches!(moved_second.preview_status, PreviewStatus::Failed));
    assert!(matches!(moved_first.preview_status, PreviewStatus::Failed));
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
            .update_active_preview(&original, Some(&artifact))
            .expect("record ready preview before cleanup");
    } else {
        original.preview_status = PreviewStatus::Failed;
        original.preview_issue_code = Some("preview_decode_failed".to_owned());
        original.preview_issue_message = Some("retained cross-root evidence".to_owned());
        catalog
            .update_active_preview(&original, None)
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
            connection
                .execute_batch(
                    "DROP TABLE library_change_preview_repair_contract;
                     DROP TABLE library_metadata_inventory_entries;
                     DROP TABLE library_metadata_inventory_runs;
                     DROP TABLE library_metadata_inventory_contract;
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
    assert_eq!(destination.completed_count, 1);
    let moved = catalog
        .load_incremental_location_by_relative_path(destination_root_id, "new.png")
        .expect("load destination location")
        .expect("destination location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, original.file_identity);
    if cleanup_after_source || repair_stale_handoff_preview {
        assert!(matches!(moved.preview_status, PreviewStatus::Pending));
        assert!(moved.preview_path.is_empty());
        assert!(moved.preview_issue_code.is_none());
        assert!(moved.preview_issue_message.is_none());
    } else {
        assert!(matches!(moved.preview_status, PreviewStatus::Failed));
        assert_eq!(
            moved.preview_issue_code.as_deref(),
            Some("preview_decode_failed")
        );
    }

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

struct CatalogFixture {
    source: TempDir,
    _storage: TempDir,
    catalog: SqliteCatalog,
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
    catalog
        .begin_scan(&request, root_id, &canonical_root)
        .expect("begin root scan");
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
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path,
            preview_path: String::new(),
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: file.file_identity,
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
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin baseline scan");
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

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};
use tempfile::tempdir;

use crate::application::{enqueue_library_change_plan, plan_library_changes};
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeCatchUpEvidence,
    LibraryChangeCatchUpQueueBatch, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLane, LibraryChangeObservation, LibraryChangeObservationKind, LibraryChangeOrigin,
    LibraryChangePlanningContext, LibraryChangePlanningLimits, LibraryChangeQueueHealth,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRecoveryAuthority,
    LibraryRecoveryAuthorityReason, LibraryRootAvailability, MetadataInventoryComparisonStatus,
    MetadataInventoryComparisonUpdate, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryFrontierEntry, MetadataInventoryPage, MetadataInventoryPlaceholderState,
    MetadataInventoryRunRequest, MetadataInventoryRunStatus, MetadataInventoryScope,
    PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalEnrollmentBatch,
    PersistentJournalRangeState, PersistentJournalSourceRange, PersistentJournalVolumeIdentity,
    ScanRequest, persistent_journal_batch_id,
};
use crate::ports::{CatalogRepository, MetadataInventoryRepository, PersistentJournalRepository};

use super::*;

#[test]
fn migrates_v16_without_losing_existing_catalog_rows() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v16 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info(version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (16);
             CREATE TABLE catalog_state(revision INTEGER NOT NULL);
             INSERT INTO catalog_state(revision) VALUES (7);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY,
               path TEXT NOT NULL UNIQUE,
               active_scan_id TEXT,
               created_unix_ms INTEGER NOT NULL
             );
             INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('root-a', 'C:\\Source', 123);
             CREATE TABLE scan_runs (
               id TEXT PRIMARY KEY,
               root_id TEXT NOT NULL,
               status TEXT NOT NULL,
               started_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               current_directory_relative_path TEXT,
               current_directory_enumerated INTEGER NOT NULL DEFAULT 0,
               last_visited_relative_path TEXT
             );
             CREATE TABLE asset_locations (
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               preview_path TEXT NOT NULL DEFAULT '',
               preview_status TEXT NOT NULL DEFAULT 'pending',
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               file_size INTEGER NOT NULL DEFAULT 0,
               modified_unix_ms INTEGER NOT NULL DEFAULT 0,
               metadata_engine_id TEXT NOT NULL DEFAULT 'unknown',
               metadata_engine_version TEXT NOT NULL DEFAULT '0',
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT,
               capture_raw_value TEXT,
               file_identity_scheme TEXT,
               file_identity_value TEXT
             );
             CREATE TABLE preview_artifacts (
               artifact_path TEXT NOT NULL,
               lifecycle_state TEXT NOT NULL
             );
             CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL
             );
             CREATE TABLE preserved_fixture(value TEXT NOT NULL);
             INSERT INTO preserved_fixture(value) VALUES ('kept');",
        )
        .expect("v16 fixture");
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (
        version,
        revision,
        preserved,
        queue_exists,
        scan_owner_exists,
        contract_valid,
        generation,
        is_active,
    ): (i64, i64, String, bool, bool, bool, i64, bool) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT revision FROM catalog_state),
               (SELECT value FROM preserved_fixture),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_queue'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue')
                 WHERE name = 'authoritative_scan_id'),
               (SELECT root_authority_complete = 1
                FROM library_change_queue_contract WHERE singleton = 1),
               (SELECT generation FROM library_change_root_state WHERE root_id = 'root-a'),
               (SELECT is_active FROM library_change_root_state WHERE root_id = 'root-a')",
            [],
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
                ))
            },
        )
        .expect("migrated evidence");

    assert_eq!(version, super::super::SCHEMA_VERSION);
    assert_eq!(revision, 7);
    assert_eq!(preserved, "kept");
    assert!(queue_exists);
    assert!(scan_owner_exists);
    assert!(contract_valid);
    assert_eq!(generation, 1);
    assert!(is_active);
}

#[test]
fn replacement_scan_cannot_consume_live_evidence_arriving_after_enumeration() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path.clone());
    let initial = scan_request("initial-scan");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("initial-scan")
        .expect("prove initial first-import handoff");
    catalog
        .publish_scan("initial-scan", "root-a", 0, 0)
        .expect("publish initial scan");
    let request = scan_request("authoritative-scan");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin authoritative scan");
    assert_eq!(
        catalog
            .claim_next_directory("authoritative-scan")
            .expect("claim root directory")
            .as_deref(),
        Some("")
    );
    catalog
        .complete_directory_enumeration("authoritative-scan", "")
        .expect("complete root enumeration before the live file appears");

    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                2,
                1_001,
                "arrived-during-scan.jpg",
            )],
            1_001,
            policy,
        )
        .expect("enqueue evidence during scan");
    let publication = catalog
        .publish_scan("authoritative-scan", "root-a", 0, 0)
        .expect_err("unfinished live evidence must delay scan publication");
    assert_eq!(publication.code, "catalog_scan_live_changes_pending");

    let rows = catalog
        .connection
        .prepare(
            "SELECT id, status, authoritative_scan_id
             FROM library_change_queue ORDER BY id",
        )
        .expect("queue query")
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .expect("queue rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("queue evidence");
    let metrics = catalog
        .load_library_change_queue_metrics(1_001, policy)
        .expect("queue metrics");

    assert!(!report.freshness_unknown_enqueued);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "pending");
    assert!(rows.iter().all(|row| row.2.is_none()));
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.completed_count, 0);
}

#[test]
fn first_import_publishes_before_its_durable_exact_path_catch_up() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let initial = scan_request("initial-scan");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("initial-scan")
        .expect("prove initial first-import handoff");
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "arrived-during-first-import.jpg",
            )],
            1_000,
            policy,
        )
        .expect("persist first-import path catch-up");

    catalog
        .publish_scan("initial-scan", "root-a", 0, 0)
        .expect("publish first snapshot before path catch-up becomes processable");
    let state: (String, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT queue.status, roots.active_scan_id
             FROM library_change_queue AS queue
             JOIN library_roots AS roots ON roots.id = queue.root_id
             WHERE queue.relative_path = 'arrived-during-first-import.jpg'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load durable first-import catch-up");
    assert_eq!(state.0, "pending");
    assert_eq!(state.1.as_deref(), Some("initial-scan"));
}

#[test]
fn first_import_keeps_unresolved_root_freshness_after_baseline_publication() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let initial = scan_request("initial-scan");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("initial-scan")
        .expect("prove initial first-import handoff");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("persist root freshness gap");

    catalog
        .publish_scan("initial-scan", "root-a", 0, 0)
        .expect("a first snapshot makes retained root-gap recovery processable");
    let retained: (String, Option<i64>, String, String) = catalog
        .connection
        .query_row(
            "SELECT queue.status, queue.catalog_revision_at_success,
                roots.active_scan_id, journal.continuity_state
         FROM library_change_queue AS queue
         JOIN library_roots AS roots ON roots.id = queue.root_id
         JOIN library_persistent_journal_root_state AS journal ON journal.root_id = roots.id
         WHERE queue.root_id = 'root-a' AND queue.intent_kind = 'freshness_unknown'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("preserved root-gap authority");
    assert_eq!(
        retained,
        (
            "pending".to_owned(),
            None,
            "initial-scan".to_owned(),
            "live_only".to_owned()
        )
    );
}

#[test]
fn replacement_scan_publishes_while_an_exact_path_retry_remains_durable() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    let initial = scan_request("initial-scan");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("initial-scan")
        .expect("prove initial first-import handoff");
    catalog
        .publish_scan("initial-scan", "root-a", 0, 0)
        .expect("publish initial scan");

    let replacement = scan_request("replacement-scan");
    catalog
        .begin_scan(&replacement, "root-a", &replacement.root_path)
        .expect("begin replacement scan");
    assert_eq!(
        catalog
            .claim_next_directory("replacement-scan")
            .expect("claim root directory")
            .as_deref(),
        Some("")
    );
    catalog
        .complete_directory_enumeration("replacement-scan", "")
        .expect("complete root enumeration before the live file appears");

    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "arrived-after-enumeration.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue live file after enumeration");
    let leased = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_000,
            policy,
        )
        .expect("lease live file through running scan")
        .pop()
        .expect("live file lease");
    catalog
        .retry_library_change(
            leased.change.id,
            leased.lease_generation,
            &LibraryChangeFailure {
                code: "path_locked".to_owned(),
                message: "The new path remained locked.".to_owned(),
            },
            1_001,
            policy,
        )
        .expect("record exhausted live retry");

    catalog
        .publish_scan("replacement-scan", "root-a", 0, 0)
        .expect("an exact path retry must not discard an otherwise publishable snapshot");
    let retained_debt: (String, i64, Option<i64>, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT status, attempt_count, next_retry_unix_ms, authoritative_scan_id
             FROM library_change_queue WHERE id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load exhausted live retry");
    assert_eq!(retained_debt, ("retry_wait".to_owned(), 1, None, None));
    let active_scan: Option<String> = catalog
        .connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = 'root-a'",
            [],
            |row| row.get(0),
        )
        .expect("load published root");
    assert_eq!(active_scan.as_deref(), Some("replacement-scan"));
}

#[test]
fn replacement_scan_rejects_path_shaped_retry_with_root_freshness_authority() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    let initial = scan_request("initial-scan");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("initial-scan")
        .expect("prove initial first-import handoff");
    catalog
        .publish_scan("initial-scan", "root-a", 0, 0)
        .expect("publish initial scan");
    let replacement = scan_request("replacement-scan");
    catalog
        .begin_scan(&replacement, "root-a", &replacement.root_path)
        .expect("begin replacement scan");
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "unverifiable.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue path evidence");
    let leased = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_000,
            policy,
        )
        .expect("lease path evidence")
        .pop()
        .expect("path lease");
    catalog
        .retry_library_change(
            leased.change.id,
            leased.lease_generation,
            &LibraryChangeFailure {
                code: "path_locked".to_owned(),
                message: "The path remained locked.".to_owned(),
            },
            1_001,
            policy,
        )
        .expect("record exhausted retry");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET intent_kind = 'freshness_unknown'
             WHERE id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
        )
        .expect("construct legacy path-shaped freshness debt");

    let blocked = catalog
        .publish_scan("replacement-scan", "root-a", 0, 0)
        .expect_err("root-authority debt must remain publication-blocking");
    assert_eq!(blocked.code, "catalog_scan_live_changes_pending");
}

#[test]
fn abandoning_authoritative_scan_preserves_live_work_and_releases_only_frozen_work() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "worker-owned.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue worker-owned path");
    let worker_lease = catalog
        .lease_path_library_changes("root-a", generation, 1_000, policy)
        .expect("lease worker path");
    assert_eq!(worker_lease.len(), 1);
    let mut scan_owned = path_intent("root-a", generation, 2, 1_001, "scan-owned.jpg");
    scan_owned.origin = LibraryChangeOrigin::StartupCatchUp;
    catalog
        .enqueue_library_change_intents(&[scan_owned], 1_001, policy)
        .expect("enqueue scan-owned journal path");
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                3,
                1_002,
                "pending-live.jpg",
            )],
            1_002,
            policy,
        )
        .expect("enqueue pending live path");
    let request = scan_request("abandoned-authoritative-scan");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin authoritative scan");
    let ownership = catalog
        .connection
        .prepare(
            "SELECT status, authoritative_scan_id
             FROM library_change_queue ORDER BY id",
        )
        .expect("prepare scan ownership evidence")
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .expect("query scan ownership evidence")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect scan ownership evidence");
    assert_eq!(
        ownership,
        vec![
            ("leased".to_owned(), None),
            (
                "leased".to_owned(),
                Some("abandoned-authoritative-scan".to_owned()),
            ),
            ("pending".to_owned(), None),
        ]
    );
    catalog
        .abandon_scan("abandoned-authoritative-scan", "stale", 1)
        .expect("abandon authoritative scan");

    let rows = catalog
        .connection
        .prepare(
            "SELECT id, status, ready_unix_ms, lease_expires_unix_ms, authoritative_scan_id
             FROM library_change_queue ORDER BY id",
        )
        .expect("queue query")
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .expect("queue rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("queue evidence");

    assert_eq!(rows.len(), 3);
    assert_eq!(
        u64::try_from(rows[0].0).expect("worker queue id"),
        worker_lease[0].change.id.value()
    );
    assert_eq!(rows[0].1, "leased");
    assert!(rows[0].3.is_some());
    assert_eq!(rows[0].4, None);
    assert_eq!(rows[1].1, "pending");
    assert_eq!(rows[1].3, None);
    assert_eq!(rows[1].4, None);
    assert_eq!(rows[2].1, "pending");
    assert_eq!(rows[2].3, None);
    assert_eq!(rows[2].4, None);
    let released = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Journal,
            rows[1].2,
            policy,
        )
        .expect("lease released scan work");
    assert_eq!(released.len(), 1);
    assert_eq!(
        released[0].change.id.value(),
        u64::try_from(rows[1].0).expect("released queue id")
    );
    let live = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            rows[2].2,
            policy,
        )
        .expect("lease live work retained outside scan ownership");
    assert_eq!(live.len(), 1);
    assert_eq!(
        live[0].change.id.value(),
        u64::try_from(rows[2].0).expect("live queue id")
    );
}

#[test]
fn prerelease_v17_without_root_authority_marker_fails_closed_on_open() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("prerelease v17 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info(version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (17);
             CREATE TABLE catalog_state(revision INTEGER NOT NULL);
             INSERT INTO catalog_state(revision) VALUES (7);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY,
               path TEXT NOT NULL UNIQUE,
               active_scan_id TEXT,
               created_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE library_change_root_state (
               root_id TEXT PRIMARY KEY,
               generation INTEGER NOT NULL CHECK(generation > 0),
               is_active INTEGER NOT NULL CHECK(is_active IN (0, 1)),
               updated_unix_ms INTEGER NOT NULL
             );",
        )
        .expect("pre-fix schema fixture with a lost tombstone");
    drop(connection);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("unverifiable v17 must fail closed"),
        Err(error) => error,
    };

    assert_eq!(error.code, "catalog_change_queue_authority_unverifiable");
}

#[test]
fn repeated_plans_survive_restart_as_one_minimum_reconciliation() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let context = planning_context("root-a", generation);
    let first_plan = plan_library_changes(
        &context,
        vec![
            observation("root-a", generation, 1, 1_000, "album/photo.jpg"),
            observation("root-a", generation, 2, 1_050, "album/photo.jpg"),
            observation("root-a", generation, 3, 1_100, "album/photo.jpg"),
        ],
        LibraryChangePlanningLimits::default(),
    )
    .expect("first plan");
    let second_plan = plan_library_changes(
        &context,
        vec![observation(
            "root-a",
            generation,
            4,
            1_200,
            "album/photo.jpg",
        )],
        LibraryChangePlanningLimits::default(),
    )
    .expect("second plan");
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path.clone());

    enqueue_library_change_plan(&mut catalog, &first_plan, 1_100, policy)
        .expect("enqueue first plan");
    let report = enqueue_library_change_plan(&mut catalog, &second_plan, 1_200, policy)
        .expect("coalesce second plan");
    assert_eq!(report.coalesced_count, 1);
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopened catalog");
    assert!(
        reopened
            .lease_library_changes("root-a", generation, 1_699, policy)
            .expect("lease before debounce")
            .is_empty()
    );
    let leased = reopened
        .lease_library_changes("root-a", generation, 1_700, policy)
        .expect("lease after restart");

    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.intent.relative_path, "album/photo.jpg");
    assert_eq!(leased[0].change.intent.coalesced_observation_count, 4);
    assert_eq!(leased[0].change.intent.first_sequence, 1);
    assert_eq!(leased[0].change.intent.most_recent_sequence, 4);
}

#[test]
fn source_restart_sequence_reset_preserves_the_newer_evidence_tuple() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 800, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue evidence from the first source instance");
    let restarted = path_intent("root-a", generation, 1, 2_000, "photo.jpg");

    let report = catalog
        .enqueue_library_change_intents(&[restarted], 2_000, policy)
        .expect("enqueue evidence after source restart");
    let leased = catalog
        .lease_library_changes("root-a", generation, 2_000, policy)
        .expect("lease coalesced work")
        .pop()
        .expect("coalesced work");

    assert_eq!(report.coalesced_count, 1);
    assert_eq!(leased.change.intent.first_sequence, 1);
    assert_eq!(leased.change.intent.most_recent_sequence, 1);
    assert_eq!(leased.change.intent.first_observed_unix_ms, 1_000);
    assert_eq!(leased.change.intent.most_recent_observed_unix_ms, 2_000);
    assert_eq!(
        leased.change.intent.origin,
        LibraryChangeOrigin::LiveNotification,
    );
}

#[test]
fn equal_timestamp_sequence_reset_prefers_later_durable_ingress() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 800, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue evidence from the first source instance");
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue equal-time evidence after source restart");

    let leased = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease coalesced work")
        .pop()
        .expect("coalesced work");

    assert_eq!(leased.change.intent.first_sequence, 1);
    assert_eq!(leased.change.intent.most_recent_sequence, 1);
    assert_eq!(leased.change.intent.most_recent_observed_unix_ms, 1_000);
    assert_eq!(
        leased.change.intent.origin,
        LibraryChangeOrigin::LiveNotification,
    );
}

#[test]
fn wall_clock_rollback_cannot_preserve_an_older_source_tuple_or_deadline() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 800, 2_000, "photo.jpg")],
            2_000,
            policy,
        )
        .expect("enqueue evidence before clock rollback");
    let restarted = path_intent("root-a", generation, 1, 1_000, "photo.jpg");
    catalog
        .enqueue_library_change_intents(&[restarted], 1_000, policy)
        .expect("enqueue later ingress after clock rollback");

    let leased = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease without the obsolete future deadline")
        .pop()
        .expect("coalesced work");

    assert_eq!(leased.change.intent.first_observed_unix_ms, 1_000);
    assert_eq!(leased.change.intent.most_recent_observed_unix_ms, 1_000);
    assert_eq!(leased.change.intent.most_recent_sequence, 1);
    assert_eq!(
        leased.change.intent.origin,
        LibraryChangeOrigin::LiveNotification
    );
}

#[test]
fn equal_timestamp_origin_change_uses_later_durable_ingress() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let mut older = path_intent("root-a", generation, 800, 1_000, "photo.jpg");
    older.origin = LibraryChangeOrigin::ConsistencyAudit;
    catalog
        .enqueue_library_change_intents(&[older], 1_000, policy)
        .expect("enqueue older origin");
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue later live ingress");

    let leased = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease coalesced work")
        .pop()
        .expect("coalesced work");

    assert_eq!(leased.change.intent.most_recent_sequence, 1);
    assert_eq!(
        leased.change.intent.origin,
        LibraryChangeOrigin::LiveNotification,
    );
}

#[test]
fn absorption_and_degradation_keep_the_later_ingress_tuple() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let mut absorbing_catalog = queue_catalog(directory.path().join("absorbing.sqlite3"));
    absorbing_catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                800,
                2_000,
                "album/photo.jpg",
            )],
            2_000,
            immediate_policy(),
        )
        .expect("enqueue older child work");
    absorbing_catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            immediate_policy(),
        )
        .expect("absorb child after clock rollback");
    let absorbed = absorbing_catalog
        .lease_library_changes("root-a", generation, 1_000, immediate_policy())
        .expect("lease absorbed work")
        .pop()
        .expect("absorbed work");

    let capacity_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut degrading_catalog = queue_catalog(directory.path().join("degrading.sqlite3"));
    degrading_catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 800, 2_000, "a.jpg")],
            2_000,
            capacity_policy,
        )
        .expect("enqueue older bounded work");
    degrading_catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "b.jpg")],
            1_000,
            capacity_policy,
        )
        .expect("degrade after clock rollback");
    let degraded = degrading_catalog
        .lease_library_changes("root-a", generation, 1_000, capacity_policy)
        .expect("lease degraded work")
        .pop()
        .expect("degraded work");

    for change in [absorbed, degraded] {
        assert_eq!(change.change.intent.most_recent_observed_unix_ms, 1_000);
        assert_eq!(change.change.intent.most_recent_sequence, 1);
        assert_eq!(
            change.change.intent.origin,
            LibraryChangeOrigin::LiveNotification,
        );
    }
}

#[test]
fn create_then_remove_remains_one_final_state_reconciliation() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let context = planning_context("root-a", generation);
    let mut created = observation("root-a", generation, 1, 1_000, "transient.jpg");
    created.kind = LibraryChangeObservationKind::Created;
    let mut removed = observation("root-a", generation, 2, 1_100, "transient.jpg");
    removed.kind = LibraryChangeObservationKind::Removed;
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path);
    for event in [created, removed] {
        let plan = plan_library_changes(
            &context,
            vec![event],
            LibraryChangePlanningLimits::default(),
        )
        .expect("plan event");
        enqueue_library_change_plan(
            &mut catalog,
            &plan,
            plan.intents[0].most_recent_observed_unix_ms,
            policy,
        )
        .expect("enqueue plan");
    }
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_600, policy)
        .expect("lease final-state check");

    assert_eq!(leased.len(), 1);
    assert_eq!(
        leased[0].change.intent.kind,
        LibraryChangeIntentKind::Reconcile,
    );
    assert_eq!(leased[0].change.intent.coalesced_observation_count, 2);
}

#[test]
fn live_rename_then_same_target_dirty_reconcile_preserves_both_semantics() {
    assert_live_dirty_rename_coalescing(true);
}

#[test]
fn live_dirty_reconcile_then_same_target_rename_preserves_both_semantics() {
    assert_live_dirty_rename_coalescing(false);
}

#[test]
fn startup_catch_up_canonicalization_preserves_dirty_rename_lineage_in_any_order() {
    let generation = LibraryRootGeneration::initial();
    let mut rename = rename_intent(
        "root-a",
        generation,
        1,
        1_000,
        "old/photo.jpg",
        "new/photo.jpg",
    );
    rename.origin = LibraryChangeOrigin::StartupCatchUp;
    let mut dirty = path_intent("root-a", generation, 2, 1_001, "new/photo.jpg");
    dirty.origin = LibraryChangeOrigin::StartupCatchUp;

    let rename_first = normalize_persistent_journal_intents(&[rename.clone(), dirty.clone()])
        .expect("normalize rename then dirty");
    let dirty_first = normalize_persistent_journal_intents(&[dirty, rename])
        .expect("normalize dirty then rename");

    assert_eq!(rename_first, dirty_first);
    assert_eq!(rename_first.len(), 1);
    assert_dirty_rename_intent(&rename_first[0]);
}

#[test]
fn startup_catch_up_dirty_rename_conflict_degrades_to_a_root_gap() {
    let generation = LibraryRootGeneration::initial();
    let mut rename = rename_intent(
        "root-a",
        generation,
        1,
        1_000,
        "old/photo.jpg",
        "new/photo.jpg",
    );
    rename.origin = LibraryChangeOrigin::StartupCatchUp;
    let mut dirty = path_intent("root-a", generation, 2, 1_001, "new/photo.jpg");
    dirty.origin = LibraryChangeOrigin::StartupCatchUp;
    let mut conflicting = rename_intent(
        "root-a",
        generation,
        3,
        1_002,
        "old/photo.jpg",
        "elsewhere/photo.jpg",
    );
    conflicting.origin = LibraryChangeOrigin::StartupCatchUp;

    let normalized = normalize_persistent_journal_intents(&[conflicting, dirty, rename])
        .expect("normalize conflicting dirty rename evidence");

    assert_eq!(normalized.len(), 1);
    assert_eq!(
        normalized[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(normalized[0].scope, LibraryChangeScope::Root);
    assert!(normalized[0].relative_path.is_empty());
    assert!(normalized[0].previous_relative_path.is_none());
    assert_eq!(normalized[0].first_sequence, 1);
    assert_eq!(normalized[0].most_recent_sequence, 3);
    assert_eq!(normalized[0].coalesced_observation_count, 3);
}

#[test]
fn paired_rename_persists_both_paths_across_restart() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut rename = intent(
        "root-a",
        generation,
        1,
        1_000,
        LibraryChangeIntentKind::RenameCandidate,
        LibraryChangeScope::Path,
        "new/photo.jpg",
    );
    rename.previous_relative_path = Some("old/photo.jpg".to_owned());
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(&[rename], 1_000, policy)
        .expect("enqueue rename");
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopened catalog");
    let leased = reopened
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease rename");

    assert_eq!(leased.len(), 1);
    assert_eq!(
        leased[0].change.intent.kind,
        LibraryChangeIntentKind::RenameCandidate,
    );
    assert_eq!(leased[0].change.intent.relative_path, "new/photo.jpg");
    assert_eq!(
        leased[0].change.intent.previous_relative_path.as_deref(),
        Some("old/photo.jpg"),
    );
}

#[test]
fn divergent_renames_from_one_old_path_degrade_to_a_root_gap() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                1,
                1_000,
                "old/photo.jpg",
                "new-a/photo.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue first rename");

    let report = catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                2,
                1_001,
                "old/photo.jpg",
                "new-b/photo.jpg",
            )],
            1_001,
            policy,
        )
        .expect("degrade conflicting rename");
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease root gap");

    assert_eq!(report.superseded_count, 1);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(leased.len(), 1);
    assert_eq!(
        leased[0].change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
    assert_eq!(leased[0].change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(leased[0].change.intent.coalesced_observation_count, 2);
}

#[test]
fn later_old_path_evidence_preserves_precise_work_and_invalidates_a_leased_rename() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                1,
                1_000,
                "old/photo.jpg",
                "new/photo.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue rename");
    let rename_lease = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease rename")
        .pop()
        .expect("leased rename");

    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "old/photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue old-path evidence");
    let stale_outcome = catalog
        .complete_library_change(
            rename_lease.change.id,
            rename_lease.lease_generation,
            0,
            1_002,
        )
        .expect("reject stale rename completion");
    let precise = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_001,
            policy,
        )
        .expect("lease precise old-path reconciliation")
        .pop()
        .expect("precise old-path reconciliation");
    let recovery = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_001, policy)
        .expect("lease conservative rename recovery")
        .expect("conservative rename recovery");

    assert_eq!(stale_outcome, LibraryChangeLeaseUpdateOutcome::Superseded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(
        precise.change.intent.kind,
        LibraryChangeIntentKind::Reconcile,
    );
    assert_eq!(precise.change.intent.scope, LibraryChangeScope::Path);
    assert_eq!(precise.change.intent.relative_path, "old/photo.jpg");
    assert_eq!(
        recovery.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
    assert_eq!(recovery.change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(recovery.change.intent.coalesced_observation_count, 2);
}

#[test]
fn partial_subtree_overlap_invalidates_a_cross_subtree_rename_lease() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                1,
                1_000,
                "album/old/photo.jpg",
                "outside/photo.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue cross-subtree rename");
    let rename_lease = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease rename")
        .pop()
        .expect("leased rename");

    let report = catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                2,
                1_001,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_001,
            policy,
        )
        .expect("enqueue partially overlapping subtree");
    let stale_outcome = catalog
        .complete_library_change(
            rename_lease.change.id,
            rename_lease.lease_generation,
            0,
            1_002,
        )
        .expect("reject stale rename completion");
    let replacement = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease conservative replacement")
        .pop()
        .expect("replacement root work");

    assert_eq!(stale_outcome, LibraryChangeLeaseUpdateOutcome::Superseded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(
        replacement.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
    assert_eq!(replacement.change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(replacement.change.intent.coalesced_observation_count, 2);
}

#[test]
fn old_subtree_descendant_preserves_precise_work_and_invalidates_a_leased_directory_rename() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut rename = rename_intent("root-a", generation, 1, 1_000, "old/album", "new/album");
    rename.scope = LibraryChangeScope::Subtree;
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(&[rename], 1_000, policy)
        .expect("enqueue directory rename");
    let rename_lease = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease directory rename")
        .pop()
        .expect("leased directory rename");

    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                2,
                1_001,
                "old/album/new.jpg",
            )],
            1_001,
            policy,
        )
        .expect("enqueue old subtree descendant");
    let stale_outcome = catalog
        .complete_library_change(
            rename_lease.change.id,
            rename_lease.lease_generation,
            0,
            1_002,
        )
        .expect("reject stale directory rename completion");
    let precise = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_001,
            policy,
        )
        .expect("lease precise descendant reconciliation")
        .pop()
        .expect("precise descendant reconciliation");
    let recovery = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_001, policy)
        .expect("lease conservative namespace recovery")
        .expect("conservative namespace recovery");

    assert_eq!(stale_outcome, LibraryChangeLeaseUpdateOutcome::Superseded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(
        precise.change.intent.kind,
        LibraryChangeIntentKind::Reconcile,
    );
    assert_eq!(precise.change.intent.scope, LibraryChangeScope::Path);
    assert_eq!(precise.change.intent.relative_path, "old/album/new.jpg");
    assert!(precise.change.intent.previous_relative_path.is_none());
    assert_eq!(
        recovery.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
    assert_eq!(recovery.change.intent.scope, LibraryChangeScope::Root);
}

#[test]
fn divergent_nested_directory_renames_degrade_to_a_root_gap() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut first = rename_intent("root-a", generation, 1, 1_000, "old/album", "new/album");
    first.scope = LibraryChangeScope::Subtree;
    let mut second = rename_intent(
        "root-a",
        generation,
        2,
        1_001,
        "old/album/nested",
        "elsewhere/nested",
    );
    second.scope = LibraryChangeScope::Subtree;
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(&[first], 1_000, policy)
        .expect("enqueue first directory rename");
    let report = catalog
        .enqueue_library_change_intents(&[second], 1_001, policy)
        .expect("degrade conflicting nested rename");
    let root = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease root gap")
        .pop()
        .expect("root gap");

    assert!(report.freshness_unknown_enqueued);
    assert_eq!(root.change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(
        root.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
}

#[test]
fn parent_subtree_keeps_precise_dirty_child_work_across_restart() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path.clone());

    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Path,
                "album/photo.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue child");
    let report = catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                2,
                1_100,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_100,
            policy,
        )
        .expect("enqueue subtree");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen queue catalog");
    let metrics = catalog
        .load_library_change_queue_metrics(1_100, policy)
        .expect("metrics");
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_600, policy)
        .expect("lease precise child");

    assert_eq!(report.inserted_count, 1);
    assert_eq!(report.superseded_count, 0);
    assert_eq!(metrics.pending_count, 2);
    assert_eq!(metrics.superseded_count, 0);
    assert_eq!(leased.len(), 2);
    let precise = leased
        .iter()
        .find(|change| change.change.intent.scope == LibraryChangeScope::Path)
        .expect("precise child remains independently leased");
    assert_eq!(precise.change.intent.relative_path, "album/photo.jpg");
    assert_eq!(precise.change.intent.coalesced_observation_count, 1);
}

#[test]
fn directory_rename_does_not_absorb_a_precise_dirty_descendant() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path.clone());
    let mut rename = intent(
        "root-a",
        generation,
        1,
        1_000,
        LibraryChangeIntentKind::RenameCandidate,
        LibraryChangeScope::Subtree,
        "new",
    );
    rename.previous_relative_path = Some("old".to_owned());
    catalog
        .enqueue_library_change_intents(&[rename], 1_000, policy)
        .expect("enqueue directory rename");
    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "new/photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue dirty descendant");
    drop(catalog);

    let catalog = SqliteCatalog::open(path).expect("reopen queue catalog");
    let shape: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT COUNT(*),
                    SUM(scope = 'subtree' AND intent_kind = 'rename_candidate'),
                    SUM(scope = 'path' AND intent_kind = 'reconcile')
             FROM library_change_queue
             WHERE root_id = 'root-a' AND status = 'pending'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load retained dirty rename evidence");

    assert_eq!(report.inserted_count, 1);
    assert_eq!(report.coalesced_count, 0);
    assert_eq!(report.superseded_count, 0);
    assert_eq!(shape, (2, 1, 1));
}

#[test]
fn root_metrics_are_isolated_from_other_roots() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let transaction = catalog.connection.transaction().expect("root transaction");
    transaction
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('root-b', 'C:\\Other', 1)",
            [],
        )
        .expect("register root-b");
    let other_generation =
        activate_root_change_queue(&transaction, "root-b", 1).expect("root-b generation authority");
    transaction.commit().expect("commit root-b registration");
    assert_eq!(other_generation, generation);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "a.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue root-a change");
    let second_report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-b", generation, 1, 1_000, "b.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue root-b change");
    assert_eq!(second_report.inserted_count, 1);
    assert_eq!(second_report.stale_generation_count, 0);

    let root_metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 1_000, policy)
        .expect("root metrics");
    let other_root_metrics = catalog
        .load_library_change_root_queue_metrics("root-b", generation, 1_000, policy)
        .expect("other root metrics");
    let global_metrics = catalog
        .load_library_change_queue_metrics(1_000, policy)
        .expect("global metrics");

    assert_eq!(root_metrics.pending_count, 1);
    assert_eq!(other_root_metrics.pending_count, 1);
    assert_eq!(global_metrics.pending_count, 2);
}

#[test]
fn adapter_rejects_non_normalized_intents_before_persistence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let error = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "album\\photo.jpg",
            )],
            1_000,
            policy,
        )
        .expect_err("non-normalized path");
    let metrics = catalog
        .load_library_change_queue_metrics(1_000, policy)
        .expect("empty metrics");

    assert_eq!(error.code, "change_queue_intent_invalid");
    assert_eq!(metrics.pending_count, 0);
}

#[test]
fn later_same_path_invalidates_the_earlier_lease() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue first");
    let first_lease = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("first lease")
        .pop()
        .expect("leased change");

    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue later evidence");
    let stale_outcome = catalog
        .complete_library_change(
            first_lease.change.id,
            first_lease.lease_generation,
            0,
            1_002,
        )
        .expect("reject stale completion");
    let replacement = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("replacement lease")
        .pop()
        .expect("replacement work");

    assert_eq!(stale_outcome, LibraryChangeLeaseUpdateOutcome::Superseded);
    assert_ne!(replacement.change.id, first_lease.change.id);
    assert_eq!(replacement.change.intent.coalesced_observation_count, 2);
}

#[test]
fn newer_root_generation_supersedes_old_work_and_rejects_late_enqueue() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let first_generation = LibraryRootGeneration::initial();
    let next_generation = first_generation.next().expect("next generation");
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", first_generation, 1, 1_000, "old.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue old generation");
    let pending_gap_id = seed_pending_journal_claim(&mut catalog, 1_010);
    let explicit_gap_id = seed_explicit_recovery_claim(&catalog, 1_020);
    assert!(
        catalog
            .connection
            .query_row(
                "SELECT next_retry_unix_ms IS NOT NULL
                 FROM library_change_queue WHERE id = ?1",
                [pending_gap_id],
                |row| row.get::<_, bool>(0),
            )
            .expect("pending-journal retry deadline"),
        "pending-journal retirement must accept its scheduled retry deadline"
    );
    let mut retained_range = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "0".repeat(64),
            root_id: "root-a".to_owned(),
            root_generation: first_generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "watcher-gap-volume".to_owned(),
                volume_serial: 41,
            },
            journal_id: JournalIdentifier::new(83).expect("journal"),
            requested_start_usn: JournalUsn::new(1_024).expect("start"),
            requested_end_usn: JournalUsn::new(1_034).expect("end"),
            covered_until_usn: JournalUsn::new(1_034).expect("covered"),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 1_030,
            checkpointed_unix_ms: None,
        },
        intents: Vec::new(),
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    retained_range.range.batch_id = persistent_journal_batch_id(&retained_range);
    catalog
        .enroll_persistent_journal_batch(&retained_range, 1_030, policy)
        .expect("seed durable range for old generation");
    let replacement_report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", next_generation, 2, 1_100, "new.jpg")],
            1_100,
            policy,
        )
        .expect("enqueue new generation");
    let stale_report = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                first_generation,
                3,
                1_200,
                "late.jpg",
            )],
            1_200,
            policy,
        )
        .expect("reject stale generation");

    assert_eq!(replacement_report.superseded_count, 3);
    assert_eq!(stale_report.stale_generation_count, 1);
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_root_state
                 WHERE root_id = 'root-a'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("retained journal generations"),
        2
    );
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_source_ranges
                 WHERE id = ?1 AND root_generation = 1",
                [retained_range.range.batch_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("retained historical source range"),
        1
    );
    assert!(
        catalog
            .lease_library_changes("root-a", first_generation, 1_200, policy)
            .expect("old generation lease")
            .is_empty()
    );
    let current = catalog
        .lease_library_changes("root-a", next_generation, 1_200, policy)
        .expect("new generation lease");
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].change.intent.relative_path, "new.jpg");
    let retired_claims: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_live_gap_recovery_claims
             WHERE gap_change_id IN (?1, ?2)",
            rusqlite::params![pending_gap_id, explicit_gap_id],
            |row| row.get(0),
        )
        .expect("retired claims");
    let retained_gaps: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT COUNT(*), SUM(status = 'superseded')
             FROM library_change_queue WHERE id IN (?1, ?2)",
            rusqlite::params![pending_gap_id, explicit_gap_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained terminal gaps");
    assert_eq!(retired_claims, 0);
    assert_eq!(retained_gaps, (2, 2));
    drop(catalog);
    SqliteCatalog::open(path).expect("reopen advanced-generation catalog");
}

#[test]
fn unregistering_a_root_retires_its_generation_and_unresolved_work() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue work");
    let explicit_gap_id = seed_explicit_recovery_claim(&catalog, 1_010);

    assert!(catalog.unregister_root("root-a").expect("unregister root"));
    let stale = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_100, "late.jpg")],
            1_100,
            policy,
        )
        .expect("reject retired generation");
    let metrics = catalog
        .load_library_change_queue_metrics(1_100, policy)
        .expect("retired metrics");

    assert_eq!(stale.stale_generation_count, 1);
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.superseded_count, 2);
    let retired_evidence: (i64, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                WHERE gap_change_id = ?1),
               status
             FROM library_change_queue WHERE id = ?1",
            [explicit_gap_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained retired gap");
    assert_eq!(retired_evidence, (0, "superseded".to_owned()));
    drop(catalog);
    SqliteCatalog::open(path).expect("reopen unregistered-root catalog");
}

#[test]
fn generation_retirement_rolls_back_claim_release_when_superseding_fails() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let first_generation = LibraryRootGeneration::initial();
    let next_generation = first_generation.next().expect("next generation");
    let mut catalog = queue_catalog(path);
    let explicit_gap_id = seed_explicit_recovery_claim(&catalog, 1_000);
    catalog
        .connection
        .execute_batch(&format!(
            "CREATE TRIGGER reject_test_gap_retirement
             BEFORE UPDATE OF status ON library_change_queue
             WHEN OLD.id = {explicit_gap_id} AND NEW.status = 'superseded'
             BEGIN
               SELECT RAISE(ABORT, 'injected retirement failure');
             END;"
        ))
        .expect("install retirement failure injection");

    let error = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", next_generation, 2, 1_100, "new.jpg")],
            1_100,
            immediate_policy(),
        )
        .expect_err("superseding failure must roll back claim release");
    assert_eq!(error.code, "catalog_database_error");
    let retained: (i64, String, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                WHERE gap_change_id = ?1),
               gap.status,
               (SELECT generation FROM library_change_root_state WHERE root_id = 'root-a')
             FROM library_change_queue AS gap WHERE gap.id = ?1",
            [explicit_gap_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("rolled-back retirement evidence");
    assert_eq!(retained, (1, "retry_wait".to_owned(), 1));
}

#[test]
fn generation_retirement_rejects_an_active_foreground_live_gap_owner() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let first_generation = LibraryRootGeneration::initial();
    let next_generation = first_generation.next().expect("next generation");
    let mut catalog = queue_catalog(path);
    let gap_change_id = seed_explicit_recovery_claim(&catalog, 1_000);
    catalog
        .connection
        .execute(
            "INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, preview_edge,
               root_generation_at_start, scan_owner
             ) VALUES ('active-foreground', 'root-a', 'running', 1001, 128, 1, 'foreground')",
            [],
        )
        .expect("active foreground scan");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'leased', lease_expires_unix_ms = 5000,
                 authoritative_scan_id = 'active-foreground',
                 last_failure_code = 'live_gap_v30_explicit_recovery_in_progress',
                 last_failure_message = 'The explicit recovery is in progress'
             WHERE id = ?1",
            [gap_change_id],
        )
        .expect("lease gap to foreground scan");
    catalog
        .connection
        .execute(
            "UPDATE library_live_gap_recovery_claims
             SET consumer_kind = 'foreground_scan', foreground_scan_id = 'active-foreground'
             WHERE gap_change_id = ?1",
            [gap_change_id],
        )
        .expect("bind active foreground claim");

    let error = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", next_generation, 2, 1_100, "new.jpg")],
            1_100,
            immediate_policy(),
        )
        .expect_err("active foreground ownership requires explicit abandonment");
    assert_eq!(error.code, "live_gap_retirement_consumer_conflict");
    let retained: (String, String, i64) = catalog
        .connection
        .query_row(
            "SELECT claim.consumer_kind, gap.status,
                    (SELECT generation FROM library_change_root_state WHERE root_id = 'root-a')
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.gap_change_id = ?1",
            [gap_change_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained foreground ownership");
    assert_eq!(
        retained,
        ("foreground_scan".to_owned(), "leased".to_owned(), 1),
    );
}

#[test]
fn registered_root_without_queue_work_rejects_events_after_retirement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = queue_catalog(path);
    assert!(catalog.unregister_root("root-a").expect("unregister root"));
    let late_generation = LibraryRootGeneration::new(7).expect("late generation");

    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", late_generation, 1, 1_000, "late.jpg")],
            1_000,
            immediate_policy(),
        )
        .expect("reject removed root");

    assert_eq!(report.stale_generation_count, 1);
    assert_eq!(report.inserted_count, 0);
}

#[test]
fn missing_generation_authority_fails_closed_instead_of_trusting_an_event() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::new(7).expect("event generation");
    let mut catalog = queue_catalog(path);
    catalog
        .connection
        .execute(
            "DELETE FROM library_change_root_state WHERE root_id = 'root-a'",
            [],
        )
        .expect("remove authority fixture");

    let error = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "late.jpg")],
            1_000,
            immediate_policy(),
        )
        .expect_err("missing authority must fail closed");

    assert_eq!(error.code, "change_queue_generation_missing");
}

#[test]
fn repeated_zero_work_reregistration_advances_before_accepting_events() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = queue_catalog(path);
    let mut generation = LibraryRootGeneration::initial();
    for cycle in 2_i64..=7 {
        assert!(catalog.unregister_root("root-a").expect("unregister root"));
        generation = register_root(&mut catalog, cycle);
        assert_eq!(generation.value(), u64::try_from(cycle).expect("cycle"));
    }
    assert_eq!(generation.value(), 7);
    assert!(
        catalog
            .unregister_root("root-a")
            .expect("retire generation 7")
    );
    let current_generation = register_root(&mut catalog, 8);

    let stale = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "stale.jpg")],
            1_000,
            immediate_policy(),
        )
        .expect("reject retired generation");
    let current = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                current_generation,
                2,
                1_001,
                "current.jpg",
            )],
            1_001,
            immediate_policy(),
        )
        .expect("accept lifecycle generation");

    assert_eq!(current_generation.value(), 8);
    assert_eq!(stale.stale_generation_count, 1);
    assert_eq!(stale.inserted_count, 0);
    assert_eq!(current.inserted_count, 1);
}

#[test]
fn scan_registration_advances_a_retired_root_before_queue_ingress() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let first_request = scan_request("scan-1");
    catalog
        .begin_scan(&first_request, "root-a", &first_request.root_path)
        .expect("register first root lifecycle");
    assert!(
        catalog
            .unregister_root("root-a")
            .expect("retire first root")
    );
    let second_request = scan_request("scan-2");
    catalog
        .begin_scan(&second_request, "root-a", &second_request.root_path)
        .expect("re-register root lifecycle");

    let first_generation = LibraryRootGeneration::initial();
    let second_generation = first_generation.next().expect("second generation");
    let stale = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                first_generation,
                1,
                1_000,
                "stale.jpg",
            )],
            1_000,
            immediate_policy(),
        )
        .expect("reject retired generation");
    let current = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                second_generation,
                2,
                1_001,
                "current.jpg",
            )],
            1_001,
            immediate_policy(),
        )
        .expect("accept registered generation");

    assert_eq!(stale.stale_generation_count, 1);
    assert_eq!(current.inserted_count, 1);
}

#[test]
fn retention_cleanup_cannot_reactivate_a_removed_root() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::new(7).expect("generation");
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue root work");
    assert!(catalog.unregister_root("root-a").expect("unregister root"));
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(i64::MAX, 2)
            .expect("clean terminal row"),
        1,
    );
    let reactivated_generation = register_root(&mut catalog, 2);

    let stale = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 2_000, "late.jpg")],
            2_000,
            policy,
        )
        .expect("reject stale work after re-registration");
    let next_generation = generation.next().expect("next generation");
    let current = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                next_generation,
                3,
                2_001,
                "current.jpg",
            )],
            2_001,
            policy,
        )
        .expect("accept advanced generation");
    let (stored_generation, is_active): (i64, bool) = catalog
        .connection
        .query_row(
            "SELECT generation, is_active
             FROM library_change_root_state WHERE root_id = 'root-a'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("permanent root generation authority");

    assert_eq!(stale.stale_generation_count, 1);
    assert_eq!(stale.inserted_count, 0);
    assert_eq!(current.inserted_count, 1);
    assert_eq!(reactivated_generation, next_generation);
    assert_eq!(stored_generation, 8);
    assert!(is_active);
}

#[test]
fn expired_lease_recovers_after_restart_with_bounded_backoff() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = retry_policy();
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue");
    let first = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("first lease");
    assert_eq!(first.len(), 1);
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopened catalog");
    assert!(
        reopened
            .lease_library_changes("root-a", generation, 1_100, policy)
            .expect("recover expired lease")
            .is_empty()
    );
    let retry_wait = reopened
        .load_library_change_queue_metrics(1_100, policy)
        .expect("retry metrics");
    let retried = reopened
        .lease_library_changes("root-a", generation, 1_110, policy)
        .expect("retry lease");

    assert_eq!(retry_wait.retry_wait_count, 1);
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].change.attempt_count, 2);
    assert_eq!(
        retried[0]
            .change
            .last_failure
            .as_ref()
            .map(|failure| failure.code.as_str()),
        Some("change_lease_expired"),
    );
}

#[test]
fn path_poll_cannot_recover_an_expired_authoritative_worker_lease() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = retry_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue authoritative work");
    let authoritative = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease authoritative work")
        .expect("authoritative lease");

    let path_work = catalog
        .lease_path_library_changes("root-a", generation, 1_101, policy)
        .expect("poll path work after the authoritative lease duration");
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 1_101, policy)
        .expect("load expired authoritative metrics");

    assert!(path_work.is_empty());
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.expired_lease_count, 1);
    catalog
        .complete_library_change(
            authoritative.change.id,
            authoritative.lease_generation,
            0,
            1_101,
        )
        .expect("the live authoritative worker retains publication authority");
}

#[test]
fn expired_final_authoritative_attempt_is_normalized_by_a_new_connection_after_owner_loss() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..retry_policy()
    };
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue authoritative work");
    let lease = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease final authoritative attempt")
        .expect("authoritative lease");
    drop(catalog);

    let mut recovery = SqliteCatalog::open(path).expect("independent recovery connection");

    assert!(
        recovery
            .has_ready_authoritative_library_change("root-a", generation, 1_101, policy)
            .expect("expired authoritative readiness")
    );
    assert!(
        recovery
            .lease_authoritative_library_change("root-a", generation, 1_101, policy)
            .expect("normalize expired final attempt")
            .is_none()
    );
    let metrics = recovery
        .load_library_change_root_queue_metrics("root-a", generation, 1_101, policy)
        .expect("exhausted authoritative metrics");
    let (status, next_retry_unix_ms, failure_code): (String, Option<i64>, Option<String>) =
        recovery
            .connection
            .query_row(
                "SELECT status, next_retry_unix_ms, last_failure_code
             FROM library_change_queue WHERE id = ?1",
                [sqlite_integer(lease.change.id.value(), "change ID").expect("change ID")],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("normalized final attempt");

    assert_eq!(status, "retry_wait");
    assert_eq!(next_retry_unix_ms, None);
    assert_eq!(failure_code.as_deref(), Some("change_lease_expired"));
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("change_lease_expired")
    );
}

#[test]
fn deferred_lease_restores_the_attempt_budget_for_normal_coordination() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue");
    let leased = catalog
        .lease_path_library_changes("root-a", generation, 1_000, policy)
        .expect("lease path work")
        .pop()
        .expect("leased change");

    let outcome = catalog
        .defer_library_change(leased.change.id, leased.lease_generation, 1_001)
        .expect("defer lease");
    let released = catalog
        .lease_path_library_changes("root-a", generation, 1_001, policy)
        .expect("lease deferred work")
        .pop()
        .expect("deferred work remains leasable");

    assert_eq!(outcome, LibraryChangeLeaseUpdateOutcome::Applied);
    assert_eq!(released.change.attempt_count, 1);
    assert_eq!(released.lease_generation, leased.lease_generation + 1);
}

#[test]
fn exhausted_retry_remains_durable_and_degrades_queue_health() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue");
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease")
        .pop()
        .expect("leased change");
    let outcome = catalog
        .retry_library_change(
            leased.change.id,
            leased.lease_generation,
            &LibraryChangeFailure {
                code: "path_locked".to_owned(),
                message: "The path remained locked.".to_owned(),
            },
            1_001,
            policy,
        )
        .expect("record exhausted retry");
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy)
        .expect("exhausted metrics");

    assert_eq!(outcome, LibraryChangeLeaseUpdateOutcome::Applied);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Degraded);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert!(
        catalog
            .lease_library_changes("root-a", generation, 2_000, policy)
            .expect("no exhausted lease")
            .is_empty()
    );
}

#[test]
fn authoritative_scheduler_ignores_future_and_exhausted_retry_rows() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let future_policy = retry_policy();
    let exhausted_policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            future_policy,
        )
        .expect("enqueue future retry");
    let future = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, future_policy)
        .expect("lease future retry")
        .expect("authoritative lease");
    catalog
        .retry_library_change(
            future.change.id,
            future.lease_generation,
            &LibraryChangeFailure {
                code: "source_busy".to_owned(),
                message: "The source is temporarily busy.".to_owned(),
            },
            1_001,
            future_policy,
        )
        .expect("record future retry");

    assert!(
        !catalog
            .has_ready_authoritative_library_change("root-a", generation, 1_010, future_policy,)
            .expect("future retry readiness")
    );
    assert!(
        catalog
            .has_ready_authoritative_library_change("root-a", generation, 1_011, future_policy,)
            .expect("due retry readiness")
    );
    let due = catalog
        .lease_authoritative_library_change("root-a", generation, 1_011, future_policy)
        .expect("lease due retry")
        .expect("due authoritative retry");
    catalog
        .complete_library_change(due.change.id, due.lease_generation, 0, 1_011)
        .expect("complete due retry");

    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                2,
                2_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            2_000,
            exhausted_policy,
        )
        .expect("enqueue exhausted retry");
    let exhausted = catalog
        .lease_authoritative_library_change("root-a", generation, 2_000, exhausted_policy)
        .expect("lease exhausted retry")
        .expect("authoritative lease");
    catalog
        .retry_library_change(
            exhausted.change.id,
            exhausted.lease_generation,
            &LibraryChangeFailure {
                code: "source_failed".to_owned(),
                message: "The source failure exhausted retry.".to_owned(),
            },
            2_001,
            exhausted_policy,
        )
        .expect("record exhausted retry");

    assert!(
        !catalog
            .has_ready_authoritative_library_change(
                "root-a",
                generation,
                i64::MAX,
                exhausted_policy,
            )
            .expect("exhausted retry readiness")
    );
}

#[test]
fn lowering_retry_limit_exposes_and_normalizes_exhausted_work() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let initial_policy = retry_policy();
    let lowered_policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..initial_policy
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            initial_policy,
        )
        .expect("enqueue");
    let lease = catalog
        .lease_library_changes("root-a", generation, 1_000, initial_policy)
        .expect("lease")
        .pop()
        .expect("leased change");
    catalog
        .retry_library_change(
            lease.change.id,
            lease.lease_generation,
            &LibraryChangeFailure {
                code: "inspection_busy".to_owned(),
                message: "The file is temporarily locked".to_owned(),
            },
            1_000,
            initial_policy,
        )
        .expect("schedule retry");

    let metrics = catalog
        .load_library_change_queue_metrics(2_000, lowered_policy)
        .expect("lowered-policy metrics");
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Degraded);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(metrics.ready_count, 0);
    assert!(
        catalog
            .lease_library_changes("root-a", generation, 2_000, lowered_policy)
            .expect("normalize exhausted retry")
            .is_empty()
    );
    let next_retry_unix_ms: Option<i64> = catalog
        .connection
        .query_row(
            "SELECT next_retry_unix_ms FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(lease.change.id.value(), "change ID").expect("change ID")],
            |row| row.get(0),
        )
        .expect("normalized retry deadline");
    assert_eq!(next_retry_unix_ms, None);
}

#[test]
fn path_poll_normalizes_a_lowered_authoritative_retry_limit_without_leasing_it() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let initial_policy = retry_policy();
    let lowered_policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..initial_policy
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            initial_policy,
        )
        .expect("enqueue authoritative work");
    let lease = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, initial_policy)
        .expect("lease authoritative work")
        .expect("authoritative lease");
    catalog
        .retry_library_change(
            lease.change.id,
            lease.lease_generation,
            &LibraryChangeFailure {
                code: "source_busy".to_owned(),
                message: "The source is temporarily busy.".to_owned(),
            },
            1_001,
            initial_policy,
        )
        .expect("schedule authoritative retry");

    assert!(
        catalog
            .lease_path_library_changes("root-a", generation, 1_002, lowered_policy)
            .expect("normalize the lowered retry limit")
            .is_empty()
    );
    let next_retry_unix_ms: Option<i64> = catalog
        .connection
        .query_row(
            "SELECT next_retry_unix_ms FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(lease.change.id.value(), "change ID").expect("change ID")],
            |row| row.get(0),
        )
        .expect("normalized authoritative retry deadline");
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 1_002, lowered_policy)
        .expect("lowered authoritative retry metrics");

    assert_eq!(next_retry_unix_ms, None);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert!(
        !catalog
            .has_ready_authoritative_library_change("root-a", generation, 1_002, lowered_policy)
            .expect("exhausted authoritative readiness")
    );
}

#[test]
fn newer_evidence_reopens_an_exhausted_change_with_a_fresh_retry_budget() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue");
    let first = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("first lease")
        .pop()
        .expect("first change");
    catalog
        .retry_library_change(
            first.change.id,
            first.lease_generation,
            &LibraryChangeFailure {
                code: "path_locked".to_owned(),
                message: "The path remained locked.".to_owned(),
            },
            1_001,
            policy,
        )
        .expect("exhaust retry");

    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_100, "photo.jpg")],
            1_100,
            policy,
        )
        .expect("enqueue newer evidence");
    let reopened = catalog
        .lease_library_changes("root-a", generation, 1_100, policy)
        .expect("reopened lease")
        .pop()
        .expect("reopened change");

    assert_eq!(reopened.change.id, first.change.id);
    assert_eq!(reopened.change.attempt_count, 1);
    assert_eq!(reopened.change.intent.coalesced_observation_count, 2);
}

#[test]
fn live_path_work_does_not_supersede_a_leased_inventory_authority() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue inventory authority");
    let authority = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease inventory authority")
        .expect("inventory authority");

    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "new/photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue live path work");
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 1_001, policy)
        .expect("queue metrics");

    assert_eq!(report.superseded_count, 0);
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.freshness_unknown_count, 1);
    assert_eq!(
        catalog
            .complete_library_change(authority.change.id, authority.lease_generation, 0, 1_002,)
            .expect("complete inventory authority"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let path_work = catalog
        .lease_path_library_changes("root-a", generation, 1_002, policy)
        .expect("lease retained path work");
    assert_eq!(path_work.len(), 1);
    assert_eq!(path_work[0].change.intent.relative_path, "new/photo.jpg");
}

#[test]
fn live_path_work_preserves_a_subtree_promoted_to_inventory() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue subtree work");
    let first = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease subtree work")
        .expect("subtree work");
    catalog
        .retry_library_change(
            first.change.id,
            first.lease_generation,
            &LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "The subtree exceeded bounded reconciliation".to_owned(),
            },
            1_001,
            policy,
        )
        .expect("promote subtree to inventory");
    let authority = catalog
        .lease_authoritative_library_change("root-a", generation, 1_011, policy)
        .expect("lease promoted inventory")
        .expect("promoted inventory");

    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_012, "album/new.jpg")],
            1_012,
            policy,
        )
        .expect("enqueue live path work");

    assert_eq!(
        catalog
            .complete_library_change(authority.change.id, authority.lease_generation, 0, 1_013,)
            .expect("complete promoted inventory"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let path_work = catalog
        .lease_path_library_changes("root-a", generation, 1_013, policy)
        .expect("lease retained path work")
        .pop()
        .expect("retained path work");
    assert_eq!(path_work.change.intent.relative_path, "album/new.jpg");
}

#[test]
fn live_path_capacity_backpressures_without_replacing_inventory_authority() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue inventory authority");
    let authority = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease inventory authority")
        .expect("inventory authority");
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "a.jpg")],
            1_001,
            policy,
        )
        .expect("fill retained path capacity");

    let error = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 3, 1_002, "b.jpg")],
            1_002,
            policy,
        )
        .expect_err("a second live path must backpressure");
    assert_eq!(error.code, "change_queue_backpressure");
    let retained = catalog
        .lease_path_library_changes("root-a", generation, 1_002, policy)
        .expect("lease first retained path")
        .pop()
        .expect("first retained path");
    catalog
        .complete_library_change(retained.change.id, retained.lease_generation, 0, 1_003)
        .expect("complete first retained path");
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 3, 1_004, "b.jpg")],
            1_004,
            policy,
        )
        .expect("retry after path capacity drains");

    assert_eq!(
        catalog
            .complete_library_change(authority.change.id, authority.lease_generation, 0, 1_005,)
            .expect("complete inventory authority"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let replacement = catalog
        .lease_path_library_changes("root-a", generation, 1_005, policy)
        .expect("lease retried path")
        .pop()
        .expect("retried path");
    assert_eq!(replacement.change.intent.relative_path, "b.jpg");
}

#[test]
fn conflicting_live_rename_replaces_a_leased_inventory_epoch_with_a_gap() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue inventory authority");
    let authority = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease inventory authority")
        .expect("inventory authority");
    catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                2,
                1_001,
                "old/photo.jpg",
                "new-a/photo.jpg",
            )],
            1_001,
            policy,
        )
        .expect("enqueue first rename");

    let report = catalog
        .enqueue_library_change_intents(
            &[rename_intent(
                "root-a",
                generation,
                3,
                1_002,
                "old/photo.jpg",
                "new-b/photo.jpg",
            )],
            1_002,
            policy,
        )
        .expect("degrade conflicting rename");

    assert_eq!(report.superseded_count, 2);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(
        catalog
            .complete_library_change(authority.change.id, authority.lease_generation, 0, 1_003,)
            .expect("reject superseded inventory authority"),
        LibraryChangeLeaseUpdateOutcome::Superseded,
    );
    let replacement = catalog
        .lease_authoritative_library_change("root-a", generation, 1_003, policy)
        .expect("lease replacement gap")
        .expect("replacement gap");
    assert_eq!(
        replacement.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(replacement.change.intent.scope, LibraryChangeScope::Root);
}

#[test]
fn normalized_capacity_overflow_degrades_to_one_root_gap() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path.clone());
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "a.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue first path");
    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_001, "b.jpg")],
            1_001,
            policy,
        )
        .expect("degrade capacity");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen degraded queue");
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease root gap");

    assert!(report.capacity_degraded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(leased.len(), 1);
    assert_eq!(
        leased[0].change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
    assert_eq!(leased[0].change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(leased[0].change.intent.coalesced_observation_count, 2);
    let metrics = catalog
        .load_library_change_queue_metrics(1_001, policy)
        .expect("freshness metrics");
    assert_eq!(metrics.freshness_unknown_count, 1);
}

#[test]
fn lowered_capacity_degrades_precise_children_instead_of_losing_dirty_evidence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let initial_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 2,
        ..immediate_policy()
    };
    let lowered_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[
                path_intent("root-a", generation, 1, 1_000, "album/a.jpg"),
                path_intent("root-a", generation, 2, 1_000, "album/b.jpg"),
            ],
            1_000,
            initial_policy,
        )
        .expect("enqueue two child paths");
    let report = catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                3,
                1_001,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_001,
            lowered_policy,
        )
        .expect("normalize under lowered capacity");
    let retained = catalog
        .lease_library_changes("root-a", generation, 1_001, lowered_policy)
        .expect("lease retained subtree");

    assert!(report.capacity_degraded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(report.superseded_count, 2);
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(
        retained[0].change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
}

#[test]
fn lowered_capacity_degrades_root_plus_precise_dirty_work_without_losing_the_gap() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let initial_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 2,
        ..immediate_policy()
    };
    let lowered_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[
                path_intent("root-a", generation, 1, 1_000, "a.jpg"),
                path_intent("root-a", generation, 2, 1_000, "b.jpg"),
            ],
            1_000,
            initial_policy,
        )
        .expect("enqueue two paths");
    let report = catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                3,
                1_001,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Root,
                "",
            )],
            1_001,
            lowered_policy,
        )
        .expect("normalize root work under lowered capacity");
    let retained = catalog
        .lease_library_changes("root-a", generation, 1_001, lowered_policy)
        .expect("lease retained root work");

    assert!(report.capacity_degraded);
    assert!(report.freshness_unknown_enqueued);
    assert_eq!(report.superseded_count, 2);
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(
        retained[0].change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown,
    );
}

#[test]
fn metrics_and_cleanup_are_structured_and_bounded() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let empty = catalog
        .load_library_change_queue_metrics(1_000, policy)
        .expect("empty metrics");
    assert_eq!(empty.health, LibraryChangeQueueHealth::Idle);
    catalog
        .enqueue_library_change_intents(
            &[
                path_intent("root-a", generation, 1, 1_000, "a.jpg"),
                path_intent("root-a", generation, 2, 1_000, "b.jpg"),
            ],
            1_000,
            policy,
        )
        .expect("enqueue changes");
    let leases = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease changes");
    for lease in &leases {
        assert_eq!(
            catalog
                .complete_library_change(lease.change.id, lease.lease_generation, 0, 1_010,)
                .expect("complete change"),
            LibraryChangeLeaseUpdateOutcome::Applied,
        );
    }
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(1_010, 1)
            .expect("bounded cleanup"),
        1,
    );
    let retained = catalog
        .load_library_change_queue_metrics(1_010, policy)
        .expect("retained metrics");
    assert_eq!(retained.completed_count, 1);
    assert_eq!(retained.pending_count, 0);
}

#[test]
fn terminal_retention_atomically_releases_all_handoff_owners() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|40".to_owned(),
    };
    catalog
        .enqueue_library_change_intents_with_catch_up(
            &[path_intent("root-a", generation, 1, 1_000, "a.jpg")],
            &evidence,
            1_000,
            policy,
        )
        .expect("enqueue catch-up work");
    let lease = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease catch-up work")
        .pop()
        .expect("catch-up lease");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO assets(id, created_unix_ms) VALUES ('asset-a', 1);
             INSERT INTO preview_artifacts(
               artifact_key, source_file_size, source_modified_unix_ms,
               source_identity_scheme, source_identity_value, algorithm_id,
               algorithm_version, orientation_contract, size_bucket,
               encoded_width, encoded_height, artifact_path, byte_size,
               lifecycle_state, created_unix_ms, last_used_unix_ms
             ) VALUES (
               'preview-a', 1, 1, 'windows-file-id-128-v1', 'volume:file',
               'preview', 1, 'orientation-v1', 128, 8, 8, 'cache/a.jpg', 1,
               'ready', 1, 1
             );
             INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               updated_unix_ms
             ) VALUES (
               'windows_usn_v1', 'volume|12|40', 'windows-file-id-128-v1', 'volume:file',
               'asset-a', 'location-a', 'root-a', 'C:/source/a.jpg', 'a.jpg',
               'cache/a.jpg', 1, NULL, 1, 8, 8, 'ready', NULL, NULL,
               'metadata', '1', NULL, NULL, NULL, NULL, 1
             );
             INSERT INTO library_change_scan_handoff_batches(
               id, source_root_id, updated_unix_ms
             ) VALUES ('batch-a', 'root-a', 1);
             INSERT INTO library_change_scan_handoff_lineage(
               batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES ('batch-a', 'windows_usn_v1', 'volume|12|40', 1);
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value
             ) VALUES (
               'batch-a', 'windows-file-id-128-v1', 'volume:file',
               'asset-a', 'location-a', 'root-a', 'C:/source/a.jpg', 'a.jpg',
               'cache/a.jpg', 1, NULL, 1, 8, 8, 'ready', NULL, NULL,
               'metadata', '1', NULL, NULL, NULL, NULL
             );",
        )
        .expect("handoff fixtures");
    catalog
        .complete_library_change(lease.change.id, lease.lease_generation, 0, 1_010)
        .expect("complete catch-up work");

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(1_010, 1)
            .expect("atomic terminal cleanup"),
        1,
    );
    let retained: (i64, i64, i64, i64, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_queue),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs),
               (SELECT COUNT(*) FROM library_change_scan_handoff_batches),
               (SELECT COUNT(*) FROM library_change_scan_handoff_lineage),
               (SELECT COUNT(*) FROM library_change_scan_handoff_items),
               (SELECT COUNT(*) FROM assets WHERE id = 'asset-a'),
               (SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = 'preview-a')",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("retention result");
    assert_eq!(retained, (0, 0, 0, 0, 0, 0, "stale".to_owned()));
}

#[test]
fn terminal_retention_does_not_bind_live_provenance_to_an_active_scan() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path.clone());
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|41".to_owned(),
    };
    catalog
        .enqueue_library_change_intents_with_catch_up(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            &evidence,
            1_000,
            policy,
        )
        .expect("enqueue catch-up gap");
    let request = scan_request("retention-owned-scan");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin authoritative scan");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', lease_expires_unix_ms = NULL,
                 updated_unix_ms = 1_010",
            [],
        )
        .expect("make frozen queue provenance terminal");

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(1_010, 1)
            .expect("clean live provenance outside the active scan"),
        1,
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen with provable frozen lineage");
    catalog
        .abandon_scan("retention-owned-scan", "cancelled", 0)
        .expect("release frozen lineage");
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(i64::MAX, 1)
            .expect("no frozen queue provenance remains"),
        0,
    );
}

#[test]
fn enqueue_runs_bounded_terminal_retention_cleanup() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        terminal_retention_millis: 100,
        cleanup_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "old.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue old work");
    let completed = catalog
        .lease_library_changes("root-a", generation, 1_000, policy)
        .expect("lease old work")
        .pop()
        .expect("old work");
    catalog
        .complete_library_change(completed.change.id, completed.lease_generation, 0, 1_000)
        .expect("complete old work");

    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 2, 1_101, "new.jpg")],
            1_101,
            policy,
        )
        .expect("enqueue with retention cleanup");
    let metrics = catalog
        .load_library_change_queue_metrics(1_101, policy)
        .expect("retained metrics");

    assert_eq!(metrics.completed_count, 0);
    assert_eq!(metrics.pending_count, 1);
}

#[test]
fn queue_metrics_report_ready_delay_without_mutating_work() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue");

    let metrics = catalog
        .load_library_change_queue_metrics(1_600, policy)
        .expect("delayed metrics");

    assert_eq!(metrics.health, LibraryChangeQueueHealth::Delayed);
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.ready_count, 1);
    assert_eq!(metrics.oldest_ready_delay_millis, 100);
}

#[test]
fn catch_up_evidence_survives_persistence_alongside_later_live_work() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let mut catch_up = path_intent("root-a", generation, 20, 1_000, "photo.jpg");
    catch_up.origin = LibraryChangeOrigin::StartupCatchUp;
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|40".to_owned(),
    };
    catalog
        .enqueue_library_change_intents_with_catch_up(&[catch_up], &evidence, 1_000, policy)
        .expect("enqueue catch-up evidence");
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_001, "photo.jpg")],
            1_001,
            policy,
        )
        .expect("enqueue live evidence");

    let catch_up = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Journal,
            1_001,
            policy,
        )
        .expect("lease retained catch-up evidence")
        .pop()
        .expect("catch-up change");
    let live = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_001,
            policy,
        )
        .expect("lease live evidence")
        .pop()
        .expect("live change");

    assert_eq!(
        catch_up.change.catch_up_source.as_deref(),
        Some("windows_usn_v1")
    );
    assert_eq!(
        catch_up.change.catch_up_watermark.as_deref(),
        Some("volume|12|40")
    );
    assert_eq!(live.change.catch_up_source, None);
}

#[test]
fn replayed_catch_up_range_coalesces_without_duplicate_work() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let mut catch_up = path_intent("root-a", generation, 20, 1_000, "photo.jpg");
    catch_up.origin = LibraryChangeOrigin::StartupCatchUp;
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|40".to_owned(),
    };

    let first = catalog
        .enqueue_library_change_intents_with_catch_up(&[catch_up.clone()], &evidence, 1_000, policy)
        .expect("first catch-up enqueue");
    let replay = catalog
        .enqueue_library_change_intents_with_catch_up(&[catch_up], &evidence, 1_001, policy)
        .expect("replayed catch-up enqueue");
    let leased = catalog
        .lease_path_library_changes("root-a", generation, 1_001, policy)
        .expect("lease coalesced work");

    assert_eq!(first.inserted_count, 1);
    assert_eq!(replay.inserted_count, 0);
    assert_eq!(replay.coalesced_count, 1);
    assert_eq!(leased.len(), 1);
    assert_eq!(
        leased[0].change.catch_up_watermark.as_deref(),
        Some("volume|12|40")
    );
}

#[test]
fn catch_up_root_batches_roll_back_together_when_one_root_cannot_enqueue() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let transaction = catalog.connection.transaction().expect("root transaction");
    for (root_id, path) in [("root-a", "C:\\SourceA"), ("root-b", "C:\\SourceB")] {
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
                [root_id, path],
            )
            .expect("registered root fixture");
        activate_root_change_queue(&transaction, root_id, 1).expect("root generation");
    }
    transaction.commit().expect("registered roots");
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_second_catch_up_root
             BEFORE INSERT ON library_change_queue
             WHEN NEW.root_id = 'root-b'
             BEGIN SELECT RAISE(ABORT, 'fixture rejection'); END;",
        )
        .expect("rejection trigger");
    let evidence = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|40".to_owned(),
    };
    let batches = [
        LibraryChangeCatchUpQueueBatch {
            intents: vec![path_intent(
                "root-a",
                LibraryRootGeneration::initial(),
                1,
                10,
                "old.jpg",
            )],
            evidence: Some(evidence.clone()),
        },
        LibraryChangeCatchUpQueueBatch {
            intents: vec![path_intent(
                "root-b",
                LibraryRootGeneration::initial(),
                2,
                10,
                "new.jpg",
            )],
            evidence: Some(evidence),
        },
    ];

    catalog
        .enqueue_library_change_catch_up_batches(&batches, 10, immediate_policy())
        .expect_err("second root rejection");

    let queued = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM library_change_queue", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("queue count");
    assert_eq!(queued, 0);
}

#[test]
fn newer_catch_up_coalescing_retains_every_unconsumed_watermark() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let intent = path_intent("root-a", generation, 1, 1_000, "photo.jpg");
    let first = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|40".to_owned(),
    };
    let second = LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume|12|80".to_owned(),
    };

    catalog
        .enqueue_library_change_intents_with_catch_up(
            std::slice::from_ref(&intent),
            &first,
            1_000,
            policy,
        )
        .expect("first watermark");
    catalog
        .enqueue_library_change_intents_with_catch_up(&[intent], &second, 1_001, policy)
        .expect("second watermark");
    let leased = catalog
        .lease_path_library_changes("root-a", generation, 1_001, policy)
        .expect("lease")
        .pop()
        .expect("change");

    assert_eq!(
        leased.change.catch_up_watermark.as_deref(),
        Some("volume|12|80")
    );
    assert_eq!(leased.change.catch_up_lineage, vec![second, first]);
}

#[test]
fn unresolved_catch_up_watermark_lineage_is_bounded() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let intent = path_intent("root-a", generation, 1, 1_000, "photo.jpg");
    for watermark in 0..64 {
        catalog
            .enqueue_library_change_intents_with_catch_up(
                std::slice::from_ref(&intent),
                &LibraryChangeCatchUpEvidence {
                    source: "windows_usn_v1".to_owned(),
                    watermark: format!("volume|12|{watermark}"),
                },
                1_000 + watermark,
                policy,
            )
            .expect("bounded watermark lineage");
    }

    let error = catalog
        .enqueue_library_change_intents_with_catch_up(
            &[intent],
            &LibraryChangeCatchUpEvidence {
                source: "windows_usn_v1".to_owned(),
                watermark: "volume|12|overflow".to_owned(),
            },
            2_000,
            policy,
        )
        .expect_err("lineage overflow");
    assert_eq!(error.code, "change_queue_catch_up_lineage_limit_exceeded");

    let leased = catalog
        .lease_path_library_changes("root-a", generation, 2_000, policy)
        .expect("lease retained lineage")
        .pop()
        .expect("change");
    assert_eq!(leased.change.catch_up_lineage.len(), 64);
    assert!(
        leased
            .change
            .catch_up_lineage
            .iter()
            .all(|evidence| evidence.watermark != "volume|12|overflow")
    );
}

#[test]
fn path_lease_waits_for_concurrent_writer_before_reading_snapshot() {
    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(catalog_path.clone());
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("queued path change");

    let mut blocker = Connection::open(catalog_path).expect("blocking catalog connection");
    let blocker_transaction = blocker
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("blocking writer transaction");
    blocker_transaction
        .execute("UPDATE catalog_state SET revision = revision", [])
        .expect("blocking writer mutation");

    let lease = thread::spawn(move || {
        catalog.lease_path_library_changes("root-a", generation, 1_000, policy)
    });
    thread::sleep(Duration::from_millis(100));
    blocker_transaction
        .commit()
        .expect("release blocking writer transaction");

    let leased = lease
        .join()
        .expect("lease worker")
        .expect("lease waits for the current writer");
    assert_eq!(leased.len(), 1);
}

#[test]
fn empty_path_lease_stays_read_only_while_another_writer_is_active() {
    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(catalog_path.clone());
    let holder = SqliteCatalog::open(catalog_path).expect("lock holder catalog");
    holder
        .connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             UPDATE catalog_state SET revision = revision;",
        )
        .expect("hold catalog writer lock");

    let started = std::time::Instant::now();
    let leased = catalog
        .lease_path_library_changes("root-a", generation, 1_000, policy)
        .expect("empty lease inspection remains read-only");

    holder
        .connection
        .execute_batch("ROLLBACK")
        .expect("release catalog writer lock");
    assert!(leased.is_empty());
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn lower_priority_lanes_cannot_consume_reserved_live_admission() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 8,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));

    for index in 0..7 {
        let mut journal = path_intent(
            "root-a",
            generation,
            index + 1,
            1_000 + i64::try_from(index).expect("timestamp"),
            &format!("journal-{index}.jpg"),
        );
        journal.origin = LibraryChangeOrigin::StartupCatchUp;
        catalog
            .enqueue_library_change_intents(
                &[journal],
                1_000 + i64::try_from(index).expect("enqueue time"),
                policy,
            )
            .expect("enqueue journal work");
        let leased = catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Journal,
                1_000 + i64::try_from(index).expect("lease time"),
                policy,
            )
            .expect("lease journal work");
        assert_eq!(leased.len(), 1);
    }

    let (recovery_authorities, recovery_controls): (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_recovery_authorities),
               (SELECT COUNT(*) FROM library_change_queue
                WHERE origin = 'metadata_inventory')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("ordinary journal backlog evidence");
    assert_eq!(recovery_authorities, 0);
    assert_eq!(recovery_controls, 0);

    let mut recovery = path_intent("root-a", generation, 8, 1_100, "recovery.jpg");
    recovery.origin = LibraryChangeOrigin::MetadataInventory;
    let low_error = catalog
        .enqueue_library_change_intents(&[recovery], 1_100, policy)
        .expect_err("lower lane cannot consume P0 reserve");
    assert_eq!(low_error.code, "change_queue_backpressure");

    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 9, 1_101, "live.jpg")],
            1_101,
            policy,
        )
        .expect("reserved P0 admission remains usable");
    let live = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_101,
            policy,
        )
        .expect("lease reserved P0 work");
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].change.intent.relative_path, "live.jpg");

    let tiny_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut tiny_catalog = queue_catalog(directory.path().join("tiny-catalog.sqlite3"));
    let mut tiny_journal = path_intent("root-a", generation, 10, 1_200, "journal.jpg");
    tiny_journal.origin = LibraryChangeOrigin::StartupCatchUp;
    let tiny_error = tiny_catalog
        .enqueue_library_change_intents(&[tiny_journal], 1_200, tiny_policy)
        .expect_err("one-entry queue remains fully reserved for P0");
    assert_eq!(tiny_error.code, "change_queue_backpressure");
    tiny_catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 11, 1_201, "live.jpg")],
            1_201,
            tiny_policy,
        )
        .expect("single reserved P0 admission remains usable");
}

#[test]
fn p2_and_p1_have_distinct_steady_admission_reserves() {
    let default_policy = LibraryChangeQueuePolicy::default();
    assert_eq!(default_policy.lane_capacity(LibraryChangeLane::Live), 4_096);
    assert_eq!(
        default_policy.lane_capacity(LibraryChangeLane::Journal),
        3_584
    );
    assert_eq!(
        default_policy.lane_capacity(LibraryChangeLane::Recovery),
        3_072
    );

    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 16,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Journal), 14);
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Recovery), 12);
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));

    let recovery = (0_u64..12)
        .map(|index| {
            let mut intent = path_intent(
                "root-a",
                generation,
                index + 1,
                1_000 + i64::try_from(index).expect("recovery timestamp"),
                &format!("recovery-{index}.jpg"),
            );
            intent.origin = LibraryChangeOrigin::MetadataInventory;
            intent
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&recovery, 1_000, policy)
        .expect("fill the P2 steady quota");

    let journal = (0_u64..2)
        .map(|index| {
            let mut intent = path_intent(
                "root-a",
                generation,
                20 + index,
                2_000 + i64::try_from(index).expect("journal timestamp"),
                &format!("journal-{index}.jpg"),
            );
            intent.origin = LibraryChangeOrigin::StartupCatchUp;
            intent
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&journal, 2_000, policy)
        .expect("P2 saturation must not consume the P1 reserve");

    let live = (0_u64..2)
        .map(|index| {
            path_intent(
                "root-a",
                generation,
                30 + index,
                3_000 + i64::try_from(index).expect("live timestamp"),
                &format!("live-{index}.jpg"),
            )
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&live, 3_000, policy)
        .expect("P2 and P1 saturation must preserve the P0 reserve");

    let (p0, p1, p2): (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               SUM(lane = 'p0_live'),
               SUM(lane = 'p1_journal'),
               SUM(lane = 'p2_recovery')
             FROM library_change_queue_lanes",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load lane occupancy");
    assert_eq!((p0, p1, p2), (2, 2, 12));
}

#[test]
fn live_watcher_gap_promotion_is_atomic_and_idempotent() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let opening = seed_current_journal_authority(&mut catalog);
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_001,
                policy,
            )
            .expect("promote proven watcher gap"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("replay promotion"),
        LibraryChangeLeaseUpdateOutcome::Superseded,
    );

    let evidence: (
        String,
        i64,
        String,
        String,
        String,
        i64,
        Option<i64>,
        String,
        String,
        i64,
    ) = catalog
        .connection
        .query_row(
            "SELECT original.status, original.superseded_by_change_id,
                    recovery.origin, lanes.lane, recovery.status,
                    recovery.lease_generation, recovery.lease_expires_unix_ms,
                    recovery.last_failure_code, authority.reason,
                    (SELECT COUNT(*) FROM library_recovery_authorities)
             FROM library_change_queue AS original
             JOIN library_change_queue AS recovery
               ON recovery.id = original.superseded_by_change_id
             JOIN library_change_queue_lanes AS lanes
               ON lanes.change_id = recovery.id
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = recovery.id
             WHERE original.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
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
                ))
            },
        )
        .expect("atomic promotion evidence");
    assert_eq!(evidence.0, "superseded");
    assert!(evidence.1 > 0);
    assert_eq!(evidence.2, "metadata_inventory");
    assert_eq!(evidence.3, "p2_recovery");
    assert_eq!(evidence.4, "pending");
    assert_eq!(evidence.5, 0);
    assert_eq!(evidence.6, None);
    assert_eq!(evidence.7, "metadata_inventory_required");
    assert_eq!(evidence.8, "watcher_uncovered_gap");
    assert_eq!(evidence.9, 1);
    let recovery_id =
        LibraryChangeId::new(u64::try_from(evidence.1).expect("positive recovery change ID"))
            .expect("recovery change ID");
    let authority = catalog
        .load_metadata_inventory_recovery_authority(recovery_id)
        .expect("load watcher-gap authority")
        .expect("watcher-gap authority");
    let boundary = authority
        .opening_boundary
        .expect("watcher-gap opening boundary");
    assert_eq!(boundary.volume, opening.volume);
    assert_eq!(boundary.root_file_reference, opening.root_file_reference);
    assert_eq!(boundary.journal_id, opening.journal_id);
    assert_eq!(boundary.next_usn, opening.next_unread_usn);
    assert_eq!(boundary.protocol_version, opening.protocol_version);
    assert_eq!(boundary.contract_version, opening.contract_version);
    let window: (String, Option<String>, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT baseline.phase, baseline.closing_next_usn,
                    checkpoint.continuity_state, checkpoint.next_unread_usn,
                    root.continuity_state, checkpoint.last_failure_code
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE baseline.change_id = ?1",
            [sqlite_integer(recovery_id.value(), "recovery change ID")
                .expect("recovery change ID")],
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
        .expect("watcher-gap recovery window");
    assert_eq!(window.0, "inventory");
    assert_eq!(window.1, None);
    assert_eq!(window.2, "recovery_required");
    assert_eq!(window.3, opening.next_unread_usn.to_canonical_text());
    assert_eq!(window.4, "recovery_required");
    assert_eq!(window.5, "metadata_inventory_required");
}

#[test]
fn failed_live_watcher_gap_promotion_rolls_back_every_record() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    seed_current_journal_authority(&mut catalog);
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_recovery_authority_failure
             BEFORE INSERT ON library_recovery_authorities
             BEGIN
               SELECT RAISE(ABORT, 'injected promotion failure');
             END;",
        )
        .expect("install transaction failure fixture");

    let error = catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            live.change.id,
            live.lease_generation,
            &failure,
            1_001,
            policy,
        )
        .expect_err("authority failure rolls back promotion");
    assert_eq!(error.code, "metadata_inventory_recovery_authority_rejected");
    let rollback_evidence: (String, Option<i64>, i64, i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT status, superseded_by_change_id,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT COUNT(*) FROM library_persistent_journal_baselines),
                    (SELECT continuity_state FROM library_persistent_journal_checkpoints
                     WHERE root_id = 'root-a'),
                    (SELECT continuity_state FROM library_persistent_journal_root_state
                     WHERE root_id = 'root-a')
             FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("rollback evidence");
    assert_eq!(rollback_evidence.0, "leased");
    assert_eq!(rollback_evidence.1, None);
    assert_eq!(rollback_evidence.2, 0);
    assert_eq!(rollback_evidence.3, 0);
    assert_eq!(rollback_evidence.4, 0);
    assert_eq!(rollback_evidence.5, "current");
    assert_eq!(rollback_evidence.6, "current");

    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_recovery_authority_failure")
        .expect("remove transaction failure fixture");
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("retry atomic promotion"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
}

#[test]
fn live_watcher_gap_promotion_respects_lower_lane_reserve() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    seed_current_journal_authority(&mut catalog);
    let mut journal = path_intent("root-a", generation, 1, 1_000, "journal.jpg");
    journal.origin = LibraryChangeOrigin::StartupCatchUp;
    catalog
        .enqueue_library_change_intents(&[journal], 1_000, policy)
        .expect("fill the lower-lane allowance");
    assert_eq!(
        catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Journal,
                1_000,
                policy,
            )
            .expect("lease lower-lane work")
            .len(),
        1,
    );
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                2,
                1_001,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_001,
            policy,
        )
        .expect("use reserved live admission");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_001, policy)
        .expect("lease reserved live work")
        .expect("reserved live lease");
    let error = catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            live.change.id,
            live.lease_generation,
            &LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "The bounded watcher scope could not be reconstructed".to_owned(),
            },
            1_002,
            policy,
        )
        .expect_err("P2 promotion cannot consume P0 reserve");
    assert_eq!(error.code, "change_queue_backpressure");

    let evidence: (String, Option<i64>, i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT status, superseded_by_change_id,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT continuity_state FROM library_persistent_journal_checkpoints
                     WHERE root_id = 'root-a'),
                    (SELECT continuity_state FROM library_persistent_journal_root_state
                     WHERE root_id = 'root-a')
             FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
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
        .expect("reserved admission evidence");
    assert_eq!(evidence.0, "leased");
    assert_eq!(evidence.1, None);
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, 0);
    assert_eq!(evidence.4, "current");
    assert_eq!(evidence.5, "current");
}

#[test]
fn live_watcher_gap_without_current_checkpoint_fails_closed_without_p2() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 900,
        })
        .expect("current journal root without checkpoint");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_001,
                policy,
            )
            .expect("persist fail-closed retry"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let evidence: (String, String, i64, i64, i64, String, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT queue.status, queue.last_failure_code,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT COUNT(*) FROM library_persistent_journal_baselines),
                    root.continuity_state, root.last_failure_code
             FROM library_change_queue AS queue
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = queue.root_id
              AND root.root_generation = queue.root_generation
             WHERE queue.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("fail-closed promotion evidence");
    assert_eq!(evidence.0, "retry_wait");
    assert_eq!(evidence.1, "metadata_inventory_required");
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, 0);
    assert_eq!(evidence.4, 0);
    assert_eq!(evidence.5, "recovery_required");
    assert_eq!(evidence.6, None);
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("idempotent stale promotion"),
        LibraryChangeLeaseUpdateOutcome::LeaseMismatch,
    );
}

#[test]
fn live_only_watcher_gap_fails_closed_without_creating_recovery_authority() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::LiveOnly,
            continuity: PersistentJournalContinuityState::LiveOnly,
            failure: None,
            updated_unix_ms: 900,
        })
        .expect("live-only journal root");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "The bounded watcher scope could not be reconstructed".to_owned(),
                },
                1_001,
                policy,
            )
            .expect("persist live-only retry"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let evidence: (String, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT queue.status,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    root.continuity_state
             FROM library_change_queue AS queue
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = queue.root_id
              AND root.root_generation = queue.root_generation
             WHERE queue.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("live-only fail-closed evidence");
    assert_eq!(evidence.0, "retry_wait");
    assert_eq!(evidence.1, 0);
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, "live_only");
}

#[test]
fn p2_authoritative_lease_requires_matching_unretired_persisted_authority() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let initial = scan_request("persisted-recovery-initial");
    catalog
        .begin_scan(&initial, "root-a", &initial.root_path)
        .expect("begin persisted recovery fixture scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove persisted recovery fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, "root-a", 0, 0)
        .expect("publish persisted recovery fixture scan");
    let mut recovery = intent(
        "root-a",
        generation,
        1,
        1_000,
        LibraryChangeIntentKind::FreshnessUnknown,
        LibraryChangeScope::Root,
        "",
    );
    recovery.origin = LibraryChangeOrigin::ConsistencyAudit;
    catalog
        .enqueue_library_change_intents(&[recovery], 1_000, policy)
        .expect("enqueue unauthorised P2-looking work");

    assert!(
        !catalog
            .has_ready_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("check unauthorised recovery")
    );
    assert!(
        catalog
            .lease_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("reject unauthorised recovery lease")
            .is_none()
    );

    catalog
        .connection
        .execute(
            "DELETE FROM library_change_queue WHERE root_id = 'root-a'",
            [],
        )
        .expect("retire unauthorised fixture work");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("persist supported baseline capability");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "persisted-recovery-run".to_owned(),
                root_id: "root-a".to_owned(),
                root_generation: generation,
                authority_reason:
                    crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "persisted-recovery-volume".to_owned(),
                    volume_serial: 1,
                },
                root_file_reference: JournalFileReference::V2([1; 8]),
                journal_id: JournalIdentifier::new(44).expect("journal ID"),
                opening_next_usn: JournalUsn::new(20).expect("opening USN"),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: 1_000,
            },
            policy,
        )
        .expect("persist matching recovery authority and baseline");
    let change_id = baseline.change_id;
    let mut competing_recovery = intent(
        "root-a",
        generation,
        99,
        900,
        LibraryChangeIntentKind::FreshnessUnknown,
        LibraryChangeScope::Root,
        "",
    );
    competing_recovery.origin = LibraryChangeOrigin::MetadataInventory;
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin competing P2 owner transaction");
    let competing_change_id =
        insert_persistent_journal_recovery_control(&transaction, &competing_recovery, 900, policy)
            .expect("insert competing P2 owner");
    transaction
        .commit()
        .expect("commit competing P2 owner transaction");
    catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: competing_change_id,
            run_id: "competing-ready-owner".to_owned(),
            root_id: "root-a".to_owned(),
            root_generation: generation,
            reason: LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
            opening_boundary: None,
            authorized_unix_ms: 900,
            retired_unix_ms: None,
        })
        .expect("authorize competing P2 owner");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET ready_unix_ms = 2000 WHERE id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID").expect("change ID")],
        )
        .expect("delay exact baseline owner");

    assert!(
        !catalog
            .has_ready_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("exact unavailable baseline owner blocks competing P2 readiness")
    );
    assert!(
        catalog
            .lease_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("exact unavailable baseline owner blocks competing P2 lease")
            .is_none()
    );
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET ready_unix_ms = 1000 WHERE id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID").expect("change ID")],
        )
        .expect("release exact baseline owner");

    assert!(
        catalog
            .has_ready_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("check authorised recovery")
    );
    assert_eq!(
        catalog
            .lease_metadata_inventory_recovery("root-a", generation, 1_000, policy)
            .expect("lease authorised recovery")
            .expect("authorised recovery lease")
            .change
            .id,
        change_id
    );
    let competing_status: String = catalog
        .connection
        .query_row(
            "SELECT status FROM library_change_queue WHERE id = ?1",
            [
                sqlite_integer(competing_change_id.value(), "competing change ID")
                    .expect("change ID"),
            ],
            |row| row.get(0),
        )
        .expect("load competing P2 status");
    assert_eq!(competing_status, "pending");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', lease_expires_unix_ms = NULL,
                 catalog_revision_at_success = 0
             WHERE id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID").expect("change ID")],
        )
        .expect("corrupt exact baseline queue state");
    let error = catalog
        .has_ready_metadata_inventory_recovery("root-a", generation, 1_000, policy)
        .expect_err("terminal exact owner must fail closed instead of appearing not ready");
    assert_eq!(error.code, "metadata_inventory_recovery_affinity_corrupt");
    let error = catalog
        .lease_metadata_inventory_recovery("root-a", generation, 1_000, policy)
        .expect_err("terminal exact owner must fail closed before lease selection");
    assert_eq!(error.code, "metadata_inventory_recovery_affinity_corrupt");
}

#[test]
fn recovery_candidate_publication_replay_is_idempotent_and_persistently_owned() {
    let directory = tempdir().expect("temporary directory");
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        "owned-replay-run",
        &["new.jpg"],
    );
    let intent = metadata_candidate_intent("new.jpg", 10);
    let update = metadata_candidate_update("new.jpg");

    let first = catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "owned-replay-run",
            std::slice::from_ref(&intent),
            std::slice::from_ref(&update),
            1_010,
            immediate_policy(),
        )
        .expect("publish first candidate page")
        .expect("authority remains leased");
    let replay = catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "owned-replay-run",
            std::slice::from_ref(&intent),
            std::slice::from_ref(&update),
            1_011,
            immediate_policy(),
        )
        .expect("replay candidate page")
        .expect("authority remains leased");
    let evidence: (i64, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT run.candidate_count,
                    (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                     WHERE run_id = run.id),
                    (SELECT COUNT(*) FROM library_change_queue AS queue
                     JOIN library_metadata_inventory_candidate_owners AS owner
                       ON owner.change_id = queue.id
                     WHERE owner.run_id = run.id),
                    entry.comparison_status
             FROM library_metadata_inventory_runs AS run
             JOIN library_metadata_inventory_entries AS entry ON entry.run_id = run.id
             WHERE run.id = 'owned-replay-run'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("owned replay evidence");

    assert_eq!(first.0.inserted_count, 1);
    assert_eq!(replay.0, LibraryChangeEnqueueReport::default());
    assert_eq!(first.1.candidate_count, 1);
    assert_eq!(replay.1.candidate_count, 1);
    assert_eq!(evidence, (1, 1, 1, "enqueued".to_owned()));

    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
        .expect("reopen owned recovery frontier");
}

#[test]
fn recovery_candidate_publication_rolls_back_queue_when_owner_insert_fails() {
    let directory = tempdir().expect("temporary directory");
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        "owner-failure-run",
        &["owner.jpg"],
    );
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_candidate_owner_failure
             BEFORE INSERT ON library_metadata_inventory_candidate_owners
             BEGIN
               SELECT RAISE(ABORT, 'injected candidate owner failure');
             END;",
        )
        .expect("install owner failure fixture");

    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "owner-failure-run",
            &[metadata_candidate_intent("owner.jpg", 10)],
            &[metadata_candidate_update("owner.jpg")],
            1_010,
            immediate_policy(),
        )
        .expect_err("owner failure must roll back queue publication");
    assert_eq!(
        recovery_publication_rollback_evidence(&catalog, "owner-failure-run"),
        ("pending".to_owned(), 0, 0, 0, None),
    );

    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_candidate_owner_failure")
        .expect("remove owner failure fixture");
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "owner-failure-run",
            &[metadata_candidate_intent("owner.jpg", 10)],
            &[metadata_candidate_update("owner.jpg")],
            1_011,
            immediate_policy(),
        )
        .expect("retry owner publication")
        .expect("authority remains leased");
}

#[test]
fn recovery_candidate_publication_rolls_back_owner_when_cursor_update_fails() {
    let directory = tempdir().expect("temporary directory");
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        "cursor-failure-run",
        &["cursor.jpg"],
    );
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_candidate_cursor_failure
             BEFORE UPDATE OF comparison_cursor ON library_metadata_inventory_runs
             WHEN NEW.comparison_cursor IS NOT OLD.comparison_cursor
             BEGIN
               SELECT RAISE(ABORT, 'injected candidate cursor failure');
             END;",
        )
        .expect("install cursor failure fixture");

    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "cursor-failure-run",
            &[metadata_candidate_intent("cursor.jpg", 10)],
            &[metadata_candidate_update("cursor.jpg")],
            1_010,
            immediate_policy(),
        )
        .expect_err("cursor failure must roll back owner and queue");
    assert_eq!(
        recovery_publication_rollback_evidence(&catalog, "cursor-failure-run"),
        ("pending".to_owned(), 0, 0, 0, None),
    );

    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_candidate_cursor_failure")
        .expect("remove cursor failure fixture");
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "cursor-failure-run",
            &[metadata_candidate_intent("cursor.jpg", 10)],
            &[metadata_candidate_update("cursor.jpg")],
            1_011,
            immediate_policy(),
        )
        .expect("retry cursor publication")
        .expect("authority remains leased");
}

#[test]
fn higher_priority_exact_work_atomically_takes_recovery_candidate_ownership() {
    let directory = tempdir().expect("temporary directory");
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        "priority-owner-run",
        &["journal.jpg", "live.jpg"],
    );
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "priority-owner-run",
            &[
                metadata_candidate_intent("journal.jpg", 10),
                metadata_candidate_intent("live.jpg", 11),
            ],
            &[
                metadata_candidate_update("journal.jpg"),
                metadata_candidate_update("live.jpg"),
            ],
            1_010,
            immediate_policy(),
        )
        .expect("publish recovery candidates")
        .expect("authority remains leased");

    let mut journal = path_intent(
        "root-a",
        LibraryRootGeneration::initial(),
        20,
        1_020,
        "journal.jpg",
    );
    journal.origin = LibraryChangeOrigin::StartupCatchUp;
    catalog
        .enqueue_library_change_intents(&[journal], 1_020, immediate_policy())
        .expect("publish matching P1 work");
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                LibraryRootGeneration::initial(),
                21,
                1_021,
                "live.jpg",
            )],
            1_021,
            immediate_policy(),
        )
        .expect("publish matching P0 work");

    let owners = catalog
        .connection
        .prepare(
            "SELECT owner.relative_path, lane.lane, queue.status,
                    (SELECT COUNT(*) FROM library_change_queue AS recovery
                     JOIN library_change_queue_lanes AS recovery_lane
                       ON recovery_lane.change_id = recovery.id
                     WHERE recovery_lane.lane = 'p2_recovery'
                       AND recovery.scope = 'path'
                       AND recovery.relative_path = owner.relative_path
                       AND recovery.status = 'superseded')
             FROM library_metadata_inventory_candidate_owners AS owner
             JOIN library_change_queue AS queue ON queue.id = owner.change_id
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE owner.run_id = 'priority-owner-run'
             ORDER BY owner.relative_path",
        )
        .expect("priority owner query")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .expect("priority owner rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("priority owner evidence");

    assert_eq!(
        owners,
        vec![
            (
                "journal.jpg".to_owned(),
                "p1_journal".to_owned(),
                "pending".to_owned(),
                1,
            ),
            (
                "live.jpg".to_owned(),
                "p0_live".to_owned(),
                "pending".to_owned(),
                1,
            ),
        ]
    );
}

#[test]
fn exhausted_recovery_candidate_prevents_authority_completion() {
    let directory = tempdir().expect("temporary directory");
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        "exhausted-owner-run",
        &["locked.jpg"],
    );
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "exhausted-owner-run",
            &[metadata_candidate_intent("locked.jpg", 10)],
            &[metadata_candidate_update("locked.jpg")],
            1_010,
            policy,
        )
        .expect("publish exhaustible candidate")
        .expect("authority remains leased");
    catalog
        .authorize_metadata_inventory_absence("exhausted-owner-run", 1_011)
        .expect("authorize empty absence page");
    let publication = catalog
        .complete_metadata_inventory("exhausted-owner-run", 1_012)
        .expect("finish candidate publication");
    assert_eq!(publication.status, MetadataInventoryRunStatus::Comparing);

    let candidate = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            LibraryRootGeneration::initial(),
            LibraryChangeLane::Recovery,
            1_020,
            policy,
        )
        .expect("lease recovery candidate")
        .pop()
        .expect("recovery candidate");
    assert_eq!(
        catalog
            .retry_library_change(
                candidate.change.id,
                candidate.lease_generation,
                &LibraryChangeFailure {
                    code: "path_locked".to_owned(),
                    message: "The candidate remained locked".to_owned(),
                },
                1_021,
                policy,
            )
            .expect("exhaust recovery candidate"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    assert_eq!(
        catalog
            .finish_metadata_inventory_recovery(
                authority.change.id,
                authority.lease_generation,
                0,
                1_022,
            )
            .expect("apply completion barrier"),
        None,
    );
    let evidence: (String, Option<i64>, String, Option<i64>, String) = catalog
        .connection
        .query_row(
            "SELECT run.status, recovery.retired_unix_ms,
                    control.status, candidate.next_retry_unix_ms, candidate.status
             FROM library_metadata_inventory_runs AS run
             JOIN library_recovery_authorities AS recovery ON recovery.run_id = run.id
             JOIN library_change_queue AS control ON control.id = recovery.change_id
             JOIN library_metadata_inventory_candidate_owners AS owner ON owner.run_id = run.id
             JOIN library_change_queue AS candidate ON candidate.id = owner.change_id
             WHERE run.id = 'exhausted-owner-run'",
            [],
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
        .expect("exhausted completion evidence");
    assert_eq!(
        evidence,
        (
            "comparing".to_owned(),
            None,
            "leased".to_owned(),
            None,
            "retry_wait".to_owned(),
        )
    );
}

#[test]
fn p2_completion_atomically_establishes_namespace_proof_and_retires_authority() {
    let directory = tempdir().expect("temporary directory");
    let run_id = "atomic-proof-run";
    let (mut catalog, authority) =
        recovery_inventory_catalog(directory.path().join("catalog.sqlite3"), run_id, &[]);
    catalog
        .authorize_metadata_inventory_absence(run_id, 1_001)
        .expect("authorize empty absence set");
    let publication = catalog
        .complete_metadata_inventory(run_id, 1_002)
        .expect("complete candidate publication");
    assert_eq!(publication.status, MetadataInventoryRunStatus::Comparing);

    assert_eq!(
        catalog
            .finish_metadata_inventory_recovery(
                authority.change.id,
                authority.lease_generation,
                0,
                1_003,
            )
            .expect("finish atomic P2 publication"),
        Some(LibraryChangeLeaseUpdateOutcome::Applied),
    );
    let evidence: (String, String, i64, i64, String, i64) = catalog
        .connection
        .query_row(
            "SELECT proof.authority_kind, control.status,
                    authority.retired_unix_ms,
                    (SELECT COUNT(*) FROM library_metadata_inventory_spools
                     WHERE run_id = ?1),
                    run.status, proof.root_generation
             FROM library_root_publication_namespaces AS proof
             JOIN library_recovery_authorities AS authority
               ON authority.root_id = proof.root_id
              AND authority.root_generation = proof.root_generation
             JOIN library_change_queue AS control ON control.id = authority.change_id
             JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
             WHERE run.id = ?1",
            [run_id],
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
        .expect("atomic P2 proof evidence");
    assert_eq!(
        evidence,
        (
            "metadata_inventory".to_owned(),
            "completed".to_owned(),
            1_003,
            0,
            "completed".to_owned(),
            1,
        )
    );
}

#[test]
fn p2_namespace_conflict_rolls_back_completion_and_preserves_the_spool() {
    let directory = tempdir().expect("temporary directory");
    let run_id = "conflicting-proof-run";
    let (mut catalog, authority) =
        recovery_inventory_catalog(directory.path().join("catalog.sqlite3"), run_id, &[]);
    catalog
        .authorize_metadata_inventory_absence(run_id, 1_001)
        .expect("authorize empty absence set");
    catalog
        .complete_metadata_inventory(run_id, 1_002)
        .expect("complete candidate publication");
    catalog
        .connection
        .execute(
            "INSERT INTO library_root_publication_namespaces(
               root_id, root_generation, identity_scheme, identity_value,
               authority_kind, established_catalog_revision,
               established_unix_ms, updated_unix_ms
             ) VALUES (
               'root-a', 1, 'windows-file-id-128-v1',
               '000000000000004d:ffffffffffffffffffffffffffffffff',
               'foreground_scan', 0, 1, 1
             )",
            [],
        )
        .expect("conflicting foreground proof");

    let error = catalog
        .finish_metadata_inventory_recovery(
            authority.change.id,
            authority.lease_generation,
            0,
            1_003,
        )
        .expect_err("conflicting proof must roll back P2 completion");
    assert_eq!(error.code, "catalog_root_publication_namespace_conflict");
    let evidence: (String, Option<i64>, String, i64, String) = catalog
        .connection
        .query_row(
            "SELECT control.status, authority.retired_unix_ms, run.status,
                    (SELECT COUNT(*) FROM library_metadata_inventory_spools
                     WHERE run_id = run.id), proof.authority_kind
             FROM library_recovery_authorities AS authority
             JOIN library_change_queue AS control ON control.id = authority.change_id
             JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
             JOIN library_root_publication_namespaces AS proof
               ON proof.root_id = authority.root_id
             WHERE run.id = ?1",
            [run_id],
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
        .expect("rolled-back P2 evidence");
    assert_eq!(
        evidence,
        (
            "leased".to_owned(),
            None,
            "comparing".to_owned(),
            1,
            "foreground_scan".to_owned(),
        )
    );
}

#[test]
fn queue_coalescing_and_leases_preserve_lane_ownership() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let live = path_intent("root-a", generation, 1, 1_000, "live.jpg");
    let mut journal = path_intent("root-a", generation, 2, 1_001, "journal.jpg");
    journal.origin = LibraryChangeOrigin::StartupCatchUp;
    let mut recovery = path_intent("root-a", generation, 3, 1_002, "recovery.jpg");
    recovery.origin = LibraryChangeOrigin::MetadataInventory;

    for (intent, now) in [(live, 1_000), (journal, 1_001), (recovery, 1_002)] {
        catalog
            .enqueue_library_change_intents(&[intent], now, policy)
            .expect("enqueue isolated lane");
    }
    let lane_rows: Vec<(String, String)> = catalog
        .connection
        .prepare(
            "SELECT queue.origin, lanes.lane
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
             WHERE queue.status = 'pending'
             ORDER BY queue.id",
        )
        .expect("lane query")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("lane rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("lane evidence");
    assert_eq!(
        lane_rows,
        vec![
            ("live_notification".to_owned(), "p0_live".to_owned()),
            ("startup_catch_up".to_owned(), "p1_journal".to_owned()),
            ("metadata_inventory".to_owned(), "p2_recovery".to_owned()),
        ]
    );

    for (lane, origin) in [
        (
            LibraryChangeLane::Live,
            LibraryChangeOrigin::LiveNotification,
        ),
        (
            LibraryChangeLane::Journal,
            LibraryChangeOrigin::StartupCatchUp,
        ),
        (
            LibraryChangeLane::Recovery,
            LibraryChangeOrigin::MetadataInventory,
        ),
    ] {
        let leased = catalog
            .lease_path_library_changes_in_lane("root-a", generation, lane, 1_010, policy)
            .expect("lease lane");
        assert_eq!(leased.len(), 1);
        assert_eq!(leased[0].change.intent.origin, origin);
    }
}

#[test]
fn one_and_two_legacy_unowned_nonpath_rows_become_one_idempotent_blocked_survivor() {
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 4,
        ..LibraryChangeQueuePolicy::default()
    };
    for legacy_count in [1_i64, 2] {
        let directory = tempdir().expect("temporary directory");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let catalog = queue_catalog(catalog_path.clone());
        catalog
            .connection
            .execute(
                "WITH RECURSIVE sequence(value) AS (
                   SELECT 0 UNION ALL SELECT value + 1 FROM sequence WHERE value + 1 < ?1
                 )
                 INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   catch_up_source, catch_up_watermark, created_unix_ms, updated_unix_ms
                 )
                 SELECT 'root-a', 1, 'reconcile', 'subtree', printf('legacy-%d', value),
                        'metadata_inventory', 1000 + value, 1000 + value,
                        printf('%d', value + 1), printf('%d', value + 1), 1,
                        'pending', 1000 + value, 0, 'windows_usn_v1',
                        printf('volume|1|%d', value + 1), 1000 + value, 1000 + value
                 FROM sequence",
                [legacy_count],
            )
            .expect("seed legacy rows");
        catalog
            .connection
            .execute(
                "INSERT INTO library_change_queue_catch_up_lineage(
                   change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 )
                 SELECT id, catch_up_source, catch_up_watermark, created_unix_ms
                 FROM library_change_queue WHERE root_id = 'root-a'",
                [],
            )
            .expect("seed lineage");
        assert!(
            catalog
                .has_ready_legacy_unowned_recovery_debt("root-a", generation, 2_000, policy,)
                .expect("single or duplicate legacy debt must be scheduled")
        );
        drop(catalog);

        let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen legacy rows");
        assert_eq!(
            catalog
                .compact_legacy_unowned_recovery_controls("root-a", generation, 2_000, policy,)
                .expect("block legacy survivor"),
            u32::try_from(legacy_count - 1).expect("superseded count")
        );
        let first_projection: (i64, i64, i64, Option<i64>) = catalog
            .connection
            .query_row(
                "SELECT
                   SUM(status = 'retry_wait' AND attempt_count = ?1
                     AND next_retry_unix_ms IS NULL
                     AND last_failure_code = 'legacy_recovery_authority_missing'),
                   SUM(status = 'superseded'),
                   SUM(status IN ('pending', 'leased')),
                   MAX(CASE WHEN status = 'retry_wait' THEN updated_unix_ms END)
                 FROM library_change_queue WHERE root_id = 'root-a'",
                [i64::from(policy.max_attempts)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("blocked projection");
        assert_eq!(first_projection.0, 1);
        assert_eq!(first_projection.1, legacy_count - 1);
        assert_eq!(first_projection.2, 0);
        assert_eq!(first_projection.3, Some(2_000));
        drop(catalog);

        let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen survivor");
        assert_eq!(
            catalog
                .compact_legacy_unowned_recovery_controls("root-a", generation, 2_100, policy,)
                .expect("idempotent blocked survivor"),
            0
        );
        let updated_unix_ms = catalog
            .connection
            .query_row(
                "SELECT updated_unix_ms FROM library_change_queue
                 WHERE status = 'retry_wait'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("survivor update time");
        assert_eq!(updated_unix_ms, 2_000);
        assert!(
            !catalog
                .has_ready_legacy_unowned_recovery_debt("root-a", generation, 2_100, policy,)
                .expect("blocked survivor must not restart a worker")
        );

        if legacy_count == 1 {
            let replacement = LibraryChangeIntent {
                root_id: "root-a".to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::MetadataInventory,
                first_observed_unix_ms: 2_200,
                most_recent_observed_unix_ms: 2_200,
                first_sequence: 3,
                most_recent_sequence: 3,
                coalesced_observation_count: 1,
            };
            let transaction = catalog
                .connection
                .transaction()
                .expect("replacement transaction");
            let replacement_id = insert_persistent_journal_recovery_control(
                &transaction,
                &replacement,
                2_200,
                policy,
            )
            .expect("explicit replacement authority");
            transaction.commit().expect("commit replacement");
            let replacement_projection = catalog
                .connection
                .query_row(
                    "SELECT
                       SUM(status = 'superseded'
                         AND last_failure_code = 'legacy_recovery_authority_missing'
                         AND superseded_by_change_id = ?1),
                       SUM(id = ?1 AND status = 'pending')
                     FROM library_change_queue WHERE root_id = 'root-a'",
                    [i64::try_from(replacement_id.value()).expect("replacement ID")],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .expect("replacement projection");
            assert_eq!(replacement_projection, (1, 1));
        }
    }
}

#[test]
fn legacy_unowned_nonpath_debt_compacts_to_one_blocked_control_across_reopen() {
    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 4,
        ..LibraryChangeQueuePolicy::default()
    };
    let catalog = queue_catalog(catalog_path.clone());
    let legacy_shapes = [
        (
            "reconcile",
            "subtree",
            "legacy-subtree",
            None,
            "metadata_inventory",
        ),
        ("reconcile", "root", "", None, "consistency_audit"),
        ("freshness_unknown", "root", "", None, "user_refresh"),
        (
            "rename_candidate",
            "subtree",
            "renamed-subtree",
            Some("prior-subtree"),
            "metadata_inventory",
        ),
        (
            "reconcile",
            "subtree",
            "legacy-peer",
            None,
            "consistency_audit",
        ),
    ];
    for (index, (kind, scope, relative_path, previous_relative_path, origin)) in
        legacy_shapes.into_iter().enumerate()
    {
        let ordinal = i64::try_from(index + 1).expect("legacy ordinal");
        catalog
            .connection
            .execute(
                "INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'root-a', 1, ?1, ?2, ?3, ?4,
                   ?5, ?6, ?6, ?7, ?7, 1, 'pending', ?6, 0,
                   'windows_usn_v1', ?8, ?6, ?6
                 )",
                rusqlite::params![
                    kind,
                    scope,
                    relative_path,
                    previous_relative_path,
                    origin,
                    1_000 + ordinal,
                    ordinal.to_string(),
                    format!("volume|12|{ordinal}"),
                ],
            )
            .expect("insert valid legacy non-path debt");
        let change_id = catalog.connection.last_insert_rowid();
        catalog
            .connection
            .execute(
                "INSERT INTO library_change_queue_catch_up_lineage(
                   change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES (?1, 'windows_usn_v1', ?2, ?3)",
                rusqlite::params![change_id, format!("volume|12|{ordinal}"), 1_000 + ordinal,],
            )
            .expect("insert legacy catch-up lineage");
    }
    assert!(
        catalog
            .has_ready_legacy_unowned_recovery_debt("root-a", generation, 2_000, policy)
            .expect("inspect initial debt")
    );
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("validate legacy shape");
    assert_eq!(
        catalog
            .compact_legacy_unowned_recovery_controls("root-a", generation, 2_001, policy)
            .expect("compact first bounded page"),
        3
    );
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path).expect("reopen after bounded page");
    assert_eq!(
        catalog
            .compact_legacy_unowned_recovery_controls("root-a", generation, 2_002, policy)
            .expect("finish compacting legacy debt"),
        1
    );
    assert_eq!(
        catalog
            .compact_legacy_unowned_recovery_controls("root-a", generation, 2_003, policy)
            .expect("idempotent compact replay"),
        0
    );
    assert!(
        !catalog
            .has_ready_legacy_unowned_recovery_debt("root-a", generation, 2_003, policy)
            .expect("inspect compacted debt")
    );
    let evidence: (i64, i64, i64, i64, i64, i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               SUM(status = 'retry_wait'),
               SUM(status = 'superseded'),
               SUM(status = 'superseded' AND superseded_by_change_id IS NOT NULL),
               SUM(status = 'retry_wait' AND scope = 'root'
                   AND intent_kind = 'freshness_unknown' AND relative_path = ''),
               SUM(status = 'retry_wait' AND coalesced_observation_count = 5
                   AND attempt_count = ?1 AND next_retry_unix_ms IS NULL
                   AND last_failure_code = 'legacy_recovery_authority_missing'),
               (SELECT COUNT(*)
                FROM library_change_queue_catch_up_lineage AS lineage
                JOIN library_change_queue AS owned ON owned.id = lineage.change_id
                WHERE owned.root_id = 'root-a' AND owned.root_generation = 1),
               (SELECT COUNT(*) FROM library_recovery_authorities),
               (SELECT COUNT(*) FROM library_persistent_journal_root_state
                WHERE continuity_state = 'current'),
               (SELECT COUNT(*) FROM library_metadata_inventory_runs
                WHERE absence_authority = 1)
             FROM library_change_queue
             WHERE root_id = 'root-a' AND root_generation = 1",
            [i64::from(policy.max_attempts)],
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
        .expect("compacted debt evidence");
    assert_eq!(evidence, (1, 4, 4, 1, 1, 5, 0, 0, 0));
    assert!(
        catalog
            .journal_admission_has_headroom("root-a", generation, policy)
            .expect("legacy control must not starve P1")
    );
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: "root-a".to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "journal-after-legacy.jpg".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_004,
                most_recent_observed_unix_ms: 2_004,
                first_sequence: 10,
                most_recent_sequence: 10,
                coalesced_observation_count: 1,
            }],
            2_004,
            policy,
        )
        .expect("admit P1 beside blocked legacy survivor");
    assert_eq!(
        catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Journal,
                2_004,
                policy,
            )
            .expect("lease admitted P1")
            .len(),
        1
    );
}

#[test]
fn exactly_3584_mixed_nonpath_legacy_rows_release_capacity_without_authority() {
    const LEGACY_NONPATH_P2: i64 = 3_584;

    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 64,
        ..LibraryChangeQueuePolicy::default()
    };
    let catalog = queue_catalog(catalog_path.clone());
    catalog
        .connection
        .execute(
            "WITH RECURSIVE sequence(value) AS (
               SELECT 0
               UNION ALL
               SELECT value + 1 FROM sequence WHERE value + 1 < ?1
             )
             INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms,
               catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
               created_unix_ms, updated_unix_ms
             )
             SELECT
               'root-a', 1,
               CASE value % 4
                 WHEN 2 THEN 'freshness_unknown'
                 WHEN 3 THEN 'rename_candidate'
                 ELSE 'reconcile'
               END,
               CASE value % 4 WHEN 0 THEN 'subtree' WHEN 3 THEN 'subtree' ELSE 'root' END,
               CASE value % 4
                 WHEN 0 THEN printf('subtree-%04d', value)
                 WHEN 3 THEN printf('renamed-%04d', value)
                 ELSE ''
               END,
               CASE value % 4 WHEN 3 THEN printf('prior-%04d', value) ELSE NULL END,
               CASE value % 4
                 WHEN 1 THEN 'consistency_audit'
                 WHEN 2 THEN 'user_refresh'
                 ELSE 'metadata_inventory'
               END,
               1000 + value, 1000 + value,
               printf('%d', value + 1), printf('%d', value + 1),
               1, 'pending', 1000 + value, 0,
               'windows_usn_v1', printf('volume|12|%d', value + 1),
               1000 + value, 1000 + value
             FROM sequence",
            [LEGACY_NONPATH_P2],
        )
        .expect("seed mixed valid legacy non-path debt");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT id, catch_up_source, catch_up_watermark, created_unix_ms
             FROM library_change_queue
             WHERE root_id = 'root-a' AND root_generation = 1",
            [],
        )
        .expect("seed mixed legacy lineage");
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("validate legacy debt");
    let first = catalog
        .compact_legacy_unowned_recovery_controls("root-a", generation, 5_000, policy)
        .expect("compact first bounded mixed page");
    assert_eq!(first, 63);
    drop(catalog);

    let mut catalog = SqliteCatalog::open(catalog_path).expect("reopen mixed legacy debt");
    let mut superseded = u64::from(first);
    let mut page = 1_i64;
    loop {
        page += 1;
        let compacted = catalog
            .compact_legacy_unowned_recovery_controls("root-a", generation, 5_000 + page, policy)
            .expect("compact next bounded mixed page");
        if compacted == 0 {
            break;
        }
        superseded = superseded.saturating_add(u64::from(compacted));
    }
    assert_eq!(
        superseded,
        u64::try_from(LEGACY_NONPATH_P2 - 1).expect("expected superseded debt")
    );
    let evidence: (i64, i64, i64, i64, i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               SUM(status = 'pending'),
               SUM(status = 'superseded'),
               SUM(status = 'retry_wait'),
               SUM(status = 'retry_wait' AND attempt_count = ?2
                   AND last_failure_code = 'legacy_recovery_authority_missing'),
               SUM(status = 'retry_wait' AND scope = 'root'
                   AND intent_kind = 'freshness_unknown' AND relative_path = ''),
               SUM(status = 'retry_wait' AND coalesced_observation_count = ?1),
               (SELECT COUNT(*)
                FROM library_change_queue_catch_up_lineage AS lineage
                JOIN library_change_queue AS owned ON owned.id = lineage.change_id
                WHERE owned.root_id = 'root-a' AND owned.root_generation = 1),
               (SELECT COUNT(*) FROM library_recovery_authorities)
             FROM library_change_queue
             WHERE root_id = 'root-a' AND root_generation = 1",
            rusqlite::params![LEGACY_NONPATH_P2, i64::from(policy.max_attempts)],
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
                ))
            },
        )
        .expect("mixed debt survivor evidence");
    assert_eq!(
        evidence,
        (0, LEGACY_NONPATH_P2 - 1, 1, 1, 1, 1, LEGACY_NONPATH_P2, 0,)
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 6_000, policy)
        .expect("load blocked survivor metrics");
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Degraded);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(metrics.freshness_unknown_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("legacy_recovery_authority_missing")
    );
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(i64::MAX, 128)
            .expect("retain superseded lineage while survivor is unresolved"),
        0
    );
    assert!(
        catalog
            .journal_admission_has_headroom("root-a", generation, policy)
            .expect("mixed non-path debt must release P1 capacity")
    );

    let replacement_intent = LibraryChangeIntent {
        root_id: "root-a".to_owned(),
        root_generation: generation,
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::MetadataInventory,
        first_observed_unix_ms: 6_001,
        most_recent_observed_unix_ms: 6_001,
        first_sequence: 4_000,
        most_recent_sequence: 4_000,
        coalesced_observation_count: 1,
    };
    let transaction = catalog
        .connection
        .transaction()
        .expect("begin replacement authority control");
    let replacement_id = insert_persistent_journal_recovery_control(
        &transaction,
        &replacement_intent,
        6_001,
        policy,
    )
    .expect("insert replacement authority control");
    transaction
        .commit()
        .expect("commit replacement authority control");
    let replacement_shape: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               SUM(status = 'superseded'
                   AND last_failure_code = 'legacy_recovery_authority_missing'
                   AND superseded_by_change_id = ?1),
               SUM(id = ?1 AND status = 'pending'),
               SUM(status IN ('pending', 'leased', 'retry_wait'))
             FROM library_change_queue
             WHERE root_id = 'root-a' AND root_generation = 1",
            [i64::try_from(replacement_id.value()).expect("replacement ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load replacement shape");
    assert_eq!(replacement_shape, (1, 1, 1));
}

#[test]
fn capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 4,
        max_lease_batch: 4,
        max_attempts: 3,
        ..immediate_policy()
    };
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Recovery), 2);
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let p2_fillers = ["p2-a.jpg", "p2-b.jpg"]
        .into_iter()
        .enumerate()
        .map(|(index, relative_path)| {
            let mut filler = path_intent(
                "root-a",
                generation,
                u64::try_from(index + 1).expect("P2 sequence"),
                1_000,
                relative_path,
            );
            filler.origin = LibraryChangeOrigin::UserRefresh;
            filler
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&p2_fillers, 1_000, policy)
        .expect("fill P2 capacity");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                3,
                1_001,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_001,
            policy,
        )
        .expect("enqueue P0 live gap");
    let gap = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_002, policy)
        .expect("lease P0 gap")
        .expect("P0 gap");
    assert_eq!(
        catalog
            .defer_library_change_for_capacity(
                gap.change.id,
                gap.lease_generation,
                LibraryChangeCapacityDeferral::MetadataInventoryLane,
                1_002,
                policy,
            )
            .expect("defer full P2 lane"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                4,
                1_003,
                "visible-now.jpg",
            )],
            1_003,
            policy,
        )
        .expect("enqueue normal P0 path beside capacity wait");
    let ready_path = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_003,
            policy,
        )
        .expect("lease fair P0 path")
        .pop()
        .expect("capacity-wait gap must yield to normal P0 path");
    assert_eq!(ready_path.change.intent.relative_path, "visible-now.jpg");
    catalog
        .complete_library_change(ready_path.change.id, ready_path.lease_generation, 0, 1_003)
        .expect("complete fair P0 path");

    for attempt in 1..=policy.max_attempts {
        let failed_unix_ms = 1_100 + i64::from(attempt) * 100;
        let leased = catalog
            .lease_live_authoritative_library_change("root-a", generation, failed_unix_ms, policy)
            .expect("lease real-failure gap")
            .expect("real-failure retry budget");
        catalog
            .retry_library_change(
                leased.change.id,
                leased.lease_generation,
                &LibraryChangeFailure {
                    code: "real_processing_failure".to_owned(),
                    message: "The source operation genuinely failed".to_owned(),
                },
                failed_unix_ms,
                policy,
            )
            .expect("persist real processing failure");
    }
    assert!(
        catalog
            .lease_live_authoritative_library_change("root-a", generation, 2_000, policy)
            .expect("load terminal real failure")
            .is_none()
    );
    let terminal: (String, i64, Option<i64>, String) = catalog
        .connection
        .query_row(
            "SELECT status, attempt_count, next_retry_unix_ms, last_failure_code
             FROM library_change_queue
             WHERE root_id = 'root-a' AND intent_kind = 'freshness_unknown'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("real failure terminal evidence");
    assert_eq!(
        terminal,
        (
            "retry_wait".to_owned(),
            i64::from(policy.max_attempts),
            None,
            "real_processing_failure".to_owned(),
        )
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 2_000, policy)
        .expect("real failure metrics");
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("real_processing_failure")
    );
}

#[test]
fn leased_capacity_deferred_live_gap_preserves_precise_p0_work_and_its_lease() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 4,
        max_lease_batch: 4,
        max_attempts: 3,
        ..immediate_policy()
    };
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Recovery), 2);
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let p2_fillers = ["p2-a.jpg", "p2-b.jpg"]
        .into_iter()
        .enumerate()
        .map(|(index, relative_path)| {
            let mut filler = path_intent(
                "root-a",
                generation,
                u64::try_from(index + 1).expect("P2 sequence"),
                1_000,
                relative_path,
            );
            filler.origin = LibraryChangeOrigin::UserRefresh;
            filler
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&p2_fillers, 1_000, policy)
        .expect("fill P2 capacity");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                3,
                1_001,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_001,
            policy,
        )
        .expect("enqueue P0 live gap");
    let first_gap_lease = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_002, policy)
        .expect("lease P0 gap")
        .expect("P0 gap");
    assert_eq!(
        catalog
            .defer_library_change_for_capacity(
                first_gap_lease.change.id,
                first_gap_lease.lease_generation,
                LibraryChangeCapacityDeferral::MetadataInventoryLane,
                1_002,
                policy,
            )
            .expect("defer full P2 lane"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let leased_gap = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_012, policy)
        .expect("re-lease due capacity gap")
        .expect("due capacity gap");
    assert_eq!(leased_gap.change.id, first_gap_lease.change.id);

    let first_intent = path_intent("root-a", generation, 4, 1_013, "visible-now.jpg");
    let mut first_report = LibraryChangeEnqueueReport::default();
    let transaction = catalog
        .connection
        .transaction()
        .expect("begin exact leased-gap coalescing transaction");
    enqueue_one(
        &transaction,
        &first_intent,
        EnqueueContext {
            enqueued_unix_ms: 1_013,
            catalog_revision: 0,
            policy,
            evidence: None,
            protected_change_ids: &[],
            allow_scope_degradation: true,
        },
        &mut first_report,
    )
    .expect("enqueue precise P0 work beside leased capacity gap");
    transaction
        .commit()
        .expect("commit exact leased-gap coalescing transaction");
    assert_eq!(first_report.inserted_count, 1);
    assert_eq!(first_report.superseded_count, 0);
    assert!(!first_report.freshness_unknown_enqueued);
    let duplicate_intent = path_intent("root-a", generation, 5, 1_014, "visible-now.jpg");
    let mut duplicate_report = LibraryChangeEnqueueReport::default();
    let transaction = catalog
        .connection
        .transaction()
        .expect("begin duplicate leased-gap coalescing transaction");
    enqueue_one(
        &transaction,
        &duplicate_intent,
        EnqueueContext {
            enqueued_unix_ms: 1_014,
            catalog_revision: 0,
            policy,
            evidence: None,
            protected_change_ids: &[],
            allow_scope_degradation: true,
        },
        &mut duplicate_report,
    )
    .expect("coalesce duplicate precise P0 work");
    transaction
        .commit()
        .expect("commit duplicate leased-gap coalescing transaction");
    assert_eq!(duplicate_report.inserted_count, 0);
    assert_eq!(duplicate_report.coalesced_count, 1);
    assert_eq!(duplicate_report.superseded_count, 0);

    let evidence: (
        i64,
        String,
        i64,
        Option<i64>,
        String,
        i64,
        String,
        String,
        i64,
    ) = catalog
        .connection
        .query_row(
            "SELECT
               gap.id, gap.status, gap.lease_generation, gap.lease_expires_unix_ms,
               gap.last_failure_code,
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims AS claim
                WHERE claim.gap_change_id = gap.id),
               path.status, path.relative_path, path.coalesced_observation_count
             FROM library_change_queue AS gap
             JOIN library_change_queue_lanes AS gap_lane ON gap_lane.change_id = gap.id
             JOIN library_change_queue AS path
               ON path.root_id = gap.root_id
              AND path.root_generation = gap.root_generation
              AND path.scope = 'path'
             JOIN library_change_queue_lanes AS path_lane ON path_lane.change_id = path.id
             WHERE gap.id = ?1 AND gap_lane.lane = 'p0_live'
               AND path_lane.lane = 'p0_live'",
            [i64::try_from(leased_gap.change.id.value()).expect("gap change ID")],
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
        .expect("load leased gap and precise P0 evidence");
    assert_eq!(
        evidence,
        (
            i64::try_from(leased_gap.change.id.value()).expect("gap ID"),
            "leased".to_owned(),
            i64::try_from(leased_gap.lease_generation).expect("lease generation"),
            Some(leased_gap.lease_expires_unix_ms),
            LibraryChangeCapacityDeferral::MetadataInventoryLane
                .failure_code()
                .to_owned(),
            0,
            "pending".to_owned(),
            "visible-now.jpg".to_owned(),
            2,
        )
    );

    let precise = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_014,
            policy,
        )
        .expect("lease precise P0 work")
        .pop()
        .expect("precise P0 work remains independently leasable");
    assert_eq!(precise.change.intent.relative_path, "visible-now.jpg");
    let retained_gap: (String, i64, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, lease_generation, lease_expires_unix_ms
             FROM library_change_queue WHERE id = ?1",
            [i64::try_from(leased_gap.change.id.value()).expect("gap change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load retained gap lease");
    assert_eq!(
        retained_gap,
        (
            "leased".to_owned(),
            i64::try_from(leased_gap.lease_generation).expect("lease generation"),
            Some(leased_gap.lease_expires_unix_ms),
        )
    );
}

#[test]
fn generic_retry_rejects_capacity_reserved_codes_without_mutating_the_lease() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 2,
        lease_duration_millis: 100,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue root live gap");
    let leased = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease root live gap")
        .expect("root live gap");
    let state = |catalog: &SqliteCatalog| {
        catalog
            .connection
            .query_row(
                "SELECT status, attempt_count, next_retry_unix_ms, lease_generation,
                        lease_expires_unix_ms, last_failure_code, last_failure_message,
                        updated_unix_ms
                 FROM library_change_queue WHERE id = ?1",
                [i64::try_from(leased.change.id.value()).expect("change ID")],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )
            .expect("load queue state")
    };
    let before = state(&catalog);
    for code in [
        LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
        "live_gap_p2_capacity_future_reserved",
    ] {
        let result = catalog.retry_library_change(
            leased.change.id,
            leased.lease_generation,
            &LibraryChangeFailure {
                code: code.to_owned(),
                message: "A generic caller must not mint typed capacity state".to_owned(),
            },
            1_001,
            policy,
        );
        if result.is_ok() {
            let forged = catalog
                .lease_live_authoritative_library_change("root-a", generation, 1_011, policy)
                .expect("re-lease forged capacity state")
                .expect("forged capacity state stays retryable");
            assert!(
                catalog
                    .lease_live_authoritative_library_change(
                        "root-a",
                        generation,
                        forged.lease_expires_unix_ms,
                        policy,
                    )
                    .expect("recover forged expired lease")
                    .is_none()
            );
            let forged_state = state(&catalog);
            panic!(
                "generic retry forged reserved capacity state and lease expiry refunded it: {forged_state:?}"
            );
        }
        let error = result.expect_err("generic retry must reject reserved capacity codes");
        assert_eq!(error.code, "change_queue_failure_code_reserved");
        assert_eq!(state(&catalog), before);
    }
}

#[test]
fn forged_capacity_code_with_wrong_scope_does_not_refund_an_expired_lease() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        lease_duration_millis: 100,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                1,
                1_000,
                "wrong-scope.jpg",
            )],
            1_000,
            policy,
        )
        .expect("enqueue P0 path");
    let leased = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Live,
            1_000,
            policy,
        )
        .expect("lease P0 path")
        .pop()
        .expect("P0 path");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = ?1, last_failure_message = 'forged'
             WHERE id = ?2",
            rusqlite::params![
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
                i64::try_from(leased.change.id.value()).expect("change ID"),
            ],
        )
        .expect("forge reserved code on wrong scope");
    assert!(
        catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Live,
                leased.lease_expires_unix_ms,
                policy,
            )
            .expect("recover expired wrong-scope lease")
            .is_empty()
    );
    let terminal: (String, i64, Option<i64>, String) = catalog
        .connection
        .query_row(
            "SELECT status, attempt_count, next_retry_unix_ms, last_failure_code
             FROM library_change_queue WHERE id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load wrong-scope terminal state");
    assert_eq!(
        terminal,
        (
            "retry_wait".to_owned(),
            1,
            None,
            "change_lease_expired".to_owned(),
        )
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 1_200, policy)
        .expect("load wrong-scope metrics");
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("change_lease_expired")
    );
}

#[test]
fn forged_capacity_code_with_a_recovery_claim_does_not_escape_terminal_state() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    seed_current_journal_authority(&mut catalog);
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue root live gap");
    let leased = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease root live gap")
        .expect("root live gap");
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                leased.change.id,
                leased.lease_generation,
                &LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "The watcher gap needs a durable consumer".to_owned(),
                },
                1_001,
                policy,
            )
            .expect("bind pending journal claim"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET attempt_count = ?1, next_retry_unix_ms = 1_002,
                 last_failure_code = ?2, last_failure_message = 'forged'
             WHERE id = ?3 AND status = 'retry_wait'",
            rusqlite::params![
                i64::from(policy.max_attempts),
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
                i64::try_from(leased.change.id.value()).expect("change ID"),
            ],
        )
        .expect("forge reserved code on claimed gap");
    assert!(
        catalog
            .lease_live_authoritative_library_change("root-a", generation, 2_000, policy)
            .expect("enforce claimed-gap retry budget")
            .is_none()
    );
    let terminal: (Option<i64>, String, i64) = catalog
        .connection
        .query_row(
            "SELECT queue.next_retry_unix_ms, queue.last_failure_code,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = queue.id
                       AND claim.consumer_kind = 'pending_journal')
             FROM library_change_queue AS queue WHERE queue.id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load claimed-gap terminal state");
    assert_eq!(
        terminal,
        (
            None,
            LibraryChangeCapacityDeferral::MetadataInventoryLane
                .failure_code()
                .to_owned(),
            1,
        )
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 2_000, policy)
        .expect("load claimed-gap metrics");
    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some(LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code())
    );
}

#[test]
fn forged_capacity_code_in_the_wrong_lane_does_not_escape_terminal_state() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let mut wrong_lane = intent(
        "root-a",
        generation,
        1,
        1_000,
        LibraryChangeIntentKind::FreshnessUnknown,
        LibraryChangeScope::Root,
        "",
    );
    wrong_lane.origin = LibraryChangeOrigin::UserRefresh;
    catalog
        .enqueue_library_change_intents(&[wrong_lane], 1_000, policy)
        .expect("enqueue wrong-lane root work");
    let leased = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease wrong-lane root work")
        .expect("wrong-lane root work");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'retry_wait', attempt_count = ?1, next_retry_unix_ms = 1_002,
                 lease_expires_unix_ms = NULL, last_failure_code = ?2,
                 last_failure_message = 'forged'
             WHERE id = ?3",
            rusqlite::params![
                i64::from(policy.max_attempts),
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
                i64::try_from(leased.change.id.value()).expect("change ID"),
            ],
        )
        .expect("forge reserved code in P2 lane");
    assert!(
        catalog
            .lease_authoritative_library_change("root-a", generation, 2_000, policy)
            .expect("enforce wrong-lane retry budget")
            .is_none()
    );
    let terminal: (Option<i64>, String, String) = catalog
        .connection
        .query_row(
            "SELECT queue.next_retry_unix_ms, queue.last_failure_code, lane.lane
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE queue.id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load wrong-lane terminal state");
    assert_eq!(
        terminal,
        (
            None,
            LibraryChangeCapacityDeferral::MetadataInventoryLane
                .failure_code()
                .to_owned(),
            "p2_recovery".to_owned(),
        )
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", generation, 2_000, policy)
        .expect("load wrong-lane metrics");
    assert_eq!(metrics.exhausted_retry_count, 1);
}

#[test]
fn ordinary_pending_root_does_not_absorb_precise_dirty_work() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            1_000,
            policy,
        )
        .expect("enqueue pending root work");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = ?1, last_failure_message = 'forged'
             WHERE root_id = 'root-a' AND status = 'pending'",
            [LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code()],
        )
        .expect("forge reserved code in pending status");
    let report = catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                2,
                1_001,
                "normally-covered.jpg",
            )],
            1_001,
            policy,
        )
        .expect("preserve path beside ordinary pending root");
    assert_eq!(report.inserted_count, 1);
    assert_eq!(report.coalesced_count, 0);
    assert_eq!(report.superseded_count, 0);
    let shape: (i64, String, String, i64) = catalog
        .connection
        .query_row(
            "SELECT COUNT(*), MIN(scope), MIN(status),
                    SUM(coalesced_observation_count)
             FROM library_change_queue
             WHERE root_id = 'root-a' AND status IN ('pending', 'leased', 'retry_wait')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load ordinary root coalescing evidence");
    assert_eq!(shape, (2, "path".to_owned(), "pending".to_owned(), 2));
}

pub(super) fn queue_catalog(path: PathBuf) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let generation = register_root(&mut catalog, 1);
    assert_eq!(generation, LibraryRootGeneration::initial());
    catalog
}

fn seed_explicit_recovery_claim(catalog: &SqliteCatalog, observed_unix_ms: i64) -> i64 {
    let catalog_revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("catalog revision");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               'root-a', 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               ?1, ?1, ?2, ?2, 1, 'retry_wait', ?1, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The ambiguous historical gap requires an explicit library update',
               ?3, ?1, ?1
             )",
            rusqlite::params![
                observed_unix_ms,
                observed_unix_ms.to_string(),
                catalog_revision,
            ],
        )
        .expect("insert explicit recovery gap");
    let gap_change_id = catalog.connection.last_insert_rowid();
    catalog
        .connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind, created_unix_ms
             ) VALUES (?1, 'root-a', 1, 'explicit_recovery_required', ?2)",
            rusqlite::params![gap_change_id, observed_unix_ms],
        )
        .expect("insert explicit recovery claim");
    gap_change_id
}

fn seed_pending_journal_claim(catalog: &mut SqliteCatalog, observed_unix_ms: i64) -> i64 {
    seed_current_journal_authority(catalog);
    let generation = LibraryRootGeneration::initial();
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                91,
                observed_unix_ms,
                LibraryChangeIntentKind::FreshnessUnknown,
                LibraryChangeScope::Root,
                "",
            )],
            observed_unix_ms,
            immediate_policy(),
        )
        .expect("enqueue live gap");
    let leased = catalog
        .lease_live_authoritative_library_change(
            "root-a",
            generation,
            observed_unix_ms,
            immediate_policy(),
        )
        .expect("lease live gap")
        .expect("live gap work");
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                leased.change.id,
                leased.lease_generation,
                &LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "The watcher gap needs a durable consumer".to_owned(),
                },
                observed_unix_ms + 1,
                immediate_policy(),
            )
            .expect("bind pending journal claim"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    i64::try_from(leased.change.id.value()).expect("live gap change ID")
}

fn seed_current_journal_authority(catalog: &mut SqliteCatalog) -> PersistentJournalCheckpoint {
    let checkpoint = PersistentJournalCheckpoint {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "watcher-gap-volume".to_owned(),
            volume_serial: 41,
        },
        root_file_reference: JournalFileReference::V3([7; 16]),
        journal_id: JournalIdentifier::new(83).expect("journal ID"),
        next_unread_usn: JournalUsn::new(1_024).expect("opening USN"),
        captured_exclusive_end: JournalUsn::new(1_024).expect("opening end"),
        covered_catalog_revision: 0,
        protocol_version: 5,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::Current,
        failure: None,
        updated_unix_ms: 900,
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
            updated_unix_ms: checkpoint.updated_unix_ms,
        })
        .expect("current journal authority");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&checkpoint)
        .expect("current journal checkpoint");
    checkpoint
}

fn recovery_inventory_catalog(
    path: PathBuf,
    run_id: &str,
    relative_paths: &[&str],
) -> (SqliteCatalog, LeasedLibraryChange) {
    let source_root = path
        .parent()
        .expect("catalog parent")
        .join(format!("{run_id}-source"));
    std::fs::create_dir(&source_root).expect("recovery source root");
    let source_root_path = source_root.to_string_lossy().into_owned();
    let mut catalog = queue_catalog(path);
    catalog
        .connection
        .execute(
            "UPDATE library_roots SET path = ?1 WHERE id = 'root-a'",
            [&source_root_path],
        )
        .expect("bind recovery source root");
    catalog
        .connection
        .execute(
            "INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms,
               preview_edge, root_generation_at_start
             ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 128, 1)",
            [],
        )
        .expect("published scan fixture");
    catalog
        .connection
        .execute(
            "UPDATE library_roots SET active_scan_id = 'published-scan' WHERE id = 'root-a'",
            [],
        )
        .expect("activate published scan fixture");
    let generation = LibraryRootGeneration::initial();
    let mut recovery = intent(
        "root-a",
        generation,
        1,
        1_000,
        LibraryChangeIntentKind::FreshnessUnknown,
        LibraryChangeScope::Root,
        "",
    );
    recovery.origin = LibraryChangeOrigin::ConsistencyAudit;
    catalog
        .enqueue_library_change_intents(&[recovery], 1_000, immediate_policy())
        .expect("enqueue recovery control");
    let authority = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, immediate_policy())
        .expect("lease recovery control")
        .expect("recovery control");
    catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: authority.change.id,
            run_id: run_id.to_owned(),
            root_id: "root-a".to_owned(),
            root_generation: generation,
            reason: LibraryRecoveryAuthorityReason::ContainmentFailure,
            opening_boundary: None,
            authorized_unix_ms: 1_000,
            retired_unix_ms: None,
        })
        .expect("persist recovery authority");
    let run = catalog
        .begin_metadata_inventory(&MetadataInventoryRunRequest {
            run_id: run_id.to_owned(),
            root_id: "root-a".to_owned(),
            root_generation: generation,
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 1_000,
        })
        .expect("begin recovery inventory");
    let root_identity = crate::adapters::FileDiscovery::new(&source_root_path)
        .expect("pin recovery source root")
        .metadata_inventory_root_identity()
        .expect("read recovery source identity")
        .expect("stable recovery source identity");
    catalog
        .initialize_metadata_inventory_spool(&run, &authority, &root_identity, None, None, 1_000)
        .expect("persist recovery source proof");
    catalog
        .stage_metadata_inventory_page(
            run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: relative_paths
                    .iter()
                    .map(|relative_path| MetadataInventoryEntry {
                        relative_path: (*relative_path).to_owned(),
                        kind: MetadataInventoryEntryKind::File,
                        file_size: Some(1),
                        modified_unix_ms: 1,
                        file_identity: None,
                        source_revision: None,
                        placeholder_state: MetadataInventoryPlaceholderState::Available,
                        is_reparse_point: false,
                    })
                    .collect(),
                cursor: relative_paths.last().map(|path| (*path).to_owned()),
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
            },
            1_001,
        )
        .expect("stage recovery inventory page");
    (catalog, authority)
}

fn metadata_candidate_intent(relative_path: &str, sequence: u64) -> LibraryChangeIntent {
    let mut intent = path_intent(
        "root-a",
        LibraryRootGeneration::initial(),
        sequence,
        1_010,
        relative_path,
    );
    intent.origin = LibraryChangeOrigin::MetadataInventory;
    intent
}

fn metadata_candidate_update(relative_path: &str) -> MetadataInventoryComparisonUpdate {
    MetadataInventoryComparisonUpdate {
        relative_path: relative_path.to_owned(),
        status: MetadataInventoryComparisonStatus::Enqueued,
        candidate_previous_relative_path: None,
    }
}

fn recovery_publication_rollback_evidence(
    catalog: &SqliteCatalog,
    run_id: &str,
) -> (String, i64, i64, i64, Option<String>) {
    catalog
        .connection
        .query_row(
            "SELECT entry.comparison_status, run.candidate_count,
                    (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                     WHERE run_id = run.id),
                    (SELECT COUNT(*) FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     WHERE queue.root_id = run.root_id
                       AND queue.scope = 'path'
                       AND lane.lane = 'p2_recovery'),
                    run.comparison_cursor
             FROM library_metadata_inventory_runs AS run
             JOIN library_metadata_inventory_entries AS entry ON entry.run_id = run.id
             WHERE run.id = ?1",
            [run_id],
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
        .expect("recovery publication rollback evidence")
}

fn register_root(catalog: &mut SqliteCatalog, now_unix_ms: i64) -> LibraryRootGeneration {
    let transaction = catalog.connection.transaction().expect("root transaction");
    transaction
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES (?1, ?2, 1)",
            ["root-a", "C:\\Source"],
        )
        .expect("registered root fixture");
    let generation = activate_root_change_queue(&transaction, "root-a", now_unix_ms)
        .expect("root generation authority");
    transaction.commit().expect("registered root transaction");
    generation
}

fn planning_context(
    root_id: &str,
    generation: LibraryRootGeneration,
) -> LibraryChangePlanningContext {
    LibraryChangePlanningContext {
        root_id: root_id.to_owned(),
        root_generation: generation,
        availability: LibraryRootAvailability::Available,
        source_health: LibraryChangeSourceHealth::Healthy,
    }
}

fn scan_request(scan_id: &str) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: "C:\\Source".to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    }
}

fn observation(
    root_id: &str,
    generation: LibraryRootGeneration,
    sequence: u64,
    observed_unix_ms: i64,
    relative_path: &str,
) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: root_id.to_owned(),
        root_generation: generation,
        sequence,
        observed_unix_ms,
        kind: LibraryChangeObservationKind::Modified,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
    }
}

pub(super) fn path_intent(
    root_id: &str,
    generation: LibraryRootGeneration,
    sequence: u64,
    observed_unix_ms: i64,
    relative_path: &str,
) -> LibraryChangeIntent {
    intent(
        root_id,
        generation,
        sequence,
        observed_unix_ms,
        LibraryChangeIntentKind::Reconcile,
        LibraryChangeScope::Path,
        relative_path,
    )
}

fn rename_intent(
    root_id: &str,
    generation: LibraryRootGeneration,
    sequence: u64,
    observed_unix_ms: i64,
    previous_relative_path: &str,
    relative_path: &str,
) -> LibraryChangeIntent {
    let mut intent = intent(
        root_id,
        generation,
        sequence,
        observed_unix_ms,
        LibraryChangeIntentKind::RenameCandidate,
        LibraryChangeScope::Path,
        relative_path,
    );
    intent.previous_relative_path = Some(previous_relative_path.to_owned());
    intent
}

fn intent(
    root_id: &str,
    generation: LibraryRootGeneration,
    sequence: u64,
    observed_unix_ms: i64,
    kind: LibraryChangeIntentKind,
    scope: LibraryChangeScope,
    relative_path: &str,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root_id.to_owned(),
        root_generation: generation,
        kind,
        scope,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: observed_unix_ms,
        most_recent_observed_unix_ms: observed_unix_ms,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: 1,
    }
}

fn assert_live_dirty_rename_coalescing(rename_first: bool) {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let (rename_sequence, rename_observed, dirty_sequence, dirty_observed) = if rename_first {
        (1, 1_000, 2, 1_001)
    } else {
        (2, 1_001, 1, 1_000)
    };
    let rename = rename_intent(
        "root-a",
        generation,
        rename_sequence,
        rename_observed,
        "old/photo.jpg",
        "new/photo.jpg",
    );
    let dirty = path_intent(
        "root-a",
        generation,
        dirty_sequence,
        dirty_observed,
        "new/photo.jpg",
    );
    let intents = if rename_first {
        [rename, dirty]
    } else {
        [dirty, rename]
    };
    for intent in intents {
        let observed_unix_ms = intent.most_recent_observed_unix_ms;
        catalog
            .enqueue_library_change_intents(&[intent], observed_unix_ms, immediate_policy())
            .expect("enqueue dirty rename evidence");
    }

    let leased = catalog
        .lease_library_changes("root-a", generation, 1_001, immediate_policy())
        .expect("lease dirty rename");

    assert_eq!(leased.len(), 1);
    assert_dirty_rename_intent(&leased[0].change.intent);
}

fn assert_dirty_rename_intent(intent: &LibraryChangeIntent) {
    assert_eq!(intent.kind, LibraryChangeIntentKind::Reconcile);
    assert_eq!(intent.scope, LibraryChangeScope::Path);
    assert_eq!(intent.relative_path, "new/photo.jpg");
    assert_eq!(
        intent.previous_relative_path.as_deref(),
        Some("old/photo.jpg")
    );
    assert_eq!(intent.first_sequence, 1);
    assert_eq!(intent.most_recent_sequence, 2);
    assert_eq!(intent.coalesced_observation_count, 2);
}

fn fixture_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 500,
        max_unresolved_changes: 16,
        max_lease_batch: 16,
        lease_duration_millis: 1_000,
        max_attempts: 4,
        retry_initial_delay_millis: 10,
        retry_maximum_delay_millis: 100,
        terminal_retention_millis: 60_000,
        cleanup_batch: 16,
    }
}

fn immediate_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..fixture_policy()
    }
}

fn retry_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        lease_duration_millis: 100,
        ..fixture_policy()
    }
}

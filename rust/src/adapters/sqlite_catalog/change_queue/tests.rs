use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::tempdir;

use crate::application::{enqueue_library_change_plan, plan_library_changes};
use crate::domain::{
    LibraryChangeCatchUpEvidence, LibraryChangeCatchUpQueueBatch, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningContext, LibraryChangePlanningLimits,
    LibraryChangeQueueHealth, LibraryChangeScope, LibraryChangeSourceHealth,
    LibraryRootAvailability, ScanRequest,
};
use crate::ports::CatalogRepository;

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
             CREATE TABLE asset_locations (
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               scan_id TEXT NOT NULL,
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

    assert_eq!(version, 19);
    assert_eq!(revision, 7);
    assert_eq!(preserved, "kept");
    assert!(queue_exists);
    assert!(scan_owner_exists);
    assert!(contract_valid);
    assert_eq!(generation, 1);
    assert!(is_active);
}

#[test]
fn authoritative_scan_publication_preserves_evidence_arriving_after_its_watermark() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(path);
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
        .expect("enqueue recovery gap");
    let request = scan_request("authoritative-scan");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin authoritative scan");

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
    catalog
        .publish_scan("authoritative-scan", "root-a", 0, 0)
        .expect("publish authoritative scan");

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

    assert!(report.freshness_unknown_enqueued);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, "superseded");
    assert_eq!(rows[1].1, "pending");
    assert!(rows.iter().all(|row| row.2.is_none()));
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.completed_count, 0);
}

#[test]
fn abandoning_authoritative_scan_releases_only_its_frozen_work() {
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
    catalog
        .enqueue_library_change_intents(
            &[path_intent(
                "root-a",
                generation,
                2,
                1_001,
                "scan-owned.jpg",
            )],
            1_001,
            policy,
        )
        .expect("enqueue scan-owned path");
    let request = scan_request("abandoned-authoritative-scan");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin authoritative scan");
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

    assert_eq!(rows.len(), 2);
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
    let released = catalog
        .lease_path_library_changes("root-a", generation, rows[1].2, policy)
        .expect("lease released scan work");
    assert_eq!(released.len(), 1);
    assert_eq!(
        released[0].change.id.value(),
        u64::try_from(rows[1].0).expect("released queue id")
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
    let mut restarted = path_intent("root-a", generation, 1, 2_000, "photo.jpg");
    restarted.origin = LibraryChangeOrigin::ConsistencyAudit;

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
        LibraryChangeOrigin::ConsistencyAudit,
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
    let mut restarted = path_intent("root-a", generation, 1, 1_000, "photo.jpg");
    restarted.origin = LibraryChangeOrigin::UserRefresh;
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
        LibraryChangeOrigin::UserRefresh
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
fn later_old_path_evidence_invalidates_a_leased_rename() {
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
fn old_subtree_descendant_invalidates_a_leased_directory_rename() {
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

    catalog
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
    let replacement = catalog
        .lease_library_changes("root-a", generation, 1_001, policy)
        .expect("lease descendant reconciliation")
        .pop()
        .expect("descendant reconciliation");

    assert_eq!(stale_outcome, LibraryChangeLeaseUpdateOutcome::Superseded);
    assert_eq!(
        replacement.change.intent.kind,
        LibraryChangeIntentKind::RenameCandidate,
    );
    assert_eq!(replacement.change.intent.scope, LibraryChangeScope::Subtree);
    assert_eq!(replacement.change.intent.relative_path, "new/album");
    assert_eq!(
        replacement.change.intent.previous_relative_path.as_deref(),
        Some("old/album"),
    );
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
fn parent_subtree_supersedes_unleased_child_work() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = fixture_policy();
    let mut catalog = queue_catalog(path);

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
    let metrics = catalog
        .load_library_change_queue_metrics(1_100, policy)
        .expect("metrics");
    let leased = catalog
        .lease_library_changes("root-a", generation, 1_600, policy)
        .expect("lease subtree");

    assert_eq!(report.superseded_count, 1);
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.superseded_count, 1);
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.intent.scope, LibraryChangeScope::Subtree);
    assert_eq!(leased[0].change.intent.relative_path, "album");
    assert_eq!(leased[0].change.intent.coalesced_observation_count, 2);
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
    let mut catalog = queue_catalog(path);
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", first_generation, 1, 1_000, "old.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue old generation");
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

    assert_eq!(replacement_report.superseded_count, 1);
    assert_eq!(stale_report.stale_generation_count, 1);
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
}

#[test]
fn unregistering_a_root_retires_its_generation_and_unresolved_work() {
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
        .expect("enqueue work");

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
    assert_eq!(metrics.superseded_count, 1);
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
fn normalized_capacity_overflow_degrades_to_one_root_gap() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(path);
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
fn lowered_capacity_accepts_a_parent_subtree_that_absorbs_existing_work() {
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

    assert!(!report.capacity_degraded);
    assert!(!report.freshness_unknown_enqueued);
    assert_eq!(report.superseded_count, 2);
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].change.intent.scope, LibraryChangeScope::Subtree);
    assert_eq!(retained[0].change.intent.relative_path, "album");
}

#[test]
fn lowered_capacity_accepts_root_reconciliation_that_absorbs_existing_work() {
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

    assert!(!report.capacity_degraded);
    assert!(!report.freshness_unknown_enqueued);
    assert_eq!(report.superseded_count, 2);
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].change.intent.scope, LibraryChangeScope::Root);
    assert_eq!(
        retained[0].change.intent.kind,
        LibraryChangeIntentKind::Reconcile,
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
fn catch_up_evidence_survives_persistence_and_later_live_coalescing() {
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
        .expect("coalesce live evidence");

    let leased = catalog
        .lease_path_library_changes("root-a", generation, 1_001, policy)
        .expect("lease retained evidence")
        .pop()
        .expect("change");

    assert_eq!(
        leased.change.catch_up_source.as_deref(),
        Some("windows_usn_v1")
    );
    assert_eq!(
        leased.change.catch_up_watermark.as_deref(),
        Some("volume|12|40")
    );
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

fn queue_catalog(path: PathBuf) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let generation = register_root(&mut catalog, 1);
    assert_eq!(generation, LibraryRootGeneration::initial());
    catalog
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

fn path_intent(
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

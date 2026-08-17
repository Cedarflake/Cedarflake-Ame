use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::tempdir;

use crate::application::{enqueue_library_change_plan, plan_library_changes};
use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeObservation,
    LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangePlanningContext,
    LibraryChangePlanningLimits, LibraryChangeQueueHealth, LibraryChangeScope,
    LibraryChangeSourceHealth, LibraryRootAvailability,
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
             CREATE TABLE preserved_fixture(value TEXT NOT NULL);
             INSERT INTO preserved_fixture(value) VALUES ('kept');",
        )
        .expect("v16 fixture");
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, revision, preserved, queue_exists): (i64, i64, String, bool) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT revision FROM catalog_state),
               (SELECT value FROM preserved_fixture),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_queue')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migrated evidence");

    assert_eq!(version, 17);
    assert_eq!(revision, 7);
    assert_eq!(preserved, "kept");
    assert!(queue_exists);
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
fn removed_root_without_queue_state_rejects_a_later_generation() {
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
    catalog
        .connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('root-a', 'C:\\Source', 2)",
            [],
        )
        .expect("re-register root fixture");

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

fn queue_catalog(path: PathBuf) -> SqliteCatalog {
    let catalog = SqliteCatalog::open(path).expect("catalog");
    catalog
        .connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES (?1, ?2, 1)",
            ["root-a", "C:\\Source"],
        )
        .expect("registered root fixture");
    catalog
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

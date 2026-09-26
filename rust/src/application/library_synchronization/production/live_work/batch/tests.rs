use std::cell::Cell;

use super::*;
use crate::application::library_synchronization::production::tests::{
    ProductionGapFixture, live_worker::enqueue_deletions,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue};

fn deletion_fixture(name: &str) -> ProductionGapFixture {
    let fixture = ProductionGapFixture::new_with_media_baseline(name);
    enqueue_deletions(
        &fixture,
        &fixture.storage,
        &["removed.png", "unchanged.png"],
    );
    fixture
}

#[test]
fn ready_deletions_continue_with_fresh_revisions_and_admission_times() {
    let fixture = deletion_fixture("controlled-live-continuation");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let cancelled = AtomicBool::new(false);
    let policy = LibraryChangeQueuePolicy::default();
    let mut reconciliation = LiveReconciliation {
        catalog: &mut catalog,
        root_id: &fixture.root_id,
        root_generation: LibraryRootGeneration::initial(),
        queue_policy: policy,
        recovery_policy: AuthoritativeRecoveryPolicy::default(),
        cancelled: &cancelled,
    };
    let initial_time = now_unix_ms().expect("admission clock");
    let mut admission_time = initial_time;
    let outcome = run_quantum(
        policy.max_lease_batch,
        &cancelled,
        || Duration::ZERO,
        |first| {
            let step = reconciliation.next(first, admission_time);
            admission_time += 37;
            step
        },
    );
    assert!(outcome.failure.is_none());
    assert_eq!(outcome.report.completed_count, 2);
    assert_eq!(outcome.report.applied_mutation_count, 2);
    for path in ["removed.png", "unchanged.png"] {
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, path)
                .expect("active membership")
                .is_none()
        );
    }
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).expect("evidence");
    let times: (i64, i64) = connection
        .query_row(
            "SELECT MIN(updated_unix_ms), MAX(updated_unix_ms)
         FROM library_change_queue WHERE root_id = ?1 AND status = 'completed'",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("independent durable completion times");
    assert_eq!(times, (initial_time, initial_time + 37));
}

#[test]
fn root_retirement_between_successful_scopes_stops_the_next_admission() {
    for replace_generation in [false, true] {
        let fixture = deletion_fixture("controlled-live-root-retirement");
        let mut catalog =
            SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        let cancelled = AtomicBool::new(false);
        let policy = LibraryChangeQueuePolicy::default();
        let mut reconciliation = LiveReconciliation {
            catalog: &mut catalog,
            root_id: &fixture.root_id,
            root_generation: LibraryRootGeneration::initial(),
            queue_policy: policy,
            recovery_policy: AuthoritativeRecoveryPolicy::default(),
            cancelled: &cancelled,
        };
        let now = now_unix_ms().expect("clock");
        let mut admissions = 0;
        let outcome = run_quantum(
            policy.max_lease_batch,
            &cancelled,
            || Duration::ZERO,
            |first| {
                let step = reconciliation.next(first, now)?;
                admissions += 1;
                if admissions == 1 {
                    assert!(matches!(step, LiveStep::Scope(report) if report.completed_count == 1));
                    if replace_generation {
                        reconciliation
                            .catalog
                            .enqueue_library_change_intents(
                                &[crate::domain::LibraryChangeIntent {
                                    root_id: fixture.root_id.clone(),
                                    root_generation: LibraryRootGeneration::initial()
                                        .next()
                                        .expect("next generation"),
                                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                                    scope: crate::domain::LibraryChangeScope::Subtree,
                                    relative_path: "new-owner.png".to_owned(),
                                    previous_relative_path: None,
                                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                                    first_observed_unix_ms: now - 1_000,
                                    most_recent_observed_unix_ms: now - 1_000,
                                    first_sequence: 3,
                                    most_recent_sequence: 3,
                                    coalesced_observation_count: 1,
                                }],
                                now - 1_000,
                                policy,
                            )
                            .expect("advance generation after a successful scope");
                    } else {
                        assert!(
                            reconciliation
                                .catalog
                                .unregister_root(&fixture.root_id)
                                .expect("unregister root after a successful scope")
                        );
                    }
                }
                Ok(step)
            },
        );
        assert_eq!(
            admissions, 2,
            "next admission revalidates and observes retirement"
        );
        assert!(outcome.failure.is_none());
        assert_eq!(outcome.report.leased_count, 1);
        assert_eq!(outcome.report.applied_mutation_count, 1);
        let connection =
            rusqlite::Connection::open(&fixture.storage.catalog_path).expect("evidence");
        let leases: i64 = connection
            .query_row(
                "SELECT SUM(lease_generation) FROM library_change_queue WHERE root_id = ?1",
                [&fixture.root_id],
                |row| row.get(0),
            )
            .expect("no second or replacement lease");
        assert_eq!(leases, 1);
    }
}

fn completed(mutations: u32, revision: u64) -> IncrementalLibraryChangeReport {
    IncrementalLibraryChangeReport {
        leased_count: 1,
        completed_count: 1,
        applied_mutation_count: mutations,
        catalog_revision: revision,
        ..IncrementalLibraryChangeReport::default()
    }
}

#[test]
fn no_change_scopes_still_consume_the_batch_limit() {
    let admissions = Cell::new(0);
    let outcome = run_quantum(
        64,
        &AtomicBool::new(false),
        || Duration::ZERO,
        |_| {
            admissions.set(admissions.get() + 1);
            Ok(LiveStep::Scope(completed(0, 17)))
        },
    );
    assert_eq!(admissions.get(), 64);
    assert_eq!(outcome.report.leased_count, 64);
    assert_eq!(outcome.report.completed_count, 64);
    assert_eq!(outcome.report.applied_mutation_count, 0);
    assert!(outcome.failure.is_none());
}

#[test]
fn elapsed_quantum_stops_next_admission_without_discarding_completion() {
    let elapsed = Cell::new(Duration::from_millis(99));
    let admissions = Cell::new(0);
    let outcome = run_quantum(
        64,
        &AtomicBool::new(false),
        || elapsed.get(),
        |_| {
            admissions.set(admissions.get() + 1);
            elapsed.set(Duration::from_millis(101));
            Ok(LiveStep::Scope(completed(3, 18)))
        },
    );
    assert_eq!(admissions.get(), 1);
    assert_eq!(outcome.report.applied_mutation_count, 3);
}

#[test]
fn cancellation_between_scopes_preserves_the_committed_result() {
    let cancelled = AtomicBool::new(false);
    let admissions = Cell::new(0);
    let outcome = run_quantum(
        64,
        &cancelled,
        || Duration::ZERO,
        |_| {
            admissions.set(admissions.get() + 1);
            cancelled.store(true, Ordering::Release);
            Ok(LiveStep::Scope(completed(2, 19)))
        },
    );
    assert_eq!(admissions.get(), 1);
    assert_eq!(outcome.report.applied_mutation_count, 2);
    assert!(outcome.failure.is_none());
}

#[test]
fn already_cancelled_work_never_acquires_a_lease() {
    let outcome = run_quantum(
        64,
        &AtomicBool::new(true),
        || Duration::ZERO,
        |_| panic!("cancelled work cannot acquire a lease"),
    );
    assert_eq!(outcome.report.leased_count, 0);
}

#[test]
fn second_scope_failure_retains_prior_commits_and_the_original_error() {
    let calls = Cell::new(0);
    let outcome = run_quantum(
        64,
        &AtomicBool::new(false),
        || Duration::ZERO,
        |_| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                return Ok(LiveStep::Scope(completed(4, 21)));
            }
            Err(ScanError::new("fixture_second_scope", "controlled failure"))
        },
    );
    assert_eq!(calls.get(), 2);
    assert_eq!(outcome.report.applied_mutation_count, 4);
    assert_eq!(outcome.report.catalog_revision, 21);
    assert_eq!(
        outcome.failure.expect("retained error").code,
        "fixture_second_scope"
    );
}

#[test]
fn retry_deferral_and_supersession_end_the_quantum_without_releasing_another_lease() {
    for report in [
        IncrementalLibraryChangeReport {
            leased_count: 1,
            retried_count: 1,
            ..Default::default()
        },
        IncrementalLibraryChangeReport {
            leased_count: 1,
            deferred_count: 1,
            ..Default::default()
        },
        IncrementalLibraryChangeReport {
            leased_count: 1,
            superseded_count: 1,
            ..Default::default()
        },
    ] {
        let admissions = Cell::new(0);
        let outcome = run_quantum(
            64,
            &AtomicBool::new(false),
            || Duration::ZERO,
            |_| {
                admissions.set(admissions.get() + 1);
                Ok(LiveStep::Scope(report))
            },
        );
        assert_eq!(admissions.get(), 1);
        assert_eq!(outcome.report, report);
    }
}

#[test]
fn ordinary_path_batch_is_one_bounded_terminal_step() {
    let calls = Cell::new(0);
    let report = IncrementalLibraryChangeReport {
        leased_count: 64,
        completed_count: 64,
        applied_mutation_count: 32,
        catalog_revision: 22,
        ..Default::default()
    };
    let outcome = run_quantum(
        64,
        &AtomicBool::new(false),
        || Duration::ZERO,
        |first| {
            assert!(first);
            calls.set(calls.get() + 1);
            Ok(LiveStep::PathBatch(report))
        },
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(outcome.report, report);
}

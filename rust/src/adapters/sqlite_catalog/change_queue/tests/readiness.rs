use super::*;

type Readiness = fn(
    &SqliteCatalog,
    &str,
    LibraryRootGeneration,
    i64,
    LibraryChangeQueuePolicy,
) -> Result<bool, ScanError>;

#[test]
fn lane_readiness_preserves_due_retry_lease_and_exhaustion_boundaries() {
    let cases: [(LibraryChangeOrigin, LibraryChangeScope, Readiness); 4] = [
        (
            LibraryChangeOrigin::LiveNotification,
            LibraryChangeScope::Path,
            SqliteCatalog::has_ready_live_path_library_change,
        ),
        (
            LibraryChangeOrigin::StartupCatchUp,
            LibraryChangeScope::Path,
            SqliteCatalog::has_ready_journal_path_library_change,
        ),
        (
            LibraryChangeOrigin::ConsistencyAudit,
            LibraryChangeScope::Path,
            SqliteCatalog::has_ready_legacy_unowned_recovery_debt,
        ),
        (
            LibraryChangeOrigin::LiveNotification,
            LibraryChangeScope::Root,
            SqliteCatalog::has_ready_live_authoritative_library_change,
        ),
    ];
    for (origin, scope, ready) in cases {
        let directory = tempdir().expect("temporary catalog");
        let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
        let generation = LibraryRootGeneration::initial();
        let policy = LibraryChangeQueuePolicy {
            max_attempts: 2,
            lease_duration_millis: 100,
            ..fixture_policy()
        };
        let is_path = scope == LibraryChangeScope::Path;
        let mut change = intent(
            "root-a",
            generation,
            1,
            1_000,
            if is_path {
                LibraryChangeIntentKind::Reconcile
            } else {
                LibraryChangeIntentKind::FreshnessUnknown
            },
            scope,
            if is_path { "photo.jpg" } else { "" },
        );
        change.origin = origin;
        catalog
            .enqueue_library_change_intents(&[change], 1_000, policy)
            .expect("enqueue owned lane work");
        assert!(!ready(&catalog, "root-a", generation, 1_499, policy).expect("future pending"));
        assert!(ready(&catalog, "root-a", generation, 1_500, policy).expect("due pending"));
        assert!(!ready(&catalog, "root-b", generation, 1_500, policy).expect("other root"));
        assert!(
            !ready(
                &catalog,
                "root-a",
                generation.next().expect("next generation"),
                1_500,
                policy,
            )
            .expect("other generation")
        );
        for (lane, lane_ready) in [
            (
                LibraryChangeLane::Live,
                SqliteCatalog::has_ready_live_path_library_change as Readiness,
            ),
            (
                LibraryChangeLane::Journal,
                SqliteCatalog::has_ready_journal_path_library_change as Readiness,
            ),
        ] {
            assert_eq!(
                lane_ready(&catalog, "root-a", generation, 1_500, policy)
                    .expect("path lane isolation"),
                is_path && origin.lane() == lane,
            );
        }

        let first = catalog
            .lease_library_changes("root-a", generation, 1_500, policy)
            .expect("lease due work")
            .pop()
            .expect("first lease");
        assert!(!ready(&catalog, "root-a", generation, 1_599, policy).expect("active lease"));
        let lowered_policy = LibraryChangeQueuePolicy {
            max_attempts: 1,
            ..policy
        };
        assert!(
            ready(&catalog, "root-a", generation, 1_600, lowered_policy)
                .expect("expired lease remains discoverable at the attempt limit")
        );
        assert_eq!(
            catalog
                .retry_library_change(
                    first.change.id,
                    first.lease_generation,
                    &retry_failure(),
                    1_501,
                    policy,
                )
                .expect("schedule real retry"),
            LibraryChangeLeaseUpdateOutcome::Applied,
        );
        assert!(!ready(&catalog, "root-a", generation, 1_510, policy).expect("future retry"));
        assert!(ready(&catalog, "root-a", generation, 1_511, policy).expect("due retry"));
        assert!(
            !ready(&catalog, "root-a", generation, 1_511, lowered_policy)
                .expect("lowered retry limit excludes due retry")
        );
        let second = catalog
            .lease_library_changes("root-a", generation, 1_511, policy)
            .expect("lease retry")
            .pop()
            .expect("second lease");
        assert!(
            ready(&catalog, "root-a", generation, 1_611, policy)
                .expect("expired final attempt remains discoverable")
        );
        assert_eq!(
            catalog
                .retry_library_change(
                    second.change.id,
                    second.lease_generation,
                    &retry_failure(),
                    1_512,
                    policy,
                )
                .expect("exhaust real retry"),
            LibraryChangeLeaseUpdateOutcome::Applied,
        );
        assert!(!ready(&catalog, "root-a", generation, i64::MAX, policy).expect("exhausted retry"));
    }
}

#[test]
fn owned_candidate_readiness_retains_superseded_unresolved_evidence() {
    let directory = tempdir().expect("temporary catalog");
    let run_id = "readiness-owned-candidate";
    let (mut catalog, authority) = recovery_inventory_catalog(
        directory.path().join("catalog.sqlite3"),
        run_id,
        &["new.jpg"],
    );
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            run_id,
            &[metadata_candidate_intent("new.jpg", 10)],
            &[metadata_candidate_update("new.jpg")],
            1_010,
            policy,
        )
        .expect("publish owned candidate")
        .expect("current authority");
    assert!(
        catalog
            .has_ready_metadata_inventory_recovery_candidates("root-a", generation, 1_010, policy)
            .expect("current candidate ready")
    );
    assert!(
        !catalog
            .has_ready_legacy_unowned_recovery_debt("root-a", generation, 1_010, policy)
            .expect("owned candidate is not legacy debt")
    );
    // A retained owner over a superseded row must remain visible as unresolved evidence.
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', superseded_by_change_id = ?1
             WHERE id IN (SELECT change_id FROM library_metadata_inventory_candidate_owners
                          WHERE run_id = ?2)",
            params![
                sqlite_integer(authority.change.id.value(), "change ID").expect("change ID"),
                run_id,
            ],
        )
        .expect("inject retained superseded candidate owner");
    assert!(
        !catalog
            .has_ready_metadata_inventory_recovery_candidates("root-a", generation, 1_010, policy)
            .expect("superseded candidate cannot execute")
    );
    assert!(
        catalog
            .has_unresolved_metadata_inventory_recovery_candidates("root-a", generation)
            .expect("superseded candidate remains unresolved")
    );
}

#[test]
fn readiness_validates_inputs_without_requiring_an_optional_index() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let invalid_policy = LibraryChangeQueuePolicy {
        max_attempts: 0,
        ..policy
    };
    let queries: [Readiness; 6] = [
        SqliteCatalog::has_ready_live_path_library_change,
        SqliteCatalog::has_ready_journal_path_library_change,
        SqliteCatalog::has_ready_live_authoritative_library_change,
        SqliteCatalog::has_ready_metadata_inventory_recovery_candidates,
        SqliteCatalog::has_ready_legacy_unowned_recovery_debt,
        SqliteCatalog::has_ready_metadata_inventory_recovery,
    ];
    catalog
        .connection
        .execute_batch("DROP INDEX library_change_queue_eligible;")
        .expect("remove optional access-path index");
    for ready in queries {
        assert_eq!(
            ready(&catalog, "", generation, 1_000, invalid_policy)
                .expect_err("policy validation precedes root validation")
                .code,
            "change_queue_policy_invalid",
        );
        assert_eq!(
            ready(&catalog, "", generation, 1_000, policy)
                .expect_err("invalid root")
                .code,
            "change_queue_root_id_invalid",
        );
        assert!(!ready(&catalog, "root-a", generation, 1_000, policy).expect("empty queue"));
    }
    catalog
        .enqueue_library_change_intents(
            &[path_intent("root-a", generation, 1, 1_000, "photo.jpg")],
            1_000,
            policy,
        )
        .expect("enqueue without optional index");
    assert!(
        catalog
            .has_ready_live_path_library_change("root-a", generation, 1_000, policy)
            .expect("current work is still ready")
    );
    assert!(
        !catalog
            .has_unresolved_metadata_inventory_recovery_candidates("root-a", generation)
            .expect("unresolved lookup needs no policy or time")
    );
}

fn retry_failure() -> LibraryChangeFailure {
    LibraryChangeFailure {
        code: "source_busy".to_owned(),
        message: "The controlled source is temporarily busy".to_owned(),
    }
}

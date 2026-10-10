use super::*;

#[test]
fn capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust() {
    run_capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust(false);
}
#[test]
fn subtree_capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust() {
    run_capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust(true);
}
fn run_capacity_deferred_live_gap_yields_to_ready_p0_path_and_real_failures_still_exhaust(
    subtree: bool,
) {
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
                if subtree {
                    LibraryChangeIntentKind::Reconcile
                } else {
                    LibraryChangeIntentKind::FreshnessUnknown
                },
                if subtree {
                    LibraryChangeScope::Subtree
                } else {
                    LibraryChangeScope::Root
                },
                if subtree { "album" } else { "" },
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
                "album/visible-now.jpg",
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
    assert_eq!(
        ready_path.change.intent.relative_path,
        "album/visible-now.jpg"
    );
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
             WHERE root_id = 'root-a' AND scope <> 'path'",
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
    run_leased_capacity_deferred_live_gap_preserves_precise_p0_work_and_its_lease(false);
}
#[test]
fn subtree_leased_capacity_deferred_live_gap_preserves_precise_p0_work_and_its_lease() {
    run_leased_capacity_deferred_live_gap_preserves_precise_p0_work_and_its_lease(true);
}
fn run_leased_capacity_deferred_live_gap_preserves_precise_p0_work_and_its_lease(subtree: bool) {
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
                if subtree {
                    LibraryChangeIntentKind::Reconcile
                } else {
                    LibraryChangeIntentKind::FreshnessUnknown
                },
                if subtree {
                    LibraryChangeScope::Subtree
                } else {
                    LibraryChangeScope::Root
                },
                if subtree { "album" } else { "" },
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

    let first_intent = path_intent("root-a", generation, 4, 1_013, "album/visible-now.jpg");
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
    let duplicate_intent = path_intent("root-a", generation, 5, 1_014, "album/visible-now.jpg");
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
            "album/visible-now.jpg".to_owned(),
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
    assert_eq!(precise.change.intent.relative_path, "album/visible-now.jpg");
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
fn subtree_capacity_wait_survives_expiry_and_wakes_on_release() {
    let directory = tempdir().expect("directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1000,
            policy,
        )
        .expect("gap");
    let lease = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1000, policy)
        .expect("lease")
        .expect("gap");
    catalog
        .defer_library_change_for_capacity(
            lease.change.id,
            lease.lease_generation,
            LibraryChangeCapacityDeferral::MetadataInventoryLane,
            1000,
            policy,
        )
        .expect("defer");
    let due = capacity_deferral_deadline(1000, policy);
    let lease = catalog
        .lease_live_authoritative_library_change("root-a", generation, due, policy)
        .expect("due lease")
        .expect("gap");
    assert!(
        catalog
            .lease_live_authoritative_library_change(
                "root-a",
                generation,
                lease.lease_expires_unix_ms,
                policy
            )
            .expect("expired normalization")
            .is_none()
    );
    let tx = catalog.connection.transaction().expect("read wait");
    let after = load_change(&tx, sqlite_integer(lease.change.id.value(), "id").unwrap())
        .expect("preserved wait");
    assert_eq!(after.attempt_count, 0);
    assert_eq!(
        after.last_failure.unwrap().code,
        LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code()
    );
    tx.commit().expect("finish read");
    let tx = catalog
        .connection
        .transaction()
        .expect("release transaction");
    assert_eq!(
        wake_metadata_inventory_capacity_deferrals(
            &tx,
            "root-a",
            generation,
            lease.lease_expires_unix_ms
        )
        .expect("wake"),
        1
    );
    tx.commit().expect("release");
    assert!(
        catalog
            .lease_live_authoritative_library_change(
                "root-a",
                generation,
                lease.lease_expires_unix_ms,
                policy
            )
            .expect("woken")
            .is_some()
    );
}

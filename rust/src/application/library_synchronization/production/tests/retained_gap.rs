use super::*;

#[test]
fn production_retained_subtree_debt_allows_full_p2_lane_to_drain_then_transfers_once() {
    retained_subtree_transfers_once("metadata_inventory_required");
}

#[test]
fn production_retained_expired_subtree_recovers_without_resetting_attempts() {
    retained_subtree_transfers_once("change_lease_expired");
}

fn retained_subtree_transfers_once(failure_code: &str) {
    let fixture = ProductionGapFixture::new(failure_code, 0);
    std::fs::create_dir(fixture.source_root.join("album")).expect("album");
    std::fs::create_dir(fixture.source_root.join("other")).expect("other");
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 2,
        max_lease_batch: 1,
        retry_initial_delay_millis: 1,
        retry_maximum_delay_millis: 8,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let generation = LibraryRootGeneration::initial();
    let now = now_unix_ms().expect("clock");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let mut intent = crate::domain::LibraryChangeIntent {
        root_id: fixture.root_id.clone(),
        root_generation: generation,
        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
        scope: crate::domain::LibraryChangeScope::Subtree,
        relative_path: "other".to_owned(),
        previous_relative_path: None,
        origin: crate::domain::LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: now,
        most_recent_observed_unix_ms: now,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    };
    assert_eq!(
        policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery),
        1
    );
    catalog
        .enqueue_library_change_intents(&[intent.clone()], now, policy)
        .expect("existing gap");
    let existing = catalog
        .lease_live_authoritative_library_change(&fixture.root_id, generation, now, policy)
        .expect("existing lease")
        .expect("existing gap");
    catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            existing.change.id,
            existing.lease_generation,
            &crate::domain::LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "existing bounded recovery".to_owned(),
            },
            now,
            policy,
        )
        .expect("fill P2 with existing authority");
    intent.origin = crate::domain::LibraryChangeOrigin::LiveNotification;
    intent.scope = crate::domain::LibraryChangeScope::Subtree;
    intent.relative_path = "album".to_owned();
    intent.first_sequence = 100;
    intent.most_recent_sequence = 100;
    catalog
        .enqueue_library_change_intents(&[intent], now, policy)
        .expect("gap");
    let old_lease = catalog
        .lease_live_authoritative_library_change(&fixture.root_id, generation, now, policy)
        .expect("retained lease")
        .expect("retained gap");
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).expect("evidence");
    connection
        .execute(
            "UPDATE library_change_queue SET status='retry_wait', attempt_count=?1,
        next_retry_unix_ms=NULL, lease_expires_unix_ms=NULL, last_failure_code=?2,
        last_failure_message='retained bounded page'
        WHERE relative_path='album'",
            rusqlite::params![i64::from(policy.max_attempts), failure_code],
        )
        .expect("retained debt");
    assert!(
        !catalog
            .has_ready_live_authoritative_library_change(&fixture.root_id, generation, now, policy)
            .expect("capacity backpressure")
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics(&fixture.root_id, generation, now, policy)
        .expect("recoverable debt metrics");
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(metrics.exhausted_retry_count, 0);
    connection.execute("UPDATE library_change_queue SET last_failure_code='real_failure' WHERE relative_path='album'", []).expect("unrelated failure");
    assert_eq!(
        catalog
            .load_library_change_root_queue_metrics(&fixture.root_id, generation, now, policy)
            .expect("blocked metrics")
            .exhausted_retry_count,
        1
    );
    connection
        .execute(
            "UPDATE library_change_queue SET last_failure_code=?1 WHERE relative_path='album'",
            [failure_code],
        )
        .expect("restore debt");
    drop(catalog);
    let mut production =
        fast_gap_runtime(QueuedSourceFactory::default(), test_live_only_connection());
    production.runtime.queue_policy = policy;
    let snapshot = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(snapshot.roots[0].retry_wait_count, 0);
    let evidence: (i64, i64, i64, String) = connection.query_row(
        "SELECT attempt_count, (SELECT COUNT(*) FROM library_change_queue WHERE origin='metadata_inventory'),
          (SELECT COUNT(*) FROM library_recovery_authorities WHERE retired_unix_ms IS NULL), last_failure_code
         FROM library_change_queue WHERE origin='live_notification' AND relative_path='album' AND status='superseded'", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).expect("atomic one-time transfer");
    assert_eq!(
        evidence,
        (
            i64::from(policy.max_attempts),
            2,
            0,
            failure_code.to_owned()
        )
    );
    fixture.assert_no_automatic_full_scan();
    production.stop().expect("owned stop");
    let mut reopened =
        SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("FULL reopen");
    assert_eq!(
        reopened
            .complete_library_change(
                old_lease.change.id,
                old_lease.lease_generation,
                0,
                now + 10000
            )
            .expect("late old completion"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Superseded,
    );
    assert!(
        !reopened
            .has_ready_live_authoritative_library_change(
                &fixture.root_id,
                generation,
                now + 10000,
                policy
            )
            .expect("retired debt stays retired")
    );
    connection
        .execute(
            "UPDATE library_change_queue SET status='retry_wait', attempt_count=?1,
         next_retry_unix_ms=NULL, lease_expires_unix_ms=NULL, last_failure_code=?2,
         catalog_revision_at_success=NULL
         WHERE origin='metadata_inventory'",
            rusqlite::params![i64::from(policy.max_attempts), failure_code],
        )
        .expect("exhausted P2 cannot promote again");
    assert!(
        !reopened
            .has_ready_live_authoritative_library_change(
                &fixture.root_id,
                generation,
                now + 20000,
                policy
            )
            .expect("P2 is outside retained Live promotion"),
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM library_change_queue WHERE origin='metadata_inventory'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        2,
    );
}

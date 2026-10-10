use std::time::{Duration, Instant};

use crate::domain::{
    LibraryChangeFailure, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration,
};
use crate::ports::LibraryChangeQueue;

use super::*;

#[test]
fn exhausted_live_subtree_ends_publication_without_replacing_the_catalog() {
    assert_exhausted_publication_ends("catalog_terminal_media_evidence_path_mismatch", false);
}

#[test]
fn exhausted_live_subtree_with_full_recovery_lane_ends_foreground_wait() {
    assert_exhausted_publication_ends("change_lease_expired", true);
}

fn assert_exhausted_publication_ends(failure_code: &str, full_recovery_lane: bool) {
    let fixture = Fixture::new();
    let request = fixture.replacement();
    let reader = read_catalog(&fixture.paths.catalog_path);
    let (root_id, generation): (String, i64) = reader
        .query_row(
            "SELECT root_id, generation FROM library_change_root_state WHERE is_active = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    drop(reader);
    let generation = LibraryRootGeneration::new(u64::try_from(generation).unwrap()).unwrap();
    let mut peer = SqliteCatalog::open(fixture.paths.catalog_path.clone()).unwrap();
    let mut seeded = false;
    let mut completed = false;
    let started = Instant::now();
    let error = run_scan_with_storage(
        request.clone(),
        |event| {
            if started.elapsed() > Duration::from_secs(3) {
                cancel_scan(&request.scan_id);
            }
            completed |= matches!(event, ScanEvent::Completed { .. });
            if !seeded
                && matches!(
                    event,
                    ScanEvent::Finalizing {
                        validated_items: 2,
                        total_items: 2,
                        ..
                    }
                )
            {
                seeded = true;
                let now = super::super::super::current_unix_ms().unwrap();
                let policy = LibraryChangeQueuePolicy {
                    debounce_millis: 0,
                    retry_initial_delay_millis: 1,
                    retry_maximum_delay_millis: 1,
                    ..LibraryChangeQueuePolicy::default()
                };
                peer.enqueue_library_change_intents(
                    &[LibraryChangeIntent {
                        root_id: root_id.clone(),
                        root_generation: generation,
                        kind: LibraryChangeIntentKind::Reconcile,
                        scope: LibraryChangeScope::Subtree,
                        relative_path: "album".into(),
                        previous_relative_path: None,
                        origin: LibraryChangeOrigin::LiveNotification,
                        first_observed_unix_ms: now,
                        most_recent_observed_unix_ms: now,
                        first_sequence: 1,
                        most_recent_sequence: 1,
                        coalesced_observation_count: 1,
                    }],
                    now,
                    policy,
                )
                .unwrap();
                for attempt in 0..policy.max_attempts {
                    let clock = now + i64::from(attempt) * 2;
                    let leased = peer
                        .lease_library_changes(&root_id, generation, clock, policy)
                        .unwrap();
                    assert_eq!(leased.len(), 1);
                    assert_eq!(
                        peer.retry_library_change(
                            leased[0].change.id,
                            leased[0].lease_generation,
                            &LibraryChangeFailure {
                                code: failure_code.into(),
                                message: "fixture scope failure".into()
                            },
                            clock,
                            policy,
                        )
                        .unwrap(),
                        LibraryChangeLeaseUpdateOutcome::Applied
                    );
                }
                if full_recovery_lane {
                    fill_recovery_lane(&fixture.paths.catalog_path, &root_id, now);
                    assert!(
                        !peer
                            .has_ready_live_authoritative_library_change(
                                &root_id, generation, now, policy,
                            )
                            .unwrap()
                    );
                }
            }
            true
        },
        fixture.paths.clone(),
    )
    .expect_err("exhausted work must not wait for cancellation");
    assert!(seeded);
    assert!(!completed);
    assert_eq!(error.code, "catalog_scan_live_changes_exhausted");
    assert!(error.message.contains(failure_code));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(
        publication_state(&fixture.paths.catalog_path, &request.scan_id),
        (fixture.initial_scan.clone(), "failed".into(), 1)
    );
    let retained: (String, Option<i64>) = read_catalog(&fixture.paths.catalog_path).query_row(
        "SELECT status, next_retry_unix_ms FROM library_change_queue WHERE relative_path = 'album'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(retained, ("retry_wait".into(), None));
    if full_recovery_lane {
        let pending: i64 = read_catalog(&fixture.paths.catalog_path).query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE origin='metadata_inventory' AND status='pending'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(
            pending,
            i64::from(
                LibraryChangeQueuePolicy::default()
                    .lane_capacity(crate::domain::LibraryChangeLane::Recovery)
            )
        );
    }
    drop(peer);
    fixture.assert_reopen(1, &fixture.initial_scan);
    fixture.assert_source_bytes();
}

fn fill_recovery_lane(path: &Path, root_id: &str, now: i64) {
    let mut connection = Connection::open(path).unwrap();
    let transaction = connection.transaction().unwrap();
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_root_state (
           root_id, root_generation, protocol_version, contract_version,
           capability_state, continuity_state, updated_unix_ms
         ) VALUES (?1, 1, 1, 1, 'live_only', 'live_only', ?2)
         ON CONFLICT(root_id, root_generation) DO UPDATE SET
           protocol_version=1, capability_state='live_only', continuity_state='live_only'",
            params![root_id, now],
        )
        .unwrap();
    let capacity = LibraryChangeQueuePolicy::default()
        .lane_capacity(crate::domain::LibraryChangeLane::Recovery);
    transaction
        .execute(
            "WITH RECURSIVE slots(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM slots WHERE n < ?1)
         INSERT INTO library_change_queue (
           root_id, root_generation, intent_kind, scope, relative_path, origin,
           first_observed_unix_ms, most_recent_observed_unix_ms, first_sequence,
           most_recent_sequence, coalesced_observation_count, status, ready_unix_ms,
           catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
         ) SELECT ?2, 1, 'reconcile', 'path', 'p2/' || n || '.png', 'metadata_inventory',
           ?3, ?3, CAST(n AS TEXT), CAST(n AS TEXT), 1, 'pending', ?3, 0, ?3, ?3 FROM slots",
            params![capacity, root_id, now],
        )
        .unwrap();
    transaction.commit().unwrap();
}

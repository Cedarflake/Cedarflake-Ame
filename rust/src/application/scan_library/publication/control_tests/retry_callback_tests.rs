use std::time::{Duration, Instant};

use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration,
};
use crate::ports::LibraryChangeQueue;

use super::*;

#[test]
fn publication_progress_callback_cancellation_prevents_the_next_publish() {
    let fixture = Fixture::new();
    let request = fixture.replacement();
    let reader = read_catalog(&fixture.paths.catalog_path);
    let (root_id, generation, revision): (String, i64, i64) = reader
        .query_row(
            "SELECT root_id, generation, (SELECT revision FROM catalog_state)
         FROM library_change_root_state WHERE is_active = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("current root authority");
    drop(reader);
    let generation =
        LibraryRootGeneration::new(u64::try_from(generation).expect("root generation"))
            .expect("nonzero generation");
    let revision = u64::try_from(revision).expect("catalog revision");
    let mut peer = SqliteCatalog::open(fixture.paths.catalog_path.clone()).expect("live peer");
    let mut leased: Option<LeasedLibraryChange> = None;
    let mut accepted = false;
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    let projections = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&projections);
    let _hook = set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
        observed.store(true, Ordering::Release);
    });
    run_scan_with_storage(
        request.clone(),
        |event| {
            assert!(
                Instant::now() < deadline,
                "publication did not emit bounded progress"
            );
            if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 2,
                    total_items: 2,
                    ..
                }
            ) {
                let now = super::super::super::current_unix_ms().expect("fixture clock");
                if let Some(lease) = leased.take() {
                    // The competing owner completes while the progress callback accepts cancellation.
                    // The following publication attempt would now be allowed without the control check.
                    assert_eq!(
                        peer.complete_library_change(
                            lease.change.id,
                            lease.lease_generation,
                            revision,
                            now
                        )
                        .expect("peer completes its no-op live observation"),
                        LibraryChangeLeaseUpdateOutcome::Applied
                    );
                    accepted = cancel_scan(&request.scan_id);
                    assert!(accepted);
                } else if !accepted {
                    let policy = LibraryChangeQueuePolicy {
                        debounce_millis: 0,
                        lease_duration_millis: 60_000,
                        ..LibraryChangeQueuePolicy::default()
                    };
                    peer.enqueue_library_change_intents(
                        &[LibraryChangeIntent {
                            root_id: root_id.clone(),
                            root_generation: generation,
                            kind: LibraryChangeIntentKind::Reconcile,
                            scope: LibraryChangeScope::Path,
                            relative_path: "baseline.png".to_owned(),
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
                    .expect("real live change admission");
                    let mut leases = peer
                        .lease_library_changes(&root_id, generation, now, policy)
                        .expect("real independent live lease");
                    assert_eq!(leases.len(), 1);
                    leased = leases.pop();
                }
            }
            events.push(event);
            true
        },
        fixture.paths.clone(),
    )
    .expect("callback cancellation settles");
    assert!(
        accepted,
        "the publication-delay callback ran, not only validation progress"
    );
    assert!(
        !projections.load(Ordering::Acquire),
        "no subsequent projection replacement was admitted"
    );
    assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. })));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ScanEvent::Completed { .. }))
    );
    assert_eq!(
        publication_state(&fixture.paths.catalog_path, &request.scan_id),
        (fixture.initial_scan.clone(), "cancelled".to_owned(), 1)
    );
    drop(peer);
    fixture.assert_reopen(1, &fixture.initial_scan);
    fixture.assert_source_bytes();
}

use tempfile::TempDir;

use crate::domain::{
    LibraryChangeId, LibraryChangeLane, LibraryChangeLeaseIdentity,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryRootGeneration,
};
use crate::ports::LibraryChangeQueue;

use super::super::SqliteCatalog;
use super::super::tests::{path_intent, queue_catalog};

fn fixture(count: u32) -> (TempDir, SqliteCatalog, Vec<LibraryChangeLeaseIdentity>) {
    let directory = tempfile::tempdir().expect("test catalog directory");
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 64,
        ..LibraryChangeQueuePolicy::default()
    };
    let intents = (0..count)
        .map(|index| {
            let mut intent = path_intent(
                "root-a",
                generation,
                u64::from(index) + 1,
                1_000,
                &format!("lease-{index}.jpg"),
            );
            intent.origin = LibraryChangeOrigin::MetadataInventory;
            intent
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&intents, 1_000, policy)
        .expect("enqueue recovery work");
    let leased = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Recovery,
            1_000,
            policy,
        )
        .expect("lease recovery batch");
    assert_eq!(leased.len(), count as usize);
    (
        directory,
        catalog,
        leased
            .iter()
            .map(LibraryChangeLeaseIdentity::from)
            .collect(),
    )
}

#[test]
fn lease_deferral_commits_once_and_preserves_attempt_budget_and_lane() {
    let (_directory, mut catalog, leases) = fixture(64);
    let epoch = catalog.completed_write_epoch();
    assert_eq!(
        catalog
            .defer_library_changes(&leases, 1_001)
            .expect("return batch"),
        vec![LibraryChangeLeaseUpdateOutcome::Applied; 64]
    );
    assert_eq!(catalog.completed_write_epoch() - epoch, 1);
    let shape: (i64, i64, i64, i64) = catalog.connection.query_row(
        "SELECT COUNT(*), SUM(queue.status = 'pending' AND queue.ready_unix_ms = 1001),
                SUM(queue.attempt_count = 0 AND queue.lease_expires_unix_ms IS NULL),
                SUM(lanes.lane = 'p2_recovery')
         FROM library_change_queue queue JOIN library_change_queue_lanes lanes ON lanes.change_id = queue.id",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).expect("queue shape");
    assert_eq!(shape, (64, 64, 64, 64));
}

#[test]
fn lease_deferral_failure_rolls_back_the_entire_batch() {
    let (_directory, mut catalog, leases) = fixture(3);
    catalog.connection.execute_batch(&format!(
        "CREATE TEMP TRIGGER fail_last_lease_deferral BEFORE UPDATE OF status ON library_change_queue
         WHEN NEW.id = {} AND NEW.status = 'pending'
         BEGIN SELECT RAISE(ABORT, 'controlled lease return failure'); END;", leases[2].change_id.value(),
    )).expect("install deterministic late failure");
    catalog
        .defer_library_changes(&leases, 1_001)
        .expect_err("the final lease rejects the complete return");
    let leased: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM library_change_queue WHERE status = 'leased' AND attempt_count = 1 AND lease_expires_unix_ms IS NOT NULL",
        [], |row| row.get(0),
    ).expect("original leases remain intact");
    assert_eq!(leased, 3);
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_last_lease_deferral")
        .expect("remove controlled failure");
    assert_eq!(
        catalog
            .defer_library_changes(&leases, 1_002)
            .expect("retry atomic return"),
        vec![LibraryChangeLeaseUpdateOutcome::Applied; 3]
    );
}

#[test]
fn lease_deferral_does_not_modify_newer_or_terminal_owners() {
    let (_directory, mut catalog, mut leases) = fixture(5);
    leases[1].lease_generation += 1;
    catalog
        .complete_library_change(leases[2].change_id, leases[2].lease_generation, 0, 1_001)
        .expect("complete separate lease");
    catalog.connection.execute("UPDATE library_change_queue SET status = 'superseded', lease_expires_unix_ms = NULL WHERE id = ?1", [i64::try_from(leases[3].change_id.value()).expect("lease ID")]).expect("retire separate lease");
    let mut newer = path_intent(
        "root-a",
        LibraryRootGeneration::initial(),
        99,
        1_001,
        "lease-4.jpg",
    );
    newer.origin = LibraryChangeOrigin::MetadataInventory;
    catalog
        .enqueue_library_change_intents(&[newer], 1_001, LibraryChangeQueuePolicy::default())
        .expect("newer observation replaces the old lease");
    let newer_state = |catalog: &SqliteCatalog| {
        catalog.connection.query_row(
            "SELECT id, status, lease_generation, attempt_count, ready_unix_ms, most_recent_sequence
             FROM library_change_queue WHERE relative_path = 'lease-4.jpg' AND status = 'pending'",
            [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, String>(5)?)),
        ).expect("newer pending owner")
    };
    let before = newer_state(&catalog);
    leases.push(LibraryChangeLeaseIdentity {
        change_id: LibraryChangeId::new(999_999).expect("missing identity"),
        lease_generation: 1,
    });
    let outcomes = catalog
        .defer_library_changes(&leases, 1_002)
        .expect("compare every supplied lease");
    assert_eq!(
        outcomes,
        vec![
            LibraryChangeLeaseUpdateOutcome::Applied,
            LibraryChangeLeaseUpdateOutcome::LeaseMismatch,
            LibraryChangeLeaseUpdateOutcome::LeaseMismatch,
            LibraryChangeLeaseUpdateOutcome::Superseded,
            LibraryChangeLeaseUpdateOutcome::Superseded,
            LibraryChangeLeaseUpdateOutcome::Missing,
        ]
    );
    assert_eq!(newer_state(&catalog), before);
    let actual: (i64, i64, i64) = catalog.connection.query_row(
        "SELECT SUM(status = 'completed'), SUM(status = 'superseded'), SUM(status = 'leased') FROM library_change_queue",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).expect("unchanged non-owned states");
    assert_eq!(actual, (1, 2, 1));
}

#[test]
fn lease_deferral_rejects_invalid_batches_before_writing() {
    let (_directory, mut catalog, leases) = fixture(2);
    let epoch = catalog.completed_write_epoch();
    assert!(
        catalog
            .defer_library_changes(&[], 1_001)
            .expect("empty return")
            .is_empty()
    );
    for invalid in [
        vec![leases[0]; 2],
        missing_identities(129),
        vec![LibraryChangeLeaseIdentity {
            lease_generation: 0,
            ..leases[0]
        }],
    ] {
        let error = catalog
            .defer_library_changes(&invalid, 1_001)
            .expect_err("invalid batch");
        assert_eq!(error.code, "change_queue_deferral_batch_invalid");
    }
    assert_eq!(catalog.completed_write_epoch(), epoch);
    assert_eq!(
        catalog
            .defer_library_changes(&missing_identities(128), 1_001)
            .expect("maximum unique batch"),
        vec![LibraryChangeLeaseUpdateOutcome::Missing; 128]
    );
}

fn missing_identities(count: u64) -> Vec<LibraryChangeLeaseIdentity> {
    (0..count)
        .map(|index| LibraryChangeLeaseIdentity {
            change_id: LibraryChangeId::new(1_000_000 + index).expect("unique missing identity"),
            lease_generation: 1,
        })
        .collect()
}

#[test]
fn lease_deferral_preserves_released_and_replaced_root_generations() {
    let (_directory, mut catalog, old_leases) = fixture(1);
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    assert!(
        catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Recovery,
                31_001,
                policy
            )
            .expect("expire old lease")
            .is_empty()
    );
    let renewed = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            generation,
            LibraryChangeLane::Recovery,
            32_001,
            policy,
        )
        .expect("re-lease after durable retry delay");
    assert_eq!(renewed.len(), 1);
    assert!(renewed[0].lease_generation > old_leases[0].lease_generation);
    assert_eq!(
        catalog
            .defer_library_changes(&old_leases, 32_002)
            .expect("reject stale worker"),
        vec![LibraryChangeLeaseUpdateOutcome::LeaseMismatch]
    );
    let still_leased: (i64, i64) = catalog.connection.query_row(
        "SELECT lease_generation, attempt_count FROM library_change_queue WHERE status = 'leased'", [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("current lease retained");
    assert_eq!(
        still_leased,
        (
            i64::try_from(renewed[0].lease_generation).expect("generation"),
            2
        )
    );
    let transaction = catalog
        .connection
        .transaction()
        .expect("root retirement transaction");
    super::super::super::retire_root_change_queue(&transaction, "root-a", 33_000)
        .expect("retire old root generation");
    let next_generation =
        super::super::super::activate_root_change_queue(&transaction, "root-a", 33_001)
            .expect("new root incarnation");
    transaction.commit().expect("replace generation");
    assert_ne!(next_generation, generation);
    let mut next = path_intent("root-a", next_generation, 100, 33_002, "lease-0.jpg");
    next.origin = LibraryChangeOrigin::MetadataInventory;
    catalog
        .enqueue_library_change_intents(&[next], 33_002, policy)
        .expect("enqueue replacement work");
    let next_lease = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            next_generation,
            LibraryChangeLane::Recovery,
            33_002,
            policy,
        )
        .expect("lease replacement work");
    assert_eq!(next_lease.len(), 1);
    assert_eq!(
        catalog
            .defer_library_changes(&old_leases, 33_003)
            .expect("old root cannot be resurrected"),
        vec![LibraryChangeLeaseUpdateOutcome::Superseded]
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics("root-a", next_generation, 33_003, policy)
        .expect("replacement root metrics");
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.pending_count, 0);
}

#[test]
fn lease_deferral_does_not_treat_a_missing_lane_as_a_missing_change() {
    let (_directory, mut catalog, leases) = fixture(2);
    catalog
        .connection
        .execute(
            "DELETE FROM library_change_queue_lanes WHERE change_id = ?1",
            [i64::try_from(leases[1].change_id.value()).expect("lease ID")],
        )
        .expect("corrupt one lane");
    let epoch = catalog.completed_write_epoch();
    catalog
        .defer_library_changes(&leases, 1_001)
        .expect_err("existing changes require lane authority");
    assert_eq!(catalog.completed_write_epoch(), epoch);
}

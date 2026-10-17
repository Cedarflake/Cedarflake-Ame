use std::sync::{Arc, mpsc};

use super::*;

#[test]
fn recovery_dispatch_does_not_wait_for_a_catalog_publication() {
    assert_recovery_defers_while_writer_is_held(true);
}

#[test]
fn recovery_dispatch_does_not_wait_for_an_external_sqlite_writer() {
    assert_recovery_defers_while_writer_is_held(false);
}

#[test]
fn recovery_dispatch_does_not_wait_for_an_exclusive_sqlite_lock() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut publisher, change_id) = ready_recovery_catalog(path.clone());
    let before = queue_state(&publisher.connection, change_id);
    let timeout = busy_timeout(&publisher.connection);
    // A fresh connection has not joined the WAL, so another connection can hold its read boundary.
    publisher.connection = Connection::open(&path).expect("fresh publisher connection");
    crate::adapters::sqlite_catalog::configure_catalog_connection(&publisher.connection)
        .expect("publisher configuration");
    let blocker = Connection::open(&path).expect("exclusive blocker");
    blocker
        .execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN IMMEDIATE;")
        .expect("hold the WAL read boundary");
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let result = publisher.lease_metadata_inventory_recovery(
                "root-a",
                LibraryRootGeneration::initial(),
                1_001,
                immediate_policy(),
            );
            sender.send(result).expect("return dispatch result");
            publisher
        });
        let result_while_locked = receiver.recv_timeout(Duration::from_secs(1));
        let state_while_locked = queue_state(&blocker, change_id);
        drop(blocker);
        let mut publisher = worker.join().expect("dispatch worker");
        assert!(
            result_while_locked
                .expect("status dispatch must return while the read boundary remains held")
                .expect("busy recovery selection is deferred")
                .is_none()
        );
        assert_eq!(state_while_locked, before);
        assert_eq!(queue_state(&publisher.connection, change_id), before);
        assert_eq!(busy_timeout(&publisher.connection), timeout);
        assert!(publisher.connection.is_autocommit());
        let leased = publisher
            .lease_metadata_inventory_recovery(
                "root-a",
                LibraryRootGeneration::initial(),
                1_002,
                immediate_policy(),
            )
            .expect("next dispatch after reader unlock")
            .expect("retained recovery task");
        assert_eq!(leased.change.id, change_id);
        assert_eq!(i64::from(leased.change.attempt_count), before.1 + 1);
    });
}

#[test]
fn recovery_dispatch_keeps_its_turn_ahead_of_a_repeating_writer() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, change_id) = ready_recovery_catalog(directory.path().join("catalog.sqlite3"));
    let admission = Arc::clone(&catalog.write_admission);
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (registered_tx, registered_rx) = mpsc::sync_channel(1);
    SqliteCatalog::after_next_write_registration_for_test(move || {
        drop(owner);
        let barged = admission.try_acquire(LibraryChangeLane::Recovery).is_some();
        registered_tx.send(barged).expect("report competing writer");
    });
    let leased = catalog
        .lease_metadata_inventory_recovery(
            "root-a",
            LibraryRootGeneration::initial(),
            1_001,
            immediate_policy(),
        )
        .expect("dispatch after the current writer finishes")
        .expect("recovery retains the next queue position");
    assert_eq!(leased.change.id, change_id);
    assert!(
        !registered_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        "a repeating writer overtook the waiting recovery dispatch"
    );
}

fn assert_recovery_defers_while_writer_is_held(coordinated: bool) {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut publisher, change_id) = ready_recovery_catalog(path.clone());
    let mut blocker = SqliteCatalog::open(path).expect("blocking connection");
    let before = queue_state(&publisher.connection, change_id);
    let timeout = busy_timeout(&publisher.connection);
    let permit = coordinated.then(|| blocker.write_admission.acquire(LibraryChangeLane::Recovery));
    let transaction = blocker
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("hold unrelated writer");
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let result = publisher.lease_metadata_inventory_recovery(
                "root-a",
                LibraryRootGeneration::initial(),
                1_001,
                immediate_policy(),
            );
            sender.send(result).expect("return dispatch result");
            publisher
        });
        let result_while_locked = receiver.recv_timeout(Duration::from_secs(1));
        let state_while_locked = queue_state(&transaction, change_id);
        // Release the writer before joining or asserting, including when the old blocking path runs.
        drop(transaction);
        drop(permit);
        let mut publisher = worker.join().expect("dispatch worker");
        assert!(
            result_while_locked
                .expect("status dispatch must return while the writer remains held")
                .expect("busy recovery admission is deferred")
                .is_none()
        );
        assert_eq!(state_while_locked, before);
        assert_eq!(queue_state(&publisher.connection, change_id), before);
        assert_eq!(busy_timeout(&publisher.connection), timeout);
        assert!(publisher.connection.is_autocommit());
        let leased = publisher
            .lease_metadata_inventory_recovery(
                "root-a",
                LibraryRootGeneration::initial(),
                1_002,
                immediate_policy(),
            )
            .expect("next dispatch after writer retirement")
            .expect("retained recovery task");
        assert_eq!(leased.change.id, change_id);
        assert_eq!(i64::from(leased.change.attempt_count), before.1 + 1);
    });
}

#[test]
fn failed_recovery_lease_rolls_back_and_restores_connection_configuration() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, change_id) = ready_recovery_catalog(directory.path().join("catalog.sqlite3"));
    let before = queue_state(&catalog.connection, change_id);
    let timeout = busy_timeout(&catalog.connection);
    let error = super::super::lease_write::lease_in_transaction(
        &mut catalog,
        LeaseSelection::RecoveryAuthoritative,
        |_| Ok(Some(LibraryChangeLane::Recovery)),
        |transaction| {
            transaction.execute(
                "UPDATE library_change_queue SET attempt_count = attempt_count + 1 WHERE id = ?1",
                [sqlite_integer(change_id.value(), "change ID")?],
            ).map_err(database_error)?;
            Err(ScanError::new(
                "fixture_lease_failure",
                "Rollback the partial lease",
            ))
        },
    )
    .expect_err("preserve a non-contention failure");
    assert_eq!(error.code, "fixture_lease_failure");
    assert_eq!(queue_state(&catalog.connection, change_id), before);
    assert_eq!(busy_timeout(&catalog.connection), timeout);
    assert!(catalog.connection.is_autocommit());
    assert!(
        catalog
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
            .is_some()
    );
}

#[test]
fn failed_recovery_selection_restores_connection_configuration() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, change_id) = ready_recovery_catalog(directory.path().join("catalog.sqlite3"));
    let before = queue_state(&catalog.connection, change_id);
    let timeout = busy_timeout(&catalog.connection);
    let error = super::super::lease_write::lease_in_transaction(
        &mut catalog,
        LeaseSelection::RecoveryAuthoritative,
        |connection| {
            assert_eq!(busy_timeout(connection), 0);
            Err(ScanError::new(
                "fixture_selection_failure",
                "Recovery selection failed before writer admission",
            ))
        },
        |_| panic!("failed selection cannot start a write transaction"),
    )
    .expect_err("preserve a non-contention selection failure");
    assert_eq!(error.code, "fixture_selection_failure");
    assert_eq!(queue_state(&catalog.connection, change_id), before);
    assert_eq!(busy_timeout(&catalog.connection), timeout);
    assert!(catalog.connection.is_autocommit());
    assert!(
        catalog
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
            .is_some()
    );
}

fn ready_recovery_catalog(path: PathBuf) -> (SqliteCatalog, LibraryChangeId) {
    let mut catalog = queue_catalog(path);
    let request = scan_request("recovery-admission");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin baseline");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove baseline");
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect("publish baseline");
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
        .expect("queue recovery");
    let leased = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, immediate_policy())
        .expect("lease fixture authority")
        .expect("fixture task");
    catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: leased.change.id,
            run_id: "recovery-admission-run".to_owned(),
            root_id: "root-a".to_owned(),
            root_generation: generation,
            reason: LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
            opening_boundary: None,
            authorized_unix_ms: 1_000,
            retired_unix_ms: None,
        })
        .expect("authorize durable recovery");
    catalog
        .defer_library_change(leased.change.id, leased.lease_generation, 1_000)
        .expect("leave recovery ready for dispatch");
    assert!(
        catalog
            .has_ready_metadata_inventory_recovery("root-a", generation, 1_001, immediate_policy())
            .expect("recovery is eligible")
    );
    (catalog, leased.change.id)
}

fn queue_state(connection: &Connection, change_id: LibraryChangeId) -> (String, i64, i64) {
    connection
        .query_row(
            "SELECT status,attempt_count,lease_generation FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(change_id.value(), "change ID").expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("durable task state")
}

fn busy_timeout(connection: &Connection) -> i64 {
    connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("connection timeout")
}

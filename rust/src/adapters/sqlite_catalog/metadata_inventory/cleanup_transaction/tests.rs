use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::domain::LibraryChangeLane;
use crate::ports::CatalogRepository;

use super::*;

mod cancellation;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let storage = tempfile::tempdir().expect("owned catalog storage");
    let catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    catalog
        .connection
        .execute_batch(
            "CREATE TABLE cleanup_fixture(value INTEGER); INSERT INTO cleanup_fixture VALUES (1)",
        )
        .expect("isolated transaction evidence");
    (storage, catalog)
}

fn assert_restored(catalog: &SqliteCatalog) {
    assert!(catalog.connection.is_autocommit());
    let (value, timeout): (i64, u32) = catalog
        .connection
        .query_row(
            "SELECT (SELECT value FROM cleanup_fixture), (SELECT timeout FROM pragma_busy_timeout)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("rollback and timeout evidence");
    assert_eq!(value, 1);
    assert_eq!(u128::from(timeout), SQLITE_BUSY_TIMEOUT.as_millis());
    let value: i64 = catalog
        .connection
        .query_row(
            "WITH RECURSIVE n(i) AS (
        SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000) SELECT SUM(i) FROM n",
            [],
            |row| row.get(0),
        )
        .expect("retired progress callback cannot poison later SQL");
    assert_eq!(value, 500_500);
    assert!(
        catalog
            .write_admission
            .try_acquire(LibraryChangeLane::Live)
            .is_some()
    );
}

#[test]
fn bounded_cleanup_declines_an_existing_writer_without_calling_work() {
    let (_storage, mut catalog) = fixture();
    let permit = catalog.write_admission.acquire(LibraryChangeLane::Live);
    let result = try_cleanup_write(&mut catalog, Default::default(), |_| {
        panic!("cleanup cannot enter ahead of live writer")
    });
    assert_eq!(result.expect_err("deferred").code, "catalog_database_busy");
    drop(permit);
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_uses_zero_sqlite_wait_and_restores_timeout_after_lock_conflict() {
    let (_storage, mut catalog) = fixture();
    let peer = Connection::open(catalog.catalog_path()).expect("independent uncoordinated writer");
    peer.execute_batch("BEGIN IMMEDIATE")
        .expect("external writer holds SQLite lock");
    let result = try_cleanup_write(&mut catalog, Default::default(), |_| {
        panic!("SQLite rejected transaction before callback")
    });
    assert_eq!(
        result.expect_err("deferred external writer").code,
        "catalog_database_busy"
    );
    peer.execute_batch("ROLLBACK")
        .expect("release external writer");
    try_cleanup_write(&mut catalog, Default::default(), |transaction| {
        let timeout: i64 = transaction
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .map_err(database_error)?;
        assert_eq!(
            timeout, 0,
            "maintenance never inherits the foreground busy wait"
        );
        Ok(MetadataInventoryCleanupReport::default())
    })
    .expect("uncontended retry");
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_preserves_original_failure_and_rolls_back() {
    let (_storage, mut catalog) = fixture();
    let result = try_cleanup_write(&mut catalog, Default::default(), |transaction| {
        transaction
            .execute("UPDATE cleanup_fixture SET value = 2", [])
            .map_err(database_error)?;
        Err(ScanError::new(
            "fixture_cleanup_failure",
            "original failure",
        ))
    });
    assert_eq!(result.expect_err("failure").code, "fixture_cleanup_failure");
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_clears_interruption_before_unwind_rollback() {
    let (_storage, mut catalog) = fixture();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = try_cleanup_write(&mut catalog, Default::default(), |transaction| {
            transaction
                .execute("UPDATE cleanup_fixture SET value = 2", [])
                .expect("uncommitted mutation");
            // Rollback must succeed even when the currently installed handler rejects every VM step.
            transaction
                .progress_handler(1, Some(|| true))
                .expect("inject interruption");
            panic!("fixture cleanup panic");
        });
    }));
    assert!(result.is_err());
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_yields_and_rolls_back_when_a_foreground_writer_arrives() {
    let (_storage, mut catalog) = fixture();
    let admission = Arc::clone(&catalog.write_admission);
    let (sender, receiver) = mpsc::sync_channel(1);
    let mut worker = None;
    let result = try_cleanup_write(&mut catalog, Default::default(), |transaction| {
        transaction
            .execute("UPDATE cleanup_fixture SET value = 2", [])
            .map_err(database_error)?;
        worker = Some(std::thread::spawn(move || {
            // The real bounded foreground admission invokes the maintenance preemption callback.
            let permit = admission.acquire_user_interactive_for(Duration::ZERO);
            assert!(permit.is_none());
            sender
                .send(())
                .expect("signal actual foreground registration");
        }));
        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("foreground registration finished");
        let _: i64 = transaction
            .query_row(
                "WITH RECURSIVE n(i) AS (
            SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000) SELECT SUM(i) FROM n",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        panic!("preempted SQL must not finish");
    });
    worker
        .expect("actual foreground requester")
        .join()
        .expect("requester finished");
    assert_eq!(
        result.expect_err("cleanup yielded").code,
        "catalog_database_busy"
    );
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_preemption_allows_the_waiting_live_writer_to_commit() {
    let (_storage, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch("CREATE TABLE live_fixture(value INTEGER)")
        .expect("independent live publication evidence");
    let admission = Arc::clone(&catalog.write_admission);
    let peer_path = catalog.catalog_path().to_path_buf();
    let (preempted_sender, preempted_receiver) = mpsc::sync_channel(1);
    let (completed_sender, completed_receiver) = mpsc::sync_channel(1);
    let mut worker = None;
    let result = try_cleanup_write(&mut catalog, Default::default(), |transaction| {
        transaction
            .execute("UPDATE cleanup_fixture SET value = 2", [])
            .map_err(database_error)?;
        {
            let mut state = admission
                .state
                .lock()
                .expect("active maintenance registration");
            let original = state
                .active_preempt
                .take()
                .expect("real maintenance preemption callback");
            state.active_preempt = Some(Arc::new(move || {
                original();
                let _ = preempted_sender.try_send(());
            }));
        }
        worker = Some(std::thread::spawn(move || {
            let _permit = admission.acquire(LibraryChangeLane::Live);
            let mut peer = Connection::open(peer_path).expect("live peer connection");
            peer.busy_timeout(Duration::ZERO)
                .expect("no hidden SQLite wait");
            let transaction = peer
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("cleanup rolled back before live write");
            transaction
                .execute("INSERT INTO live_fixture VALUES (1)", [])
                .expect("live write");
            transaction.commit().expect("live commit");
            completed_sender.send(()).expect("confirm live progress");
        }));
        preempted_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("real admission preempted active maintenance");
        let _: i64 = transaction
            .query_row(
                "WITH RECURSIVE n(i) AS (
            SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000) SELECT SUM(i) FROM n",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        panic!("maintenance SQL must be interrupted");
    });
    let completed = completed_receiver.recv_timeout(Duration::from_secs(5));
    worker
        .expect("spawned owned peer")
        .join()
        .expect("peer retired");
    completed.expect("waiting live writer actually committed");
    assert_eq!(
        result.expect_err("cleanup yielded").code,
        "catalog_database_busy"
    );
    assert_restored(&catalog);
    let published: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM live_fixture", [], |row| row.get(0))
        .expect("independent visible result");
    assert_eq!(published, 1);
}

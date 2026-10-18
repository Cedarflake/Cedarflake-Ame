use crate::adapters::sqlite_catalog::tests::support::{fixture_request, load_default_snapshot};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::ports::CatalogMaintenanceControl;

use crate::adapters::sqlite_catalog::*;

#[test]
fn user_interactive_timeout_releases_its_priority_waiter() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let held_maintenance = admission
        .try_acquire(LibraryChangeLane::Recovery)
        .expect("hold maintenance writer permit");
    let waiting_admission = Arc::clone(&admission);
    let started = Instant::now();
    let waiter = thread::spawn(move || {
        waiting_admission.acquire_user_interactive_for(Duration::from_millis(25))
    });

    assert!(
        waiter
            .join()
            .expect("join timed user-interactive waiter")
            .is_none()
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    drop(held_maintenance);

    assert!(
        admission.try_acquire(LibraryChangeLane::Recovery).is_some(),
        "a timed-out user-interactive waiter must not permanently block background writers",
    );
}

#[test]
fn catalog_reads_open_without_waiting_for_an_active_writer() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");

    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");
    let reader = thread::spawn(move || {
        let started = Instant::now();
        let mut catalog = SqliteCatalog::open(path)?;
        let snapshot = load_default_snapshot(&mut catalog, 1, None)?;
        Ok::<_, ScanError>((started.elapsed(), snapshot))
    });
    let (elapsed, snapshot) = reader
        .join()
        .expect("catalog reader thread")
        .expect("open and read catalog while writer is active");
    holder
        .connection
        .execute_batch("ROLLBACK")
        .expect("release catalog writer lock");
    assert!(snapshot.roots.is_empty());
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn sqlite_write_admission_serves_waiting_live_work_before_new_recovery_work() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let active_recovery = admission.acquire(LibraryChangeLane::Recovery);
    let (acquired_sender, acquired_receiver) = mpsc::channel();

    let recovery_admission = Arc::clone(&admission);
    let recovery_sender = acquired_sender.clone();
    let recovery = thread::spawn(move || {
        let _permit = recovery_admission.acquire(LibraryChangeLane::Recovery);
        recovery_sender.send(LibraryChangeLane::Recovery).unwrap();
    });
    wait_for_admission_waiter(&admission, LibraryChangeLane::Recovery);

    let live_admission = Arc::clone(&admission);
    let live = thread::spawn(move || {
        let _permit = live_admission.acquire(LibraryChangeLane::Live);
        acquired_sender.send(LibraryChangeLane::Live).unwrap();
    });
    wait_for_admission_waiter(&admission, LibraryChangeLane::Live);

    drop(active_recovery);
    assert_eq!(
        acquired_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("first admitted writer"),
        LibraryChangeLane::Live
    );
    assert_eq!(
        acquired_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("second admitted writer"),
        LibraryChangeLane::Recovery
    );
    live.join().expect("live writer");
    recovery.join().expect("recovery writer");
}

#[test]
fn sqlite_write_admission_priority_order_keeps_maintenance_last() {
    assert_eq!(SQLITE_USER_INTERACTIVE_PRIORITY, 0);
    assert_eq!(sqlite_write_priority(LibraryChangeLane::Live), 1);
    assert_eq!(sqlite_write_priority(LibraryChangeLane::Journal), 2);
    assert_eq!(sqlite_write_priority(LibraryChangeLane::Recovery), 3);
    assert_eq!(SQLITE_MAINTENANCE_PRIORITY, 4);
}

#[test]
fn sqlite_write_admission_preempts_blocking_recovery_publication_and_wakes_epoch_waiters() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let (preempt_sender, preempt_receiver) = mpsc::channel();
    let active_publication = admission.acquire_preemptible(
        LibraryChangeLane::Recovery,
        Arc::new(move || {
            preempt_sender
                .send(())
                .expect("report publication preemption");
        }),
    );
    let observed_epoch = admission.completed_write_epoch();
    let waiting_admission = Arc::clone(&admission);
    let waiter = thread::spawn(move || {
        let _permit = waiting_admission.acquire(LibraryChangeLane::Live);
    });

    preempt_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("live writer preempts recovery publication");
    drop(active_publication);
    assert_ne!(
        admission.wait_for_completed_write_after(observed_epoch, Duration::from_secs(1)),
        observed_epoch,
    );
    waiter.join().expect("live writer");
    assert_admission_is_idle(&admission);
}

#[test]
fn sqlite_write_admission_live_waiter_preempts_maintenance_outside_the_lock_before_interrupt_install()
 {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
    let (preempt_sender, preempt_receiver) = mpsc::channel();
    let callback_admission = Arc::downgrade(&admission);
    let callback_control = control.clone();
    let active_maintenance = admission
        .try_acquire_preemptible_maintenance(Arc::new(move || {
            let was_unlocked = callback_admission
                .upgrade()
                .is_some_and(|admission| admission.state.try_lock().is_ok());
            callback_control.preempt();
            preempt_sender
                .send(was_unlocked)
                .expect("report completed maintenance preemption");
        }))
        .expect("acquire preemptible maintenance writer");
    let (acquired_sender, acquired_receiver) = mpsc::channel();
    let waiting_admission = Arc::clone(&admission);
    let waiter = thread::spawn(move || {
        let _permit = waiting_admission.acquire(LibraryChangeLane::Live);
        acquired_sender.send(()).expect("report live admission");
    });

    assert!(
        preempt_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("maintenance preemption completed outside the writer lock")
    );
    assert!(control.is_interrupted());
    let interrupt_count = Arc::new(AtomicUsize::new(0));
    let interrupt_count_for_callback = Arc::clone(&interrupt_count);
    control
        .install_interrupt(Arc::new(move || {
            interrupt_count_for_callback.fetch_add(1, Ordering::AcqRel);
        }))
        .expect("install late SQLite interrupt");
    assert_eq!(interrupt_count.load(Ordering::Acquire), 1);
    assert!(
        acquired_receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "preemption requests release; it does not revoke a live permit unsafely",
    );

    control.clear_interrupt();
    drop(active_maintenance);
    acquired_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("live writer eventually acquires the permit");
    waiter.join().expect("live waiter");
    assert_admission_is_idle(&admission);
}

#[test]
fn sqlite_write_admission_journal_and_user_waiters_preempt_recovery_maintenance() {
    for waiting_lane in [Some(LibraryChangeLane::Journal), None] {
        let admission = Arc::new(SqliteWriteAdmission::new());
        let (preempt_sender, preempt_receiver) = mpsc::channel();
        let active_maintenance = admission
            .try_acquire_preemptible_maintenance(Arc::new(move || {
                preempt_sender
                    .send(())
                    .expect("report maintenance preemption");
            }))
            .expect("acquire preemptible maintenance writer");
        let (acquired_sender, acquired_receiver) = mpsc::channel();
        let waiting_admission = Arc::clone(&admission);
        let waiter = thread::spawn(move || {
            let _permit = match waiting_lane {
                Some(lane) => Some(waiting_admission.acquire(lane)),
                None => waiting_admission.acquire_user_interactive(),
            };
            acquired_sender.send(()).expect("report priority admission");
        });

        preempt_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("higher-priority waiter preempts maintenance");
        drop(active_maintenance);
        acquired_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("higher-priority waiter eventually acquires the permit");
        waiter.join().expect("priority waiter");
        assert_admission_is_idle(&admission);
    }
}

#[test]
fn sqlite_write_admission_recovery_waiter_preempts_lower_priority_maintenance() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let (preempt_sender, preempt_receiver) = mpsc::channel();
    let active_maintenance = admission
        .try_acquire_preemptible_maintenance(Arc::new(move || {
            preempt_sender
                .send(())
                .expect("report maintenance preemption");
        }))
        .expect("acquire preemptible maintenance writer");
    let (acquired_sender, acquired_receiver) = mpsc::channel();
    let waiting_admission = Arc::clone(&admission);
    let waiter = thread::spawn(move || {
        let _permit = waiting_admission.acquire(LibraryChangeLane::Recovery);
        acquired_sender.send(()).expect("report recovery admission");
    });

    preempt_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("ordinary recovery preempts lowest-priority maintenance");
    assert!(
        acquired_receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "the maintenance permit remains owned until its callback unwinds the operation",
    );
    drop(active_maintenance);
    acquired_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("recovery waiter eventually acquires the permit");
    waiter.join().expect("recovery waiter");
    assert_admission_is_idle(&admission);
}

#[test]
fn user_interactive_write_waits_for_active_live_then_precedes_queued_live_work() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let active_live = admission.acquire(LibraryChangeLane::Live);
    let (acquired_sender, acquired_receiver) = mpsc::channel();

    let queued_live_admission = Arc::clone(&admission);
    let queued_live_sender = acquired_sender.clone();
    let queued_live = thread::spawn(move || {
        let _permit = queued_live_admission.acquire(LibraryChangeLane::Live);
        queued_live_sender.send("live").unwrap();
    });
    wait_for_admission_waiter(&admission, LibraryChangeLane::Live);

    let interactive_admission = Arc::clone(&admission);
    let interactive = thread::spawn(move || {
        let _permit = interactive_admission.acquire_user_interactive();
        acquired_sender.send("interactive").unwrap();
    });
    wait_for_user_interactive_admission_waiter(&admission);

    assert!(
        acquired_receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "interactive work must not preempt the active live transaction"
    );
    drop(active_live);
    assert_eq!(
        acquired_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("first admitted writer"),
        "interactive"
    );
    assert_eq!(
        acquired_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("second admitted writer"),
        "live"
    );
    interactive.join().expect("interactive writer");
    queued_live.join().expect("queued live writer");
}

#[test]
fn low_lane_schema_validation_does_not_block_a_live_transaction() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let session = SqliteCatalogSession::validate(path.clone()).expect("initialize catalog");
    let initializer = sqlite_schema_initializer(&catalog_admission_path(&path));
    let initialization_guard = initializer.lock().expect("hold schema initializer");
    let (completed_sender, completed_receiver) = mpsc::channel();
    let validator_path = path.clone();
    let validator = thread::spawn(move || {
        let result = SqliteCatalogSession::validate(validator_path);
        completed_sender.send(result).expect("validation result");
    });
    assert!(
        completed_receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "the recovery validator must be waiting on its independent initializer"
    );

    let started = Instant::now();
    let mut live = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("open validated live connection");
    let transaction = live
        .begin_write_in_lane(LibraryChangeLane::Live)
        .expect("begin live transaction");
    transaction.commit().expect("commit live transaction");
    assert!(started.elapsed() < Duration::from_secs(1));

    drop(initialization_guard);
    completed_receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("validator completion")
        .expect("validate catalog");
    validator.join().expect("schema validator");
}

fn assert_admission_is_idle(admission: &Arc<SqliteWriteAdmission>) {
    {
        let state = admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(!state.is_active);
        assert!(state.waiting.iter().all(|queue| queue.is_empty()));
        assert!(state.active_preempt.is_none());
    }
    drop(
        admission
            .try_acquire(LibraryChangeLane::Recovery)
            .expect("idle admission accepts the next writer"),
    );
}

fn wait_for_admission_waiter(admission: &SqliteWriteAdmission, lane: LibraryChangeLane) {
    wait_for_admission_waiter_count(admission, lane, 1);
}

fn wait_for_admission_waiter_count(
    admission: &SqliteWriteAdmission,
    lane: LibraryChangeLane,
    expected: u64,
) {
    let deadline = Instant::now() + Duration::from_secs(1);
    let priority = sqlite_write_priority(lane);
    loop {
        let waiting = admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .waiting[priority]
            .len();
        if waiting >= usize::try_from(expected).expect("bounded waiter count") {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "writer did not enter admission wait"
        );
        thread::yield_now();
    }
}

fn wait_for_user_interactive_admission_waiter(admission: &SqliteWriteAdmission) {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let waiting = admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .waiting[SQLITE_USER_INTERACTIVE_PRIORITY]
            .len();
        if waiting > 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "interactive writer did not enter admission wait"
        );
        thread::yield_now();
    }
}

#[test]
fn catalog_writers_wait_for_short_writer_contention() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");
    let contender = SqliteCatalog::open(path).expect("contending catalog");
    let configured_timeout: i64 = contender
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("configured busy timeout");
    assert_eq!(configured_timeout, 5_000);
    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");
    let attempt_started = Instant::now();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
    let writer = thread::spawn(move || {
        ready_sender.send(()).expect("announce writer attempt");
        contender
            .connection
            .execute("UPDATE catalog_state SET revision = revision", [])
    });
    ready_receiver.recv().expect("writer attempt ready");
    thread::sleep(Duration::from_millis(100));
    assert!(
        !writer.is_finished(),
        "contending writer must wait for the lock"
    );
    holder
        .connection
        .execute_batch("COMMIT")
        .expect("release catalog writer lock");
    assert_eq!(writer.join().expect("contending writer thread"), Ok(1));
    assert!(attempt_started.elapsed() >= Duration::from_millis(100));
}

#[test]
fn scan_directory_claim_waits_for_writer_before_reading_snapshot() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = fixture_request("contended-scan", "C:\\Pictures");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("scan catalog");
    catalog
        .begin_scan(&request, "contended-root", &request.root_path)
        .expect("begin contended scan");
    let holder = SqliteCatalog::open(path).expect("lock holder catalog");
    holder
        .connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             UPDATE catalog_state SET revision = revision;",
        )
        .expect("hold catalog writer lock");

    let claim = thread::spawn(move || catalog.claim_next_directory("contended-scan"));
    thread::sleep(Duration::from_millis(100));
    assert!(
        !claim.is_finished(),
        "scan claim must wait instead of failing against a stale read snapshot"
    );
    holder
        .connection
        .execute_batch("COMMIT")
        .expect("release catalog writer lock");

    assert_eq!(
        claim
            .join()
            .expect("scan claim thread")
            .expect("scan claim waits for the current writer"),
        Some(String::new())
    );
}

#[test]
fn catalog_writer_timeout_has_a_specific_error_code() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");
    let contender = SqliteCatalog::open(path).expect("contending catalog");
    contender
        .connection
        .busy_timeout(Duration::from_millis(20))
        .expect("short test timeout");
    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");

    let error = contender
        .connection
        .execute("UPDATE catalog_state SET revision = revision", [])
        .expect_err("writer must time out while the lock is held");
    let error = database_error(error);

    holder
        .connection
        .execute_batch("ROLLBACK")
        .expect("release catalog writer lock");
    assert_eq!(error.code, "catalog_database_busy");
    assert!(
        error
            .message
            .starts_with("The catalog database remained busy after waiting")
    );
}

#[test]
fn progress_handler_preempts_projection_replacement_and_rolls_back_the_transaction() {
    const OLD_LOCATION_COUNT: i64 = 4_096;

    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let initial = fixture_request("preempt-initial", "C:\\Preempt");
    catalog
        .begin_scan(&initial, "preempt-root", &initial.root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove first-import handoff");
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
               VALUES(0) UNION ALL SELECT value + 1 FROM sequence WHERE value < 4095
             )
             INSERT INTO assets(id, created_unix_ms)
             SELECT printf('preempt-asset-%05d', value), 1 FROM sequence;
             WITH RECURSIVE sequence(value) AS (
               VALUES(0) UNION ALL SELECT value + 1 FROM sequence WHERE value < 4095
             )
             INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path,
               relative_path, preview_path, file_size, modified_unix_ms,
               width, height, source_generation
             )
             SELECT 'preempt-initial', printf('preempt-asset-%05d', value),
                    printf('preempt-location-%05d', value), 'preempt-root',
                    printf('C:/Preempt/%05d.png', value), printf('%05d.png', value),
                    '', 20, 30, 40, 50, 1
             FROM sequence;
             UPDATE catalog_state SET next_source_generation = 2;",
        )
        .expect("seed large initial projection");
    catalog
        .publish_scan(
            &initial.scan_id,
            "preempt-root",
            OLD_LOCATION_COUNT as u64,
            0,
        )
        .expect("publish initial projection");

    let replacement = fixture_request("preempt-replacement", &initial.root_path);
    catalog
        .begin_scan(&replacement, "preempt-root", &replacement.root_path)
        .expect("begin replacement scan");
    let admission = Arc::clone(&catalog.write_admission);
    let waiter_slot = Arc::new(Mutex::new(None));
    let waiter_slot_for_hook = Arc::clone(&waiter_slot);
    let (acquired_sender, acquired_receiver) = mpsc::channel();
    let _hook =
        set_before_scan_projection_replacement_hook(&replacement.scan_id, move |preempted| {
            let waiter = thread::spawn(move || {
                let _permit = admission.acquire(LibraryChangeLane::Live);
                acquired_sender.send(()).expect("report live admission");
            });
            *waiter_slot_for_hook.lock().expect("live waiter slot") = Some(waiter);
            let deadline = Instant::now() + Duration::from_secs(1);
            while !preempted.load(Ordering::Acquire) {
                assert!(
                    Instant::now() < deadline,
                    "live waiter did not preempt scan publication"
                );
                thread::yield_now();
            }
        });

    let error = catalog
        .publish_scan(&replacement.scan_id, "preempt-root", 0, 0)
        .expect_err("live work preempts the replacement transaction");
    assert_eq!(error.code, "catalog_scan_publication_preempted");
    acquired_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("live writer acquires after publication rollback");
    waiter_slot
        .lock()
        .expect("live waiter slot")
        .take()
        .expect("live waiter")
        .join()
        .expect("join live waiter");

    let rolled_back: (String, String, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT roots.active_scan_id,
                    (SELECT status FROM scan_runs WHERE id = ?2),
                    (SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1),
                    (SELECT revision FROM catalog_state)
             FROM library_roots AS roots WHERE roots.id = 'preempt-root'",
            params![initial.scan_id, replacement.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("rolled-back publication state");
    assert_eq!(
        rolled_back,
        (initial.scan_id, "running".to_owned(), OLD_LOCATION_COUNT, 1,)
    );

    catalog
        .publish_scan(&replacement.scan_id, "preempt-root", 0, 0)
        .expect("cleared progress handler permits a later publication");
    let replacement_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1",
            [&replacement.scan_id],
            |row| row.get(0),
        )
        .expect("replacement projection");
    assert_eq!(replacement_count, 0);
}

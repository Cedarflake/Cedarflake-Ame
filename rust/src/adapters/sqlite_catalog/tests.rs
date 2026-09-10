use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::domain::{
    GallerySortDirection, GallerySortKey, GalleryTimeBucket, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueueHealth, LibraryChangeScope,
};
use crate::ports::{
    CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository,
    LibraryChangeQueue,
};

use super::migrations::downgrade_source_revision_contract_to_v30_for_test;
use super::*;

mod query_snapshot;
mod root_unregistration;

const TEST_QUERY_ID: &str = "test-default-query";
type GalleryQueryFixture<'a> = (&'a str, &'a str, Option<&'a str>, Option<i64>, i64);

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

fn remove_v27_spool_contract_for_test(connection: &Connection) {
    downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
             DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
             DROP TABLE IF EXISTS library_live_gap_recovery_claims;
             DROP TABLE IF EXISTS library_live_gap_recovery_contract;
             DROP INDEX IF EXISTS library_scan_publication_namespace_root;
             DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
             DROP TABLE IF EXISTS library_root_publication_namespaces;
             DROP TABLE IF EXISTS library_root_publication_namespace_contract;
             DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard;
             DROP TRIGGER library_metadata_inventory_spool_binding_update_guard;
             DROP INDEX library_metadata_inventory_spool_entries_order;
             DROP TABLE library_metadata_inventory_spool_entries;
             DROP INDEX library_metadata_inventory_spool_directories_state;
             DROP TABLE library_metadata_inventory_spool_directories;
             DROP TABLE library_metadata_inventory_spools;
             DROP TABLE library_metadata_inventory_spool_contract;
             PRAGMA user_version = 26;
             UPDATE schema_info SET version = 26;",
        )
        .expect("restore v26 spool-free schema");
}

fn add_catalog_state_to_legacy_fixture(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT COUNT(*) FROM scan_runs WHERE status = 'completed';",
        )
        .expect("add canonical catalog state to legacy fixture");
}

fn load_default_snapshot(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    catalog.load_snapshot(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
        None,
        None,
    )
}

fn load_default_timeline(catalog: &mut SqliteCatalog) -> Result<GalleryTimeline, ScanError> {
    catalog.load_gallery_timeline(&GalleryQuery::default(), TEST_QUERY_ID)
}

fn load_default_layout_manifest_chunk(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    catalog.load_gallery_layout_manifest_chunk(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
    )
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
    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_observed_unlocked = Arc::new(AtomicBool::new(false));
    let callback_admission = Arc::downgrade(&admission);
    let callback_control = control.clone();
    let callback_count_for_preempt = Arc::clone(&callback_count);
    let callback_observed_unlocked_for_preempt = Arc::clone(&callback_observed_unlocked);
    let active_maintenance = admission
        .try_acquire_preemptible_maintenance(Arc::new(move || {
            let was_unlocked = callback_admission
                .upgrade()
                .is_some_and(|admission| admission.state.try_lock().is_ok());
            callback_observed_unlocked_for_preempt.store(was_unlocked, Ordering::Release);
            callback_count_for_preempt.fetch_add(1, Ordering::AcqRel);
            callback_control.preempt();
        }))
        .expect("acquire preemptible maintenance writer");
    let (acquired_sender, acquired_receiver) = mpsc::channel();
    let waiting_admission = Arc::clone(&admission);
    let waiter = thread::spawn(move || {
        let _permit = waiting_admission.acquire(LibraryChangeLane::Live);
        acquired_sender.send(()).expect("report live admission");
    });

    wait_for_atomic_count(&callback_count, 1, "maintenance preemption callback");
    assert!(callback_observed_unlocked.load(Ordering::Acquire));
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

#[test]
fn validated_session_reopens_one_hundred_times_without_full_validation() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    reset_full_schema_validation_count(&path);
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    assert_eq!(full_schema_validation_count(&path), 1);

    for _ in 0..100 {
        session
            .open_in_lane(LibraryChangeLane::Live)
            .expect("O(1) validated reopen");
    }

    assert_eq!(full_schema_validation_count(&path), 1);
}

#[test]
fn validated_session_fails_closed_after_schema_cookie_changes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    let connection = Connection::open(path).expect("tamper connection");
    connection
        .execute_batch("CREATE TABLE unexpected_runtime_table(value INTEGER)")
        .expect("change schema cookie");

    let error = match session.open_in_lane(LibraryChangeLane::Live) {
        Ok(_) => panic!("a changed schema cookie must stale the runtime session"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_validated_session_stale");
}

#[test]
fn validated_session_fails_closed_after_header_identity_changes() {
    for pragma in ["PRAGMA application_id = 0", "PRAGMA user_version = 26"] {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.sqlite3");
        let session =
            SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
        let connection = Connection::open(path).expect("tamper connection");
        connection
            .execute_batch(pragma)
            .expect("change header identity");

        let error = match session.open_in_lane(LibraryChangeLane::Live) {
            Ok(_) => panic!("a changed catalog header must stale the runtime session"),
            Err(error) => error,
        };
        assert_eq!(error.code, "catalog_validated_session_stale");
    }
}

#[test]
fn validated_session_fails_closed_after_same_path_file_replacement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let displaced_path = directory.path().join("displaced.sqlite3");
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    fs::rename(&path, &displaced_path).expect("displace validated catalog");
    SqliteCatalogSession::validate(path.clone()).expect("create replacement catalog");

    let error = match session.open_in_lane(LibraryChangeLane::Live) {
        Ok(_) => panic!("a same-path file replacement must stale the runtime session"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_validated_session_stale");
}

#[test]
fn validated_session_ignores_normal_data_version_changes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    reset_full_schema_validation_count(&path);
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    let connection = Connection::open(path.clone()).expect("data writer");
    connection
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .expect("change ordinary catalog data");
    drop(connection);

    session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("ordinary data changes keep the validated session current");
    assert_eq!(full_schema_validation_count(&path), 1);
}

#[test]
fn large_v26_catalog_validates_once_then_reopens_in_constant_time() {
    const ROW_COUNT: i64 = 1_024;

    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('runtime-load-root', 'C:/runtime-load-root', 1);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('runtime-load-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'runtime-load-root', 1, 0, 1, 'unknown', 'baseline_required', 1
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, started_unix_ms, updated_unix_ms,
               last_issue_code, last_issue_message
             ) VALUES (
               'runtime-load-run', 'runtime-load-root', 1, 1, 'root', '',
               'failed', 1025, 1024, 1, 1, 'fixture_terminal', 'fixture terminal run'
             );
             WITH RECURSIVE sequence(value) AS (
               VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 1024
             )
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               catalog_revision_at_success, catch_up_source, catch_up_watermark,
               created_unix_ms, updated_unix_ms
             )
             SELECT
               10000 + value, 'runtime-load-root', 1, 'reconcile', 'path',
               printf('entry-%04d.jpg', value), 'startup_catch_up', 1, 1,
               printf('%d', value), printf('%d', value), 1,
               'completed', 1, 0, 0, 'runtime_load_v1',
               printf('watermark-%04d', value), 1, 1
             FROM sequence;
             INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT id, catch_up_source, catch_up_watermark, 1
             FROM library_change_queue WHERE root_id = 'runtime-load-root';
             INSERT INTO library_metadata_inventory_candidate_owners(
               run_id, candidate_key, change_id, candidate_role,
               relative_path, owned_unix_ms
             )
             SELECT 'runtime-load-run', printf('candidate-%04d', id - 10000), id,
                    'present', relative_path, 1
             FROM library_change_queue WHERE root_id = 'runtime-load-root';
             WITH RECURSIVE sequence(value) AS (
               VALUES(0) UNION ALL SELECT value + 1 FROM sequence WHERE value < 1023
             )
             INSERT INTO library_metadata_inventory_frontier(
               run_id, ordinal, relative_directory, state,
               enumerated_entry_count, updated_unix_ms
             )
             SELECT 'runtime-load-run', value, printf('directory-%04d', value),
                    'completed', 1, 1
             FROM sequence;",
        )
        .expect("seed large current catalog");
    let counts: (i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_queue
                WHERE root_id = 'runtime-load-root'),
               (SELECT COUNT(*) FROM library_change_queue_catch_up_lineage),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("large fixture counts");
    assert_eq!(counts, (ROW_COUNT, ROW_COUNT, ROW_COUNT, ROW_COUNT));
    remove_v27_spool_contract_for_test(&catalog.connection);
    drop(catalog);

    reset_full_schema_validation_count(&path);
    let validation_started = Instant::now();
    let session = SqliteCatalogSession::validate(path.clone()).expect("migrate and validate v26");
    let validation_elapsed = validation_started.elapsed();
    assert_eq!(full_schema_validation_count(&path), 1);

    let reopen_started = Instant::now();
    let mut maximum_reopen_elapsed = Duration::ZERO;
    for _ in 0..100 {
        let single_reopen_started = Instant::now();
        session
            .open_in_lane(LibraryChangeLane::Live)
            .expect("constant-time validated reopen");
        maximum_reopen_elapsed = maximum_reopen_elapsed.max(single_reopen_started.elapsed());
    }
    let reopen_elapsed = reopen_started.elapsed();
    assert_eq!(full_schema_validation_count(&path), 1);
    assert!(
        validation_elapsed < Duration::from_secs(10),
        "large v26 migration and validation took {validation_elapsed:?}"
    );
    assert!(
        maximum_reopen_elapsed < Duration::from_secs(1),
        "one validated reopen took {maximum_reopen_elapsed:?}"
    );
    eprintln!(
        "large-v26-validation rows={ROW_COUNT} validation={validation_elapsed:?} reopens100={reopen_elapsed:?} max_reopen={maximum_reopen_elapsed:?} full_validations={}",
        full_schema_validation_count(&path)
    );
}

#[test]
fn migrates_v25_through_v27_recovery_execution_and_spool_contracts() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    remove_v27_spool_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TRIGGER library_metadata_inventory_candidate_owner_update_guard;
             DROP TRIGGER library_metadata_inventory_candidate_owner_insert_guard;
             DROP INDEX library_metadata_inventory_candidate_owners_change;
             DROP INDEX library_metadata_inventory_frontier_state;
             DROP TABLE library_metadata_inventory_candidate_owners;
             DROP TABLE library_metadata_inventory_frontier;
             DROP TABLE library_recovery_execution_contract;
             UPDATE schema_info SET version = 25;",
        )
        .expect("restore v25 recovery execution shape");
    drop(catalog);

    let migrated = SqliteCatalog::open(path.clone()).expect("migrate v25 catalog");
    let evidence: (i64, i64, i64, i64) = migrated
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT complete FROM library_recovery_execution_contract),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("current recovery execution evidence");
    assert_eq!(evidence, (SCHEMA_VERSION, 1, 0, 0));
    drop(migrated);
    SqliteCatalog::open(path).expect("reopen migrated current catalog");
}

#[test]
fn migrates_v26_through_the_current_spool_contract_and_reopens() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    remove_v27_spool_contract_for_test(&catalog.connection);
    drop(catalog);

    let migrated = SqliteCatalog::open(path.clone()).expect("migrate v26 catalog");
    let evidence: (i64, i64, i64, i64, i64) = migrated
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT user_version FROM pragma_user_version),
               (SELECT contract_version
                FROM library_metadata_inventory_spool_contract WHERE singleton = 1),
               (SELECT complete
                FROM library_metadata_inventory_spool_contract WHERE singleton = 1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("current spool contract evidence");
    assert_eq!(evidence, (SCHEMA_VERSION, SCHEMA_VERSION, 4, 1, 0));
    drop(migrated);
    SqliteCatalog::open(path).expect("reopen migrated current catalog");
}

#[test]
fn current_v27_rejects_missing_spool_order_index_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX library_metadata_inventory_spool_entries_order")
        .expect("corrupt spool order contract");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("corrupt v27 spool contract must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable"
    );
}

#[test]
fn current_v28_rejects_orphaned_spool_binding_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('bound-root', 'C:/bound-root', 1);
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('bound-scan', 'bound-root', 'completed', 1, 1, 128);
             UPDATE library_roots SET active_scan_id = 'bound-scan' WHERE id = 'bound-root';
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('bound-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'bound-root', 1, 5, 1, 'supported', 'recovery_required', 1
             );
             INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state, updated_unix_ms
             ) VALUES (
               'bound-root', 1, 'bound-volume', '1',
               3, X'01010101010101010101010101010101', '44',
               '20', '20', 0, 5, 1, 'recovery_required', 1
             );
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               999, 'bound-root', 1, 'freshness_unknown', 'root', '',
               'consistency_audit', 1, 1, '1', '1', 1,
               'pending', 1, 0, 1, 1
             );
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (999, 'bound-run', 'bound-root', 1, 'containment_failure', 1);
             INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms
             ) VALUES (
               999, 'bound-root', 1, 'bound-volume', '1',
               3, X'01010101010101010101010101010101', '44',
               '20', 5, 1, 'inventory', 1, 1
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, started_unix_ms, updated_unix_ms
             ) VALUES (
               'bound-run', 'bound-root', 1, 1, 'root', '',
               'running', 1, 1, 1
             );",
        )
        .expect("insert valid recovery binding fixture");
    drop(catalog);
    let catalog = SqliteCatalog::open(path.clone()).expect("reopen valid recovery binding fixture");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value,
               scope_kind, scope_relative_path, state,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               'bound-run', 999, 'mismatched-root', 1,
               'windows-file-id-128-v1', 'test-volume:test-root',
               'root', '', 'enumerating', 1, 1
             );",
        )
        .expect("insert mismatched spool binding with valid foreign keys");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("orphaned v28 spool binding must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable"
    );
}

#[test]
fn current_v27_rejects_missing_candidate_ownership_index_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX library_metadata_inventory_candidate_owners_change")
        .expect("corrupt candidate ownership contract");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("corrupt v27 catalog must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}

fn wait_for_atomic_count(value: &AtomicUsize, expected: usize, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while value.load(Ordering::Acquire) < expected {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        thread::yield_now();
    }
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
fn migrates_v20_terminal_media_evidence_without_rebuilding_the_catalog() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "v20-evidence-scan",
        "v20-evidence-root",
        "C:\\V20EvidenceSource",
        "v20-evidence-location",
    );
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 20;",
        )
        .expect("restore v20 schema");
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("migrate v20 catalog");
    let version: i64 = reopened
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let marker: i64 = reopened
        .connection
        .query_row(
            "SELECT complete FROM library_terminal_media_evidence_contract
             WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .expect("terminal evidence contract marker");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(marker, 1);
    assert!(
        reopened
            .load_active_location("v20-evidence-location")
            .expect("preserved active location")
            .is_some()
    );
}

#[test]
fn preview_artifact_index_rolls_back_when_active_location_is_stale() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-index-scan",
        "preview-index-root",
        "C:\\PreviewIndexSource",
        "preview-index-location",
    );
    let mut location = catalog
        .load_active_location("preview-index-location")
        .expect("active location query")
        .expect("active location");
    location.modified_unix_ms += 1;
    location.preview_path = "C:\\AmeCache\\preview.jpg".to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "preview-artifact-key".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: location.width,
        height: location.height,
    };

    let error = catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect_err("stale preview publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
}

#[test]
fn preview_publication_adopts_a_null_revision_once_under_the_exact_lease() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-adoption-scan",
        "preview-adoption-root",
        "C:\\PreviewAdoptionSource",
        "preview-adoption-location",
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_revision_token = NULL
             WHERE location_id = 'preview-adoption-location'",
            [],
        )
        .expect("clear legacy revision");
    let mut location = catalog
        .load_active_location("preview-adoption-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    location.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000002".to_owned(),
    });

    catalog
        .update_active_preview(&location, None, Some(&request))
        .expect("adopt source revision");

    let adopted: Option<String> = catalog
        .connection
        .query_row(
            "SELECT source_revision_token FROM asset_locations
             WHERE location_id = 'preview-adoption-location'",
            [],
            |row| row.get(0),
        )
        .expect("adopted revision");
    assert_eq!(
        adopted.as_deref(),
        Some("windows-file-change-time-100ns-v1:0000000000000002")
    );
}

#[test]
fn hardlink_preview_revision_adoption_is_idempotent_for_matching_observations() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "matching-adoption-scan",
        "matching-adoption-root",
        "C:\\MatchingAdoptionSource",
        "matching-adoption-a",
    );
    insert_active_identity_alias(&mut catalog, "matching-adoption-a", "matching-adoption-b");
    let mut first = catalog
        .load_active_location("matching-adoption-a")
        .expect("first location query")
        .expect("first location");
    let mut second = catalog
        .load_active_location("matching-adoption-b")
        .expect("second location query")
        .expect("second location");
    let first_request = exact_preview_request(&first);
    let second_request = exact_preview_request(&second);
    let revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000011".to_owned(),
    };
    first.source_revision = Some(revision.clone());
    second.source_revision = Some(revision);

    catalog
        .update_active_preview(&first, None, Some(&first_request))
        .expect("first hardlink revision adoption");
    catalog
        .update_active_preview(&second, None, Some(&second_request))
        .expect("matching stale-null adoption is idempotent");

    let state = catalog
        .connection
        .query_row(
            "SELECT COUNT(source_revision_token), COUNT(DISTINCT source_revision_token)
             FROM asset_locations
             WHERE file_identity_scheme = 'windows-file-id-128-v1'
               AND file_identity_value = '0000000000000001:00000000000000000000000000000001'",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .expect("matching hardlink revision state");
    assert_eq!(state, (2, 1));
}

#[test]
fn hardlink_preview_revision_adoption_rejects_a_conflicting_observation() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "conflicting-adoption-scan",
        "conflicting-adoption-root",
        "C:\\ConflictingAdoptionSource",
        "conflicting-adoption-a",
    );
    insert_active_identity_alias(
        &mut catalog,
        "conflicting-adoption-a",
        "conflicting-adoption-b",
    );
    let mut first = catalog
        .load_active_location("conflicting-adoption-a")
        .expect("first location query")
        .expect("first location");
    let mut second = catalog
        .load_active_location("conflicting-adoption-b")
        .expect("second location query")
        .expect("second location");
    let first_request = exact_preview_request(&first);
    let second_request = exact_preview_request(&second);
    first.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000011".to_owned(),
    });
    second.source_revision = Some(SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000022".to_owned(),
    });

    catalog
        .update_active_preview(&first, None, Some(&first_request))
        .expect("first hardlink revision adoption");
    let error = catalog
        .update_active_preview(&second, None, Some(&second_request))
        .expect_err("conflicting hardlink revision must be rejected");

    assert_eq!(error.code, "active_preview_location_stale");
    let revisions = catalog
        .connection
        .prepare(
            "SELECT DISTINCT source_revision_token
             FROM asset_locations
             WHERE file_identity_scheme = 'windows-file-id-128-v1'
               AND file_identity_value = '0000000000000001:00000000000000000000000000000001'",
        )
        .expect("prepare hardlink revisions")
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .expect("query hardlink revisions")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect hardlink revisions");
    assert_eq!(
        revisions,
        vec![Some(
            "windows-file-change-time-100ns-v1:0000000000000011".to_owned()
        )]
    );
}

#[cfg(windows)]
#[test]
fn staged_source_change_never_mutates_the_active_projection_before_publication() {
    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("same.jpg");
    fs::write(&source_path, b"source-version-one").expect("initial source");
    let root_path = source.path().to_string_lossy().into_owned();
    let discovery = crate::adapters::FileDiscovery::new(&root_path).expect("source discovery");
    let first_file = match discovery.visit_relative_path("same.jpg").outcome {
        crate::adapters::FileVisitOutcome::File(file) => file,
        _ => panic!("initial source file"),
    };
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    let first_scan = fixture_request("source-v1-scan", &root_path);
    catalog
        .begin_scan(&first_scan, "source-root", &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&first_scan.scan_id)
        .expect("prove initial source first-import handoff");
    let first_location = AssetLocationView {
        asset_id: "source-asset".to_owned(),
        location_id: "source-location".to_owned(),
        root_id: "source-root".to_owned(),
        scan_id: first_scan.scan_id.clone(),
        absolute_path: first_file.absolute_path.clone(),
        display_path: first_file.absolute_path.clone(),
        relative_path: first_file.relative_path.clone(),
        preview_path: "C:\\Cache\\source-v1.jpg".to_owned(),
        file_size: first_file.file_size,
        created_unix_ms: first_file.created_unix_ms,
        modified_unix_ms: first_file.modified_unix_ms,
        file_identity: first_file.file_identity.clone(),
        source_revision: first_file.source_revision.clone(),
        source_generation: 0,
        width: 10,
        height: 10,
        preview_status: PreviewStatus::Ready,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    };
    catalog
        .stage_location(&first_scan.scan_id, "source-root", &first_location)
        .expect("stage initial source");
    catalog
        .publish_scan(&first_scan.scan_id, "source-root", 1, 0)
        .expect("publish initial source");
    let active_before = catalog
        .load_active_location("source-location")
        .expect("load initial source")
        .expect("initial source location");
    let assert_active_unchanged = |actual: &AssetLocationView| {
        assert_eq!(actual.asset_id, active_before.asset_id);
        assert_eq!(actual.scan_id, active_before.scan_id);
        assert_eq!(actual.file_size, active_before.file_size);
        assert_eq!(actual.modified_unix_ms, active_before.modified_unix_ms);
        assert_eq!(actual.file_identity, active_before.file_identity);
        assert_eq!(actual.source_revision, active_before.source_revision);
        assert_eq!(actual.source_generation, active_before.source_generation);
        assert_eq!(actual.preview_path, active_before.preview_path);
        assert_eq!(
            std::mem::discriminant(&actual.preview_status),
            std::mem::discriminant(&active_before.preview_status),
        );
    };

    fs::write(&source_path, b"source-version-two").expect("change source before staging");
    let second_file = match discovery.visit_relative_path("same.jpg").outcome {
        crate::adapters::FileVisitOutcome::File(file) => file,
        _ => panic!("changed source file"),
    };
    assert_eq!(second_file.file_identity, first_file.file_identity);
    assert_ne!(second_file.source_revision, first_file.source_revision);
    let second_scan = fixture_request("source-v2-scan", &root_path);
    catalog
        .begin_scan(&second_scan, "source-root", &root_path)
        .expect("begin replacement scan");
    let mut second_location = first_location;
    second_location.scan_id = second_scan.scan_id.clone();
    second_location.preview_path.clear();
    second_location.file_size = second_file.file_size;
    second_location.created_unix_ms = second_file.created_unix_ms;
    second_location.modified_unix_ms = second_file.modified_unix_ms;
    second_location.file_identity = second_file.file_identity.clone();
    second_location.source_revision = second_file.source_revision.clone();
    second_location.source_generation = 0;
    second_location.preview_status = PreviewStatus::Pending;
    catalog
        .stage_location(&second_scan.scan_id, "source-root", &second_location)
        .expect("stage changed source");
    assert_eq!(
        catalog
            .count_staged_file_states(&second_scan.scan_id)
            .expect("flush staged source"),
        1
    );
    let active_after_flush = catalog
        .load_active_location("source-location")
        .expect("load active source after staging")
        .expect("active source after staging");
    assert_active_unchanged(&active_after_flush);

    fs::write(&source_path, b"source-version-three").expect("change source after staged flush");
    catalog
        .abandon_scan(&second_scan.scan_id, "stale", 1)
        .expect("abandon stale replacement scan");
    let active_after_abandon = catalog
        .load_active_location("source-location")
        .expect("load active source after abandon")
        .expect("active source after abandon");
    assert_active_unchanged(&active_after_abandon);
    drop(catalog);
    let reopened = SqliteCatalog::open(catalog_path).expect("reopen coherent catalog");
    let active_after_reopen = reopened
        .load_active_location("source-location")
        .expect("load active source after reopen")
        .expect("active source after reopen");
    assert_active_unchanged(&active_after_reopen);
}

#[test]
fn scan_publication_atomically_advances_active_hardlink_aliases() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "hardlink-root-a-v1",
        "hardlink-root-a",
        "C:\\HardlinkRootA",
        "hardlink-location-a",
    );
    publish_fixture(
        &mut catalog,
        "hardlink-root-b-v1",
        "hardlink-root-b",
        "C:\\HardlinkRootB",
        "hardlink-location-b",
    );
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations
             SET file_identity_scheme = 'windows-file-id-128-v1',
                 file_identity_value =
                   '0000000000000001:00000000000000000000000000000044',
                 source_revision_token =
                   'windows-file-change-time-100ns-v1:0000000000000044',
                 source_generation = 1, preview_path = '', preview_status = 'pending';
             UPDATE catalog_state SET next_source_generation = 2;",
        )
        .expect("establish shared active identity");
    let replacement = fixture_request("hardlink-root-a-v2", "C:\\HardlinkRootA");
    catalog
        .begin_scan(&replacement, "hardlink-root-a", "C:\\HardlinkRootA")
        .expect("begin hardlink replacement scan");
    catalog
        .connection
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value, source_revision_token,
               source_generation
             )
             SELECT ?1, asset_id, location_id, root_id, absolute_path, relative_path,
                    '', 44, created_unix_ms, 55, file_local_time,
                    parent_relative_path, natural_name_key, 88, 66, 'pending', NULL, NULL,
                    'metadata-v2', '2', NULL, NULL, NULL, NULL,
                    file_identity_scheme, file_identity_value,
                    'windows-file-change-time-100ns-v1:0000000000000055', 2
             FROM asset_locations
             WHERE scan_id = 'hardlink-root-a-v1'
               AND location_id = 'hardlink-location-a'",
            [&replacement.scan_id],
        )
        .expect("stage replacement identity observation");
    catalog
        .connection
        .execute("UPDATE catalog_state SET next_source_generation = 3", [])
        .expect("advance provisional source allocator");
    let sibling_before = catalog
        .load_active_location("hardlink-location-b")
        .expect("load sibling before publication")
        .expect("sibling before publication");
    assert_eq!(sibling_before.file_size, 20);
    assert_eq!(sibling_before.source_generation, 1);

    catalog
        .publish_scan(&replacement.scan_id, "hardlink-root-a", 1, 0)
        .expect("publish replacement identity observation");

    let first = catalog
        .load_active_location("hardlink-location-a")
        .expect("load published first alias")
        .expect("published first alias");
    let second = catalog
        .load_active_location("hardlink-location-b")
        .expect("load published second alias")
        .expect("published second alias");
    for location in [first, second] {
        assert_eq!(location.file_size, 44);
        assert_eq!(location.modified_unix_ms, 55);
        assert_eq!((location.width, location.height), (88, 66));
        assert_eq!(location.metadata_engine_id, "metadata-v2");
        assert_eq!(location.metadata_engine_version, "2");
        assert_eq!(
            location.source_revision,
            Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000055".to_owned(),
            })
        );
        assert_eq!(location.source_generation, 3);
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
    }
}

#[test]
fn preview_publication_rejects_a_stale_source_generation() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-generation-scan",
        "preview-generation-root",
        "C:\\PreviewGenerationSource",
        "preview-generation-location",
    );
    let location = catalog
        .load_active_location("preview-generation-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = source_generation + 1
             WHERE location_id = 'preview-generation-location'",
            [],
        )
        .expect("advance source generation");

    let error = catalog
        .update_active_preview(&location, None, Some(&request))
        .expect_err("stale source generation");

    assert_eq!(error.code, "active_preview_location_stale");
}

#[test]
fn unrelated_catalog_revision_does_not_invalidate_an_exact_preview_lease() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-unrelated-scan",
        "preview-unrelated-root",
        "C:\\PreviewUnrelatedSource",
        "preview-unrelated-location",
    );
    let location = catalog
        .load_active_location("preview-unrelated-location")
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    catalog
        .connection
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .expect("advance unrelated catalog revision");

    catalog
        .update_active_preview(&location, None, Some(&request))
        .expect("exact lease remains valid");
}

#[test]
fn prerelease_missing_active_preview_is_downgraded_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "prerelease-missing-preview-scan",
        "prerelease-missing-preview-root",
        "C:\\PrereleaseMissingPreview",
        "prerelease-missing-preview-location",
    );
    let handoff_count: i64 = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs)
               + (SELECT COUNT(*) FROM library_change_scan_handoff_items)",
            [],
            |row| row.get(0),
        )
        .expect("terminal handoff count");
    assert_eq!(handoff_count, 0);
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 19;",
        )
        .expect("restore prerelease preview repair marker");
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("repair missing active preview");
    let repaired = reopened
        .load_active_location("prerelease-missing-preview-location")
        .expect("repaired active location query")
        .expect("repaired active location");

    assert!(repaired.preview_path.is_empty());
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert!(repaired.preview_issue_code.is_none());
    assert!(repaired.preview_issue_message.is_none());
}

#[test]
fn prerelease_stale_active_preview_and_owner_are_downgraded_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "prerelease-stale-preview-scan",
        "prerelease-stale-preview-root",
        "C:\\PrereleaseStalePreview",
        "prerelease-stale-preview-location",
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        "prerelease-stale-preview-location",
        "prerelease-stale-preview-artifact",
        "C:\\AmeCache\\prerelease-stale-preview.jpg",
    );
    catalog
        .connection
        .execute(
            "UPDATE preview_artifacts SET lifecycle_state = 'stale'
             WHERE artifact_key = ?1",
            [&artifact.artifact_key],
        )
        .expect("restore prerelease stale artifact");
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 19;",
        )
        .expect("restore prerelease preview repair marker");
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("repair stale active preview");
    let repaired = reopened
        .load_active_location("prerelease-stale-preview-location")
        .expect("repaired active location query")
        .expect("repaired active location");

    assert!(repaired.preview_path.is_empty());
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert_eq!(
        preview_reference_count(&reopened, &artifact.artifact_key),
        0
    );
    assert_eq!(
        preview_lifecycle_state(&reopened, &artifact.artifact_key),
        "evictable"
    );
}

#[test]
fn preview_publication_rejects_same_timestamp_file_identity_replacement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-original-scan",
        "preview-identity-root",
        "C:\\PreviewIdentitySource",
        "preview-identity-location",
    );
    let mut original = catalog
        .load_active_location("preview-identity-location")
        .expect("original location query")
        .expect("original location");
    let replacement_scan = fixture_request("preview-replacement-scan", "C:\\PreviewIdentitySource");
    let mut replacement = original.clone();
    replacement.absolute_path = "C:\\PreviewIdentitySource\\replacement.png".to_owned();
    replacement.display_path = replacement.absolute_path.clone();
    replacement.relative_path = "replacement.png".to_owned();
    replacement.file_identity = Some(FileIdentityEvidence {
        scheme: "windows-file-id-v1".to_owned(),
        value: "volume:replacement".to_owned(),
    });
    replacement.preview_path.clear();
    replacement.preview_status = PreviewStatus::Pending;
    catalog
        .begin_scan(
            &replacement_scan,
            "preview-identity-root",
            &replacement_scan.root_path,
        )
        .expect("begin replacement scan");
    catalog
        .stage_location(
            &replacement_scan.scan_id,
            "preview-identity-root",
            &replacement,
        )
        .expect("stage replacement");
    catalog
        .publish_scan(&replacement_scan.scan_id, "preview-identity-root", 1, 0)
        .expect("publish replacement scan");
    original.preview_path = "C:\\AmeCache\\stale-preview.jpg".to_owned();
    original.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "stale-preview-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: original.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: original.width,
        height: original.height,
    };

    let error = catalog
        .update_active_preview(&original, Some(&artifact), None)
        .expect_err("stale identity publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
    let active = catalog
        .load_active_location("preview-identity-location")
        .expect("active replacement query")
        .expect("active replacement");
    assert_eq!(active.absolute_path, replacement.absolute_path);
    assert!(matches!(active.preview_status, PreviewStatus::Pending));
}

#[test]
fn preview_usage_touches_are_coarsened_to_page_publication_intervals() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-touch-scan",
        "preview-touch-root",
        "C:\\PreviewTouchSource",
        "preview-touch-location",
    );
    let mut location = catalog
        .load_active_location("preview-touch-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = "C:\\AmeCache\\preview-touch.jpg".to_owned();
    let artifact = PreviewArtifact {
        artifact_key: "preview-touch-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect("publish preview artifact");
    catalog
        .connection
        .execute(
            "UPDATE preview_artifacts SET last_used_unix_ms = 0 WHERE artifact_key = ?1",
            [&artifact.artifact_key],
        )
        .expect("age preview artifact");
    let visible = vec![(location.location_id, location.preview_path)];

    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("first usage touch"),
        1
    );
    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("coarsened usage touch"),
        0
    );
}

#[test]
fn preview_root_activation_resets_only_artifacts_outside_the_new_root() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, preview_root) in [
        ("old", "C:\\OldPreviewRoot"),
        ("current", "D:\\CurrentPreviewRoot"),
    ] {
        let scan_id = format!("root-switch-{suffix}-scan");
        let root_id = format!("root-switch-{suffix}-root");
        let location_id = format!("root-switch-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\RootSwitchSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("{preview_root}\\{suffix}.jpg");
        location.preview_status = PreviewStatus::Ready;
        let artifact = PreviewArtifact {
            artifact_key: format!("root-switch-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 256,
            encoded_height: 192,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("publish preview artifact");
    }

    let reset = catalog
        .reset_previews_outside_root("D:\\CurrentPreviewRoot\\")
        .expect("reset old preview root");

    assert_eq!(reset, 1);
    let old = catalog
        .load_active_location("root-switch-old-location")
        .expect("old location query")
        .expect("old location");
    let current = catalog
        .load_active_location("root-switch-current-location")
        .expect("current location query")
        .expect("current location");
    assert!(matches!(old.preview_status, PreviewStatus::Pending));
    assert!(old.preview_path.is_empty());
    assert_eq!((old.width, old.height), (40, 50));
    assert!(matches!(current.preview_status, PreviewStatus::Ready));
    assert_eq!(current.preview_path, "D:\\CurrentPreviewRoot\\current.jpg");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");
    assert_eq!(artifact_count, 1);
    assert!(
        catalog
            .is_preview_artifact_path_indexed(
                "C:\\EquivalentPreviewAlias\\current.jpg",
                Some("root-switch-current"),
            )
            .expect("artifact-key ownership lookup")
    );
}

#[test]
fn preview_reclamation_orders_stale_before_lru_and_preserves_protected_locations() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, last_used) in [("stale", 30_i64), ("lru", 10_i64), ("protected", 1_i64)] {
        let scan_id = format!("reclaim-{suffix}-scan");
        let root_id = format!("reclaim-{suffix}-root");
        let location_id = format!("reclaim-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\ReclaimSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("C:\\AmeCache\\{suffix}.jpg");
        let artifact = PreviewArtifact {
            artifact_key: format!("artifact-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 40,
            encoded_height: 50,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("publish preview artifact");
        catalog
            .connection
            .execute(
                "UPDATE preview_artifacts
                 SET lifecycle_state = ?2, last_used_unix_ms = ?3
                 WHERE artifact_key = ?1",
                params![
                    artifact.artifact_key,
                    if suffix == "stale" { "stale" } else { "ready" },
                    last_used,
                ],
            )
            .expect("set reclamation evidence");
    }

    let protected = vec!["reclaim-protected-location".to_owned()];
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &protected,
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("reclamation candidates");

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.artifact_key.as_str())
            .collect::<Vec<_>>(),
        ["artifact-stale", "artifact-lru"],
    );
    assert!(
        catalog
            .remove_reclaimed_preview(&candidates[0])
            .expect("remove reclaimed preview")
    );
    let reclaimed = catalog
        .load_active_location("reclaim-stale-location")
        .expect("reclaimed location query")
        .expect("reclaimed location");
    assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
    assert!(reclaimed.preview_path.is_empty());
    assert_eq!((reclaimed.width, reclaimed.height), (40, 50));
}

#[test]
fn handoff_preview_owners_survive_staling_and_reclamation_until_explicit_cleanup() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "handoff-owner-scan",
        "handoff-owner-root",
        "C:\\HandoffOwnerSource",
        "handoff-owner-location",
    );
    let mut location = catalog
        .load_active_location("handoff-owner-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = "C:\\AmeCache\\handoff-owner.jpg".to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "handoff-owner-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect("publish handoff-owned preview");
    let identity = FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "handoff-owner-volume:file".to_owned(),
    };
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET file_identity_scheme = ?2, file_identity_value = ?3
             WHERE location_id = ?1",
            params![location.location_id, identity.scheme, identity.value],
        )
        .expect("assign handoff identity");
    location.file_identity = Some(identity);
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               updated_unix_ms
             )
             SELECT 'windows_usn_v1', 'legacy-owner',
                    file_identity_scheme, file_identity_value,
                    asset_id, location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code, preview_issue_message,
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value, 1
             FROM asset_locations
             WHERE scan_id = 'handoff-owner-scan' AND location_id = 'handoff-owner-location';
             INSERT INTO library_change_scan_handoff_batches(id, source_root_id, updated_unix_ms)
             VALUES ('handoff-owner-batch', 'handoff-owner-root', 1);
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value
             )
             SELECT 'handoff-owner-batch', file_identity_scheme, file_identity_value,
                    asset_id, location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code, preview_issue_message,
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value
             FROM asset_locations
             WHERE scan_id = 'handoff-owner-scan' AND location_id = 'handoff-owner-location';",
        )
        .expect("handoff preview owners");

    location.preview_path.clear();
    location.preview_status = PreviewStatus::Pending;
    catalog
        .update_active_preview(&location, None, None)
        .expect("detach active preview owner");
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("handoff-protected reclamation candidates");
    assert!(candidates.is_empty());
    assert!(
        !catalog
            .remove_reclaimed_preview(&PreviewReclamationCandidate {
                artifact_key: artifact.artifact_key.clone(),
                path: artifact.path.clone(),
            })
            .expect("defensive handoff reclamation guard")
    );

    catalog
        .reset_all_previews_for_cleanup()
        .expect("explicit handoff preview cleanup");
    let state: (i64, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM preview_artifacts WHERE artifact_key = ?1),
               (SELECT preview_status FROM library_change_catch_up_handoffs
                WHERE catch_up_watermark = 'legacy-owner'),
               (SELECT preview_path FROM library_change_catch_up_handoffs
                WHERE catch_up_watermark = 'legacy-owner'),
               (SELECT preview_status FROM library_change_scan_handoff_items
                WHERE batch_id = 'handoff-owner-batch'),
               (SELECT preview_path FROM library_change_scan_handoff_items
                WHERE batch_id = 'handoff-owner-batch')",
            [&artifact.artifact_key],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("cleaned handoff preview state");
    assert_eq!(
        state,
        (
            0,
            "pending".to_owned(),
            String::new(),
            "pending".to_owned(),
            String::new()
        )
    );
}

#[test]
fn shared_preview_is_protected_and_reset_through_every_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let artifact = PreviewArtifact {
        artifact_key: "shared-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: "C:\\AmeCache\\shared.jpg".to_owned(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: 40,
        height: 50,
    };
    for suffix in ["first", "second"] {
        let scan_id = format!("shared-{suffix}-scan");
        let root_id = format!("shared-{suffix}-root");
        let location_id = format!("shared-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\SharedSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = artifact.path.clone();
        location.preview_status = PreviewStatus::Ready;
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("share preview artifact");
    }
    let reference_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations
             WHERE artifact_key = ?1",
            [&artifact.artifact_key],
            |row| row.get(0),
        )
        .expect("shared references");
    assert_eq!(reference_count, 2);

    let protected = vec!["shared-first-location".to_owned()];
    let protected_candidates = catalog
        .load_preview_reclamation_candidates(
            &protected,
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("protected candidates");
    assert!(protected_candidates.is_empty());

    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("unprotected candidates");
    assert_eq!(candidates.len(), 1);
    assert!(
        catalog
            .remove_reclaimed_preview(&candidates[0])
            .expect("remove shared preview")
    );
    for location_id in ["shared-first-location", "shared-second-location"] {
        let location = catalog
            .load_active_location(location_id)
            .expect("reclaimed location query")
            .expect("reclaimed location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
    }
}

#[test]
fn unregistering_root_detaches_preview_references_before_location_identity_can_return() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_id = "unregister-preview-root";
    let location_id = "unregister-preview-location";
    publish_gallery_fixture(
        &mut catalog,
        "unregister-preview-scan",
        root_id,
        "C:\\UnregisterPreviewSource",
        &[(location_id, None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        location_id,
        "unregister-preview-artifact",
        "C:\\AmeCache\\unregister-preview.jpg",
    );

    assert!(catalog.unregister_root(root_id).expect("unregister root"));
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "stale"
    );

    publish_gallery_fixture(
        &mut catalog,
        "readded-preview-scan",
        root_id,
        "C:\\UnregisterPreviewSource",
        &[(location_id, None, 40)],
    );
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[location_id.to_owned()],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("reclamation candidates after re-registering root");
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.artifact_key.as_str())
            .collect::<Vec<_>>(),
        [artifact.artifact_key.as_str()]
    );
}

#[test]
fn unregistering_root_preserves_shared_assets_and_preview_owners() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "shared-remove-scan",
        "shared-remove-root",
        "C:\\SharedRemove",
        &[("shared-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "shared-keep-scan",
        "shared-keep-root",
        "C:\\SharedKeep",
        &[("shared-keep-location", None, 30)],
    );
    let shared_asset_id = catalog
        .load_active_location("shared-remove-location")
        .expect("removed location query")
        .expect("removed location")
        .asset_id;
    let replaced_asset_id = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query")
        .expect("kept location")
        .asset_id;
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET asset_id = ?1 WHERE location_id = ?2",
            params![shared_asset_id, "shared-keep-location"],
        )
        .expect("share logical asset across roots");
    catalog
        .connection
        .execute("DELETE FROM assets WHERE id = ?1", [replaced_asset_id])
        .expect("remove replaced fixture asset");

    let artifact = publish_preview_artifact(
        &mut catalog,
        "shared-remove-location",
        "shared-remove-artifact",
        "C:\\AmeCache\\shared-remove.jpg",
    );
    publish_preview_artifact(
        &mut catalog,
        "shared-keep-location",
        "shared-remove-artifact",
        "C:\\AmeCache\\shared-remove.jpg",
    );
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 2);

    assert!(
        catalog
            .unregister_root("shared-remove-root")
            .expect("unregister one shared root")
    );

    let kept = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query")
        .expect("kept shared location");
    assert_eq!(kept.asset_id, shared_asset_id);
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let shared_asset_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE id = ?1",
            [shared_asset_id.clone()],
            |row| row.get(0),
        )
        .expect("shared asset count");
    assert_eq!(shared_asset_count, 1);

    drop(catalog);
    let cancelled = Arc::new(AtomicBool::new(true));
    let control = CatalogMaintenanceControl::new(cancelled);
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    assert!(matches!(
        maintenance
            .try_reclaim_incremental(256, &control)
            .expect("cancelled post-removal maintenance result"),
        CatalogMaintenanceAttempt::Interrupted
    ));

    let catalog = SqliteCatalog::open(path).expect("reopen after interrupted maintenance");
    let removed_root_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_roots WHERE id = 'shared-remove-root'",
            [],
            |row| row.get(0),
        )
        .expect("removed root count after maintenance interruption");
    assert_eq!(removed_root_count, 0);
    let kept = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query after maintenance interruption")
        .expect("kept shared location after maintenance interruption");
    assert_eq!(kept.asset_id, shared_asset_id);
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
}

#[test]
fn unregistering_root_does_not_rewrite_unrelated_orphan_state() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "bounded-remove-scan",
        "bounded-remove-root",
        "C:\\BoundedRemove",
        &[("bounded-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "bounded-keep-scan",
        "bounded-keep-root",
        "C:\\BoundedKeep",
        &[("bounded-keep-location", None, 40)],
    );
    let removed_artifact = publish_preview_artifact(
        &mut catalog,
        "bounded-remove-location",
        "bounded-remove-artifact",
        "C:\\AmeCache\\bounded-remove.jpg",
    );
    let unrelated_artifact = publish_preview_artifact(
        &mut catalog,
        "bounded-keep-location",
        "bounded-unrelated-artifact",
        "C:\\AmeCache\\bounded-unrelated.jpg",
    );
    catalog
        .connection
        .execute(
            "DELETE FROM preview_artifact_locations WHERE artifact_key = ?1",
            [&unrelated_artifact.artifact_key],
        )
        .expect("make unrelated preview artifact unreferenced");
    catalog
        .connection
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES ('unrelated-orphan-asset', 1)",
            [],
        )
        .expect("insert unrelated orphan asset");

    assert!(
        catalog
            .unregister_root("bounded-remove-root")
            .expect("unregister bounded root")
    );

    assert_eq!(
        preview_lifecycle_state(&catalog, &removed_artifact.artifact_key),
        "stale"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, &unrelated_artifact.artifact_key),
        "ready"
    );
    let unrelated_orphan_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE id = 'unrelated-orphan-asset'",
            [],
            |row| row.get(0),
        )
        .expect("unrelated orphan count");
    assert_eq!(unrelated_orphan_count, 1);
    assert!(
        catalog
            .load_active_location("bounded-keep-location")
            .expect("unrelated location query")
            .is_some()
    );
}

#[test]
fn unregistering_root_reclaims_only_its_terminal_handoff_owners() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "handoff-scope-remove-scan",
        "handoff-scope-remove-root",
        "C:\\HandoffScopeRemove",
        &[("handoff-scope-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "handoff-scope-keep-scan",
        "handoff-scope-keep-root",
        "C:\\HandoffScopeKeep",
        &[
            ("handoff-scope-owned-preview-location", None, 40),
            ("handoff-scope-unrelated-preview-location", None, 41),
        ],
    );
    let owned_artifact = publish_preview_artifact(
        &mut catalog,
        "handoff-scope-owned-preview-location",
        "handoff-scope-owned-artifact",
        "C:\\AmeCache\\handoff-scope-owned.jpg",
    );
    let unrelated_artifact = publish_preview_artifact(
        &mut catalog,
        "handoff-scope-unrelated-preview-location",
        "handoff-scope-unrelated-artifact",
        "C:\\AmeCache\\handoff-scope-unrelated.jpg",
    );
    catalog
        .connection
        .execute_batch(
            "DELETE FROM preview_artifact_locations
               WHERE artifact_key IN (
                 'handoff-scope-owned-artifact',
                 'handoff-scope-unrelated-artifact'
               );
             UPDATE asset_locations
               SET preview_path = '', preview_status = 'pending'
               WHERE location_id IN (
                 'handoff-scope-owned-preview-location',
                 'handoff-scope-unrelated-preview-location'
               );
             INSERT INTO assets(id, created_unix_ms) VALUES
               ('handoff-scope-owned-asset', 50),
               ('handoff-scope-unrelated-orphan', 50);
             INSERT INTO scan_run_catch_up_lineage(
               scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES (
               'handoff-scope-remove-scan',
               'handoff-scope-source', 'handoff-scope-watermark', 50
             );
             INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, metadata_engine_id,
               metadata_engine_version, updated_unix_ms
             ) VALUES (
               'handoff-scope-source', 'handoff-scope-watermark',
               'windows-file-id-128-v1', 'handoff-scope-identity',
               'handoff-scope-owned-asset', 'handoff-scope-source-location',
               'handoff-scope-remove-root',
               'C:/HandoffScopeRemove/owned.jpg', 'owned.jpg',
               'C:\\AmeCache\\handoff-scope-owned.jpg',
               1, 50, 50, 1, 1, 'ready', 'fixture-metadata', '1', 50
             );",
        )
        .expect("seed terminal handoff and unrelated orphan state");
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_scoped_handoff_root_unregister
             BEFORE DELETE ON library_roots
             BEGIN
               SELECT RAISE(ABORT, 'injected scoped handoff unregister failure');
             END;",
        )
        .expect("install scoped handoff unregister failure fixture");

    let error = catalog
        .unregister_root("handoff-scope-remove-root")
        .expect_err("scoped handoff cleanup must roll back atomically");
    assert_eq!(error.code, "catalog_database_error");
    assert_eq!(
        preview_lifecycle_state(&catalog, &owned_artifact.artifact_key),
        "ready"
    );
    let rolled_back: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-owned-asset'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs
                WHERE catch_up_source = 'handoff-scope-source'
                  AND catch_up_watermark = 'handoff-scope-watermark')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("rolled-back scoped handoff projection");
    assert_eq!(rolled_back, (1, 1));
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_scoped_handoff_root_unregister;")
        .expect("remove scoped handoff unregister failure fixture");

    assert!(
        catalog
            .unregister_root("handoff-scope-remove-root")
            .expect("unregister handoff owner root")
    );

    assert_eq!(
        preview_lifecycle_state(&catalog, &owned_artifact.artifact_key),
        "stale"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, &unrelated_artifact.artifact_key),
        "ready"
    );
    let projection: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-owned-asset'),
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-unrelated-orphan'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs
                WHERE catch_up_source = 'handoff-scope-source'
                  AND catch_up_watermark = 'handoff-scope-watermark')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("post-unregister handoff cleanup projection");
    assert_eq!(projection, (0, 1, 0));
}

#[test]
fn unregistering_root_rolls_back_its_complete_catalog_cleanup() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "rollback-remove-scan",
        "rollback-remove-root",
        "C:\\RollbackRemove",
        &[("rollback-remove-location", None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        "rollback-remove-location",
        "rollback-remove-artifact",
        "C:\\AmeCache\\rollback-remove.jpg",
    );
    let revision_before = load_default_snapshot(&mut catalog, 10, None)
        .expect("snapshot before rollback fixture")
        .revision;
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_root_unregister
             BEFORE DELETE ON library_roots
             BEGIN
               SELECT RAISE(ABORT, 'injected root unregister failure');
             END;",
        )
        .expect("install unregister failure fixture");

    let error = catalog
        .unregister_root("rollback-remove-root")
        .expect_err("injected unregister failure must roll back");
    assert_eq!(error.code, "catalog_database_error");

    let snapshot =
        load_default_snapshot(&mut catalog, 10, None).expect("snapshot after rollback fixture");
    assert_eq!(snapshot.revision, revision_before);
    assert_eq!(snapshot.roots.len(), 1);
    assert!(
        catalog
            .load_active_location("rollback-remove-location")
            .expect("rolled back location query")
            .is_some()
    );
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
}

#[test]
fn publishing_replacement_scans_stales_artifact_only_after_its_last_location_leaves() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let artifact_key = "removed-shared-artifact";
    let artifact_path = "C:\\AmeCache\\removed-shared.jpg";
    for suffix in ["first", "second"] {
        let scan_id = format!("removed-shared-{suffix}-scan");
        let root_id = format!("removed-shared-{suffix}-root");
        let location_id = format!("removed-shared-{suffix}-location");
        publish_gallery_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\RemovedSharedSource\\{suffix}"),
            &[(location_id.as_str(), None, 30)],
        );
        publish_preview_artifact(&mut catalog, &location_id, artifact_key, artifact_path);
    }
    assert_eq!(preview_reference_count(&catalog, artifact_key), 2);

    publish_empty_replacement_scan(
        &mut catalog,
        "removed-shared-first-replacement",
        "removed-shared-first-root",
        "C:\\RemovedSharedSource\\first",
    );
    assert_eq!(preview_reference_count(&catalog, artifact_key), 1);
    assert_eq!(preview_lifecycle_state(&catalog, artifact_key), "ready");

    publish_empty_replacement_scan(
        &mut catalog,
        "removed-shared-second-replacement",
        "removed-shared-second-root",
        "C:\\RemovedSharedSource\\second",
    );
    assert_eq!(preview_reference_count(&catalog, artifact_key), 0);
    assert_eq!(preview_lifecycle_state(&catalog, artifact_key), "stale");
}

#[test]
fn abandoning_staged_scan_preserves_active_preview_reference() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_id = "abandoned-preview-root";
    let location_id = "abandoned-preview-location";
    publish_gallery_fixture(
        &mut catalog,
        "abandoned-preview-active-scan",
        root_id,
        "C:\\AbandonedPreviewSource",
        &[(location_id, None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        location_id,
        "abandoned-preview-artifact",
        "C:\\AmeCache\\abandoned-preview.jpg",
    );
    let staged_scan_id = "abandoned-preview-staged-scan";
    let request = fixture_request(staged_scan_id, "C:\\AbandonedPreviewSource");
    let active = catalog
        .load_active_location(location_id)
        .expect("active location query")
        .expect("active location");
    catalog
        .begin_scan(&request, root_id, &request.root_path)
        .expect("begin staged scan");
    catalog
        .stage_location(staged_scan_id, root_id, &active)
        .expect("stage matching location");
    assert_eq!(
        catalog
            .count_staged_file_states(staged_scan_id)
            .expect("flush staged location"),
        1
    );

    catalog
        .abandon_scan(staged_scan_id, "cancelled", 0)
        .expect("abandon staged scan");

    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let retained = catalog
        .load_active_location(location_id)
        .expect("retained active location query")
        .expect("retained active location");
    assert_eq!(retained.preview_path, artifact.path);
    assert!(matches!(retained.preview_status, PreviewStatus::Ready));
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
fn migrates_v1_catalog_without_losing_the_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v1 catalog");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
                 CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (1);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 INSERT INTO library_roots VALUES ('root-1', 'C:\\Pictures', 'scan-1', 10);
                 INSERT INTO scan_runs VALUES ('scan-1', 'root-1', 'completed', 11, 12, 1, 0);
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'location-1', 'root-1', 'C:\\Pictures\\one.png',
                   'one.png', 'C:\\Cache\\one.jpg', 20, 30, 40, 50
                 );",
        )
        .expect("v1 schema");
    drop(connection);

    let mut catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.roots.len(), 1);
    assert_eq!(snapshot.assets.len(), 1);
    assert_eq!(snapshot.assets[0].asset_id, "legacy:scan-1:location-1");
}

#[test]
fn migrates_v2_catalog_revision_from_completed_scans() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v2 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (2);
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('scan-1', 'root-1', 'completed', 1, 2, 1, 0),
                   ('scan-2', 'root-2', 'cancelled', 3, 4, 0, 0);",
        )
        .expect("v2 schema");
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, revision): (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, catalog_state.revision
                 FROM schema_info CROSS JOIN catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("schema state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(revision, 1);
}

#[test]
fn migrates_v3_without_treating_an_uncheckpointed_scan_as_recoverable() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v3 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (3);
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('scan-running', 'root-1', 'running', 11, NULL, 0, 0),
                   ('scan-complete', 'root-2', 'completed', 12, 13, 1, 0);
                 INSERT INTO library_roots VALUES
                   ('root-1', 'C:\\Pictures', NULL, 1),
                   ('root-2', 'C:\\Photos', 'scan-complete', 2);",
        )
        .expect("v3 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, running_status): (i64, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, scan_runs.status
                 FROM schema_info JOIN scan_runs ON scan_runs.id = 'scan-running'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("migrated state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(running_status, "interrupted_unrecoverable");
    assert!(
        catalog
            .load_recoverable_scan()
            .expect("recoverable scan query")
            .is_none()
    );
}

#[test]
fn migrates_v4_tasks_without_inventing_a_missing_directory_frontier() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v4 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (4);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0,
                   max_items INTEGER, max_entries INTEGER,
                   preview_edge INTEGER NOT NULL,
                   last_visited_relative_path TEXT,
                   visited_entries INTEGER NOT NULL DEFAULT 0,
                   accepted_items INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('running', 'root-1', 'running', 1, NULL, 0, 0, 500, 2000, 512,
                    'old.png', 10, 2),
                   ('paused', 'root-2', 'paused', 2, NULL, 0, 0, 500, 2000, 512,
                    'old.png', 20, 3),
                   ('complete', 'root-3', 'completed', 3, 4, 1, 0, 500, 2000, 512,
                    NULL, 30, 4);",
        )
        .expect("v4 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let statuses = catalog
        .connection
        .prepare("SELECT id, status FROM scan_runs ORDER BY id")
        .expect("status statement")
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("status rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("stored statuses");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(
        statuses,
        vec![
            ("complete".to_owned(), "completed".to_owned()),
            ("paused".to_owned(), "interrupted_unrecoverable".to_owned(),),
            ("running".to_owned(), "interrupted_unrecoverable".to_owned(),),
        ]
    );
}

#[test]
fn migrates_v5_tasks_without_inventing_a_missing_entry_snapshot() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v5 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (5);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0,
                   max_items INTEGER, max_entries INTEGER,
                   preview_edge INTEGER NOT NULL,
                   current_directory_relative_path TEXT,
                   last_visited_relative_path TEXT,
                   visited_entries INTEGER NOT NULL DEFAULT 0,
                   accepted_items INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE scan_directory_frontier (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   scan_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   UNIQUE(scan_id, relative_path)
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('running', 'root-1', 'running', 1, NULL, 0, 0, NULL, NULL,
                    512, 'wide', 'wide\\255.png', 256, 10),
                   ('paused', 'root-2', 'paused', 2, NULL, 0, 0, NULL, NULL,
                    512, 'other', NULL, 0, 0),
                   ('complete', 'root-3', 'completed', 3, 4, 1, 0, NULL, NULL,
                    512, NULL, NULL, 1, 1);
                 INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES
                   ('running', 'pending'), ('paused', 'pending');",
        )
        .expect("v5 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let statuses = catalog
        .connection
        .prepare("SELECT id, status FROM scan_runs ORDER BY id")
        .expect("status statement")
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("status rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("stored statuses");
    let frontier_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM scan_directory_frontier", [], |row| {
            row.get(0)
        })
        .expect("frontier count");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(
        statuses,
        vec![
            ("complete".to_owned(), "completed".to_owned()),
            ("paused".to_owned(), "interrupted_unrecoverable".to_owned()),
            ("running".to_owned(), "interrupted_unrecoverable".to_owned()),
        ]
    );
    assert_eq!(frontier_count, 0);
}

#[test]
fn migrates_v6_previews_as_pending_when_source_revision_is_unknown() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v6 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (6);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50
                 );",
        )
        .expect("v6 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, preview_path, preview_status, engine_id): (i64, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.preview_path,
                        asset_locations.preview_status, asset_locations.metadata_engine_id
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migrated preview");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(preview_path.is_empty());
    assert_eq!(preview_status, "pending");
    assert_eq!(engine_id, "ame-invalidated-media-metadata");
}

#[test]
fn migrates_v7_locations_as_unanalyzed_metadata() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v7 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (7);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL,
                   preview_status TEXT NOT NULL,
                   preview_issue_code TEXT,
                   preview_issue_message TEXT,
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50, 'ready', NULL, NULL
                 );",
        )
        .expect("v7 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, engine_id, engine_version, capture_time): (i64, String, String, Option<String>) =
        catalog
            .connection
            .query_row(
                "SELECT schema_info.version, asset_locations.metadata_engine_id,
                        asset_locations.metadata_engine_version,
                        asset_locations.capture_local_time
                 FROM schema_info CROSS JOIN asset_locations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated metadata state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(engine_id, "ame-invalidated-media-metadata");
    assert_eq!(engine_version, "0");
    assert!(capture_time.is_none());
}

#[test]
fn migrates_v8_locations_with_unknown_file_identity() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v8 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (8);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, preview_status TEXT NOT NULL,
                   preview_issue_code TEXT, preview_issue_message TEXT,
                   metadata_engine_id TEXT NOT NULL,
                   metadata_engine_version TEXT NOT NULL,
                   capture_local_time TEXT, capture_offset_minutes INTEGER,
                   capture_time_source TEXT, capture_raw_value TEXT,
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50, 'ready', NULL, NULL,
                   'kamadak-exif', '0.6.1', NULL, NULL, NULL, NULL
                 );",
        )
        .expect("v8 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, scheme, value): (i64, Option<String>, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.file_identity_scheme,
                        asset_locations.file_identity_value
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated file identity");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(scheme.is_none());
    assert!(value.is_none());
}

#[test]
fn migrates_v9_with_bounded_reconciliation_indexes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v9 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (9);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL,
                   asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL,
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   modified_unix_ms INTEGER NOT NULL,
                   capture_local_time TEXT,
                   file_identity_scheme TEXT,
                   file_identity_value TEXT,
                   preview_path TEXT NOT NULL DEFAULT '',
                   preview_status TEXT NOT NULL DEFAULT 'pending',
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'archive/one.png', 1, NULL,
                   'windows-file-id-128-v1',
                   '0000000000000001:00000000000000000000000000000002',
                   '', 'pending'
                 );",
        )
        .expect("v9 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, asset_id, identity_value): (i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.asset_id,
                        asset_locations.file_identity_value
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated row");
    let index_names = catalog
        .connection
        .prepare(
            "SELECT name FROM sqlite_master
                 WHERE type = 'index' AND tbl_name = 'asset_locations'",
        )
        .expect("index query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("index rows")
        .collect::<Result<HashSet<_>, _>>()
        .expect("index names");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(asset_id, "asset-1");
    assert_eq!(
        identity_value,
        "0000000000000001:00000000000000000000000000000002"
    );
    assert!(index_names.contains("asset_locations_active_file_identity"));
    assert!(index_names.contains("asset_locations_location_id"));
    assert!(index_names.contains("asset_locations_asset_id"));
    assert!(index_names.contains("asset_locations_gallery_time"));
    assert!(index_names.contains("asset_locations_gallery_created"));
    assert!(index_names.contains("asset_locations_gallery_modified"));
    assert!(index_names.contains("asset_locations_gallery_name"));
    assert!(index_names.contains("asset_locations_parent_folder"));
}

#[test]
fn migrates_v10_by_adding_the_gallery_time_index_without_rewriting_rows() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v10 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (10);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL,
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   modified_unix_ms INTEGER NOT NULL,
                   capture_local_time TEXT,
                   preview_path TEXT NOT NULL DEFAULT '',
                   preview_status TEXT NOT NULL DEFAULT 'pending',
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'location-1', 'root-1', 'Album/img10.png', 123,
                   '2025-08-07T10:20:30.000000000', '', 'pending'
                 );",
        )
        .expect("v10 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, capture_time, parent_path, name_key): (i64, Option<String>, String, String) =
        catalog
            .connection
            .query_row(
                "SELECT schema_info.version, asset_locations.capture_local_time,
                        asset_locations.parent_relative_path,
                        asset_locations.natural_name_key
                 FROM schema_info CROSS JOIN asset_locations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated gallery row");
    let gallery_index: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'asset_locations_gallery_time'",
            [],
            |row| row.get(0),
        )
        .expect("gallery index");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(capture_time.is_none());
    assert_eq!(parent_path, "Album");
    assert_eq!(name_key, natural_name_key("Album/img10.png"));
    assert_eq!(gallery_index, 1);
}

#[test]
fn migrates_v12_by_materializing_the_file_time_fallback_key() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v12 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (12);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               capture_local_time TEXT,
               preview_path TEXT NOT NULL DEFAULT '',
               preview_status TEXT NOT NULL DEFAULT 'pending',
               PRIMARY KEY(scan_id, location_id)
             );
             INSERT INTO asset_locations VALUES (
               'scan-1', 'location-1', 'root-1', 'Album/photo.png',
               1749988800000, 1784116800000, NULL, '', 'pending'
             );
             CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (capture_local_time IS NULL), IFNULL(capture_local_time, '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (created_unix_ms IS NULL), IFNULL(created_unix_ms, 0),
                 root_id, location_id, scan_id
               );",
        )
        .expect("v12 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, file_local_time, capture_local_time): (i64, String, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.file_local_time,
                    asset_locations.capture_local_time
             FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated fallback row");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(file_local_time.starts_with("2025-06-"));
    assert!(capture_local_time.is_none());
}

#[test]
fn migrates_v13_with_an_empty_preview_artifact_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v13 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (13);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL, root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL DEFAULT 'pending'
             );",
        )
        .expect("v13 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("preview artifact count");
    let index_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name LIKE 'preview_artifacts_%'",
            [],
            |row| row.get(0),
        )
        .expect("preview artifact indexes");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(artifact_count, 0);
    let ownership_index_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name = 'preview_artifact_locations_location'",
            [],
            |row| row.get(0),
        )
        .expect("preview ownership index");

    assert_eq!(index_count, 2);
    assert_eq!(ownership_index_count, 1);
}

#[test]
fn migrates_v14_preview_ownership_to_every_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v14 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (14);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             INSERT INTO library_roots VALUES
               ('root-1', 'scan-1'), ('root-2', 'scan-2');
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL, root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL
             );
             INSERT INTO asset_locations VALUES
               ('scan-1', 'asset-1', 'location-1', 'root-1', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-2', 'asset-2', 'location-2', 'root-2', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready');
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               location_id TEXT NOT NULL,
               source_file_size INTEGER NOT NULL,
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL,
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL,
               encoded_width INTEGER NOT NULL,
               encoded_height INTEGER NOT NULL,
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL,
               lifecycle_state TEXT NOT NULL,
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL
             );
             CREATE INDEX preview_artifacts_location
               ON preview_artifacts(location_id, size_bucket, lifecycle_state);
             CREATE INDEX preview_artifacts_reclamation
               ON preview_artifacts(lifecycle_state, last_used_unix_ms, artifact_key);
             CREATE INDEX preview_artifacts_compatibility
               ON preview_artifacts(
                 location_id, source_file_size, source_modified_unix_ms,
                 algorithm_id, algorithm_version, orientation_contract, size_bucket
               );
             INSERT INTO preview_artifacts VALUES (
               'shared-artifact', 'location-2', 20, 30, NULL, NULL,
               'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
               'C:\\Cache\\shared.jpg', 1024, 'ready', 40, 50
             );",
        )
        .expect("v14 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let locations = catalog
        .connection
        .prepare(
            "SELECT location_id FROM preview_artifact_locations
             WHERE artifact_key = 'shared-artifact' ORDER BY location_id",
        )
        .expect("ownership query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("ownership rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("ownership collection");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(locations.is_empty());
    assert_eq!(
        preview_lifecycle_state(&catalog, "shared-artifact"),
        "evictable"
    );
}

#[test]
fn migrates_v15_by_reconciling_preview_ownership_with_active_locations() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v15 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (15);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             INSERT INTO library_roots VALUES
               ('root-1', 'scan-1'),
               ('root-2', 'scan-2'),
               ('root-retired', 'scan-current');
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL
             );
             INSERT INTO asset_locations VALUES
               ('scan-1', 'location-1', 'root-1', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-2', 'location-2', 'root-2', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-retired', 'location-retired', 'root-retired', 'retired.jpg',
                'C:\\Cache\\retired.jpg', 'ready');
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               source_file_size INTEGER NOT NULL,
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL,
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL,
               encoded_width INTEGER NOT NULL,
               encoded_height INTEGER NOT NULL,
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL,
               lifecycle_state TEXT NOT NULL,
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL
             );
             INSERT INTO preview_artifacts VALUES
               ('shared-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\shared.jpg', 1024, 'ready', 40, 50),
               ('retired-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\retired.jpg', 1024, 'ready', 40, 50),
               ('wrong-path-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\wrong.jpg', 1024, 'ready', 40, 50);
             CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL,
               PRIMARY KEY(artifact_key, location_id),
               FOREIGN KEY(artifact_key) REFERENCES preview_artifacts(artifact_key)
                 ON DELETE CASCADE
             );
             CREATE INDEX preview_artifact_locations_location
               ON preview_artifact_locations(location_id, artifact_key);
             INSERT INTO preview_artifact_locations VALUES
               ('shared-artifact', 'location-1'),
               ('shared-artifact', 'location-2'),
               ('retired-artifact', 'location-retired'),
               ('wrong-path-artifact', 'location-1');",
        )
        .expect("v15 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let shared_locations = catalog
        .connection
        .prepare(
            "SELECT location_id FROM preview_artifact_locations
             WHERE artifact_key = 'shared-artifact' ORDER BY location_id",
        )
        .expect("shared ownership query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("shared ownership rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("shared ownership collection");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(shared_locations.is_empty());
    assert_eq!(preview_reference_count(&catalog, "retired-artifact"), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, "retired-artifact"),
        "evictable"
    );
    assert_eq!(preview_reference_count(&catalog, "wrong-path-artifact"), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, "wrong-path-artifact"),
        "evictable"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, "shared-artifact"),
        "evictable"
    );
}

#[test]
fn gallery_time_query_uses_its_ordering_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT location_id FROM asset_locations
                 ORDER BY (COALESCE(capture_local_time, file_local_time) IS NULL),
                          IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                          modified_unix_ms DESC, root_id, location_id
                 LIMIT 100",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_gallery_time")),
        "unexpected gallery-time query plan: {details:?}"
    );
}

#[test]
fn active_relative_path_lookup_uses_its_complete_lookup_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
             SELECT locations.location_id
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
             WHERE locations.root_id = ?1 AND locations.relative_path = ?2
             ORDER BY locations.location_id
             LIMIT 1",
        )
        .expect("query plan")
        .query_map(["root-a", "album/photo.jpg"], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_root_relative")),
        "unexpected active path query plan: {details:?}"
    );
}

#[test]
fn orphan_cleanup_plan_uses_the_asset_identity_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT 1 FROM assets
                 WHERE NOT EXISTS (
                   SELECT 1 FROM asset_locations
                   WHERE asset_locations.asset_id = assets.id
                 )",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_asset_id")),
        "unexpected orphan-cleanup query plan: {details:?}"
    );
}

#[test]
fn capture_time_evidence_round_trips_with_engine_identity() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-capture", "C:\\Pictures");
    catalog
        .begin_scan(&request, "root-capture", "C:\\Pictures")
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove capture fixture first-import handoff");
    catalog
        .stage_location(
            "scan-capture",
            "root-capture",
            &AssetLocationView {
                asset_id: "asset-capture".to_owned(),
                location_id: "location-capture".to_owned(),
                root_id: "root-capture".to_owned(),
                scan_id: "scan-capture".to_owned(),
                absolute_path: "C:\\Pictures\\capture.jpg".to_owned(),
                display_path: "C:\\Pictures\\capture.jpg".to_owned(),
                relative_path: "capture.jpg".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: Some(FileIdentityEvidence {
                    scheme: "windows-file-id-128-v1".to_owned(),
                    value: "0000000000000001:00000000000000000000000000000002".to_owned(),
                }),
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 1,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "kamadak-exif".to_owned(),
                metadata_engine_version: "0.6.1".to_owned(),
                capture_time: Some(CaptureTimeEvidence {
                    local_time: "2025-07-08T09:10:11.123000000".to_owned(),
                    offset_minutes: Some(480),
                    source: CaptureTimeSource::Original,
                    raw_value: "2025:07:08 09:10:11|123|+08:00".to_owned(),
                }),
            },
        )
        .expect("stage location");
    catalog
        .publish_scan("scan-capture", "root-capture", 1, 0)
        .expect("publish scan");

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot");
    let asset = snapshot.assets.first().expect("stored asset");
    let capture = asset.capture_time.as_ref().expect("capture evidence");

    assert_eq!(asset.metadata_engine_id, "kamadak-exif");
    assert_eq!(asset.metadata_engine_version, "0.6.1");
    assert_eq!(
        asset
            .file_identity
            .as_ref()
            .map(|identity| identity.scheme.as_str()),
        Some("windows-file-id-128-v1")
    );
    assert_eq!(capture.local_time, "2025-07-08T09:10:11.123000000");
    assert_eq!(capture.offset_minutes, Some(480));
    assert!(matches!(capture.source, CaptureTimeSource::Original));
}

#[test]
fn directory_entry_frontier_is_idempotent_sorted_and_windowed() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = fixture_request("wide-scan", "C:\\Pictures");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    catalog
        .begin_scan(&request, "wide-root", "C:\\Pictures")
        .expect("begin scan");
    assert_eq!(
        catalog
            .claim_next_directory("wide-scan")
            .expect("claim root"),
        Some(String::new())
    );
    assert!(
        !catalog
            .is_current_directory_enumerated("wide-scan", "")
            .expect("enumeration state")
    );

    let mut entries = (0..1025)
        .rev()
        .map(|index| format!("image-{index:04}.png"))
        .collect::<Vec<_>>();
    catalog
        .stage_directory_entries("wide-scan", "", &entries[..17])
        .expect("partial enumeration");
    entries.push("image-0000.png".to_owned());
    for batch in entries.chunks(113) {
        catalog
            .stage_directory_entries("wide-scan", "", batch)
            .expect("entry batch");
    }
    catalog
        .complete_directory_enumeration("wide-scan", "")
        .expect("complete enumeration");

    let mut loaded = Vec::new();
    loop {
        let window = catalog
            .load_directory_entry_window("wide-scan", "", loaded.last().map(String::as_str), 128)
            .expect("entry window");
        assert!(window.len() <= 128);
        if window.is_empty() {
            break;
        }
        loaded.extend(window);
    }

    assert_eq!(loaded.len(), 1025);
    assert!(loaded.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        catalog
            .has_directory_entry("wide-scan", "", "image-0512.png")
            .expect("entry identity")
    );
    catalog
        .complete_directory("wide-scan", &ScanCheckpoint::default())
        .expect("complete directory");
    let queued: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM scan_directory_entries", [], |row| {
            row.get(0)
        })
        .expect("entry count");
    assert_eq!(queued, 0);
}

#[test]
fn persists_and_validates_a_published_root_replacement_checkpoint() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let raw_root_path = r"\\?\C:\Pictures";
    let request = fixture_request("scan-resume", raw_root_path);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let published = fixture_request("published-before-resume", raw_root_path);
    catalog
        .begin_scan(&published, "root-resume", raw_root_path)
        .expect("initial import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&published.scan_id)
        .expect("initial capture");
    catalog
        .publish_scan(&published.scan_id, "root-resume", 0, 0)
        .expect("trustworthy prior baseline");
    let initial = catalog
        .begin_scan(&request, "root-resume", raw_root_path)
        .expect("begin scan");
    assert_eq!(initial.visited_entries, 0);
    let checkpoint = ScanCheckpoint {
        last_visited_relative_path: Some("nested/last.png".to_owned()),
        visited_entries: 128,
        accepted_items: 40,
        issue_count: 3,
        requires_previous_snapshot: true,
    };
    catalog
        .checkpoint_scan("scan-resume", &checkpoint)
        .expect("persist checkpoint");
    drop(catalog);

    let mut restored = SqliteCatalog::open(path).expect("restored catalog");
    let recoverable = restored
        .load_recoverable_scan()
        .expect("recoverable scan")
        .expect("stored running scan");
    assert_eq!(recoverable.scan_id, "scan-resume");
    assert_eq!(recoverable.root_path, raw_root_path);
    assert_eq!(recoverable.display_root_path, "C:\\Pictures");
    assert_eq!(recoverable.visited_entries, 128);
    assert_eq!(recoverable.accepted_items, 40);
    assert_eq!(recoverable.issue_count, 3);
    let create_error = restored
        .begin_scan(&request, "unexpected-root", "C:\\Unexpected")
        .expect_err("new scans cannot reuse a checkpoint identifier");
    assert_eq!(create_error.code, "catalog_scan_already_exists");
    let unexpected_root_count = restored
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_roots WHERE id = 'unexpected-root'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("unexpected root count");
    assert_eq!(unexpected_root_count, 0);
    let resumed = restored
        .resume_scan(&request, "root-resume", raw_root_path)
        .expect("resume scan");
    assert_eq!(
        resumed.last_visited_relative_path,
        checkpoint.last_visited_relative_path
    );
    assert!(resumed.requires_previous_snapshot);
    let publish_error = restored
        .publish_scan("scan-resume", "root-resume", 40, 3)
        .expect_err("an unresolved prior issue cannot publish");
    assert_eq!(
        publish_error.code,
        "catalog_scan_requires_previous_snapshot"
    );

    let mut changed_request = request.clone();
    changed_request.preview_edge = 256;
    let error = restored
        .resume_scan(&changed_request, "root-resume", "C:\\Pictures")
        .expect_err("changed parameters cannot reuse a checkpoint");
    assert_eq!(error.code, "catalog_scan_resume_mismatch");
}

#[test]
fn keyset_pages_retain_multiple_roots_and_reject_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");

    publish_fixture(&mut catalog, "scan-1", "root-1", "C:\\One", "location-1");
    publish_fixture(&mut catalog, "scan-2", "root-2", "C:\\Two", "location-2");

    let first_asset_id = catalog
        .load_active_location("location-1")
        .expect("existing location")
        .expect("active location")
        .asset_id;
    let first_page = load_default_snapshot(&mut catalog, 1, None).expect("first page");
    let cursor = first_page.next_cursor.clone().expect("next cursor");
    let second_page = load_default_snapshot(&mut catalog, 1, Some(&cursor)).expect("second page");

    assert_eq!(first_asset_id, "asset-scan-1");
    assert_eq!(first_page.revision, 2);
    assert_eq!(first_page.roots.len(), 2);
    assert_eq!(first_page.assets.len(), 1);
    assert_eq!(second_page.assets.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert_ne!(
        first_page.assets[0].location_id,
        second_page.assets[0].location_id,
    );

    publish_fixture(&mut catalog, "scan-3", "root-3", "C:\\Three", "location-3");
    let error = load_default_snapshot(&mut catalog, 1, Some(&cursor))
        .expect_err("published changes invalidate old cursors");
    assert_eq!(error.code, "catalog_cursor_stale");
}

fn insert_publication_gate_live_change(
    catalog: &SqliteCatalog,
    root_id: &str,
    intent_kind: &str,
    scope: &str,
    relative_path: &str,
    status: &str,
    authoritative_scan_id: Option<&str>,
) -> i64 {
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("publication gate catalog revision");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, next_retry_unix_ms, lease_expires_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, authoritative_scan_id,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, ?2, ?3, ?4, 'live_notification', 50, 50, '1', '1', 1,
               ?5, 50, CASE WHEN ?5 = 'retry_wait' THEN 75 ELSE NULL END,
               CASE WHEN ?5 = 'leased' THEN 100 ELSE NULL END,
               CASE WHEN ?5 = 'retry_wait' THEN 'fixture_retry' ELSE NULL END,
               CASE WHEN ?5 = 'retry_wait' THEN 'Fixture retry debt' ELSE NULL END,
               ?6, ?7, 50, 50
             )",
            params![
                root_id,
                intent_kind,
                scope,
                relative_path,
                status,
                revision,
                authoritative_scan_id,
            ],
        )
        .expect("insert publication gate live change");
    catalog.connection.last_insert_rowid()
}

#[test]
fn first_import_allows_durable_exact_path_work_but_keeps_it_pending() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let request = fixture_request("first-path-scan", "C:\\FirstPath");
    catalog
        .begin_scan(&request, "first-path-root", &request.root_path)
        .expect("begin first import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove first import change capture");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "first-path-root",
        "reconcile",
        "path",
        "changed.jpg",
        "pending",
        None,
    );

    catalog
        .publish_scan(&request.scan_id, "first-path-root", 0, 0)
        .expect("publish first import around durable exact path work");

    let retained: (String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained first-import path work");
    assert_eq!(retained, ("pending".to_owned(), None));
}

#[test]
fn first_import_retains_non_path_live_work_without_claiming_freshness() {
    for (kind, scope, relative_path) in [
        ("freshness_unknown", "root", ""),
        ("reconcile", "root", ""),
        ("reconcile", "subtree", "incoming"),
    ] {
        for status in ["pending", "leased", "retry_wait"] {
            assert_first_import_retains_live_work(kind, scope, relative_path, status);
        }
    }
}

fn assert_first_import_retains_live_work(
    kind: &str,
    scope: &str,
    relative_path: &str,
    status: &str,
) {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let request = fixture_request("first-root-gap-scan", "C:\\FirstRootGap");
    catalog
        .begin_scan(&request, "first-root-gap", &request.root_path)
        .expect("begin first import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove first import change capture");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "first-root-gap",
        kind,
        scope,
        relative_path,
        status,
        None,
    );

    catalog
        .publish_scan(&request.scan_id, "first-root-gap", 0, 0)
        .expect("publish a baseline so live work can become eligible");
    let retained: (String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("unconsumed live work");
    assert_eq!(retained, (status.to_owned(), None));
    let authority: (String, String) = catalog
        .connection
        .query_row(
            "SELECT roots.active_scan_id, journal.continuity_state
             FROM library_roots AS roots
             JOIN library_persistent_journal_root_state AS journal ON journal.root_id = roots.id
             WHERE roots.id = 'first-root-gap'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("non-current baseline authority");
    assert_eq!(authority, (request.scan_id, "live_only".to_owned()));
}

#[test]
fn replacement_blocks_pending_path_work_but_preserves_retry_wait_after_publication() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "path-gate-initial",
        "path-gate-root",
        "C:\\PathGate",
        "path-gate-location",
    );
    let replacement = fixture_request("path-gate-replacement", "C:\\PathGate");
    catalog
        .begin_scan(&replacement, "path-gate-root", &replacement.root_path)
        .expect("begin replacement");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "path-gate-root",
        "reconcile",
        "path",
        "changed.jpg",
        "pending",
        Some(&replacement.scan_id),
    );
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET change_queue_high_watermark = ?2 WHERE id = ?1",
            params![replacement.scan_id, change_id],
        )
        .expect("bind replacement high watermark");

    let error = catalog
        .publish_scan(&replacement.scan_id, "path-gate-root", 0, 0)
        .expect_err("pending path work blocks replacement");
    assert_eq!(error.code, "catalog_scan_live_changes_pending");

    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'retry_wait', next_retry_unix_ms = 75,
                 last_failure_code = 'fixture_retry',
                 last_failure_message = 'Fixture retry debt'
             WHERE id = ?1",
            [change_id],
        )
        .expect("move path work to durable retry debt");
    catalog
        .publish_scan(&replacement.scan_id, "path-gate-root", 0, 0)
        .expect("retry-wait exact path does not hold the old snapshot hostage");

    let retained: (String, Option<String>, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, authoritative_scan_id, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained path retry debt");
    assert_eq!(retained, ("retry_wait".to_owned(), None, None));
}

#[test]
fn keyset_walk_returns_each_location_once_across_many_pages() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-many", "C:\\Many");
    catalog
        .begin_scan(&request, "root-many", "C:\\Many")
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove keyset fixture first-import handoff");
    let transaction = catalog
        .connection
        .transaction()
        .expect("fixture transaction");
    for index in 0..1_025 {
        let asset_id = format!("asset-{index:04}");
        let location_id = format!("location-{index:04}");
        let relative_path = format!("{index:04}.png");
        transaction
            .execute(
                "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
                [&asset_id],
            )
            .expect("fixture asset");
        transaction
            .execute(
                "INSERT INTO asset_locations(
                       scan_id, asset_id, location_id, root_id, absolute_path,
                       relative_path, preview_path, file_size, modified_unix_ms,
                       width, height, file_identity_scheme, file_identity_value,
                       source_generation
                     ) VALUES (
                       'scan-many', ?1, ?2, 'root-many', ?3, ?4, ?5, 20, 30, 40, 50,
                       'fixture-identity-v1', ?6, 1
                     )",
                params![
                    asset_id,
                    location_id,
                    format!("C:\\Many\\{relative_path}"),
                    relative_path,
                    format!("C:\\Cache\\{index:04}.jpg"),
                    format!("identity-{index:04}"),
                ],
            )
            .expect("fixture location");
    }
    transaction
        .execute("UPDATE catalog_state SET next_source_generation = 2", [])
        .expect("advance fixture source generation allocator");
    transaction.commit().expect("fixture commit");
    catalog
        .publish_scan("scan-many", "root-many", 1_025, 0)
        .expect("publish fixture");

    let mut cursor = None;
    let mut location_ids = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 128, cursor.as_ref()).expect("keyset page");
        location_ids.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    let unique_ids = location_ids.iter().collect::<HashSet<_>>();
    assert_eq!(location_ids.len(), 1_025);
    assert_eq!(unique_ids.len(), 1_025);
    assert_eq!(
        location_ids.first().map(String::as_str),
        Some("location-0000")
    );
    assert_eq!(
        location_ids.last().map(String::as_str),
        Some("location-1024")
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

#[test]
fn gallery_keyset_orders_capture_and_fallback_times_across_roots() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-1",
        "gallery-root-1",
        "C:\\One",
        &[
            ("known-old", Some("2024-01-02T03:04:05.000000000"), 100),
            ("known-tie-low", Some("2025-01-02T03:04:05.000000000"), 100),
            ("unknown-old", None, 100),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-2",
        "gallery-root-2",
        "C:\\Two",
        &[
            ("known-new", Some("2026-01-02T03:04:05.000000000"), 50),
            ("known-tie-high", Some("2025-01-02T03:04:05.000000000"), 200),
            ("unknown-new", None, 200),
        ],
    );

    let mut cursor = None;
    let mut locations = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 2, cursor.as_ref()).expect("gallery page");
        locations.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(
        locations,
        [
            "known-new",
            "known-tie-high",
            "known-tie-low",
            "known-old",
            "unknown-new",
            "unknown-old",
        ]
    );
}

#[test]
fn gallery_time_fallback_prefers_capture_then_creation_then_modification() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "fallback-scan",
        "fallback-root",
        "C:\\Pictures",
        &[
            (
                "created-fallback",
                "created.png",
                None,
                Some(1_749_988_800_000),
                1_784_116_800_000,
            ),
            (
                "modified-fallback",
                "modified.png",
                None,
                None,
                1_715_774_400_000,
            ),
            (
                "capture",
                "capture.png",
                Some("2023-04-15T12:00:00.000000000"),
                Some(1_784_116_800_000),
                1_784_116_800_000,
            ),
        ],
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("fallback snapshot");
    assert_eq!(
        snapshot
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["created-fallback", "modified-fallback", "capture"]
    );
    assert!(snapshot.assets[0].capture_time.is_none());
    assert!(snapshot.assets[1].capture_time.is_none());

    let timeline = load_default_timeline(&mut catalog).expect("fallback timeline");
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.month_key.as_deref())
            .collect::<Vec<_>>(),
        [Some("2025-06"), Some("2024-05"), Some("2023-04")]
    );
}

#[test]
fn gallery_query_combines_source_folder_search_and_all_sort_modes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-1",
        "query-root-1",
        "C:\\One",
        &[
            (
                "img10",
                "Album/img10.png",
                Some("2026-01-02T03:04:05.000000000"),
                Some(30),
                40,
            ),
            (
                "img2",
                "Album/Sub/img2.png",
                Some("2025-01-02T03:04:05.000000000"),
                Some(10),
                20,
            ),
            ("cat", "Other/Cat.png", None, None, 60),
        ],
    );
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-2",
        "query-root-2",
        "C:\\Two",
        &[(
            "img1-other",
            "Album/img1.png",
            Some("2024-01-02T03:04:05.000000000"),
            Some(5),
            5,
        )],
    );

    let source_query = GalleryQuery {
        root_id: Some("query-root-1".to_owned()),
        ..GalleryQuery::default()
    };
    let source_page = catalog
        .load_snapshot(10, &source_query, "source-query", None, None, None)
        .expect("source query");
    assert_eq!(
        source_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img10", "img2", "cat"]
    );

    let direct_folder_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        include_descendants: false,
        ..source_query.clone()
    };
    let direct_folder_page = catalog
        .load_snapshot(
            10,
            &direct_folder_query,
            "direct-folder-query",
            None,
            None,
            None,
        )
        .expect("direct folder query");
    assert_eq!(direct_folder_page.assets.len(), 1);
    assert_eq!(direct_folder_page.assets[0].location_id, "img10");

    let descendant_search_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        search_text: "IMG".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let descendant_search_page = catalog
        .load_snapshot(
            10,
            &descendant_search_query,
            "descendant-search-query",
            None,
            None,
            None,
        )
        .expect("descendant search query");
    assert_eq!(
        descendant_search_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10"]
    );

    let all_name_query = GalleryQuery {
        search_text: "img".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let first_name_page = catalog
        .load_snapshot(2, &all_name_query, "name-query", None, None, None)
        .expect("first name page");
    let second_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            first_name_page.next_cursor.as_ref(),
            None,
            None,
        )
        .expect("second name page");
    let name_locations = first_name_page
        .assets
        .iter()
        .chain(&second_name_page.assets)
        .map(|asset| asset.location_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(name_locations, ["img1-other", "img2", "img10"]);
    let previous_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            None,
            second_name_page.previous_cursor.as_ref(),
            None,
        )
        .expect("previous name page");
    assert_eq!(
        previous_name_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img1-other", "img2"]
    );
    assert!(previous_name_page.previous_cursor.is_none());

    let created_query = GalleryQuery {
        sort_key: GallerySortKey::CreatedTime,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let created_page = catalog
        .load_snapshot(10, &created_query, "created-query", None, None, None)
        .expect("created query");
    assert_eq!(
        created_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10", "cat"]
    );

    let modified_query = GalleryQuery {
        sort_key: GallerySortKey::ModifiedTime,
        sort_direction: GallerySortDirection::Descending,
        ..source_query
    };
    let modified_page = catalog
        .load_snapshot(10, &modified_query, "modified-query", None, None, None)
        .expect("modified query");
    assert_eq!(
        modified_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["cat", "img10", "img2"]
    );
}

#[test]
fn gallery_location_anchor_resolves_name_order_and_direction() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-scan",
        "anchor-root",
        "C:\\Pictures",
        &[
            ("img10", "img10.png", None, Some(10), 10),
            ("img2", "img2.png", None, Some(20), 20),
            ("img1", "img1.png", None, Some(30), 30),
            ("img3", "img3.png", None, Some(40), 40),
            ("img4", "img4.png", None, Some(50), 50),
        ],
    );
    let ascending = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_location(3, &ascending, "name-ascending", "img3")
        .expect("ascending location anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("img3"));
    assert_eq!(resolution.ordinal, Some(2));
    assert_eq!(resolution.window_start_ordinal, 1);
    assert_eq!(
        page.assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img3", "img4"]
    );

    let descending = GalleryQuery {
        sort_direction: GallerySortDirection::Descending,
        ..ascending
    };
    let page = catalog
        .load_snapshot_around_location(3, &descending, "name-descending", "img3")
        .expect("descending location anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.ordinal, Some(2));
    assert_eq!(resolution.window_start_ordinal, 1);
    assert_eq!(
        page.assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img4", "img3", "img2"]
    );
}

#[test]
fn gallery_asset_anchor_follows_a_renamed_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "asset-anchor-scan",
        "asset-anchor-root",
        "C:\\Pictures",
        &[
            ("before", "Album/before.png", None, Some(10), 10),
            ("neighbor", "Album/neighbor.png", None, Some(20), 20),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET location_id = 'after', relative_path = 'Album/after.png',
                 absolute_path = 'C:\\Pictures\\Album\\after.png'
             WHERE location_id = 'before'",
            [],
        )
        .expect("rename active location while retaining its asset identity");

    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "stable-asset-anchor",
            "before",
            "asset-asset-anchor-scan-before",
            0,
        )
        .expect("stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.requested_location_id, "before");
    assert_eq!(resolution.location_id.as_deref(), Some("after"));
    assert_eq!(resolution.ordinal, Some(0));
    assert_eq!(page.assets[0].asset_id, "asset-asset-anchor-scan-before");
    assert_eq!(page.assets[0].location_id, "after");
}

#[test]
fn gallery_asset_anchor_prefers_the_requested_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "multi-location-scan",
        "multi-location-root",
        "C:\\Pictures",
        &[
            ("first", "Album/first.png", None, Some(10), 10),
            ("second", "Album/second.png", None, Some(20), 20),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET asset_id = 'asset-multi-location-scan-first'
             WHERE location_id = 'second'",
            [],
        )
        .expect("attach second location to the same asset");
    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };

    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "multi-location-anchor",
            "second",
            "asset-multi-location-scan-first",
            1,
        )
        .expect("preferred stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("second"));
    assert_eq!(resolution.ordinal, Some(1));
    let preferred = catalog
        .load_active_location_by_asset_id("asset-multi-location-scan-first", Some("second"))
        .expect("load preferred location")
        .expect("preferred asset location");
    assert_eq!(preferred.location_id, "second");
}

#[test]
fn missing_asset_anchor_falls_back_to_the_nearest_surviving_ordinal() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "missing-anchor-scan",
        "missing-anchor-root",
        "C:\\Pictures",
        &[
            ("first", "Album/first.png", None, Some(10), 10),
            ("middle", "Album/middle.png", None, Some(20), 20),
            ("third", "Album/third.png", None, Some(30), 30),
        ],
    );
    catalog
        .connection
        .execute(
            "DELETE FROM asset_locations WHERE location_id = 'middle'",
            [],
        )
        .expect("remove anchor location");
    let query = GalleryQuery {
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };

    let page = catalog
        .load_snapshot_around_asset(
            3,
            &query,
            "missing-asset-anchor",
            "middle",
            "asset-missing-anchor-scan-middle",
            1,
        )
        .expect("fallback stable asset anchor");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.location_id.as_deref(), Some("third"));
    assert_eq!(resolution.ordinal, Some(1));
    assert_eq!(resolution.window_start_ordinal, 0);
}

#[test]
fn gallery_location_anchor_falls_back_when_filtered_out() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-filter-scan",
        "anchor-filter-root",
        "C:\\Pictures",
        &[
            ("match", "Album/match.png", None, Some(10), 10),
            ("excluded", "Other/excluded.png", None, Some(20), 20),
        ],
    );
    let query = GalleryQuery {
        root_id: Some("anchor-filter-root".to_owned()),
        folder_relative_path: Some("Album".to_owned()),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let page = catalog
        .load_snapshot_around_location(3, &query, "filtered", "excluded")
        .expect("filtered location fallback");
    let resolution = page.query_anchor_resolution.expect("resolution");
    assert_eq!(resolution.requested_location_id, "excluded");
    assert_eq!(resolution.location_id, None);
    assert_eq!(resolution.ordinal, None);
    assert_eq!(resolution.window_start_ordinal, 0);
    assert_eq!(page.assets[0].location_id, "match");
}

#[test]
fn gallery_location_anchor_covers_time_sort_boundaries_missing_values_and_ties() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-time-scan-a",
        "anchor-time-root-a",
        "C:\\TimeA",
        &[
            (
                "low",
                "same.png",
                Some("2024-01-01T00:00:01.000000000"),
                Some(1_000),
                1_000,
            ),
            (
                "tie-a",
                "same.png",
                Some("2024-01-01T00:00:02.000000000"),
                Some(2_000),
                2_000,
            ),
        ],
    );
    publish_gallery_query_fixture(
        &mut catalog,
        "anchor-time-scan-b",
        "anchor-time-root-b",
        "C:\\TimeB",
        &[
            (
                "tie-b",
                "same.png",
                Some("2024-01-01T00:00:02.000000000"),
                Some(2_000),
                2_000,
            ),
            (
                "high",
                "same.png",
                Some("2024-01-01T00:00:03.000000000"),
                Some(3_000),
                3_000,
            ),
            ("missing", "same.png", None, None, 4_000),
        ],
    );
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_local_time = NULL WHERE location_id = 'missing'",
            [],
        )
        .expect("clear derived file time for missing-value fixture");

    for (sort_key, ascending, descending) in [
        (
            GallerySortKey::CaptureTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["high", "tie-a", "tie-b", "low", "missing"],
        ),
        (
            GallerySortKey::CreatedTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["high", "tie-a", "tie-b", "low", "missing"],
        ),
        (
            GallerySortKey::ModifiedTime,
            ["low", "tie-a", "tie-b", "high", "missing"],
            ["missing", "high", "tie-a", "tie-b", "low"],
        ),
    ] {
        for (direction, expected) in [
            (GallerySortDirection::Ascending, ascending),
            (GallerySortDirection::Descending, descending),
        ] {
            let query = GalleryQuery {
                sort_key: sort_key.clone(),
                sort_direction: direction.clone(),
                ..GalleryQuery::default()
            };
            let query_id = format!("anchor-{sort_key:?}-{direction:?}");

            let tied = catalog
                .load_snapshot_around_location(3, &query, &query_id, "tie-b")
                .expect("resolve tied time anchor");
            let tied_resolution = tied.query_anchor_resolution.expect("tie resolution");
            let tied_ordinal = expected
                .iter()
                .position(|location_id| *location_id == "tie-b")
                .expect("tie fixture ordinal");
            let tied_window_start = tied_ordinal.saturating_sub(1);
            assert_eq!(tied_resolution.ordinal, Some(tied_ordinal as u64));
            assert_eq!(
                tied_resolution.window_start_ordinal,
                tied_window_start as u64
            );
            assert_eq!(
                tied.assets
                    .iter()
                    .map(|asset| asset.location_id.as_str())
                    .collect::<Vec<_>>(),
                expected[tied_window_start..tied_window_start + 3]
            );

            let first = catalog
                .load_snapshot_around_location(3, &query, &query_id, expected[0])
                .expect("resolve first time anchor");
            let first_resolution = first.query_anchor_resolution.expect("first resolution");
            assert_eq!(first_resolution.ordinal, Some(0));
            assert_eq!(first_resolution.window_start_ordinal, 0);
            assert_eq!(first.assets[0].location_id, expected[0]);

            let last = catalog
                .load_snapshot_around_location(3, &query, &query_id, expected[4])
                .expect("resolve last time anchor");
            let last_resolution = last.query_anchor_resolution.expect("last resolution");
            assert_eq!(last_resolution.ordinal, Some(4));
            assert_eq!(last_resolution.window_start_ordinal, 3);
            assert_eq!(
                last.assets.last().expect("last window item").location_id,
                expected[4]
            );
        }
    }
}

#[test]
fn folder_pages_are_bounded_scoped_and_revision_safe() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "folder-scan",
        "folder-root",
        "C:\\Pictures",
        &[
            ("album", "Album/one.png", None, Some(1), 1),
            ("nested", "Album/Sub/two.png", None, Some(2), 2),
            ("other", "Other/three.png", None, Some(3), 3),
        ],
    );

    let first = catalog
        .load_folder_page("folder-root", "", 1, None)
        .expect("first root folder page");
    assert_eq!(first.folders.len(), 1);
    assert_eq!(first.folders[0].relative_path, "Album");
    assert_eq!(first.folders[0].direct_asset_count, 1);
    assert_eq!(first.folders[0].descendant_asset_count, 2);
    let cursor = first.next_cursor.expect("folder cursor");

    let second = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect("second root folder page");
    assert_eq!(second.folders.len(), 1);
    assert_eq!(second.folders[0].relative_path, "Other");
    assert!(second.next_cursor.is_none());

    let nested = catalog
        .load_folder_page("folder-root", "Album", 10, None)
        .expect("nested folder page");
    assert_eq!(nested.folders.len(), 1);
    assert_eq!(nested.folders[0].relative_path, "Album/Sub");

    let traversal = catalog
        .load_folder_page("folder-root", "../Outside", 10, None)
        .expect_err("folder traversal must be rejected");
    assert_eq!(traversal.code, "catalog_source_scope_invalid");

    publish_fixture(
        &mut catalog,
        "other-scan",
        "other-root",
        "C:\\Other",
        "other-location",
    );
    let replacement = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect("published changes replace the folder window");
    assert_eq!(replacement.folders[0].relative_path, "Album");
    assert_eq!(
        replacement.disposition,
        crate::domain::LibraryFolderPageDisposition::Replace
    );
    assert!(replacement.revision > cursor.revision);
    let wrong_scope = catalog
        .load_folder_page("folder-root", "Album", 1, Some(&cursor))
        .expect_err("a cursor cannot cross folder scopes");
    assert_eq!(wrong_scope.code, "catalog_folder_cursor_stale");
}

#[test]
fn gallery_timeline_uses_file_time_when_capture_time_is_missing() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-1",
        "timeline-root-1",
        "C:\\One",
        &[
            ("one-2025-a", Some("2025-08-07T03:04:05.000000000"), 10),
            ("one-2025-b", Some("2025-08-01T03:04:05.000000000"), 20),
            ("one-unknown", None, 30),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-2",
        "timeline-root-2",
        "C:\\Two",
        &[
            ("two-2026", Some("2026-01-02T03:04:05.000000000"), 40),
            ("two-2024", Some("2024-12-31T03:04:05.000000000"), 50),
            ("two-unknown", None, 60),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.revision, 2);
    assert_eq!(timeline.total_items, 6);
    assert_eq!(
        timeline.buckets,
        [
            GalleryTimeBucket {
                month_key: Some("2026-01".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("2025-08".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
            GalleryTimeBucket {
                month_key: Some("2024-12".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("1970-01".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
        ]
    );

    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-3",
        "timeline-root-1",
        "C:\\One",
        &[("one-replacement", Some("2023-07-01T03:04:05.000000000"), 70)],
    );
    let rescanned = load_default_timeline(&mut catalog).expect("timeline after rescan");
    assert_eq!(rescanned.revision, 3);
    assert_eq!(rescanned.total_items, 4);
    assert!(
        rescanned
            .buckets
            .iter()
            .all(|bucket| { bucket.month_key.as_deref() != Some("2025-08") })
    );
}

#[test]
fn gallery_timeline_bounds_aspect_ratio_weight_and_falls_back_for_missing_dimensions() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "timeline-dimensions",
        "timeline-dimensions-root",
        "C:\\Dimensions",
        &[
            (
                "panorama",
                Some("2026-01-02T03:04:05.000000000"),
                10,
                1000,
                100,
            ),
            (
                "portrait",
                Some("2026-01-02T03:04:05.000000000"),
                20,
                100,
                1000,
            ),
            (
                "square",
                Some("2026-01-02T03:04:05.000000000"),
                30,
                500,
                500,
            ),
            ("missing", Some("2026-01-02T03:04:05.000000000"), 40, 0, 0),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.buckets.len(), 1);
    assert_eq!(timeline.buckets[0].item_count, 4);
    assert_eq!(timeline.buckets[0].aspect_ratio_milli_sum, 7_200);
}

#[test]
fn layout_manifest_chunks_preserve_order_ordinals_and_final_geometry_evidence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "layout-manifest-scan",
        "layout-manifest-root",
        "C:\\Layout",
        &[
            ("wide", Some("2026-08-09T12:00:00"), 4_000, 200, 100),
            (
                "unknown-dimensions",
                Some("2026-08-09T11:00:00"),
                3_000,
                0,
                0,
            ),
            (
                "extreme-wide",
                Some("2026-08-08T12:00:00"),
                2_000,
                1_000,
                100,
            ),
            ("extreme-tall", Some("2026-08-07T12:00:00"), 1_000, 10, 100),
        ],
    );

    let first = load_default_layout_manifest_chunk(&mut catalog, 2, None)
        .expect("first layout manifest chunk");
    assert_eq!(first.start_ordinal, 0);
    assert_eq!(first.total_items, 4);
    assert_eq!(first.location_ids, ["wide", "unknown-dimensions"]);
    assert_eq!(first.aspect_ratio_milli, [2_000, 1_000]);
    assert_eq!(first.flags, [LAYOUT_FLAG_DIMENSIONS_KNOWN, 0]);
    assert_eq!(first.date_group_indices, [0, 0]);
    assert_eq!(
        first.date_groups,
        [GalleryLayoutDateGroup {
            date_key: Some("2026-08-09".to_owned()),
        }]
    );

    let cursor = first.next_cursor.expect("next layout cursor");
    assert_eq!(cursor.next_ordinal, 2);
    assert_eq!(cursor.total_items, 4);
    let second = load_default_layout_manifest_chunk(&mut catalog, 2, Some(&cursor))
        .expect("second layout manifest chunk");
    assert_eq!(second.start_ordinal, 2);
    assert_eq!(second.total_items, 4);
    assert_eq!(second.location_ids, ["extreme-wide", "extreme-tall"]);
    assert_eq!(second.aspect_ratio_milli, [5_000, 200]);
    assert_eq!(second.flags, [1, 1]);
    assert_eq!(second.date_group_indices, [0, 1]);
    assert_eq!(
        second.date_groups,
        [
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-08".to_owned()),
            },
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-07".to_owned()),
            },
        ]
    );
    assert!(second.next_cursor.is_none());
}

#[test]
fn layout_manifest_rejects_invalid_limits_and_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-1",
        "layout-stale-root-1",
        "C:\\One",
        &[("first", Some("2026-08-09T12:00:00"), 1_000)],
    );

    let zero_limit = load_default_layout_manifest_chunk(&mut catalog, 0, None)
        .expect_err("zero layout chunk limit must fail");
    assert_eq!(zero_limit.code, "catalog_layout_chunk_limit_invalid");
    let excessive_limit =
        load_default_layout_manifest_chunk(&mut catalog, MAX_LAYOUT_MANIFEST_CHUNK_ITEMS + 1, None)
            .expect_err("excessive layout chunk limit must fail");
    assert_eq!(excessive_limit.code, "catalog_layout_chunk_limit_invalid");

    let first = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("first layout manifest chunk");
    let cursor = first.next_cursor;
    assert!(cursor.is_none(), "one item does not produce a next cursor");

    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-2",
        "layout-stale-root-2",
        "C:\\Two",
        &[("second", Some("2026-08-08T12:00:00"), 900)],
    );
    let page = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("layout page with a cursor");
    let cursor = page.next_cursor.expect("next layout cursor");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-3",
        "layout-stale-root-3",
        "C:\\Three",
        &[("third", Some("2026-08-07T12:00:00"), 800)],
    );

    let stale = load_default_layout_manifest_chunk(&mut catalog, 1, Some(&cursor))
        .expect_err("stale layout cursor must fail");
    assert_eq!(stale.code, "catalog_layout_cursor_stale");
}

#[test]
fn unregistering_a_root_removes_only_catalog_state_and_preserves_source_bytes() {
    let directory = tempdir().expect("temporary directory");
    let source_directory = directory.path().join("source");
    std::fs::create_dir(&source_directory).expect("source directory");
    let source_file = source_directory.join("one.png");
    let source_bytes = b"irreplaceable source bytes";
    std::fs::write(&source_file, source_bytes).expect("source fixture");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_path = source_directory.to_string_lossy();
    publish_gallery_fixture(
        &mut catalog,
        "remove-scan",
        "remove-root",
        &root_path,
        &[("one", Some("2026-01-02T03:04:05.000000000"), 10)],
    );
    let revision_before = load_default_snapshot(&mut catalog, 10, None)
        .expect("snapshot before removal")
        .revision;

    assert!(
        catalog
            .unregister_root("remove-root")
            .expect("unregister root")
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot after removal");
    assert!(snapshot.roots.is_empty());
    assert!(snapshot.assets.is_empty());
    assert_eq!(snapshot.revision, revision_before + 1);
    assert_eq!(
        load_default_timeline(&mut catalog)
            .expect("timeline after removal")
            .total_items,
        0
    );
    assert_eq!(
        std::fs::read(&source_file).expect("source remains readable"),
        source_bytes
    );
    assert!(
        !catalog
            .unregister_root("remove-root")
            .expect("second unregister is idempotent")
    );
}

#[test]
fn gallery_anchor_cursors_begin_at_capture_or_fallback_month_without_gaps() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "anchor-scan",
        "anchor-root",
        "C:\\Pictures",
        &[
            ("newer", Some("2026-02-01T00:00:00.000000000"), 50),
            ("selected-new", Some("2025-08-20T00:00:00.000000000"), 40),
            ("selected-old", Some("2025-08-01T00:00:00.000000000"), 30),
            ("older", Some("2024-01-01T00:00:00.000000000"), 20),
            ("unknown", None, 10),
        ],
    );
    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    let month_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("2025-08".to_owned()),
        item_offset: 0,
    };
    let month_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_anchor),
        )
        .expect("month page");
    assert_eq!(
        month_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-new", "selected-old", "older"]
    );
    assert!(month_page.next_cursor.is_some());
    assert!(month_page.previous_cursor.is_some());

    let month_middle = GalleryTimeAnchor {
        item_offset: 1,
        ..month_anchor.clone()
    };
    let middle_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_middle),
        )
        .expect("month middle page");
    assert_eq!(
        middle_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-old", "older", "unknown"]
    );
    let first_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            middle_page.previous_cursor.as_ref(),
            None,
        )
        .expect("first previous page");
    assert_eq!(first_previous_page.assets[0].location_id, "selected-new");
    assert!(first_previous_page.previous_cursor.is_some());
    assert!(first_previous_page.next_cursor.is_some());

    let oldest_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            first_previous_page.previous_cursor.as_ref(),
            None,
        )
        .expect("oldest previous page");
    assert_eq!(oldest_previous_page.assets[0].location_id, "newer");
    assert!(oldest_previous_page.previous_cursor.is_none());
    assert!(oldest_previous_page.next_cursor.is_some());

    let fallback_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("1970-01".to_owned()),
        item_offset: 0,
    };
    let fallback_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&fallback_anchor),
        )
        .expect("fallback page");
    assert_eq!(fallback_page.assets.len(), 1);
    assert_eq!(fallback_page.assets[0].location_id, "unknown");
    assert!(fallback_page.next_cursor.is_none());
}

fn publish_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    location_id: &str,
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(scan_id)
        .expect("prove fixture first-import handoff");
    catalog
        .stage_location(
            scan_id,
            root_id,
            &AssetLocationView {
                asset_id: format!("asset-{scan_id}"),
                location_id: location_id.to_owned(),
                root_id: root_id.to_owned(),
                scan_id: scan_id.to_owned(),
                absolute_path: format!("{root_path}\\one.png"),
                display_path: format!("{root_path}\\one.png"),
                relative_path: "one.png".to_owned(),
                preview_path: format!("C:\\Cache\\{scan_id}.jpg"),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 1,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Ready,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("stage location");
    catalog
        .publish_scan(scan_id, root_id, 1, 0)
        .expect("publish scan");
}

fn insert_active_identity_alias(
    catalog: &mut SqliteCatalog,
    original_location_id: &str,
    alias_location_id: &str,
) {
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET file_identity_scheme = 'windows-file-id-128-v1',
                 file_identity_value =
                   '0000000000000001:00000000000000000000000000000001',
                 source_revision_token = NULL
             WHERE location_id = ?1",
            [original_location_id],
        )
        .expect("assign original identity");
    let alias_asset_id = format!("asset-{alias_location_id}");
    catalog
        .connection
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
            [&alias_asset_id],
        )
        .expect("insert alias asset");
    catalog
        .connection
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value, source_revision_token,
               source_generation
             )
             SELECT scan_id, ?2, ?3, root_id, absolute_path, 'alias.png',
                    '', file_size, created_unix_ms, modified_unix_ms,
                    file_local_time, '', 'alias.png', width, height,
                    'pending', NULL, NULL, metadata_engine_id, metadata_engine_version,
                    capture_local_time, capture_offset_minutes, capture_time_source,
                    capture_raw_value, file_identity_scheme, file_identity_value, NULL,
                    source_generation
             FROM asset_locations WHERE location_id = ?1",
            params![original_location_id, alias_asset_id, alias_location_id],
        )
        .expect("insert identity alias");
}

fn publish_preview_artifact(
    catalog: &mut SqliteCatalog,
    location_id: &str,
    artifact_key: &str,
    artifact_path: &str,
) -> PreviewArtifact {
    let mut location = catalog
        .load_active_location(location_id)
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    if location.source_revision.is_none() {
        location.source_revision = Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0000000000000001".to_owned(),
        });
    }
    location.preview_path = artifact_path.to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: artifact_key.to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), Some(&request))
        .expect("publish preview artifact");
    artifact
}

fn exact_preview_request(location: &AssetLocationView) -> PreviewRequest {
    PreviewRequest {
        location_id: location.location_id.clone(),
        expected_root_id: location.root_id.clone(),
        expected_scan_id: location.scan_id.clone(),
        expected_source_revision: location.source_revision.clone(),
        expected_source_generation: location.source_generation,
        preview_edge: 256,
        retry_failed: false,
        protected_location_ids: Vec::new(),
    }
}

fn publish_empty_replacement_scan(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin empty replacement scan");
    catalog
        .publish_scan(scan_id, root_id, 0, 0)
        .expect("publish empty replacement scan");
}

fn ensure_legacy_scan_runs_contract(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS scan_runs (
               id TEXT PRIMARY KEY,
               root_id TEXT NOT NULL,
               status TEXT NOT NULL,
               started_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               asset_count INTEGER NOT NULL DEFAULT 0,
               issue_count INTEGER NOT NULL DEFAULT 0,
               max_items INTEGER,
               max_entries INTEGER,
               preview_edge INTEGER NOT NULL,
               current_directory_relative_path TEXT,
               current_directory_enumerated INTEGER NOT NULL DEFAULT 0,
               last_visited_relative_path TEXT,
               visited_entries INTEGER NOT NULL DEFAULT 0,
               accepted_items INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT (SELECT COUNT(*) FROM scan_runs WHERE status = 'completed')
               WHERE NOT EXISTS (SELECT 1 FROM catalog_state);",
        )
        .expect("legacy scan run contract");
    ensure_legacy_asset_location_contract(connection);
}

fn ensure_legacy_asset_location_contract(connection: &Connection) {
    let version = connection
        .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("legacy asset location schema version");
    let columns = [
        (1, "absolute_path", "TEXT NOT NULL DEFAULT ''"),
        (1, "preview_path", "TEXT NOT NULL DEFAULT ''"),
        (1, "file_size", "INTEGER NOT NULL DEFAULT 0"),
        (1, "modified_unix_ms", "INTEGER NOT NULL DEFAULT 0"),
        (1, "width", "INTEGER NOT NULL DEFAULT 0"),
        (1, "height", "INTEGER NOT NULL DEFAULT 0"),
        (2, "asset_id", "TEXT NOT NULL DEFAULT ''"),
        (7, "preview_status", "TEXT NOT NULL DEFAULT 'pending'"),
        (7, "preview_issue_code", "TEXT"),
        (7, "preview_issue_message", "TEXT"),
        (8, "metadata_engine_id", "TEXT NOT NULL DEFAULT 'unknown'"),
        (8, "metadata_engine_version", "TEXT NOT NULL DEFAULT '0'"),
        (8, "capture_local_time", "TEXT"),
        (8, "capture_offset_minutes", "INTEGER"),
        (8, "capture_time_source", "TEXT"),
        (8, "capture_raw_value", "TEXT"),
        (9, "file_identity_scheme", "TEXT"),
        (9, "file_identity_value", "TEXT"),
        (12, "created_unix_ms", "INTEGER"),
        (12, "parent_relative_path", "TEXT NOT NULL DEFAULT ''"),
        (12, "natural_name_key", "TEXT NOT NULL DEFAULT ''"),
        (13, "file_local_time", "TEXT"),
    ];
    for (introduced_in, name, definition) in columns {
        if version < introduced_in {
            continue;
        }
        let exists = connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM pragma_table_info('asset_locations') WHERE name = ?1
                 )",
                [name],
                |row| row.get::<_, bool>(0),
            )
            .expect("legacy asset location column query");
        if !exists {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE asset_locations ADD COLUMN {name} {definition}"
                ))
                .expect("complete legacy asset location fixture");
        }
    }
}

fn preview_reference_count(catalog: &SqliteCatalog, artifact_key: &str) -> i64 {
    catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview reference count")
}

fn preview_lifecycle_state(catalog: &SqliteCatalog, artifact_key: &str) -> String {
    catalog
        .connection
        .query_row(
            "SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview lifecycle state")
}

fn publish_gallery_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64)],
) {
    let dimensioned_locations = locations
        .iter()
        .map(|(location_id, capture_local_time, modified_unix_ms)| {
            (*location_id, *capture_local_time, *modified_unix_ms, 40, 50)
        })
        .collect::<Vec<_>>();
    publish_gallery_dimension_fixture(catalog, scan_id, root_id, root_path, &dimensioned_locations);
}

fn publish_gallery_query_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[GalleryQueryFixture<'_>],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin query fixture scan");
    prove_gallery_first_import_handoff_if_needed(catalog, scan_id, root_id);
    for (location_id, relative_path, capture_local_time, created_unix_ms, modified_unix_ms) in
        locations
    {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    scan_id: scan_id.to_owned(),
                    absolute_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    display_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    relative_path: (*relative_path).to_owned(),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: *created_unix_ms,
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: 40,
                    height: 50,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage query fixture location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("query fixture location count"),
            0,
        )
        .expect("publish query fixture scan");
}

fn publish_gallery_dimension_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64, u32, u32)],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin gallery scan");
    prove_gallery_first_import_handoff_if_needed(catalog, scan_id, root_id);
    for (location_id, capture_local_time, modified_unix_ms, width, height) in locations {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    scan_id: scan_id.to_owned(),
                    absolute_path: format!("{root_path}\\{location_id}.png"),
                    display_path: format!("{root_path}\\{location_id}.png"),
                    relative_path: format!("{location_id}.png"),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: Some(*modified_unix_ms - 1),
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: *width,
                    height: *height,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage gallery location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("gallery location count"),
            0,
        )
        .expect("publish gallery scan");
}

fn prove_gallery_first_import_handoff_if_needed(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
) {
    let is_first_import = catalog
        .connection
        .query_row(
            "SELECT active_scan_id IS NULL FROM library_roots WHERE id = ?1",
            [root_id],
            |row| row.get::<_, bool>(0),
        )
        .expect("load gallery fixture publication state");
    if is_first_import {
        catalog
            .prove_live_only_first_import_handoff_for_test(scan_id)
            .expect("prove gallery fixture first-import handoff");
    }
}

fn fixture_request(scan_id: &str, root_path: &str) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_path.to_owned(),
        max_items: Some(500),
        max_entries: Some(2_000),
        preview_edge: 512,
    }
}

fn publication_identity(hex_digit: char) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("000000000000004d:{}", hex_digit.to_string().repeat(32)),
    }
}

fn seed_current_explicit_recovery_claim(
    catalog: &SqliteCatalog,
    root_id: &str,
    observed_unix_ms: i64,
) {
    let catalog_revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("catalog revision");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               ?2, ?2, ?3, ?3, 1, 'retry_wait', ?2, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The ambiguous historical gap requires an explicit library update',
               ?4, ?2, ?2
             )",
            params![
                root_id,
                observed_unix_ms,
                observed_unix_ms.to_string(),
                catalog_revision,
            ],
        )
        .expect("insert explicit recovery gap");
    let gap_change_id = catalog.connection.last_insert_rowid();
    catalog
        .connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               created_unix_ms
             ) VALUES (?1, ?2, 1, 'explicit_recovery_required', ?3)",
            params![gap_change_id, root_id, observed_unix_ms],
        )
        .expect("insert explicit recovery claim");
}

fn downgrade_live_gap_contract_to_v29(connection: &Connection) {
    downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER library_live_gap_recovery_claim_insert_guard;
             DROP INDEX library_live_gap_recovery_claims_root;
             DROP TABLE library_live_gap_recovery_claims;
             DROP TABLE library_live_gap_recovery_contract;
             PRAGMA user_version = 29;
             UPDATE schema_info SET version = 29;",
        )
        .expect("downgrade fixture to v29");
}

pub(super) fn migrate_v29_ambiguous_gap_fixture(
    catalog_path: &Path,
    root_id: &str,
    root_path: &str,
    with_publication_namespace: bool,
) {
    let mut connection = Connection::open(catalog_path).expect("open ambiguous v29 fixture");
    super::migrations::migrate_schema(&mut connection).expect("create current fixture schema");
    connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
            params![root_id, root_path],
        )
        .expect("insert ambiguous fixture root");
    connection
        .execute(
            "INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES (?1, 1, 1, 1)",
            [root_id],
        )
        .expect("insert ambiguous fixture generation");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (?1, 1, 0, 1, 'unknown', 'baseline_required', 1)",
            [root_id],
        )
        .expect("insert ambiguous fixture journal state");
    if with_publication_namespace {
        connection
            .execute(
                "INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value,
                   authority_kind, established_catalog_revision,
                   established_unix_ms, updated_unix_ms
                 ) VALUES (
                   ?1, 1, 'windows-file-id-128-v1',
                   '000000000000004d:01010101010101010101010101010101',
                   'foreground_scan', 0, 1, 1
                 )",
                [root_id],
            )
            .expect("insert ambiguous fixture publication namespace");
    }
    connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, lease_expires_unix_ms, attempt_count,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               41, 41, '1', '1', 1, 'leased', 41, 99, 3, 0, 41, 41
             )",
            [root_id],
        )
        .expect("insert ambiguous v29 gap");
    downgrade_live_gap_contract_to_v29(&connection);
}

fn migrate_v29_foreground_owned_gap_fixture(
    catalog_path: &Path,
    root_id: &str,
    root_path: &str,
    scan_id: &str,
    is_paused: bool,
) -> ScanRequest {
    let mut catalog = SqliteCatalog::open(catalog_path.to_path_buf()).expect("catalog");
    let identity = publication_identity('f');
    let initial = fixture_request(&format!("{scan_id}-initial"), root_path);
    catalog
        .begin_scan_with_publication_namespace(&initial, root_id, root_path, &identity)
        .expect("begin initial publication");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove v29 fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial catalog");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               41, 41, '41', '41', 1, 'pending', 41, 1, 41, 41
             )",
            [root_id],
        )
        .expect("insert v29 foreground-owned gap");
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan_with_publication_namespace(&request, root_id, root_path, &identity)
        .expect("begin foreground owner");
    if is_paused {
        catalog
            .pause_scan(&request.scan_id, &ScanCheckpoint::default())
            .expect("pause foreground owner");
    }
    let owned: (String, String, String, i64) = catalog
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.authoritative_scan_id,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id)
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.authoritative_scan_id = scans.id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("v29 foreground ownership evidence");
    assert_eq!(owned.0, if is_paused { "paused" } else { "running" });
    assert_eq!(owned.1, "foreground");
    assert_eq!(owned.2, request.scan_id);
    assert_eq!(owned.3, 0);
    downgrade_live_gap_contract_to_v29(&catalog.connection);
    drop(catalog);
    request
}

fn assert_v29_foreground_owned_gap_resumes_after_migration(is_paused: bool) {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = if is_paused {
        "v29-paused-foreground-root"
    } else {
        "v29-running-foreground-root"
    };
    let root_path = if is_paused {
        "C:\\V29PausedForeground"
    } else {
        "C:\\V29RunningForeground"
    };
    let scan_id = if is_paused {
        "v29-paused-foreground-scan"
    } else {
        "v29-running-foreground-scan"
    };
    let request = migrate_v29_foreground_owned_gap_fixture(
        &catalog_path,
        root_id,
        root_path,
        scan_id,
        is_paused,
    );

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate owned v29 gap");
    let migrated: (String, String, String, i64, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.status,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id),
                    gap.authoritative_scan_id
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.authoritative_scan_id = scans.id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("migrated foreground ownership evidence");
    assert_eq!(
        migrated,
        (
            if is_paused { "paused" } else { "running" }.to_owned(),
            "foreground".to_owned(),
            "leased".to_owned(),
            0,
            Some(request.scan_id.clone()),
        )
    );
    catalog
        .resume_scan(&request, root_id, root_path)
        .expect("resume migrated foreground scan");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish migrated foreground scan");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("reopen published v30 catalog");
    let published: (String, String, String, i64, String) = reopened
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.status,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id),
                    roots.active_scan_id
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.root_id = scans.root_id
              AND gap.root_generation = scans.root_generation_at_start
              AND gap.id <= scans.change_queue_high_watermark
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("published migrated foreground evidence");
    assert_eq!(
        published,
        (
            "completed".to_owned(),
            "foreground".to_owned(),
            "completed".to_owned(),
            0,
            request.scan_id,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load synchronized queue metrics");
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.explicit_recovery_required_count, 0);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Idle);
}

#[test]
fn v29_running_foreground_scan_retains_owned_gap_through_resume_and_publication() {
    assert_v29_foreground_owned_gap_resumes_after_migration(false);
}

#[test]
fn v29_paused_foreground_scan_retains_owned_gap_through_resume_and_publication() {
    assert_v29_foreground_owned_gap_resumes_after_migration(true);
}

fn seed_pending_journal_claim(catalog: &SqliteCatalog, root_id: &str) {
    catalog
        .connection
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET protocol_version = 5, capability_state = 'supported',
                 continuity_state = 'current', last_failure_code = NULL,
                 last_failure_message = NULL, updated_unix_ms = 50
             WHERE root_id = ?1 AND root_generation = 1",
            [root_id],
        )
        .expect("make pending-journal root current");
    catalog
        .connection
        .execute(
            "INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state,
               updated_unix_ms
             ) VALUES (
               ?1, 1, 'volume-guid', '77', 3, ?2, '44', '55', '55', 1,
               5, 1, 'current', 50
             )",
            params![root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal checkpoint");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'live_notification',
               50, 50, '2', '2', 1, 'retry_wait', 50, 0, 60,
               'live_gap_waiting_for_journal_range',
               'The P0 live gap is waiting for its durable journal range',
               1, 50, 50
             )",
            [root_id],
        )
        .expect("insert pending-journal gap");
    let gap_change_id = catalog.connection.last_insert_rowid();
    catalog
        .connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               opening_volume_guid, opening_volume_serial,
               opening_root_reference_version, opening_root_file_reference,
               opening_journal_id, opening_next_usn, protocol_version,
               contract_version, created_unix_ms
             ) VALUES (
               ?1, ?2, 1, 'pending_journal', 'volume-guid', '77', 3, ?3,
               '44', '55', 5, 1, 50
             )",
            params![gap_change_id, root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal claim");
}

fn interrupted_explicit_foreground_with_handoff_assets(
    catalog_path: &Path,
) -> (String, String, ScanRequest) {
    let root_id = "crash-cleanup-root".to_owned();
    let root_path = "C:\\CrashCleanupRoot".to_owned();
    migrate_v29_ambiguous_gap_fixture(catalog_path, &root_id, &root_path, false);
    let mut catalog =
        SqliteCatalog::open(catalog_path.to_path_buf()).expect("migrate explicit crash fixture");
    let request = fixture_request("crash-cleanup-scan", &root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            &root_id,
            &root_path,
            &publication_identity('9'),
        )
        .expect("begin interrupted explicit foreground scan");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               catch_up_source, catch_up_watermark, created_unix_ms, updated_unix_ms
             ) VALUES (
               8100, 'crash-cleanup-root', 1, 'reconcile', 'path',
               'handoff-owner.jpg', 'startup_catch_up', 50, 50, '50', '50', 1,
               'pending', 50, 0, 'crash-cleanup-source',
               'crash-cleanup-watermark', 50, 50
             );
             INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES (
               8100, 'crash-cleanup-source', 'crash-cleanup-watermark', 50
             );
             INSERT INTO assets(id, created_unix_ms) VALUES
               ('crash-catch-up-asset', 50),
               ('crash-scan-handoff-asset', 50),
               ('crash-true-orphan', 50);
             INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, metadata_engine_id,
               metadata_engine_version, updated_unix_ms
             ) VALUES (
               'crash-cleanup-source', 'crash-cleanup-watermark',
               'windows-file-id-128-v1', 'crash-catch-up-identity',
               'crash-catch-up-asset', 'crash-catch-up-location',
               'crash-cleanup-root', 'C:/CrashCleanupRoot/catch-up.jpg',
               'catch-up.jpg', '', 1, 50, 50, 1, 1, 'pending',
               'fixture-metadata', '1', 50
             );
             INSERT INTO library_change_scan_handoff_batches(
               id, source_root_id, updated_unix_ms
             ) VALUES ('crash-cleanup-batch', 'crash-cleanup-root', 50);
             INSERT INTO library_change_scan_handoff_lineage(
               batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES (
               'crash-cleanup-batch', 'crash-cleanup-source',
               'crash-cleanup-watermark', 50
             );
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, metadata_engine_id,
               metadata_engine_version
             ) VALUES (
               'crash-cleanup-batch', 'windows-file-id-128-v1',
               'crash-scan-handoff-identity', 'crash-scan-handoff-asset',
               'crash-scan-handoff-location', 'crash-cleanup-root',
               'C:/CrashCleanupRoot/scan-handoff.jpg', 'scan-handoff.jpg',
               '', 1, 50, 50, 1, 1, 'pending', 'fixture-metadata', '1'
             );",
        )
        .expect("seed handoff-owned and orphan assets");
    drop(catalog);
    (root_id, root_path, request)
}

fn crash_cleanup_asset_projection(connection: &Connection) -> (i64, i64, i64, i64) {
    connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets WHERE id = 'crash-catch-up-asset'),
               (SELECT COUNT(*) FROM assets WHERE id = 'crash-scan-handoff-asset'),
               (SELECT COUNT(*) FROM assets WHERE id = 'crash-true-orphan'),
               (SELECT COUNT(*)
                FROM library_change_catch_up_handoffs AS handoffs
                LEFT JOIN assets ON assets.id = handoffs.asset_id
                WHERE assets.id IS NULL)
               + (SELECT COUNT(*)
                  FROM library_change_scan_handoff_items AS handoffs
                  LEFT JOIN assets ON assets.id = handoffs.asset_id
                  WHERE assets.id IS NULL)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("crash cleanup asset projection")
}

#[test]
fn explicit_recovery_claim_is_consumed_only_by_its_foreground_scan_publication() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-publication-root";
    let root_path = "C:\\ExplicitPublication";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("explicit-publication-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('a'),
        )
        .expect("begin explicit foreground recovery");
    let active: (String, String, String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.next_retry_unix_ms
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("active explicit foreground claim");
    assert_eq!(
        active,
        (
            "foreground_scan".to_owned(),
            request.scan_id.clone(),
            "leased".to_owned(),
            None,
        )
    );

    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove explicit recovery first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish explicit foreground recovery");
    drop(catalog);
    let reopened = SqliteCatalog::open(catalog_path).expect("reopen published explicit recovery");
    let consumed: (String, String, i64, String, Option<String>, i64) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id,
                    claim.consumed_unix_ms IS NOT NULL, gap.status,
                    gap.last_failure_code, gap.catalog_revision_at_success
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("consumed explicit foreground claim");
    assert_eq!(
        consumed,
        (
            "foreground_scan".to_owned(),
            request.scan_id,
            1,
            "completed".to_owned(),
            None,
            1,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load consumed explicit recovery claim metrics");
    assert_eq!(metrics.explicit_recovery_required_count, 0);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Idle);
}

#[test]
fn abandoned_explicit_recovery_scan_restores_the_original_typed_claim() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-abandon-root";
    let root_path = "C:\\ExplicitAbandon";
    let mut catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("published recovery catalog");
    let baseline_request = fixture_request("explicit-abandon-baseline", root_path);
    catalog
        .begin_scan(&baseline_request, root_id, root_path)
        .expect("begin published recovery baseline");
    catalog
        .prove_live_only_first_import_handoff_for_test(&baseline_request.scan_id)
        .expect("prove recovery baseline handoff");
    catalog
        .stage_location(
            &baseline_request.scan_id,
            root_id,
            &AssetLocationView {
                asset_id: "explicit-abandon-asset".to_owned(),
                location_id: "explicit-abandon-location".to_owned(),
                root_id: root_id.to_owned(),
                scan_id: baseline_request.scan_id.clone(),
                absolute_path: format!("{root_path}\\one.png"),
                display_path: format!("{root_path}\\one.png"),
                relative_path: "one.png".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 0,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("stage recovery baseline with allocated source generation");
    catalog
        .publish_scan(&baseline_request.scan_id, root_id, 1, 0)
        .expect("publish recovery baseline");
    drop(catalog);
    let mut catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("reopen valid v31 recovery baseline");
    seed_current_explicit_recovery_claim(&catalog, root_id, 41);
    let request = fixture_request("explicit-abandon-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('b'),
        )
        .expect("begin explicit foreground recovery");
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("abandon explicit foreground recovery");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("reopen abandoned explicit recovery");
    let baseline: (String, i64, i64, i64) = reopened
        .connection
        .query_row(
            "SELECT root.active_scan_id, state.is_active, catalog.revision,
                (SELECT COUNT(*) FROM asset_locations AS location
                 WHERE location.root_id = root.id AND location.scan_id = root.active_scan_id)
         FROM library_roots AS root
         JOIN library_change_root_state AS state ON state.root_id = root.id
         CROSS JOIN catalog_state AS catalog WHERE root.id = ?1",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("published baseline survives abandoned replacement");
    assert_eq!(baseline, ("explicit-abandon-baseline".to_owned(), 1, 1, 1));
    let restored: (
        String,
        Option<String>,
        String,
        Option<i64>,
        String,
        Option<String>,
    ) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.next_retry_unix_ms, gap.last_failure_code,
                    gap.authoritative_scan_id
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("restored explicit claim");
    assert_eq!(
        restored,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            None,
            "live_gap_v30_explicit_recovery_required".to_owned(),
            None,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load explicit recovery claim metrics");
    assert_eq!(metrics.explicit_recovery_required_count, 1);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Degraded);
}

#[test]
fn reopening_an_interrupted_explicit_recovery_scan_restores_the_claim_atomically() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-crash-root";
    let root_path = "C:\\ExplicitCrash";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("explicit-crash-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('c'),
        )
        .expect("begin explicit foreground recovery");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("recover interrupted explicit scan");
    let restored: (String, Option<String>, String, String) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status, scans.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = ?2
             WHERE claim.root_id = ?1",
            params![root_id, request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("recovered interrupted explicit claim");
    assert_eq!(
        restored,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            "failed".to_owned(),
        )
    );
}

#[test]
fn foreground_scan_does_not_capture_a_pending_journal_claim() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "pending-journal-scan-root";
    let root_path = "C:\\PendingJournalScan";
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    let initial = fixture_request("pending-journal-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove pending-journal fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    seed_pending_journal_claim(&catalog, root_id);

    let foreground = fixture_request("pending-journal-foreground", root_path);
    catalog
        .begin_scan(&foreground, root_id, root_path)
        .expect("begin unrelated foreground scan");
    drop(catalog);
    let mut reopened = SqliteCatalog::open(catalog_path).expect("reopen pending-journal scan");
    let retained: (String, String, Option<String>, Option<i64>) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, gap.status, gap.authoritative_scan_id,
                    scans.change_queue_high_watermark
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = ?2
             WHERE claim.root_id = ?1",
            params![root_id, foreground.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("retained pending-journal owner");
    assert_eq!(
        retained,
        (
            "pending_journal".to_owned(),
            "retry_wait".to_owned(),
            None,
            None,
        )
    );
    reopened
        .abandon_scan(&foreground.scan_id, "cancelled", 0)
        .expect("clean up foreground scan");
}

#[test]
fn current_v30_rejects_an_inexact_explicit_recovery_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-explicit-claim-root";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, "C:\\InvalidExplicitClaim", false);
    let catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'pending'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt explicit claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact explicit claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_an_inexact_active_foreground_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-active-claim-root";
    let root_path = "C:\\InvalidActiveClaim";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("invalid-active-claim-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('d'),
        )
        .expect("begin foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = 'wrong_foreground_state',
                 last_failure_message = 'wrong foreground state'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt active foreground claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact active foreground claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_an_active_foreground_claim_bound_to_authoritative_recovery() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-active-owner-root";
    let root_path = "C:\\Private\\InvalidActiveOwner";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("explicit recovery catalog");
    let request = fixture_request("invalid-active-owner-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('6'),
        )
        .expect("begin foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET scan_owner = 'authoritative_recovery' WHERE id = ?1",
            [&request.scan_id],
        )
        .expect("bind claim to wrong active owner");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(opened) => {
            drop(opened);
            None
        }
        Err(error) => Some(error),
    };
    let connection = Connection::open(catalog_path).expect("inspect rejected active owner");
    let retained: (
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
    ) = connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.authoritative_scan_id, scans.status, scans.scan_owner
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("retained malformed active owner");
    assert_eq!(
        retained,
        (
            "foreground_scan".to_owned(),
            Some(request.scan_id.clone()),
            "leased".to_owned(),
            Some(request.scan_id),
            "running".to_owned(),
            "authoritative_recovery".to_owned(),
        ),
        "validation must not terminate the foreign-owned scan or unblock its gap"
    );
    let error = error.expect("foreign-owned active claim must fail closed");
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
    assert!(!error.message.contains(root_path));
}

#[test]
fn current_v30_rejects_an_inexact_consumed_foreground_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-consumed-claim-root";
    let root_path = "C:\\InvalidConsumedClaim";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("invalid-consumed-claim-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('e'),
        )
        .expect("begin foreground recovery");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove consumed recovery first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = 'wrong_consumed_state',
                 last_failure_message = 'wrong consumed state'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt consumed foreground claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact consumed foreground claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_a_consumed_foreground_claim_bound_to_authoritative_recovery() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-consumed-owner-root";
    let root_path = "C:\\Private\\InvalidConsumedOwner";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("explicit recovery catalog");
    let request = fixture_request("invalid-consumed-owner-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('7'),
        )
        .expect("begin foreground recovery");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove consumed recovery owner first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("consume foreground claim");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET scan_owner = 'authoritative_recovery' WHERE id = ?1",
            [&request.scan_id],
        )
        .expect("bind consumed claim to wrong owner");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(opened) => {
            drop(opened);
            None
        }
        Err(error) => Some(error),
    };
    let connection = Connection::open(catalog_path).expect("inspect rejected consumed owner");
    let retained: (String, String, i64, String, String, String) = connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id,
                    claim.consumed_unix_ms IS NOT NULL, gap.status,
                    scans.status, scans.scan_owner
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("retained malformed consumed owner");
    assert_eq!(
        retained,
        (
            "foreground_scan".to_owned(),
            request.scan_id,
            1,
            "completed".to_owned(),
            "completed".to_owned(),
            "authoritative_recovery".to_owned(),
        )
    );
    let error = error.expect("foreign-owned consumed claim must fail closed");
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
    assert!(!error.message.contains(root_path));
}

#[test]
fn interrupted_explicit_recovery_cleanup_preserves_handoff_owned_assets() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let (root_id, _, request) = interrupted_explicit_foreground_with_handoff_assets(&catalog_path);

    let reopened = SqliteCatalog::open(catalog_path.clone()).expect("recover interrupted scan");
    assert_eq!(
        crash_cleanup_asset_projection(&reopened.connection),
        (1, 1, 0, 0),
        "only the truly unreferenced asset may be removed"
    );
    let recovered: (String, Option<String>, String, String) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status, scans.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = ?2
             WHERE claim.root_id = ?1",
            params![root_id, request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("recovered crash state");
    assert_eq!(
        recovered,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            "failed".to_owned(),
        )
    );
    drop(reopened);

    let reopened_again = SqliteCatalog::open(catalog_path).expect("idempotent crash reopen");
    assert_eq!(
        crash_cleanup_asset_projection(&reopened_again.connection),
        (1, 1, 0, 0)
    );
}

#[test]
fn interrupted_explicit_recovery_cleanup_failure_rolls_back_before_retry() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let (root_id, _, request) = interrupted_explicit_foreground_with_handoff_assets(&catalog_path);
    let connection = Connection::open(&catalog_path).expect("install cleanup failure");
    connection
        .execute_batch(
            "CREATE TRIGGER crash_cleanup_failure
             BEFORE DELETE ON assets
             WHEN OLD.id = 'crash-true-orphan'
             BEGIN
               SELECT RAISE(ABORT, 'injected crash cleanup failure');
             END;",
        )
        .expect("install cleanup failure trigger");
    drop(connection);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(_) => panic!("injected cleanup failure must abort reopen"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_database_error");
    let connection = Connection::open(&catalog_path).expect("inspect cleanup rollback");
    let rolled_back: (String, String, String, Option<String>, i64) = connection
        .query_row(
            "SELECT claim.consumer_kind, gap.status, scans.status,
                    gap.authoritative_scan_id,
                    (SELECT COUNT(*) FROM assets WHERE id LIKE 'crash-%-asset'
                       OR id = 'crash-true-orphan')
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
             WHERE claim.root_id = ?1",
            [&root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("rolled-back crash state");
    assert_eq!(
        rolled_back,
        (
            "foreground_scan".to_owned(),
            "leased".to_owned(),
            "running".to_owned(),
            Some(request.scan_id),
            3,
        )
    );
    assert_eq!(crash_cleanup_asset_projection(&connection), (1, 1, 1, 0));
    connection
        .execute_batch("DROP TRIGGER crash_cleanup_failure")
        .expect("remove cleanup failure trigger");
    drop(connection);

    let reopened = SqliteCatalog::open(catalog_path.clone()).expect("retry crash cleanup");
    assert_eq!(
        crash_cleanup_asset_projection(&reopened.connection),
        (1, 1, 0, 0)
    );
    drop(reopened);
    let reopened_again = SqliteCatalog::open(catalog_path).expect("idempotent cleanup retry");
    assert_eq!(
        crash_cleanup_asset_projection(&reopened_again.connection),
        (1, 1, 0, 0)
    );
}

#[test]
fn foreground_publication_binding_is_atomic_and_replacement_retires_the_generation() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "publication-root";
    let root_path = "C:\\PublicationRoot";
    let first = fixture_request("publication-first", root_path);
    let first_identity = publication_identity('1');
    catalog
        .begin_scan_with_publication_namespace(&first, root_id, root_path, &first_identity)
        .expect("bind first foreground scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&first.scan_id)
        .expect("prove publication fixture first-import handoff");
    let before_publish: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = ?1 AND root_generation = 1),
               (SELECT COUNT(*) FROM library_root_publication_namespaces WHERE root_id = ?2),
               (SELECT generation FROM library_change_root_state WHERE root_id = ?2)",
            rusqlite::params![first.scan_id, root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("pre-publication binding");
    assert_eq!(before_publish, (1, 0, 1));

    catalog
        .publish_scan(&first.scan_id, root_id, 0, 0)
        .expect("publish first foreground scan");
    let first_publication: (i64, String, String, i64, String) = catalog
        .connection
        .query_row(
            "SELECT proof.root_generation, proof.identity_value, proof.authority_kind,
                    (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                     WHERE scan_id = ?1), roots.active_scan_id
             FROM library_root_publication_namespaces AS proof
             JOIN library_roots AS roots ON roots.id = proof.root_id
             WHERE proof.root_id = ?2",
            rusqlite::params![first.scan_id, root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("atomic foreground proof");
    assert_eq!(
        first_publication,
        (
            1,
            first_identity.value.clone(),
            "foreground_scan".to_owned(),
            0,
            first.scan_id.clone(),
        )
    );

    let replacement = fixture_request("publication-replacement", root_path);
    let replacement_identity = publication_identity('2');
    catalog
        .begin_scan_with_publication_namespace(
            &replacement,
            root_id,
            root_path,
            &replacement_identity,
        )
        .expect("bind replacement generation");
    let replacement_binding: (i64, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT state.generation,
                    (SELECT COUNT(*) FROM library_root_publication_namespaces
                     WHERE root_id = state.root_id),
                    (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                     WHERE scan_id = ?2 AND root_generation = state.generation),
                    roots.active_scan_id
             FROM library_change_root_state AS state
             JOIN library_roots AS roots ON roots.id = state.root_id
             WHERE state.root_id = ?1",
            rusqlite::params![root_id, replacement.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("replacement retirement evidence");
    assert_eq!(
        replacement_binding,
        (2, 0, 1, first.scan_id.clone()),
        "replacement must retire generation-one proof without changing the last trusted catalog"
    );

    catalog
        .abandon_scan(&replacement.scan_id, "failed", 1)
        .expect("cancel replacement scan");
    let after_cancel: (i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM library_root_publication_namespaces WHERE root_id = ?2),
               (SELECT active_scan_id FROM library_roots WHERE id = ?2)",
            rusqlite::params![replacement.scan_id, root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("cancelled publication evidence");
    assert_eq!(after_cancel, (0, 0, first.scan_id));
}

#[test]
fn foreground_resume_rejects_a_different_namespace_identity_without_mutation() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let request = fixture_request("publication-resume", "C:\\PublicationResume");
    let expected = publication_identity('a');
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            "resume-root",
            &request.root_path,
            &expected,
        )
        .expect("begin bound scan");

    let error = catalog
        .resume_scan_with_publication_namespace(
            &request,
            "resume-root",
            &request.root_path,
            &publication_identity('b'),
        )
        .expect_err("resume cannot replace its original namespace identity");
    assert_eq!(error.code, "catalog_scan_publication_namespace_mismatch");
    let binding: (String, String) = catalog
        .connection
        .query_row(
            "SELECT identity_value, scans.status
             FROM library_scan_publication_namespace_bindings AS binding
             JOIN scan_runs AS scans ON scans.id = binding.scan_id
             WHERE binding.scan_id = ?1",
            [&request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained resume binding");
    assert_eq!(binding, (expected.value, "running".to_owned()));
}

#[test]
fn one_root_cannot_have_overlapping_authoritative_scans() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut first = SqliteCatalog::open(path.clone()).expect("first catalog");
    let first_request = fixture_request("root-scan-a", "C:\\Pictures");
    first
        .begin_scan(&first_request, "root-a", &first_request.root_path)
        .expect("begin first scan");
    let mut second = SqliteCatalog::open(path).expect("second catalog");
    let second_request = fixture_request("root-scan-b", "C:\\Pictures");

    let error = second
        .begin_scan(&second_request, "root-a", &second_request.root_path)
        .expect_err("overlapping root scan must fail");

    assert_eq!(error.code, "catalog_root_scan_in_progress");
    first
        .abandon_scan(&first_request.scan_id, "cancelled", 0)
        .expect("release first scan");
    second
        .begin_scan(&second_request, "root-a", &second_request.root_path)
        .expect("begin second scan after release");
}

#[test]
fn crash_recovery_keeps_only_the_newest_unpublished_foreground_import() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let older = fixture_request("initial-import-a", "C:\\InitialA");
    let newer = fixture_request("initial-import-z", "C:\\InitialZ");
    catalog
        .begin_scan(&older, "initial-root-a", &older.root_path)
        .expect("begin older initial import");
    catalog
        .begin_scan(&newer, "initial-root-z", &newer.root_path)
        .expect("begin newer initial import");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen after simulated crash");

    let recoverable = catalog
        .load_single_recoverable_foreground_scan()
        .expect("converge recoverable imports")
        .expect("newest initial import remains recoverable");

    assert_eq!(recoverable.scan_id, newer.scan_id);
    let statuses: (String, String, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT status FROM scan_runs WHERE id = ?1),
               (SELECT status FROM scan_runs WHERE id = ?2),
               (SELECT COUNT(*) FROM scan_directory_frontier WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM scan_directory_entries WHERE scan_id = ?1)",
            params![older.scan_id, newer.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("converged initial imports");
    assert_eq!(statuses, ("failed".to_owned(), "running".to_owned(), 0, 0),);

    let replacement = fixture_request("initial-import-a-retry", &older.root_path);
    catalog
        .begin_scan(&replacement, "initial-root-a", &replacement.root_path)
        .expect("retired initial root can be imported again");
    catalog
        .abandon_scan(&replacement.scan_id, "cancelled", 0)
        .expect("clean up replacement import");
    catalog
        .abandon_scan(&newer.scan_id, "cancelled", 0)
        .expect("clean up retained import");
}

#[test]
fn crash_recovery_abandons_updates_and_preserves_the_paused_initial_import() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");

    let initial_a = fixture_request("published-a", "C:\\PublishedA");
    catalog
        .begin_scan(&initial_a, "published-root-a", &initial_a.root_path)
        .expect("begin first published root");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial_a.scan_id)
        .expect("prove first crash fixture first-import handoff");
    catalog
        .publish_scan(&initial_a.scan_id, "published-root-a", 0, 0)
        .expect("publish first root");
    seed_current_explicit_recovery_claim(&catalog, "published-root-a", 101);
    let update_a = fixture_request("crashed-update-a", &initial_a.root_path);
    catalog
        .begin_scan(&update_a, "published-root-a", &update_a.root_path)
        .expect("begin first update");

    let initial_b = fixture_request("published-b", "C:\\PublishedB");
    catalog
        .begin_scan(&initial_b, "published-root-b", &initial_b.root_path)
        .expect("begin second published root");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial_b.scan_id)
        .expect("prove second crash fixture first-import handoff");
    catalog
        .publish_scan(&initial_b.scan_id, "published-root-b", 0, 0)
        .expect("publish second root");
    seed_current_explicit_recovery_claim(&catalog, "published-root-b", 102);
    let update_b = fixture_request("crashed-update-b", &initial_b.root_path);
    catalog
        .begin_scan(&update_b, "published-root-b", &update_b.root_path)
        .expect("begin second update");

    let paused = fixture_request("paused-initial-import", "C:\\PausedInitial");
    catalog
        .begin_scan(&paused, "paused-initial-root", &paused.root_path)
        .expect("begin paused initial import");
    catalog
        .pause_scan(&paused.scan_id, &ScanCheckpoint::default())
        .expect("pause initial import");
    let running = fixture_request("running-initial-import", "C:\\RunningInitial");
    catalog
        .begin_scan(&running, "running-initial-root", &running.root_path)
        .expect("begin running initial import");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen after simulated crash");

    let recoverable = catalog
        .load_single_paused_foreground_scan()
        .expect("converge foreground recovery")
        .expect("paused initial import remains recoverable");
    assert_eq!(recoverable.scan_id, paused.scan_id);
    assert!(
        catalog
            .load_single_recoverable_foreground_scan()
            .expect("running recovery projection")
            .is_none(),
    );

    let statuses: (String, String, String, String, i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT status FROM scan_runs WHERE id = ?1),
               (SELECT status FROM scan_runs WHERE id = ?2),
               (SELECT status FROM scan_runs WHERE id = ?3),
               (SELECT status FROM scan_runs WHERE id = ?4),
               (SELECT COUNT(*) FROM scan_directory_frontier
                WHERE scan_id IN (?1, ?2, ?4)),
               (SELECT COUNT(*) FROM scan_directory_entries
                WHERE scan_id IN (?1, ?2, ?4)),
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id IN (?1, ?2, ?4)),
               (SELECT COUNT(*) FROM asset_locations
                WHERE scan_id IN (?1, ?2, ?4))",
            params![
                update_a.scan_id,
                update_b.scan_id,
                paused.scan_id,
                running.scan_id
            ],
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
                ))
            },
        )
        .expect("converged foreground scans");
    assert_eq!(
        statuses,
        (
            "failed".to_owned(),
            "failed".to_owned(),
            "paused".to_owned(),
            "failed".to_owned(),
            0,
            0,
            0,
            0,
        ),
    );
    let restored_claims: Vec<(String, String, Option<String>, String)> = catalog
        .connection
        .prepare(
            "SELECT claim.root_id, claim.consumer_kind, claim.foreground_scan_id,
                    gap.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id IN ('published-root-a', 'published-root-b')
             ORDER BY claim.root_id",
        )
        .expect("restored claims statement")
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("restored claim rows")
        .collect::<Result<_, _>>()
        .expect("restored claims");
    assert_eq!(
        restored_claims,
        vec![
            (
                "published-root-a".to_owned(),
                "explicit_recovery_required".to_owned(),
                None,
                "retry_wait".to_owned(),
            ),
            (
                "published-root-b".to_owned(),
                "explicit_recovery_required".to_owned(),
                None,
                "retry_wait".to_owned(),
            ),
        ],
    );

    let retry_a = fixture_request("retry-update-a", &initial_a.root_path);
    catalog
        .begin_scan(&retry_a, "published-root-a", &retry_a.root_path)
        .expect("first root can update again");
    let retry_b = fixture_request("retry-update-b", &initial_b.root_path);
    catalog
        .begin_scan(&retry_b, "published-root-b", &retry_b.root_path)
        .expect("second root can update again");
    catalog
        .abandon_scan(&retry_a.scan_id, "cancelled", 0)
        .expect("clean up first retry");
    catalog
        .abandon_scan(&retry_b.scan_id, "cancelled", 0)
        .expect("clean up second retry");
    catalog
        .abandon_scan(&paused.scan_id, "cancelled", 0)
        .expect("clean up paused import");
}

#[test]
fn legacy_automatic_scans_are_retired_while_foreground_checkpoint_remains_recoverable() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let first_authoritative = fixture_request("sync-recovery-a", "C:\\RecoveryA");
    let second_authoritative = fixture_request("sync-recovery-b", "C:\\RecoveryB");
    let foreground = fixture_request("foreground-c", "C:\\ForegroundC");
    catalog
        .begin_authoritative_scan(
            &first_authoritative,
            "root-recovery-a",
            &first_authoritative.root_path,
        )
        .expect("first authoritative scan");
    catalog
        .begin_authoritative_scan(
            &second_authoritative,
            "root-recovery-b",
            &second_authoritative.root_path,
        )
        .expect("second authoritative scan");
    catalog
        .begin_scan(&foreground, "root-foreground-c", &foreground.root_path)
        .expect("foreground scan");
    let first_recovery = catalog
        .load_authoritative_recoverable_scan_after(None)
        .expect("first authoritative recovery")
        .expect("first authoritative scan");
    let second_recovery = catalog
        .load_authoritative_recoverable_scan_after(Some(&first_recovery.scan_id))
        .expect("second authoritative recovery")
        .expect("second authoritative scan");
    let end_of_page = catalog
        .load_authoritative_recoverable_scan_after(Some(&second_recovery.scan_id))
        .expect("authoritative recovery end");
    let ownership_error = catalog
        .resume_scan(
            &first_authoritative,
            "root-recovery-a",
            &first_authoritative.root_path,
        )
        .expect_err("foreground lifecycle cannot claim authoritative scan");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET status = 'paused' WHERE id = ?1",
            [&second_authoritative.scan_id],
        )
        .expect("pause second authoritative scan");
    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy automatic scans");
    let foreground_recovery = catalog
        .load_recoverable_scan()
        .expect("foreground recovery")
        .expect("foreground scan is recoverable");
    let remaining_authoritative = catalog
        .load_authoritative_recoverable_scan_after(None)
        .expect("remaining authoritative recovery");
    let retired_statuses: Vec<String> = catalog
        .connection
        .prepare("SELECT status FROM scan_runs WHERE id IN (?1, ?2) ORDER BY id")
        .expect("retired status statement")
        .query_map(
            rusqlite::params![first_authoritative.scan_id, second_authoritative.scan_id],
            |row| row.get(0),
        )
        .expect("retired status rows")
        .collect::<Result<_, _>>()
        .expect("retired statuses");

    assert_eq!(first_recovery.scan_id, first_authoritative.scan_id);
    assert_eq!(second_recovery.scan_id, second_authoritative.scan_id);
    assert!(end_of_page.is_none());
    assert_eq!(ownership_error.code, "catalog_scan_resume_mismatch");
    assert_eq!(retired, 2);
    assert_eq!(foreground_recovery.scan_id, foreground.scan_id);
    assert!(remaining_authoritative.is_none());
    assert_eq!(
        retired_statuses,
        vec!["superseded".to_owned(), "superseded".to_owned()]
    );
}

#[test]
fn authoritative_resume_after_root_unregister_fails_without_recreating_catalog_state() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_id = "removed-recovery-root";
    let recovery = fixture_request("removed-recovery-scan", "C:\\RemovedRecovery");
    catalog
        .begin_authoritative_scan(&recovery, root_id, &recovery.root_path)
        .expect("begin authoritative scan");
    let loaded = catalog
        .load_authoritative_recoverable_scan_after(None)
        .expect("load authoritative checkpoint")
        .expect("authoritative checkpoint");
    assert_eq!(loaded.scan_id, recovery.scan_id);

    assert!(catalog.unregister_root(root_id).expect("unregister root"));
    let error = catalog
        .resume_authoritative_scan(&recovery, root_id, &recovery.root_path)
        .expect_err("removed checkpoint must fail closed");

    assert_eq!(error.code, "catalog_scan_resume_missing");
    let counts: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_roots WHERE id = ?1),
               (SELECT COUNT(*) FROM scan_runs WHERE root_id = ?1),
               (SELECT COUNT(*) FROM scan_directory_frontier
                  WHERE scan_id = ?2)",
            rusqlite::params![root_id, recovery.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("catalog counts");
    assert_eq!(counts, (0, 0, 0));
}

#[test]
fn legacy_root_audit_and_its_exclusive_recovery_scan_are_retired() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "legacy-audit-root";
    let root_path = "C:\\LegacyAudit";
    let initial = fixture_request("legacy-audit-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove legacy audit first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .expect("enqueue legacy audit");
    let recovery = fixture_request("legacy-audit-recovery", root_path);
    catalog
        .begin_authoritative_scan(&recovery, root_id, root_path)
        .expect("begin legacy audit recovery scan");

    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy audit");
    let scan_status: String = catalog
        .connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = ?1",
            [&recovery.scan_id],
            |row| row.get(0),
        )
        .expect("retired scan status");
    let metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 2_000, policy)
        .expect("retired queue metrics");

    assert_eq!(retired, 1);
    assert_eq!(scan_status, "superseded");
    assert!(
        catalog
            .load_authoritative_recoverable_scan_after(None)
            .expect("recoverable scans")
            .is_none()
    );
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.superseded_count, 1);

    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "retry.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 3_000,
                most_recent_observed_unix_ms: 3_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            3_000,
            policy,
        )
        .expect("enqueue historical path retry");

    assert_eq!(
        catalog
            .retire_legacy_automatic_full_scans(4_000)
            .expect("preserve historical path retry"),
        0
    );
    let retained = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 4_000, policy)
        .expect("retained path metrics");
    assert_eq!(retained.pending_count, 1);
}

#[test]
fn legacy_automatic_full_scan_releases_live_gap_to_metadata_inventory() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "legacy-live-gap-root";
    let root_path = "C:\\LegacyLiveGap";
    let initial = fixture_request("legacy-live-gap-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove legacy live-gap first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .expect("enqueue live evidence gap");
    let recovery = fixture_request("legacy-live-gap-recovery", root_path);
    catalog
        .begin_authoritative_scan(&recovery, root_id, root_path)
        .expect("begin legacy automatic full scan");
    let leased_metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 1_000, policy)
        .expect("leased queue metrics");

    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy automatic full scan");
    let released_unix_ms = unix_time_ms();
    let released_metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, released_unix_ms, policy)
        .expect("released queue metrics");
    let active_scan_id: String = catalog
        .connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = ?1",
            [root_id],
            |row| row.get(0),
        )
        .expect("active published scan");
    let leased = catalog
        .lease_authoritative_library_change(root_id, generation, released_unix_ms, policy)
        .expect("lease released inventory authority")
        .expect("inventory authority remains queued");

    assert_eq!(leased_metrics.leased_count, 1);
    assert_eq!(retired, 1);
    assert_eq!(released_metrics.pending_count, 1);
    assert_eq!(released_metrics.leased_count, 0);
    assert_eq!(released_metrics.freshness_unknown_count, 1);
    assert_eq!(active_scan_id, initial.scan_id);
    assert_eq!(
        leased.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(leased.change.intent.scope, LibraryChangeScope::Root);
    assert!(
        catalog
            .load_authoritative_recoverable_scan_after(None)
            .expect("legacy recoverable scans")
            .is_none()
    );
}

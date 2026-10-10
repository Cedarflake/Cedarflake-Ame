use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::AtomicUsize;

use crate::domain::ScanRequest;
use crate::ports::CatalogRepository;
use rusqlite::{Connection, ErrorCode};

use super::*;

#[test]
fn publication_retirement_disables_progress_and_late_cloned_priority_callback() {
    let requested = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&requested);
    let state = PublicationInterruption::new(&ScanPublicationControl::new(move || {
        observed.load(Ordering::Acquire)
    }));
    let interrupts = Arc::new(AtomicUsize::new(0));
    let observed_interrupts = Arc::clone(&interrupts);
    let callback = state.preemption_callback(move || {
        observed_interrupts.fetch_add(1, Ordering::AcqRel);
    });
    let late_callback = Arc::clone(&callback);
    callback();
    assert_eq!(interrupts.load(Ordering::Acquire), 1);
    assert!(state.is_interrupted());
    state.retire();
    requested.store(true, Ordering::Release);
    late_callback();
    assert_eq!(interrupts.load(Ordering::Acquire), 1);
    assert!(!state.is_interrupted());
}

#[test]
fn publication_unwind_retires_progress_before_same_connection_rollback() {
    let source = tempfile::tempdir().expect("controlled empty source");
    let storage = tempfile::tempdir().expect("isolated catalog");
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    let request = ScanRequest {
        scan_id: "publication-unwind".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, "root", &request.root_path)
        .expect("begin real scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("legitimate first-import publication authority");
    let requested = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&requested);
    let control = ScanPublicationControl::new(move || observed.load(Ordering::Acquire));
    let _hook =
        super::super::set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
            requested.store(true, Ordering::Release);
            panic!("injected publication phase panic");
        });
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        publish_scan_with_proof(&mut catalog, &request.scan_id, "root", 0, 0, None, &control)
    }));
    assert!(interrupted.is_err());
    assert!(
        catalog.connection.is_autocommit(),
        "rollback finished despite requested control"
    );
    let sum: i64 = catalog.connection.query_row("WITH RECURSIVE seq(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM seq WHERE n<10000) SELECT SUM(n) FROM seq", [], |row| row.get(0))
        .expect("same-connection SQL outlives the retired progress handler");
    assert_eq!(sum, 50_005_000);
    let active: Option<String> = catalog
        .connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id='root'",
            [],
            |row| row.get(0),
        )
        .expect("publication rolled back");
    assert!(active.is_none());
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("same-connection terminal cleanup can write after unwinding");
    let status: String = catalog
        .connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id=?1",
            [&request.scan_id],
            |row| row.get(0),
        )
        .expect("terminal cleanup committed");
    assert_eq!(status, "cancelled");
}

#[test]
fn publication_database_error_mapping_distinguishes_interrupts_from_sql_failure() {
    let connection = Connection::open_in_memory().expect("isolated database");
    let invalid = connection
        .execute("INSERT INTO nonexistent VALUES (1)", [])
        .expect_err("invalid SQL");
    let state = PublicationInterruption::new(&ScanPublicationControl::new(|| true));
    assert_eq!(
        state.classify_error(database_error(invalid)).code,
        "catalog_database_error",
        "requested control must not relabel real SQL faults"
    );
    connection
        .progress_handler(1, Some(|| true))
        .expect("install interrupt");
    let interrupted = connection
        .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
        .expect_err("actual SQLite interrupt");
    assert_eq!(
        interrupted.sqlite_error_code(),
        Some(ErrorCode::OperationInterrupted)
    );
    let failure = database_error(interrupted);
    assert_eq!(failure.code, "catalog_database_interrupted");
    assert_eq!(
        state.classify_error(failure).code,
        "catalog_scan_publication_controlled"
    );
    connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("remove interrupt");
}

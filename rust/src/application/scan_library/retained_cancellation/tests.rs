use rusqlite::Connection;
use tempfile::TempDir;

use crate::adapters::SqliteCatalog;
use crate::domain::{LibraryRootGeneration, ScanCheckpoint, ScanRequest};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::super::execution_registry::{
    ScanRegistration, catalog_has_active_scan, first_import_capture_lease, register_scan,
    reserve_retained_cancellation,
};
use super::*;

struct Fixture {
    directory: TempDir,
    path: std::path::PathBuf,
    request: ScanRequest,
}

impl Fixture {
    fn new(paused: bool) -> Self {
        let directory = tempfile::tempdir().expect("derived fixture");
        let path = directory.path().join("catalog.sqlite3");
        let request = ScanRequest {
            scan_id: format!("retained-{}", directory.path().display()),
            root_path: directory
                .path()
                .join("absent-source")
                .to_string_lossy()
                .into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
        catalog
            .begin_scan(&request, "root", &request.root_path)
            .expect("retained row");
        let checkpoint = ScanCheckpoint {
            last_visited_relative_path: Some("previous.png".to_owned()),
            visited_entries: 3,
            ..ScanCheckpoint::default()
        };
        catalog
            .checkpoint_scan(&request.scan_id, &checkpoint)
            .expect("checkpoint");
        if paused {
            catalog
                .pause_scan(&request.scan_id, &checkpoint)
                .expect("paused");
        }
        drop(catalog);
        crate::application::catalog_session::open_catalog(&path, LibraryChangeLane::Recovery)
            .expect("validated session");
        Self {
            directory,
            path,
            request,
        }
    }

    fn evidence(&self) -> (String, i64, i64, i64) {
        Connection::open(&self.path).expect("evidence reader").query_row(
            "SELECT scan.status, scan.visited_entries,
                    (SELECT COUNT(*) FROM scan_directory_frontier WHERE scan_id = scan.id),
                    state.is_active
             FROM scan_runs AS scan JOIN library_change_root_state AS state ON state.root_id = scan.root_id
             WHERE scan.id = ?1", [&self.request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).expect("retained evidence")
    }
}

#[test]
fn retained_cancel_commits_without_starting_scan_or_accessing_missing_source() {
    for paused in [false, true] {
        let fixture = Fixture::new(paused);
        assert!(!Path::new(&fixture.request.root_path).exists());
        cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
            .expect("durable cancellation");
        cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
            .expect("already cancelled is idempotent");
        let (status, visited, frontier, active) = fixture.evidence();
        assert_eq!(
            (status.as_str(), visited, frontier, active),
            ("cancelled", 3, 0, 0)
        );
        let mut catalog = SqliteCatalog::open(fixture.path.clone()).expect("full schema reopen");
        assert!(
            catalog
                .load_single_recoverable_foreground_scan()
                .expect("recoverable")
                .is_none()
        );
        assert!(
            catalog
                .load_single_paused_foreground_scan()
                .expect("paused")
                .is_none()
        );
        assert_eq!(
            catalog
                .load_inactive_first_import_roots()
                .expect("configured dormant root")
                .len(),
            1
        );
        assert!(
            catalog
                .load_incremental_catalog_roots()
                .expect("active roots")
                .is_empty()
        );
        assert!(!Path::new(&fixture.request.root_path).exists());
        assert_eq!(
            std::fs::read_dir(fixture.directory.path())
                .expect("derived files")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .count(),
            0
        );
    }
}

#[test]
fn retained_cancel_and_running_resume_have_exclusive_registry_ownership() {
    let fixture = Fixture::new(true);
    let reservation = reserve_retained_cancellation(&fixture.request.scan_id, &fixture.path)
        .expect("retained reservation");
    assert_eq!(
        register_scan(&fixture.request.scan_id)
            .expect_err("resume denied")
            .code,
        "scan_already_active"
    );
    assert!(!super::super::cancel_scan(&fixture.request.scan_id));
    assert!(!super::super::pause_scan(&fixture.request.scan_id));
    assert!(!super::super::suspend_scan(&fixture.request.scan_id));
    let catalog = crate::application::catalog_session::open_catalog_for_foreground_scan(
        &fixture.path,
        LibraryChangeLane::Recovery,
        &fixture.request.scan_id,
    )
    .expect("protected reservation");
    assert!(catalog_has_active_scan(&fixture.path, None).expect("session protected"));
    assert!(
        first_import_capture_lease(&fixture.path, "root", LibraryRootGeneration::initial())
            .expect("not a capture owner")
            .is_none()
    );
    drop(catalog);
    drop(reservation);
    assert!(!catalog_has_active_scan(&fixture.path, None).expect("released"));
    register_scan(&fixture.request.scan_id).expect("new execution owns ID");
    let registration = ScanRegistration {
        scan_id: fixture.request.scan_id.clone(),
    };
    let before = fixture.evidence();
    assert_eq!(
        cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
            .expect_err("live execution denied")
            .code,
        "scan_retained_operation_conflict"
    );
    assert_eq!(fixture.evidence(), before);
    drop(registration);
    cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
        .expect("after execution released");
    let mut catalog = SqliteCatalog::open(fixture.path.clone()).expect("catalog");
    assert!(
        catalog
            .resume_scan(&fixture.request, "root", &fixture.request.root_path)
            .is_err()
    );
    assert_eq!(fixture.evidence().0, "cancelled");
}

#[test]
fn retained_cancel_rollback_preserves_checkpoint_and_releases_reservation() {
    let fixture = Fixture::new(true);
    let before = fixture.evidence();
    let connection = Connection::open(&fixture.path).expect("fault owner");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_retained_cancel BEFORE DELETE ON scan_directory_frontier
         BEGIN SELECT RAISE(ABORT, 'injected retained cancellation failure'); END;",
        )
        .expect("transaction failure");
    let error = cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
        .expect_err("rollback");
    assert!(
        error
            .message
            .contains("injected retained cancellation failure"),
        "{error:?}"
    );
    assert_eq!(fixture.evidence(), before);
    assert!(!catalog_has_active_scan(&fixture.path, None).expect("released failure"));
    connection
        .execute_batch("DROP TRIGGER reject_retained_cancel")
        .expect("release fault");
    cancel_retained_scan_at_path(&fixture.request.scan_id, &fixture.path)
        .expect("retry same retained operation");
    SqliteCatalog::open(fixture.path).expect("fully verifiable terminal catalog");
}

#[test]
fn retained_cancel_rejects_unknown_completed_published_and_obsolete_scan_owners() {
    let fixture = Fixture::new(true);
    assert_eq!(
        cancel_retained_scan_at_path("unknown-retained-scan", &fixture.path)
            .expect_err("unknown ID")
            .code,
        "scan_retained_not_found"
    );
    let mut catalog = SqliteCatalog::open(fixture.path.clone()).expect("catalog");
    let connection = Connection::open(&fixture.path).expect("controlled state");
    for sql in [
        "UPDATE scan_runs SET status = 'failed'",
        "UPDATE scan_runs SET status = 'completed'",
        "UPDATE scan_runs SET status = 'paused'; UPDATE library_roots SET active_scan_id = (SELECT id FROM scan_runs LIMIT 1)",
        "UPDATE library_roots SET active_scan_id = NULL; UPDATE library_change_root_state SET generation = generation + 1",
    ] {
        connection
            .execute_batch(sql)
            .expect("noncancellable fixture state");
        let before = fixture.evidence();
        assert_eq!(
            catalog
                .cancel_retained_scan(&fixture.request.scan_id)
                .expect_err("state guarded inside transaction")
                .code,
            "scan_retained_not_cancellable"
        );
        assert_eq!(fixture.evidence(), before);
    }
    connection
        .execute_batch("UPDATE scan_runs SET scan_owner = 'authoritative_recovery'")
        .expect("different owner");
    assert_eq!(
        catalog
            .cancel_retained_scan(&fixture.request.scan_id)
            .expect_err("not foreground")
            .code,
        "scan_retained_owner_mismatch"
    );
}

#[test]
fn retained_cancel_preserves_a_published_baseline_and_its_replacement() {
    let fixture = Fixture::new(false);
    let mut catalog = SqliteCatalog::open(fixture.path.clone()).expect("catalog");
    catalog
        .prove_live_only_first_import_handoff_for_test(&fixture.request.scan_id)
        .expect("healthy first-import handoff");
    catalog
        .publish_scan(&fixture.request.scan_id, "root", 0, 0)
        .expect("published empty baseline");
    let replacement = ScanRequest {
        scan_id: format!("{}-replacement", fixture.request.scan_id),
        ..fixture.request.clone()
    };
    catalog
        .begin_scan(&replacement, "root", &fixture.request.root_path)
        .expect("replacement");
    catalog
        .pause_scan(&replacement.scan_id, &ScanCheckpoint::default())
        .expect("retained replacement");
    let before = catalog
        .load_incremental_catalog_root("root")
        .expect("before")
        .expect("root");
    for scan_id in [&fixture.request.scan_id, &replacement.scan_id] {
        assert_eq!(
            cancel_retained_scan_at_path(scan_id, &fixture.path)
                .expect_err("published library is not an unfinished import")
                .code,
            "scan_retained_not_cancellable"
        );
    }
    let after = catalog
        .load_incremental_catalog_root("root")
        .expect("after")
        .expect("root");
    assert_eq!(after.active_scan_id, before.active_scan_id);
    assert_eq!(after.catalog_revision, before.catalog_revision);
    let status: String = Connection::open(&fixture.path)
        .expect("reader")
        .query_row(
            "SELECT status FROM scan_runs WHERE id = ?1",
            [&replacement.scan_id],
            |row| row.get(0),
        )
        .expect("replacement status");
    assert_eq!(status, "paused");
}

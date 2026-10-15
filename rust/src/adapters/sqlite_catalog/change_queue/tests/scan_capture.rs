use super::*;

#[test]
fn foreground_update_preserves_inventory_control_and_candidate_ownership() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut catalog, control_id) = pending_inventory_catalog(path.clone());
    let before = recovery_rows(&catalog.connection);
    let mut covered = metadata_candidate_intent("covered.jpg", 20);
    covered.origin = LibraryChangeOrigin::ConsistencyAudit;
    catalog
        .enqueue_library_change_intents(&[covered], 1_020, immediate_policy())
        .expect("ordinary scan-covered change");
    let request = foreground_request(&catalog);
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin explicit update");
    assert_eq!(recovery_rows(&catalog.connection), before);
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect("publish explicit update");
    assert_eq!(recovery_rows(&catalog.connection), before);
    let covered_status: String = catalog
        .connection
        .query_row(
            "SELECT status FROM library_change_queue WHERE relative_path = 'covered.jpg'",
            [],
            |row| row.get(0),
        )
        .expect("ordinary change status");
    assert_eq!(covered_status, "completed");
    drop(catalog);
    let mut reopened = SqliteCatalog::open(path).expect("reopen after update");
    assert_eq!(recovery_rows(&reopened.connection), before);
    assert_recovery_can_continue(&mut reopened, control_id);
}

#[test]
fn retained_scan_releases_previously_captured_inventory_work_on_publication() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, control_id) =
        pending_inventory_catalog(directory.path().join("catalog.sqlite3"));
    let request = foreground_request(&catalog);
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin retained update");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'leased', lease_generation = lease_generation + 1,
                 lease_expires_unix_ms = 9999999999999, authoritative_scan_id = ?1",
            [&request.scan_id],
        )
        .expect("retain the older client's captured recovery tasks");
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect("publish retained update");
    for row in recovery_rows(&catalog.connection) {
        assert_eq!(row.status, "pending");
        assert!(row.scan_id.is_none());
    }
    assert_recovery_can_continue(&mut catalog, control_id);
}

#[test]
fn failed_scan_publication_rolls_back_recovery_release() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, _) = pending_inventory_catalog(directory.path().join("catalog.sqlite3"));
    let request = foreground_request(&catalog);
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin update");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'leased', lease_expires_unix_ms = 9999999999999,
                 authoritative_scan_id = ?1",
            [&request.scan_id],
        )
        .expect("retain old scan capture");
    let before = recovery_rows(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_after_recovery_release
             BEFORE UPDATE OF last_consistency_audit_unix_ms ON library_change_root_state
             BEGIN SELECT RAISE(ABORT, 'fixture publication failure'); END;",
        )
        .expect("inject final publication failure");
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect_err("publication rolls back");
    assert_eq!(recovery_rows(&catalog.connection), before);
    let active: String = catalog
        .connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = 'root-a'",
            [],
            |row| row.get(0),
        )
        .expect("prior projection");
    assert_eq!(active, "published-scan");
}

#[test]
fn reopen_restores_recovery_completed_by_an_older_foreground_scan() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (catalog, control_id) = incorrectly_completed_inventory_catalog(path.clone());
    let progress = catalog
        .load_metadata_inventory_run("scan-overlap")
        .expect("staged inventory progress");
    let rows_before = recovery_rows(&catalog.connection);
    assert!(rows_before.iter().all(|row| row.status == "completed"));
    drop(catalog);

    let reopened = SqliteCatalog::open(path.clone()).expect("repair the old scan capture");
    let repaired = recovery_rows(&reopened.connection);
    assert_eq!(repaired.len(), rows_before.len());
    for (before, after) in rows_before.iter().zip(&repaired) {
        assert_eq!(after.status, "pending");
        assert_eq!(after.id, before.id);
        assert_eq!(after.attempt_count, before.attempt_count);
        assert_eq!(after.lease_generation, before.lease_generation);
    }
    assert_eq!(
        reopened
            .load_metadata_inventory_run("scan-overlap")
            .unwrap(),
        progress
    );
    drop(reopened);
    let mut reopened = SqliteCatalog::open(path).expect("idempotent second open");
    assert_eq!(recovery_rows(&reopened.connection), repaired);
    assert_recovery_can_continue(&mut reopened, control_id);
}

#[test]
fn recovery_repair_preserves_a_candidate_completed_before_the_foreground_scan() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (catalog, control_id) = incorrectly_completed_inventory_catalog(path.clone());
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET updated_unix_ms = updated_unix_ms - 1
             WHERE id <> ?1",
            [sqlite_integer(control_id.value(), "control ID").unwrap()],
        )
        .expect("candidate completed independently before the scan");
    drop(catalog);
    let mut reopened = SqliteCatalog::open(path).expect("repair only the captured control");
    for row in recovery_rows(&reopened.connection) {
        if row.id == i64::try_from(control_id.value()).unwrap() {
            assert_eq!(row.status, "pending");
        } else {
            assert_eq!(row.status, "completed");
        }
    }
    assert_recovery_can_continue(&mut reopened, control_id);
}

#[test]
fn recovery_repair_rejects_completion_without_the_foreground_publication_witness() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (catalog, control_id) = incorrectly_completed_inventory_catalog(path.clone());
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET updated_unix_ms = updated_unix_ms - 1
             WHERE id = ?1",
            [sqlite_integer(control_id.value(), "control ID").unwrap()],
        )
        .expect("unrelated invalid terminal owner");
    let before = recovery_rows(&catalog.connection);
    drop(catalog);
    assert!(SqliteCatalog::open(path.clone()).is_err());
    let connection = Connection::open(path).expect("inspect rejected catalog");
    assert_eq!(recovery_rows(&connection), before);
}

#[test]
fn recovery_repair_rolls_back_candidates_when_control_restoration_fails() {
    let directory = tempdir().expect("catalog directory");
    let (mut catalog, control_id) =
        incorrectly_completed_inventory_catalog(directory.path().join("catalog.sqlite3"));
    let before = recovery_rows(&catalog.connection);
    catalog
        .connection
        .execute_batch(&format!(
            "CREATE TEMP TRIGGER fail_control_restoration
             BEFORE UPDATE OF status ON library_change_queue
             WHEN OLD.id = {} AND NEW.status = 'pending'
             BEGIN SELECT RAISE(ABORT, 'fixture recovery restoration failure'); END;",
            control_id.value()
        ))
        .expect("inject a failure after candidate restoration");
    super::super::super::migrations::migrate_schema(&mut catalog.connection)
        .expect_err("restore candidates and control in one transaction");
    assert_eq!(recovery_rows(&catalog.connection), before);
}

fn incorrectly_completed_inventory_catalog(path: PathBuf) -> (SqliteCatalog, LibraryChangeId) {
    let (mut catalog, control_id) = pending_inventory_catalog(path);
    let request = foreground_request(&catalog);
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin the foreground update");
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect("publish the foreground update");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', authoritative_scan_id = NULL,
                 next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                 catalog_revision_at_success = (
                   SELECT revision FROM catalog_state
                 ),
                 updated_unix_ms = (SELECT completed_unix_ms FROM scan_runs WHERE id = ?1)",
            [&request.scan_id],
        )
        .expect("reproduce the older foreground scan's terminal queue writes");
    (catalog, control_id)
}

fn pending_inventory_catalog(path: PathBuf) -> (SqliteCatalog, LibraryChangeId) {
    let (mut catalog, authority) = recovery_inventory_catalog_for_reason(
        path,
        "scan-overlap",
        &["pending.jpg"],
        LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
    );
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            "scan-overlap",
            &[metadata_candidate_intent("pending.jpg", 10)],
            &[metadata_candidate_update("pending.jpg")],
            1_010,
            immediate_policy(),
        )
        .expect("publish recovery candidate")
        .expect("current authority");
    catalog
        .defer_library_change(authority.change.id, authority.lease_generation, 1_011)
        .expect("yield recovery before foreground update");
    (catalog, authority.change.id)
}

fn foreground_request(catalog: &SqliteCatalog) -> ScanRequest {
    let root_path = catalog
        .connection
        .query_row(
            "SELECT path FROM library_roots WHERE id = 'root-a'",
            [],
            |row| row.get(0),
        )
        .expect("source root");
    ScanRequest {
        root_path,
        ..scan_request("foreground-update")
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RecoveryRow {
    id: i64,
    status: String,
    scan_id: Option<String>,
    attempt_count: i64,
    lease_generation: i64,
}

fn recovery_rows(connection: &Connection) -> Vec<RecoveryRow> {
    let mut statement = connection
        .prepare(
            "SELECT queue.id, queue.status, queue.authoritative_scan_id,
                    queue.attempt_count, queue.lease_generation
             FROM library_change_queue AS queue
             WHERE EXISTS(
               SELECT 1 FROM library_recovery_authorities WHERE change_id = queue.id
             ) OR EXISTS(
               SELECT 1 FROM library_metadata_inventory_candidate_owners WHERE change_id = queue.id
             )
             ORDER BY queue.id",
        )
        .expect("recovery state");
    statement
        .query_map([], |row| {
            Ok(RecoveryRow {
                id: row.get(0)?,
                status: row.get(1)?,
                scan_id: row.get(2)?,
                attempt_count: row.get(3)?,
                lease_generation: row.get(4)?,
            })
        })
        .expect("recovery rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("recovery values")
}

fn assert_recovery_can_continue(catalog: &mut SqliteCatalog, control_id: LibraryChangeId) {
    let leased = catalog
        .lease_metadata_inventory_recovery(
            "root-a",
            LibraryRootGeneration::initial(),
            super::super::super::unix_time_ms(),
            immediate_policy(),
        )
        .expect("inventory authority remains valid")
        .expect("inventory can resume");
    assert_eq!(leased.change.id, control_id);
    let run = catalog
        .load_metadata_inventory_run("scan-overlap")
        .expect("retained inventory");
    assert!(run.is_some());
}

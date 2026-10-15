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

fn pending_inventory_catalog(path: PathBuf) -> (SqliteCatalog, LibraryChangeId) {
    let (mut catalog, authority) =
        recovery_inventory_catalog(path, "scan-overlap", &["pending.jpg"]);
    catalog
        .connection
        .execute(
            "UPDATE library_recovery_authorities SET reason = 'watcher_uncovered_gap'
             WHERE change_id = ?1",
            [sqlite_integer(authority.change.id.value(), "control ID").expect("control ID")],
        )
        .expect("use a dispatchable watcher recovery fixture");
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

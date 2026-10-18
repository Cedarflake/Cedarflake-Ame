use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, migrate_v29_ambiguous_gap_fixture, publication_identity,
    seed_current_explicit_recovery_claim,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

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
            None,
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

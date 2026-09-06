use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use exif::experimental::Writer;
use exif::{Field, In, Tag, Value};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rusqlite::Connection;
use tempfile::tempdir;

#[cfg(windows)]
use crate::adapters::force_publication_namespace_guard_failure_for_test;
use crate::adapters::remove_persistent_journal_v22_contract_for_test;
use crate::domain::{
    GalleryQuery, LibraryChangeCatchUpEvidence, LibraryChangeCatchUpQueueBatch,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanIssue,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

use super::*;

mod finalization_live;

fn load_test_snapshot(storage: &StoragePaths) -> crate::domain::CatalogSnapshot {
    SqliteCatalog::open(storage.catalog_path.clone())
        .expect("test catalog")
        .load_snapshot(
            100,
            &GalleryQuery::default(),
            "test-query",
            None,
            None,
            None,
        )
        .expect("test snapshot")
}

fn enqueue_root_freshness_unknown(storage: &StoragePaths, root_id: &str, observed_unix_ms: i64) {
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("queue catalog");
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: observed_unix_ms,
                most_recent_observed_unix_ms: observed_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            observed_unix_ms,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("enqueue root freshness gap");
}

fn begin_authoritative_checkpoint(storage: &StoragePaths, request: &ScanRequest) {
    let canonical_root = FileDiscovery::new(&request.root_path)
        .expect("authoritative discovery")
        .canonical_root()
        .expect("authoritative canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    SqliteCatalog::open(storage.catalog_path.clone())
        .expect("authoritative catalog")
        .begin_authoritative_scan(request, &root_id, &canonical_root)
        .expect("begin authoritative checkpoint");
}

fn seed_explicit_recovery_claim(storage: &StoragePaths, root_id: &str) {
    let connection = Connection::open(&storage.catalog_path).expect("explicit recovery catalog");
    let catalog_revision: i64 = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("catalog revision");
    connection
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
               41, 41, '1', '1', 1, 'retry_wait', 41, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The ambiguous historical gap requires an explicit library update',
               ?2, 41, 41
             )",
            rusqlite::params![root_id, catalog_revision],
        )
        .expect("insert explicit recovery gap");
    let gap_change_id = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               created_unix_ms
             ) VALUES (?1, ?2, 1, 'explicit_recovery_required', 41)",
            rusqlite::params![gap_change_id, root_id],
        )
        .expect("insert explicit recovery claim");
}

fn scan_status_and_staging_counts(
    storage: &StoragePaths,
    scan_id: &str,
) -> (String, i64, i64, i64, i64, i64) {
    Connection::open(&storage.catalog_path)
        .expect("scan projection catalog")
        .query_row(
            "SELECT
               (SELECT status FROM scan_runs WHERE id = ?1),
               (SELECT COUNT(*) FROM scan_directory_frontier WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM scan_directory_entries WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM scan_run_catch_up_lineage WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1)",
            [scan_id],
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
        .expect("scan status and staging counts")
}

fn assert_explicit_recovery_claim(storage: &StoragePaths, root_id: &str) {
    let claim: (String, Option<String>, String) = Connection::open(&storage.catalog_path)
        .expect("recovery-claim catalog")
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1 AND claim.consumed_unix_ms IS NULL",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("explicit recovery claim");
    assert_eq!(
        claim,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
        ),
    );
}

#[cfg(windows)]
#[test]
fn explicit_update_recaptures_missing_root_identity_before_consuming_recovery_claim() {
    let source = tempdir().expect("source directory");
    RgbaImage::from_pixel(4, 4, Rgba([20, 40, 60, 255]))
        .save_with_format(source.path().join("image.png"), ImageFormat::Png)
        .expect("source fixture");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let root_path = source.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "missing-proof-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("publish baseline");
    let canonical_root = FileDiscovery::new(&root_path)
        .expect("root discovery")
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    Connection::open(&storage_paths.catalog_path)
        .expect("catalog database")
        .execute(
            "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
            [&root_id],
        )
        .expect("simulate migrated root without durable proof");
    seed_explicit_recovery_claim(&storage_paths, &root_id);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "missing-proof-explicit-update".to_owned(),
            root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("explicit update");

    let (identity_count, unconsumed_claim_count, active_scan_id): (i64, i64, String) =
        Connection::open(&storage_paths.catalog_path)
            .expect("updated catalog")
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_root_publication_namespaces
                    WHERE root_id = ?1 AND root_generation = 1),
                   (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                    WHERE root_id = ?1 AND consumed_unix_ms IS NULL),
                   (SELECT active_scan_id FROM library_roots WHERE id = ?1)",
                [&root_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("updated root authority");
    assert_eq!(identity_count, 1);
    assert_eq!(unconsumed_claim_count, 0);
    assert_eq!(active_scan_id, "missing-proof-explicit-update");
}

#[test]
fn missing_foreground_checkpoint_resume_fails_without_creating_catalog_state_or_events() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut events = Vec::new();

    let error = resume_scan_with_storage(
        ScanRequest {
            scan_id: "missing-checkpoint".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect_err("missing checkpoint must fail closed");

    assert_eq!(error.code, "catalog_scan_resume_missing");
    assert!(events.is_empty());
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let counts: (i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_roots),
               (SELECT COUNT(*) FROM scan_runs),
               (SELECT COUNT(*) FROM scan_directory_frontier)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("catalog counts");
    assert_eq!(counts, (0, 0, 0));
}

#[test]
fn cancellation_is_registered_before_storage_resolution_and_scan_begin() {
    use std::sync::mpsc;
    use std::thread;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: "cancel-before-begin".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let (resolver_reached_tx, resolver_reached_rx) = mpsc::sync_channel(0);
    let (release_resolver_tx, release_resolver_rx) = mpsc::sync_channel(0);
    let catalog_path = storage_paths.catalog_path.clone();
    let scan = thread::spawn(move || {
        let mut events = Vec::new();
        let result = run_scan_with_storage_reason(
            request,
            |event| {
                events.push(event);
                true
            },
            || {
                resolver_reached_tx
                    .send(())
                    .expect("announce storage resolution");
                release_resolver_rx
                    .recv()
                    .expect("release storage resolution");
                Ok(storage_paths)
            },
            FullScanReason::ExplicitUserRequest,
        );
        (result, events)
    });

    resolver_reached_rx
        .recv()
        .expect("scan reached storage resolution");
    assert!(cancel_scan("cancel-before-begin"));
    release_resolver_tx
        .send(())
        .expect("continue cancelled scan");
    let (result, events) = scan.join().expect("cancelled scan thread");
    result.expect("registered cancellation must converge");

    assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. })));
    let status: String = Connection::open(catalog_path)
        .expect("catalog database")
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'cancel-before-begin'",
            [],
            |row| row.get(0),
        )
        .expect("cancelled scan status");
    assert_eq!(status, "cancelled");
}

#[test]
fn cancellation_wins_when_the_event_sink_detaches_at_scan_start() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut cancelled = false;

    run_scan_with_storage(
        ScanRequest {
            scan_id: "cancel-while-start-sink-detaches".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                cancelled = cancel_scan("cancel-while-start-sink-detaches");
                return false;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("cancellation and sink detach converge");

    assert!(cancelled);
    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "cancel-while-start-sink-detaches"),
        ("cancelled".to_owned(), 0, 0, 0, 0, 0),
    );
    assert!(
        load_recoverable_scan_from_path(&storage_paths.catalog_path)
            .expect("load recovery after cancellation")
            .is_none(),
    );
}

#[test]
fn concurrent_foreground_preview_load_and_sync_opens_reuse_one_validated_session() {
    use std::sync::mpsc;
    use std::thread;

    let source_a = tempdir().expect("first source directory");
    let source_b = tempdir().expect("second source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    super::super::catalog_session::reset_catalog_session(&storage_paths.catalog_path);
    crate::adapters::reset_full_schema_validation_count(&storage_paths.catalog_path);
    let root_a_path = source_a.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "shared-session-baseline".to_owned(),
            root_path: root_a_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("publish first-root baseline");
    let canonical_root_a = FileDiscovery::new(&root_a_path)
        .expect("first-root discovery")
        .canonical_root()
        .expect("first-root canonical path")
        .to_string_lossy()
        .into_owned();
    let root_a_id = stable_id("library-root-v1", &canonical_root_a);
    seed_explicit_recovery_claim(&storage_paths, &root_a_id);

    let (started_tx, started_rx) = mpsc::sync_channel(0);
    let (release_tx, release_rx) = mpsc::sync_channel(0);
    let first_storage = storage_paths.clone();
    let first = thread::spawn(move || {
        let mut release_rx = Some(release_rx);
        run_scan_with_storage(
            ScanRequest {
                scan_id: "shared-session-first-update".to_owned(),
                root_path: root_a_path,
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |event| {
                if matches!(event, ScanEvent::Started { .. }) {
                    started_tx.send(()).expect("announce first update begin");
                    release_rx
                        .take()
                        .expect("first update release receiver")
                        .recv_timeout(std::time::Duration::from_secs(10))
                        .expect("release first update");
                }
                true
            },
            first_storage,
        )
    });
    started_rx.recv().expect("first update began");

    drop(
        super::super::catalog_session::open_catalog(
            &storage_paths.catalog_path,
            LibraryChangeLane::Recovery,
        )
        .expect("preview catalog open reuses the validated session"),
    );
    drop(
        super::super::catalog_session::open_catalog_reader(&storage_paths.catalog_path)
            .expect("gallery load reuses the validated session"),
    );
    drop(
        super::super::catalog_session::validated_catalog_session(&storage_paths.catalog_path)
            .expect("synchronization reuses the validated session"),
    );
    let schema_connection =
        Connection::open(&storage_paths.catalog_path).expect("schema-cookie writer");
    let schema_version: i64 = schema_connection
        .query_row("PRAGMA schema_version", [], |row| row.get(0))
        .expect("read schema cookie");
    schema_connection
        .execute_batch(&format!("PRAGMA schema_version = {}", schema_version + 1))
        .expect("invalidate the shared catalog session");
    drop(schema_connection);
    let stale_result = super::super::catalog_session::open_catalog(
        &storage_paths.catalog_path,
        LibraryChangeLane::Recovery,
    );
    Connection::open(&storage_paths.catalog_path)
        .expect("schema-cookie restorer")
        .execute_batch(&format!("PRAGMA schema_version = {schema_version}"))
        .expect("restore schema cookie");
    let stale_error = stale_result
        .err()
        .expect("session renewal must wait for the active foreground scan");
    assert_eq!(
        stale_error.code,
        "catalog_validated_session_stale_while_scan_active",
    );
    run_scan_with_storage(
        ScanRequest {
            scan_id: "shared-session-second-update".to_owned(),
            root_path: source_b.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("second root update");

    let active: (String, String) = Connection::open(&storage_paths.catalog_path)
        .expect("active catalog projection")
        .query_row(
            "SELECT scans.status, claim.consumer_kind
             FROM scan_runs AS scans
             JOIN library_live_gap_recovery_claims AS claim
               ON claim.foreground_scan_id = scans.id
             WHERE scans.id = 'shared-session-first-update'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("first update remains active");
    assert_eq!(active, ("running".to_owned(), "foreground_scan".to_owned()));
    assert_eq!(
        crate::adapters::full_schema_validation_count(&storage_paths.catalog_path),
        1,
        "foreground, preview, gallery, and synchronization opens must share startup validation",
    );
    assert!(
        load_recoverable_scan_from_path(&storage_paths.catalog_path)
            .expect("recovery projection is deferred while a foreground scan is active")
            .is_none(),
    );
    assert!(
        load_paused_scan_from_path(&storage_paths.catalog_path)
            .expect("paused projection is deferred while a foreground scan is active")
            .is_none(),
    );
    let still_active: String = Connection::open(&storage_paths.catalog_path)
        .expect("post-recovery catalog projection")
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'shared-session-first-update'",
            [],
            |row| row.get(0),
        )
        .expect("active scan survives recovery projection calls");
    assert_eq!(still_active, "running");

    release_tx.send(()).expect("release first update");
    first
        .join()
        .expect("first update thread")
        .expect("first update publishes after concurrent opens");
    let completed: (String, i64) = Connection::open(&storage_paths.catalog_path)
        .expect("completed catalog projection")
        .query_row(
            "SELECT scans.status, claim.consumed_unix_ms IS NOT NULL
             FROM scan_runs AS scans
             JOIN library_live_gap_recovery_claims AS claim
               ON claim.foreground_scan_id = scans.id
             WHERE scans.id = 'shared-session-first-update'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("completed first update");
    assert_eq!(completed, ("completed".to_owned(), 1));
}

#[test]
fn invalidated_catalog_session_cannot_recover_a_live_foreground_claim() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let root_path = source.path().to_string_lossy().into_owned();
    let request = |scan_id: &str| ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    run_scan_with_storage(
        request("invalidation-baseline"),
        |_| true,
        storage_paths.clone(),
    )
    .expect("publish baseline");
    let root_id = stable_id(
        "library-root-v1",
        &FileDiscovery::new(&root_path)
            .expect("root discovery")
            .canonical_root()
            .expect("canonical root")
            .to_string_lossy(),
    );
    seed_explicit_recovery_claim(&storage_paths, &root_id);
    crate::adapters::reset_full_schema_validation_count(&storage_paths.catalog_path);
    let mut observed_active_claim = false;

    run_scan_with_storage(
        request("invalidation-active-update"),
        |event| {
            if !matches!(event, ScanEvent::Started { .. }) {
                return true;
            }
            super::super::catalog_session::invalidate_catalog_session(&storage_paths.catalog_path)
                .expect("simulate completed maintenance invalidation");
            let error = super::super::catalog_session::open_catalog(
                &storage_paths.catalog_path,
                LibraryChangeLane::Recovery,
            )
            .err()
            .expect("an invalidated session must wait for the live scan");
            assert_eq!(
                error.code,
                "catalog_validated_session_stale_while_scan_active",
            );
            let active: (String, String) = Connection::open(&storage_paths.catalog_path)
                .expect("active ownership connection")
                .query_row(
                    "SELECT scans.status, claim.consumer_kind
                     FROM scan_runs AS scans
                     JOIN library_live_gap_recovery_claims AS claim
                       ON claim.foreground_scan_id = scans.id
                     WHERE scans.id = 'invalidation-active-update'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("live foreground claim remains owned");
            assert_eq!(active, ("running".to_owned(), "foreground_scan".to_owned()));
            assert_eq!(
                crate::adapters::full_schema_validation_count(&storage_paths.catalog_path),
                0,
            );
            observed_active_claim = true;
            true
        },
        storage_paths.clone(),
    )
    .expect("live foreground update still publishes");
    assert!(observed_active_claim);
    drop(
        super::super::catalog_session::open_catalog(
            &storage_paths.catalog_path,
            LibraryChangeLane::Recovery,
        )
        .expect("renew only after foreground ownership ends"),
    );
    assert_eq!(
        crate::adapters::full_schema_validation_count(&storage_paths.catalog_path),
        1,
    );
}

#[test]
fn unexpected_failure_after_begin_abandons_all_staged_scan_state() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("image.png");
    RgbaImage::from_pixel(4, 4, Rgba([20, 40, 60, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("source fixture");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let root_path = source.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "injected-failure-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("publish failure-test baseline");
    let canonical_root = FileDiscovery::new(&root_path)
        .expect("failure-test discovery")
        .canonical_root()
        .expect("failure-test canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    seed_explicit_recovery_claim(&storage_paths, &root_id);
    let mut failure_installed = false;

    let error = run_scan_with_storage(
        ScanRequest {
            scan_id: "injected-post-begin-failure".to_owned(),
            root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !failure_installed && matches!(event, ScanEvent::Started { .. }) {
                Connection::open(&storage_paths.catalog_path)
                    .expect("failure injector catalog")
                    .execute_batch(
                        "CREATE TRIGGER fail_injected_scan_staging
                         BEFORE INSERT ON scan_directory_entries
                         WHEN NEW.scan_id = 'injected-post-begin-failure'
                         BEGIN
                           SELECT RAISE(ABORT, 'injected scan staging failure');
                         END;",
                    )
                    .expect("install staging failure");
                failure_installed = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect_err("injected staging failure must escape the scan");

    assert_eq!(error.code, "catalog_database_error");
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let projection: (String, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT status FROM scan_runs WHERE id = 'injected-post-begin-failure'),
               (SELECT COUNT(*) FROM scan_directory_frontier
                WHERE scan_id = 'injected-post-begin-failure'),
               (SELECT COUNT(*) FROM scan_directory_entries
                WHERE scan_id = 'injected-post-begin-failure'),
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = 'injected-post-begin-failure'),
               (SELECT COUNT(*) FROM asset_locations
                WHERE scan_id = 'injected-post-begin-failure')",
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
        .expect("failed scan projection");
    assert_eq!(projection, ("failed".to_owned(), 0, 0, 0, 0));
    let restored_claim: (String, Option<String>, String, String) = connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.last_failure_code
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("restored failure claim");
    assert_eq!(
        restored_claim,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            "live_gap_v30_explicit_recovery_required".to_owned(),
        ),
    );
}

#[test]
fn unexpected_first_import_failure_is_abandoned_instead_of_becoming_recoverable() {
    let source = tempdir().expect("source directory");
    RgbaImage::from_pixel(4, 4, Rgba([20, 40, 60, 255]))
        .save_with_format(source.path().join("image.png"), ImageFormat::Png)
        .expect("source fixture");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut failure_installed = false;

    let error = run_scan_with_storage(
        ScanRequest {
            scan_id: "injected-first-import-failure".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !failure_installed && matches!(event, ScanEvent::Started { .. }) {
                Connection::open(&storage_paths.catalog_path)
                    .expect("failure injector catalog")
                    .execute_batch(
                        "CREATE TRIGGER fail_injected_first_import_staging
                         BEFORE INSERT ON scan_directory_entries
                         WHEN NEW.scan_id = 'injected-first-import-failure'
                         BEGIN
                           SELECT RAISE(ABORT, 'injected first-import staging failure');
                         END;",
                    )
                    .expect("install first-import failure");
                failure_installed = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect_err("injected first-import failure must escape the scan");

    assert_eq!(error.code, "catalog_database_error");
    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "injected-first-import-failure"),
        ("failed".to_owned(), 0, 0, 0, 0, 0),
    );
    assert!(
        load_recoverable_scan_from_path(&storage_paths.catalog_path)
            .expect("load recovery after failed first import")
            .is_none(),
    );
}

#[test]
fn scan_failure_reports_primary_and_cleanup_errors_without_masking_either() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("image.png");
    RgbaImage::from_pixel(4, 4, Rgba([20, 40, 60, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("source fixture");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut failures_installed = false;

    let error = run_scan_with_storage(
        ScanRequest {
            scan_id: "injected-primary-and-cleanup-failure".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !failures_installed && matches!(event, ScanEvent::Started { .. }) {
                Connection::open(&storage_paths.catalog_path)
                    .expect("failure injector catalog")
                    .execute_batch(
                        "CREATE TRIGGER fail_injected_scan_staging_and_cleanup
                         BEFORE INSERT ON scan_directory_entries
                         WHEN NEW.scan_id = 'injected-primary-and-cleanup-failure'
                         BEGIN
                           SELECT RAISE(ABORT, 'injected primary scan failure');
                         END;
                         CREATE TRIGGER fail_injected_scan_cleanup
                         BEFORE UPDATE OF status ON scan_runs
                         WHEN OLD.id = 'injected-primary-and-cleanup-failure'
                           AND NEW.status = 'failed'
                         BEGIN
                           SELECT RAISE(ABORT, 'injected scan cleanup failure');
                         END;",
                    )
                    .expect("install scan failures");
                failures_installed = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect_err("primary and cleanup failures must escape the scan");

    assert_eq!(error.code, "scan_failure_cleanup_failed");
    assert!(
        error
            .message
            .contains("Scan failed [catalog_database_error]")
    );
    assert!(error.message.contains("injected primary scan failure"));
    assert!(
        error
            .message
            .contains("cleanup failed [catalog_database_error]")
    );
    assert!(error.message.contains("injected scan cleanup failure"));
    let status: String = Connection::open(&storage_paths.catalog_path)
        .expect("catalog database")
        .query_row(
            "SELECT status FROM scan_runs
             WHERE id = 'injected-primary-and-cleanup-failure'",
            [],
            |row| row.get(0),
        )
        .expect("unrecoverable cleanup projection");
    assert_eq!(status, "running");
}

#[test]
fn published_root_updates_do_not_leave_recoverable_state_when_detached_paused_or_suspended() {
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("image.png");
    RgbaImage::from_pixel(4, 4, Rgba([20, 40, 60, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("source fixture");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let root_path = source.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "published-root-control-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("publish baseline");
    let canonical_root = FileDiscovery::new(&root_path)
        .expect("update-control discovery")
        .canonical_root()
        .expect("update-control canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    seed_explicit_recovery_claim(&storage_paths, &root_id);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "detached-published-root-update".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| !matches!(event, ScanEvent::Started { .. }),
        storage_paths.clone(),
    )
    .expect("detached update converges");
    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "detached-published-root-update"),
        ("cancelled".to_owned(), 0, 0, 0, 0, 0),
    );
    assert_explicit_recovery_claim(&storage_paths, &root_id);

    let mut pause_requested = false;
    let mut pause_events = Vec::new();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "paused-published-root-update".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !pause_requested && matches!(event, ScanEvent::Started { .. }) {
                assert!(pause_scan("paused-published-root-update"));
                pause_requested = true;
            }
            pause_events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("paused update converges");
    assert!(matches!(
        pause_events.last(),
        Some(ScanEvent::Cancelled { .. })
    ));
    assert!(
        !pause_events
            .iter()
            .any(|event| matches!(event, ScanEvent::Paused { .. }))
    );
    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "paused-published-root-update"),
        ("cancelled".to_owned(), 0, 0, 0, 0, 0),
    );
    assert_explicit_recovery_claim(&storage_paths, &root_id);

    let mut suspend_requested = false;
    run_scan_with_storage(
        ScanRequest {
            scan_id: "suspended-published-root-update".to_owned(),
            root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !suspend_requested && matches!(event, ScanEvent::Started { .. }) {
                assert!(suspend_scan("suspended-published-root-update"));
                suspend_requested = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("suspended update converges");
    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "suspended-published-root-update"),
        ("cancelled".to_owned(), 0, 0, 0, 0, 0),
    );
    assert_explicit_recovery_claim(&storage_paths, &root_id);
}

#[test]
fn detached_first_import_remains_the_single_recoverable_foreground_scan() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "detached-first-import".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| !matches!(event, ScanEvent::Started { .. }),
        storage_paths.clone(),
    )
    .expect("detached first import retains its checkpoint");

    assert_eq!(
        scan_status_and_staging_counts(&storage_paths, "detached-first-import").0,
        "running",
    );
    let mut catalog = super::super::catalog_session::open_catalog(
        &storage_paths.catalog_path,
        LibraryChangeLane::Recovery,
    )
    .expect("reuse validated catalog session");
    let recoverable = catalog
        .load_single_recoverable_foreground_scan()
        .expect("load first-import recovery")
        .expect("first import remains recoverable");
    assert_eq!(recoverable.scan_id, "detached-first-import");
    catalog
        .abandon_scan("detached-first-import", "cancelled", 0)
        .expect("clean up retained first import");
}

fn orientation_jpeg_fixture(orientation: u16, width: u32, height: u32) -> Vec<u8> {
    let mut exif_writer = Writer::new();
    let orientation_field = Field {
        tag: Tag::Orientation,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![orientation]),
    };
    exif_writer.push_field(&orientation_field);
    let mut exif = Cursor::new(Vec::new());
    exif_writer.write(&mut exif, false).expect("encode EXIF");

    let pixels = vec![96; usize::try_from(width * height * 3).expect("pixel count")];
    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new(&mut jpeg);
    encoder
        .set_exif_metadata(exif.into_inner())
        .expect("set orientation EXIF");
    encoder
        .encode(&pixels, width, height, ExtendedColorType::Rgb8)
        .expect("encode orientation JPEG");
    jpeg
}

#[test]
fn validates_scan_limits() {
    let request = ScanRequest {
        scan_id: "scan-1".to_owned(),
        root_path: "C:\\pictures".to_owned(),
        max_items: Some(0),
        max_entries: Some(100),
        preview_edge: 320,
    };

    let error = validate_request(&request).expect_err("zero limit must be rejected");
    assert_eq!(error.code, "item_limit_invalid");
}

#[test]
fn stable_ids_change_with_namespace() {
    assert_ne!(
        stable_id("library-root-v1", "C:\\pictures"),
        stable_id("asset-location-v1", "C:\\pictures"),
    );
}

#[test]
fn scan_issue_presentation_path_hides_windows_device_prefixes() {
    let issue = user_visible_issue(ScanIssue {
        path: Some(r"\\?\C:\Pictures\broken.png".to_owned()),
        code: "fixture".to_owned(),
        message: "fixture".to_owned(),
    });

    assert_eq!(issue.path.as_deref(), Some(r"C:\Pictures\broken.png"));
}

#[test]
fn discovery_accepts_png_magic_with_wrong_extension() {
    let directory = tempdir().expect("temporary directory");
    let file_path = directory.path().join("image.data");
    fs::write(&file_path, b"\x89PNG\r\n\x1a\nrest").expect("fixture write");
    let discovery =
        FileDiscovery::new(&directory.path().to_string_lossy()).expect("valid discovery root");

    let accepted = discovery
        .entry_paths_in_directory("")
        .expect("directory entries")
        .map(|relative_path| discovery.visit_relative_path(&relative_path))
        .any(|visit| matches!(visit.outcome, FileVisitOutcome::File(_)));

    assert!(accepted);
}

#[test]
fn completed_scan_publishes_metadata_then_materializes_an_external_preview() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("像素.data");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
    let preview_root = storage.path().join("previews");
    let storage_paths = StoragePaths {
        catalog_path: catalog_path.clone(),
        preview_root: preview_root.clone(),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: "end-to-end-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("completed scan");

    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    let started_root_path = events
        .iter()
        .find_map(|event| match event {
            ScanEvent::Started { root_path, .. } => Some(root_path),
            _ => None,
        })
        .expect("started event");
    assert_eq!(started_root_path, &user_visible_path(started_root_path));
    let pending_asset = events
        .iter()
        .find_map(|event| match event {
            ScanEvent::AssetDiscovered { asset, .. } => Some((**asset).clone()),
            _ => None,
        })
        .expect("asset event");
    assert!(pending_asset.preview_path.is_empty());
    assert!(matches!(
        pending_asset.preview_status,
        PreviewStatus::Pending
    ));
    let finalization_progress = events
        .iter()
        .filter_map(|event| match event {
            ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } => Some((*validated_items, *total_items)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(finalization_progress, [(0, 1), (1, 1)]);
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 1,
            issue_count: 0,
            ..
        })
    ));
    let published_asset = load_test_snapshot(&storage_paths)
        .assets
        .into_iter()
        .find(|asset| asset.location_id == pending_asset.location_id)
        .expect("published asset");
    assert!(published_asset.source_generation > 0);

    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: published_asset.location_id.clone(),
            expected_root_id: published_asset.root_id.clone(),
            expected_scan_id: published_asset.scan_id.clone(),
            expected_source_revision: published_asset.source_revision.clone(),
            expected_source_generation: published_asset.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("materialized preview");
    let preview_path = PathBuf::from(&previewed.preview_path);
    assert!(matches!(previewed.preview_status, PreviewStatus::Ready));
    assert!(preview_path.starts_with(&preview_root));
    assert!(!preview_path.starts_with(source.path()));
    assert!(preview_path.is_file());
    assert_eq!(
        fs::read(&source_path).expect("source after preview"),
        original_bytes,
    );

    fs::write(&preview_path, b"truncated cached preview").expect("corrupt cached preview");
    let automatically_repaired = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: previewed.location_id.clone(),
            expected_root_id: previewed.root_id.clone(),
            expected_scan_id: previewed.scan_id.clone(),
            expected_source_revision: previewed.source_revision.clone(),
            expected_source_generation: previewed.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("ordinary ready request repairs corrupt cache");
    assert!(matches!(
        automatically_repaired.preview_status,
        PreviewStatus::Ready
    ));
    image::open(&preview_path).expect("automatically repaired preview decodes");
    assert_eq!(
        fs::read(&source_path).expect("source after automatic repair"),
        original_bytes,
    );

    let repaired = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: previewed.location_id,
            expected_root_id: previewed.root_id.clone(),
            expected_scan_id: previewed.scan_id.clone(),
            expected_source_revision: previewed.source_revision.clone(),
            expected_source_generation: previewed.source_generation,
            preview_edge: 256,
            retry_failed: true,
            protected_location_ids: Vec::new(),
        },
        storage_paths,
    )
    .expect("repair ready preview");
    assert!(matches!(repaired.preview_status, PreviewStatus::Ready));
    image::open(&preview_path).expect("repaired preview decodes");
    assert_eq!(
        fs::read(&source_path).expect("source after repair"),
        original_bytes,
    );

    let connection = Connection::open(catalog_path).expect("published catalog");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'end-to-end-scan'",
            [],
            |row| row.get(0),
        )
        .expect("scan status");
    let active_scan: String = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan");
    let artifact: (i64, i64, i64, String, i64, String) = connection
        .query_row(
            "SELECT source_file_size, size_bucket, encoded_width,
                    lifecycle_state, byte_size, artifact_path
             FROM preview_artifacts",
            [],
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
        .expect("preview artifact evidence");
    assert_eq!(status, "completed");
    assert_eq!(active_scan, "end-to-end-scan");
    assert_eq!(
        artifact.0,
        i64::try_from(original_bytes.len()).expect("source size")
    );
    assert_eq!(artifact.1, 256);
    assert!(artifact.2 > 0);
    assert_eq!(artifact.3, "ready");
    assert!(artifact.4 > 0);
    assert_eq!(PathBuf::from(artifact.5), preview_path);
}

#[cfg(windows)]
#[test]
fn publication_guard_capability_failure_preserves_catalog_and_requires_recovery() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("source.png");
    RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let root_path = source.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "publication-guard-baseline".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("baseline scan");
    let canonical_root = FileDiscovery::new(&root_path)
        .expect("root discovery")
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    connection
        .execute_batch(&format!(
            "UPDATE library_persistent_journal_root_state
             SET protocol_version = 5, contract_version = 1,
                 capability_state = 'supported', continuity_state = 'current',
                 last_failure_code = NULL, last_failure_message = NULL,
                 updated_unix_ms = 1
             WHERE root_id = '{root_id}' AND root_generation = 1;
             INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state, updated_unix_ms
             ) VALUES (
               '{root_id}', 1, 'controlled-volume', '1',
               3, X'01010101010101010101010101010101', '7',
               '10', '10', 0, 5, 1, 'current', 1
             )
             ON CONFLICT(root_id) DO UPDATE SET
               root_generation = excluded.root_generation,
               volume_guid = excluded.volume_guid,
               volume_serial = excluded.volume_serial,
               root_reference_version = excluded.root_reference_version,
               root_file_reference = excluded.root_file_reference,
               journal_id = excluded.journal_id,
               next_unread_usn = excluded.next_unread_usn,
               captured_exclusive_end = excluded.captured_exclusive_end,
               covered_catalog_revision = excluded.covered_catalog_revision,
               protocol_version = excluded.protocol_version,
               contract_version = excluded.contract_version,
               continuity_state = excluded.continuity_state,
               last_failure_code = NULL,
               last_failure_message = NULL,
               updated_unix_ms = excluded.updated_unix_ms;"
        ))
        .expect("current journal projection");
    let revision_before = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("catalog revision");
    drop(connection);

    let _forced_failure = force_publication_namespace_guard_failure_for_test();
    let error = run_scan_with_storage(
        ScanRequest {
            scan_id: "publication-guard-failure".to_owned(),
            root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect_err("publication guard capability must fail closed");
    assert_eq!(error.code, "root_publication_namespace_guard_unsupported");

    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let projection = connection
        .query_row(
            "SELECT roots.active_scan_id, failed.status,
                    journal.continuity_state, checkpoint.continuity_state,
                    checkpoint.last_failure_code,
                    (SELECT revision FROM catalog_state),
                    (SELECT COUNT(*) FROM asset_locations WHERE scan_id = roots.active_scan_id),
                    (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                     WHERE scan_id = 'publication-guard-failure'),
                    (SELECT COUNT(*) FROM library_recovery_authorities)
             FROM library_roots AS roots
             JOIN scan_runs AS failed ON failed.root_id = roots.id
               AND failed.id = 'publication-guard-failure'
             JOIN library_persistent_journal_root_state AS journal
               ON journal.root_id = roots.id AND journal.root_generation = 1
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = roots.id AND checkpoint.root_generation = 1
             WHERE roots.id = ?1",
            [&root_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        )
        .expect("failed publication projection");
    assert_eq!(projection.0, "publication-guard-baseline");
    assert_eq!(projection.1, "failed");
    assert_eq!(projection.2, "recovery_required");
    assert_eq!(projection.3, "recovery_required");
    assert_eq!(
        projection.4.as_deref(),
        Some("root_publication_namespace_guard_unsupported")
    );
    assert_eq!(projection.5, revision_before);
    assert_eq!(projection.6, 1);
    assert_eq!(projection.7, 0);
    assert_eq!(projection.8, 0);
}

#[test]
fn bidirectional_full_scan_catch_up_preserves_assets_and_invalidates_moved_previews() {
    assert_bidirectional_full_scan_catch_up("full-handoff", false, false);
}

#[test]
fn prerelease_handoff_migration_deduplicates_hard_links_and_replaces_unversioned_identity() {
    assert_bidirectional_full_scan_catch_up("full-handoff-recovery", true, true);
}

fn assert_bidirectional_full_scan_catch_up(
    scenario: &str,
    inject_hard_link_location: bool,
    repair_missing_handoff_preview: bool,
) {
    let source = tempdir().expect("source directory");
    let destination = tempdir().expect("destination directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("source.png");
    let destination_path = destination.path().join("destination.png");
    let moved_to_destination_path = destination.path().join("source.png");
    let moved_to_source_path = source.path().join("destination.png");
    let source_initial_scan_id = format!("{scenario}-source-initial");
    let destination_initial_scan_id = format!("{scenario}-destination-initial");
    let source_recovery_scan_id = format!("{scenario}-source-recovery");
    let destination_recovery_scan_id = format!("{scenario}-destination-recovery");
    RgbaImage::from_pixel(8, 6, Rgba([70, 100, 130, 255]))
        .save(&source_path)
        .expect("source fixture image");
    RgbaImage::from_pixel(7, 5, Rgba([130, 100, 70, 255]))
        .save(&destination_path)
        .expect("destination fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let source_root_path = source.path().to_string_lossy().into_owned();
    let destination_root_path = destination.path().to_string_lossy().into_owned();
    run_scan_with_storage(
        ScanRequest {
            scan_id: source_initial_scan_id.clone(),
            root_path: source_root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial source scan");
    run_scan_with_storage(
        ScanRequest {
            scan_id: destination_initial_scan_id,
            root_path: destination_root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial destination scan");
    let initial = load_test_snapshot(&storage_paths);
    let initial_source_asset = initial
        .assets
        .iter()
        .find(|asset| asset.relative_path == "source.png")
        .expect("initial source asset");
    let initial_destination_asset = initial
        .assets
        .iter()
        .find(|asset| asset.relative_path == "destination.png")
        .expect("initial destination asset");
    let source_asset_id = initial_source_asset.asset_id.clone();
    let source_location_id = initial_source_asset.location_id.clone();
    let destination_asset_id = initial_destination_asset.asset_id.clone();
    let destination_location_id = initial_destination_asset.location_id.clone();
    let source_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: source_location_id.clone(),
            expected_root_id: initial_source_asset.root_id.clone(),
            expected_scan_id: initial_source_asset.scan_id.clone(),
            expected_source_revision: initial_source_asset.source_revision.clone(),
            expected_source_generation: initial_source_asset.source_generation,
            preview_edge: 128,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("source preview");
    let destination_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: destination_location_id.clone(),
            expected_root_id: initial_destination_asset.root_id.clone(),
            expected_scan_id: initial_destination_asset.scan_id.clone(),
            expected_source_revision: initial_destination_asset.source_revision.clone(),
            expected_source_generation: initial_destination_asset.source_generation,
            preview_edge: 128,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("destination preview");
    assert!(matches!(
        source_preview.preview_status,
        PreviewStatus::Ready
    ));
    assert!(matches!(
        destination_preview.preview_status,
        PreviewStatus::Ready
    ));
    if inject_hard_link_location {
        let hard_link_location_id = "000-full-handoff-source-hard-link";
        let hard_link_absolute_path = source
            .path()
            .join("source-hard-link.png")
            .to_string_lossy()
            .into_owned();
        let connection = Connection::open(&storage_paths.catalog_path).expect("hard-link catalog");
        connection
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
                 SELECT scan_id, asset_id, ?1, root_id, ?2, 'source-hard-link.png',
                        preview_path, file_size, created_unix_ms, modified_unix_ms,
                        file_local_time, '', 'source-hard-link.png', width, height,
                        preview_status, preview_issue_code, preview_issue_message,
                         metadata_engine_id, metadata_engine_version, capture_local_time,
                         capture_offset_minutes, capture_time_source, capture_raw_value,
                         file_identity_scheme, file_identity_value, source_revision_token,
                         source_generation
                 FROM asset_locations
                 WHERE scan_id = ?4 AND location_id = ?3",
                rusqlite::params![
                    hard_link_location_id,
                    hard_link_absolute_path,
                    source_location_id,
                    source_initial_scan_id,
                ],
            )
            .expect("duplicate hard-link identity location");
        connection
            .execute(
                "INSERT OR IGNORE INTO preview_artifact_locations(artifact_key, location_id)
                 SELECT artifact_key, ?1 FROM preview_artifact_locations WHERE location_id = ?2",
                rusqlite::params![hard_link_location_id, source_location_id],
            )
            .expect("duplicate hard-link preview owner");
        let duplicate_identity_count = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM asset_locations AS candidate
                 WHERE candidate.scan_id = ?2
                   AND (candidate.file_identity_scheme, candidate.file_identity_value) = (
                     SELECT original.file_identity_scheme, original.file_identity_value
                     FROM asset_locations AS original
                     WHERE original.scan_id = ?2
                       AND original.location_id = ?1
                   )",
                rusqlite::params![source_location_id, source_initial_scan_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("duplicate hard-link identity count");
        assert_eq!(duplicate_identity_count, 2);
    }

    fs::rename(&source_path, &moved_to_destination_path).expect("source-first move");
    fs::rename(&destination_path, &moved_to_source_path).expect("destination-first move");
    let source_canonical = FileDiscovery::new(&source_root_path)
        .expect("source discovery")
        .canonical_root()
        .expect("source canonical root")
        .to_string_lossy()
        .into_owned();
    let destination_canonical = FileDiscovery::new(&destination_root_path)
        .expect("destination discovery")
        .canonical_root()
        .expect("destination canonical root")
        .to_string_lossy()
        .into_owned();
    let source_root_id = stable_id("library-root-v1", &source_canonical);
    let destination_root_id = stable_id("library-root-v1", &destination_canonical);
    let generation = LibraryRootGeneration::initial();
    let intent = |root_id: String| LibraryChangeIntent {
        root_id,
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
    };
    let mut catch_up_catalog =
        SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catch-up catalog");
    for watermark in 0..64 {
        let evidence = LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: format!("volume|44|{watermark}"),
        };
        catch_up_catalog
            .enqueue_library_change_catch_up_batches(
                &[
                    LibraryChangeCatchUpQueueBatch {
                        intents: vec![intent(source_root_id.clone())],
                        evidence: Some(evidence.clone()),
                    },
                    LibraryChangeCatchUpQueueBatch {
                        intents: vec![intent(destination_root_id.clone())],
                        evidence: Some(evidence),
                    },
                ],
                1_000 + watermark,
                LibraryChangeQueuePolicy::default(),
            )
            .expect("atomic cross-root catch-up enqueue");
    }
    drop(catch_up_catalog);

    begin_authoritative_checkpoint(
        &storage_paths,
        &ScanRequest {
            scan_id: source_recovery_scan_id.clone(),
            root_path: source_root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
    );
    run_scan_with_storage_reason(
        ScanRequest {
            scan_id: source_recovery_scan_id,
            root_path: source_root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("source-first authoritative scan");
    let source_publication = Connection::open(&storage_paths.catalog_path)
        .expect("source publication catalog")
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs),
               (SELECT COUNT(*) FROM library_change_scan_handoff_batches),
               (SELECT COUNT(*) FROM library_change_scan_handoff_items),
               (SELECT COUNT(*) FROM library_change_scan_handoff_lineage),
               (SELECT COUNT(*) FROM library_change_queue
                 WHERE root_id = ?1 AND status IN ('pending', 'leased', 'retry_wait')),
               (SELECT COUNT(*) FROM scan_run_catch_up_lineage)",
            [&destination_root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .expect("source publication evidence");
    assert_eq!(source_publication, (0, 1, 1, 64, 1, 0));
    if repair_missing_handoff_preview {
        let connection =
            Connection::open(&storage_paths.catalog_path).expect("prerelease preview catalog");
        assert_eq!(
            connection
                .execute(
                    "DELETE FROM preview_artifacts WHERE artifact_path = ?1",
                    [&source_preview.preview_path],
                )
                .expect("restore prerelease missing handoff preview"),
            1
        );
        remove_persistent_journal_v22_contract_for_test(&connection);
        connection
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
        drop(connection);
        drop(
            SqliteCatalog::open(storage_paths.catalog_path.clone())
                .expect("repair prerelease missing handoff preview"),
        );
        let migrated_handoff_evidence = Connection::open(&storage_paths.catalog_path)
            .expect("migrated handoff catalog")
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_change_catch_up_handoffs),
                   (SELECT COUNT(*) FROM library_change_scan_handoff_batches),
                   (SELECT COUNT(*) FROM library_change_scan_handoff_items)",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .expect("migrated handoff evidence");
        assert_eq!(migrated_handoff_evidence, (0, 0, 0));
    }
    begin_authoritative_checkpoint(
        &storage_paths,
        &ScanRequest {
            scan_id: destination_recovery_scan_id.clone(),
            root_path: destination_root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
    );
    run_scan_with_storage_reason(
        ScanRequest {
            scan_id: destination_recovery_scan_id,
            root_path: destination_root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("destination authoritative scan");

    let final_snapshot = load_test_snapshot(&storage_paths);
    let moved_source = final_snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "source.png")
        .expect("source-first moved asset");
    let moved_destination = final_snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "destination.png")
        .expect("destination-first moved asset");
    assert_eq!(final_snapshot.assets.len(), 2);
    if repair_missing_handoff_preview {
        assert_ne!(moved_source.asset_id, source_asset_id);
    } else {
        assert_eq!(moved_source.asset_id, source_asset_id);
    }
    assert_ne!(moved_source.asset_id, destination_asset_id);
    assert_ne!(moved_source.location_id, source_location_id);
    assert_eq!(moved_destination.asset_id, destination_asset_id);
    assert_ne!(moved_destination.location_id, destination_location_id);
    assert!(moved_source.preview_path.is_empty());
    assert!(matches!(
        moved_source.preview_status,
        PreviewStatus::Pending
    ));
    assert_ne!(moved_source.source_revision, source_preview.source_revision);
    assert!(moved_destination.preview_path.is_empty());
    assert!(matches!(
        moved_destination.preview_status,
        PreviewStatus::Pending
    ));
    assert_ne!(
        moved_destination.source_revision,
        destination_preview.source_revision
    );
    if repair_missing_handoff_preview {
        assert!(moved_source.source_generation > 0);
        assert!(moved_destination.source_generation > 0);
    } else {
        assert!(moved_source.source_generation > source_preview.source_generation);
        assert!(moved_destination.source_generation > destination_preview.source_generation);
    }
    let connection = Connection::open(storage_paths.catalog_path).expect("final catalog");
    let terminal_evidence: (i64, i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs),
               (SELECT COUNT(*) FROM library_change_scan_handoff_batches),
               (SELECT COUNT(*) FROM library_change_scan_handoff_items),
               (SELECT COUNT(*) FROM library_change_scan_handoff_lineage),
               (SELECT COUNT(*) FROM scan_run_catch_up_lineage),
               (SELECT COUNT(*) FROM library_change_queue
                 WHERE status IN ('pending', 'leased', 'retry_wait'))",
            [],
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
        .expect("terminal handoff evidence");
    assert_eq!(terminal_evidence, (0, 0, 0, 0, 0, 0));
}

#[test]
fn final_validation_crosses_windows_without_skipping_or_repeating_assets() {
    const ASSET_COUNT: u64 = 257;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let fixture_path = storage.path().join("fixture.png");
    RgbaImage::from_pixel(1, 1, Rgba([24, 48, 96, 255]))
        .save(&fixture_path)
        .expect("fixture image");
    let fixture = fs::read(&fixture_path).expect("fixture bytes");
    for index in 0..ASSET_COUNT {
        fs::write(
            source.path().join(format!("asset-{index:03}.png")),
            &fixture,
        )
        .expect("copy fixture image");
    }
    let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
    let request = ScanRequest {
        scan_id: "windowed-final-validation".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed windowed scan");

    let finalization_progress = events
        .iter()
        .filter_map(|event| match event {
            ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } => Some((*validated_items, *total_items)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        finalization_progress,
        [(0, 257), (128, 257), (256, 257), (257, 257)]
    );
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: ASSET_COUNT,
            issue_count: 0,
            ..
        })
    ));

    let connection = Connection::open(catalog_path).expect("published catalog");
    let published_locations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = 'windowed-final-validation'",
            [],
            |row| row.get(0),
        )
        .expect("published asset count");
    assert_eq!(published_locations, 257);
}

#[test]
fn failed_preview_requires_an_explicit_retry_before_reading_source_again() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("source.png");
    RgbaImage::from_pixel(16, 12, Rgba([40, 80, 120, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 1,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "failed-preview-retry".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("completed scan");
    let location = load_test_snapshot(&storage_paths).assets[0].clone();
    let location_id = location.location_id.clone();

    let failed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: location_id.clone(),
            expected_root_id: location.root_id.clone(),
            expected_scan_id: location.scan_id.clone(),
            expected_source_revision: location.source_revision.clone(),
            expected_source_generation: location.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("failed preview state");
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert_eq!(
        failed.preview_issue_code.as_deref(),
        Some("preview_cache_budget_exceeded")
    );

    fs::remove_file(&source_path).expect("remove source after failure");
    let retained = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: location_id.clone(),
            expected_root_id: location.root_id.clone(),
            expected_scan_id: location.scan_id.clone(),
            expected_source_revision: location.source_revision.clone(),
            expected_source_generation: location.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("retained failure state");
    assert_eq!(
        retained.preview_issue_code.as_deref(),
        Some("preview_cache_budget_exceeded")
    );

    let retry_error = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id,
            expected_root_id: location.root_id.clone(),
            expected_scan_id: location.scan_id.clone(),
            expected_source_revision: location.source_revision.clone(),
            expected_source_generation: location.source_generation,
            preview_edge: 256,
            retry_failed: true,
            protected_location_ids: Vec::new(),
        },
        storage_paths,
    )
    .expect_err("stale retry is superseded");
    assert_eq!(retry_error.code, "preview_request_superseded");
}

#[test]
fn budget_exhaustion_reclaims_an_unprotected_preview_and_retries_once() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let first_source = source.path().join("first.png");
    let second_source = source.path().join("second.png");
    let pixels = RgbaImage::from_pixel(64, 48, Rgba([80, 120, 200, 255]));
    pixels
        .save_with_format(&first_source, ImageFormat::Png)
        .expect("first source");
    pixels
        .save_with_format(&second_source, ImageFormat::Png)
        .expect("second source");
    let first_source_bytes = fs::read(&first_source).expect("first source bytes");
    let second_source_bytes = fs::read(&second_source).expect("second source bytes");
    let mut storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "preview-reclamation-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("completed scan");
    let snapshot = load_test_snapshot(&storage_paths);
    let first = snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "first.png")
        .expect("first asset");
    let second = snapshot
        .assets
        .iter()
        .find(|asset| asset.relative_path == "second.png")
        .expect("second asset");

    let first_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: first.location_id.clone(),
            expected_root_id: first.root_id.clone(),
            expected_scan_id: first.scan_id.clone(),
            expected_source_revision: first.source_revision.clone(),
            expected_source_generation: first.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("first preview");
    let first_preview_path = PathBuf::from(&first_preview.preview_path);
    let first_preview_size = first_preview_path
        .metadata()
        .expect("first preview metadata")
        .len();
    storage_paths.preview_budget_bytes = first_preview_size.saturating_add(1);

    let second_preview = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: second.location_id.clone(),
            expected_root_id: second.root_id.clone(),
            expected_scan_id: second.scan_id.clone(),
            expected_source_revision: second.source_revision.clone(),
            expected_source_generation: second.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: vec![second.location_id.clone()],
        },
        storage_paths.clone(),
    )
    .expect("second preview after reclamation");

    assert!(matches!(
        second_preview.preview_status,
        PreviewStatus::Ready
    ));
    assert!(!first_preview_path.exists());
    let catalog = SqliteCatalog::open(storage_paths.catalog_path).expect("catalog");
    let reclaimed = catalog
        .load_active_location(&first.location_id)
        .expect("reclaimed location query")
        .expect("reclaimed location");
    assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
    assert_eq!((reclaimed.width, reclaimed.height), (64, 48));
    assert_eq!(
        fs::read(first_source).expect("first source after"),
        first_source_bytes
    );
    assert_eq!(
        fs::read(second_source).expect("second source after"),
        second_source_bytes,
    );
}

#[test]
fn unchanged_file_is_reinspected_when_metadata_engine_identity_changes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("plain.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-engine-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET metadata_engine_id = 'old-engine', metadata_engine_version = '1',
                     capture_local_time = '1999-01-01T00:00:00.000000000',
                     capture_offset_minutes = NULL, capture_time_source = 'exif_datetime',
                     capture_raw_value = '1999:01:01 00:00:00||'
                 WHERE scan_id = 'metadata-engine-first'",
            [],
        )
        .expect("replace active evidence with old-engine fixture");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-engine-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("second scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (scan_id, engine_id, engine_version, capture_local_time): (
        String,
        String,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT locations.scan_id, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("active metadata evidence");

    assert_eq!(scan_id, "metadata-engine-second");
    assert_eq!(engine_id, "kamadak-exif");
    assert_eq!(engine_version, "0.6.1+ame-orientation-1");
    assert!(capture_local_time.is_none());
    assert_eq!(
        fs::read(&source_path).expect("source after reinspection"),
        original_bytes,
    );
}

#[test]
fn unchanged_file_reuses_capture_evidence_from_the_active_metadata_engine() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("plain.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-reuse-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET capture_local_time = '1999-01-01T00:00:00.000000000',
                     capture_offset_minutes = NULL, capture_time_source = 'exif_datetime',
                     capture_raw_value = '1999:01:01 00:00:00||'
                 WHERE scan_id = 'metadata-reuse-first'",
            [],
        )
        .expect("install compatible evidence fixture");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "metadata-reuse-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("second scan");

    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (engine_id, engine_version, capture_local_time): (String, String, Option<String>) =
        connection
            .query_row(
                "SELECT locations.metadata_engine_id,
                            locations.metadata_engine_version, locations.capture_local_time
                     FROM library_roots AS roots
                     JOIN asset_locations AS locations
                       ON locations.scan_id = roots.active_scan_id
                     LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("active metadata evidence");

    assert_eq!(engine_id, "kamadak-exif");
    assert_eq!(engine_version, "0.6.1+ame-orientation-1");
    assert_eq!(
        capture_local_time.as_deref(),
        Some("1999-01-01T00:00:00.000000000")
    );
}

#[test]
fn rescan_repairs_legacy_orientation_dimensions_and_invalidates_old_preview() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("portrait.jpg");
    let original_bytes = orientation_jpeg_fixture(6, 80, 60);
    fs::write(&source_path, &original_bytes).expect("write orientation source");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };

    run_scan_with_storage(
        ScanRequest {
            scan_id: "orientation-recovery-first".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("first orientation scan");

    fs::create_dir_all(&storage_paths.preview_root).expect("preview root");
    let legacy_preview_path = storage_paths.preview_root.join("legacy-thumbnail-v1.jpg");
    RgbImage::from_pixel(80, 60, Rgb([96, 96, 96]))
        .save(&legacy_preview_path)
        .expect("legacy preview fixture");
    let legacy_preview_text = legacy_preview_path.to_string_lossy().into_owned();
    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    connection
        .execute(
            "UPDATE asset_locations
                 SET width = 80, height = 60,
                     metadata_engine_version = '0.6.1',
                     preview_path = ?1, preview_status = 'ready'
                 WHERE scan_id = 'orientation-recovery-first'",
            [&legacy_preview_text],
        )
        .expect("install legacy orientation evidence");
    drop(connection);

    run_scan_with_storage(
        ScanRequest {
            scan_id: "orientation-recovery-second".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("orientation recovery rescan");

    let recovered = load_test_snapshot(&storage_paths)
        .assets
        .into_iter()
        .next()
        .expect("recovered asset");
    assert_eq!((recovered.width, recovered.height), (60, 80));
    assert_eq!(recovered.metadata_engine_version, "0.6.1+ame-orientation-1");
    assert!(matches!(recovered.preview_status, PreviewStatus::Pending));
    assert!(recovered.preview_path.is_empty());

    let mut catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
    let manifest = catalog
        .load_gallery_layout_manifest_chunk(
            100,
            &GalleryQuery::default(),
            "orientation-recovery-query",
            None,
        )
        .expect("orientation-corrected manifest");
    assert_eq!(manifest.aspect_ratio_milli, [750]);
    drop(catalog);

    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: recovered.location_id,
            expected_root_id: recovered.root_id.clone(),
            expected_scan_id: recovered.scan_id.clone(),
            expected_source_revision: recovered.source_revision.clone(),
            expected_source_generation: recovered.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("materialize recovered preview");
    assert_eq!((previewed.width, previewed.height), (60, 80));
    assert_ne!(PathBuf::from(&previewed.preview_path), legacy_preview_path);
    assert!(crate::adapters::is_current_preview_artifact(
        &previewed.preview_path
    ));
    assert_eq!(
        image::image_dimensions(&previewed.preview_path).expect("preview dimensions"),
        (192, 256)
    );
    assert_eq!(
        fs::read(&source_path).expect("source after recovery"),
        original_bytes,
    );
}

#[cfg(windows)]
#[test]
fn rescans_reconcile_rename_edit_replacement_and_removal_without_stale_rows() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let original_path = source.path().join("original.png");
    let moved_path = source.path().join("moved.png");
    let retained_path = source.path().join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 120, 200, 255]))
        .save_with_format(&original_path, ImageFormat::Png)
        .expect("original image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let scan = |scan_id: &str| {
        run_scan_with_storage(
            ScanRequest {
                scan_id: scan_id.to_owned(),
                root_path: source.path().to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            },
            |_| true,
            storage_paths.clone(),
        )
        .expect("completed reconciliation scan");
    };

    scan("reconcile-first");
    let first = load_test_snapshot(&storage_paths);
    assert_eq!(first.assets.len(), 1);
    let first_asset = first.assets[0].clone();
    assert!(first_asset.file_identity.is_some());
    let previewed = crate::application::preview::materialize_preview_with_storage(
        crate::domain::PreviewRequest {
            location_id: first_asset.location_id.clone(),
            expected_root_id: first_asset.root_id.clone(),
            expected_scan_id: first_asset.scan_id.clone(),
            expected_source_revision: first_asset.source_revision.clone(),
            expected_source_generation: first_asset.source_generation,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        },
        storage_paths.clone(),
    )
    .expect("first preview");
    assert!(matches!(previewed.preview_status, PreviewStatus::Ready));

    fs::rename(&original_path, &moved_path).expect("rename image");
    scan("reconcile-renamed");
    let renamed = load_test_snapshot(&storage_paths);
    assert_eq!(renamed.assets.len(), 1);
    let renamed_asset = &renamed.assets[0];
    assert_eq!(renamed_asset.asset_id, first_asset.asset_id);
    assert_eq!(renamed_asset.relative_path, "moved.png");
    assert!(renamed_asset.preview_path.is_empty());
    assert!(matches!(
        renamed_asset.preview_status,
        PreviewStatus::Pending
    ));
    assert_ne!(renamed_asset.source_revision, previewed.source_revision);
    assert!(renamed_asset.source_generation > previewed.source_generation);

    RgbaImage::from_pixel(17, 9, Rgba([10, 20, 30, 255]))
        .save_with_format(&moved_path, ImageFormat::Png)
        .expect("edit image in place");
    scan("reconcile-edited");
    let edited = load_test_snapshot(&storage_paths);
    assert_eq!(edited.assets.len(), 1);
    let edited_asset = &edited.assets[0];
    assert_eq!(edited_asset.asset_id, first_asset.asset_id);
    assert!(edited_asset.preview_path.is_empty());
    assert!(matches!(
        edited_asset.preview_status,
        PreviewStatus::Pending
    ));
    assert_eq!((edited_asset.width, edited_asset.height), (17, 9));

    fs::rename(&moved_path, &retained_path).expect("retain old file identity");
    RgbaImage::from_pixel(3, 2, Rgba([220, 100, 40, 255]))
        .save_with_format(&moved_path, ImageFormat::Png)
        .expect("replacement image");
    scan("reconcile-replaced");
    let replaced = load_test_snapshot(&storage_paths);
    assert_eq!(replaced.assets.len(), 2);
    let replacement = replaced
        .assets
        .iter()
        .find(|asset| asset.relative_path == "moved.png")
        .expect("replacement location");
    let retained = replaced
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("retained location");
    assert_ne!(replacement.asset_id, first_asset.asset_id);
    assert_eq!(retained.asset_id, first_asset.asset_id);

    let replacement_asset_id = replacement.asset_id.clone();
    fs::remove_file(&retained_path).expect("remove retained image");
    scan("reconcile-removed");
    let removed = load_test_snapshot(&storage_paths);
    assert_eq!(removed.assets.len(), 1);
    let remaining = &removed.assets[0];
    assert_eq!(remaining.asset_id, replacement_asset_id);
    let connection = Connection::open(&storage_paths.catalog_path).expect("published catalog");
    let (asset_count, location_count): (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets),
                        (SELECT COUNT(*) FROM asset_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("bounded derived rows");
    assert_eq!(asset_count, 1);
    assert_eq!(location_count, 1);
}

#[test]
fn cancelled_scan_does_not_publish_a_catalog() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("one.png");
    RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "cancelled-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(cancel_scan("cancelled-scan"));
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("cancelled scan");

    assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. })));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'cancelled-scan'",
            [],
            |row| row.get(0),
        )
        .expect("cancelled status");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    let (asset_count, location_count): (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets),
                        (SELECT COUNT(*) FROM asset_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("terminal staged rows");
    assert_eq!(status, "cancelled");
    assert_eq!(active_scan, None);
    assert_eq!(asset_count, 0);
    assert_eq!(location_count, 0);
}

#[test]
fn paused_scan_waits_for_explicit_resume_and_publishes_without_duplicates() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    for index in 0..3 {
        RgbaImage::from_pixel(4, 4, Rgba([30 + index, 60, 90, 255]))
            .save(source.path().join(format!("{index}.png")))
            .expect("fixture image");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "paused-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut paused_events = Vec::new();

    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(pause_scan("paused-scan"));
            }
            paused_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("paused scan");

    assert!(matches!(
        paused_events.last(),
        Some(ScanEvent::Paused {
            visited_entries: 1,
            accepted_items: 1,
            issue_count: 0,
            ..
        })
    ));
    let paused_catalog = SqliteCatalog::open(catalog_path.clone()).expect("paused catalog");
    assert!(
        paused_catalog
            .load_recoverable_scan()
            .expect("interrupted query")
            .is_none()
    );
    let paused = paused_catalog
        .load_paused_scan()
        .expect("paused query")
        .expect("paused task");
    assert_eq!(paused.accepted_items, 1);
    drop(paused_catalog);

    let mut resumed_events = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            resumed_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("resumed paused scan");

    assert!(matches!(
        resumed_events.last(),
        Some(ScanEvent::Completed {
            asset_count: 3,
            issue_count: 0,
            ..
        })
    ));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let (status, location_count): (String, i64) = connection
        .query_row(
            "SELECT scans.status, COUNT(locations.location_id)
                 FROM scan_runs AS scans
                 LEFT JOIN asset_locations AS locations ON locations.scan_id = scans.id
                 WHERE scans.id = 'paused-scan'
                 GROUP BY scans.status",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("published paused scan");
    assert_eq!(status, "completed");
    assert_eq!(location_count, 3);
}

#[test]
fn corrupt_image_is_isolated_and_scan_completes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    fs::write(source.path().join("broken.jpg"), b"not an image").expect("corrupt fixture");
    let request = ScanRequest {
        scan_id: "corrupt-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed scan with isolated issue");

    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "image_format_unsupported"
    )));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 1,
            ..
        })
    ));
}

#[cfg(windows)]
#[test]
fn exclusively_locked_image_is_isolated_and_scan_completes() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("locked.png");
    RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]))
        .save(&source_path)
        .expect("fixture image");
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&source_path)
        .expect("exclusive fixture lock");
    let request = ScanRequest {
        scan_id: "locked-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed scan with locked file isolated");

    drop(lock);
    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "image_open_failed"
    )));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 1,
            ..
        })
    ));
}

#[cfg(windows)]
#[test]
fn long_path_image_is_discovered_without_source_changes() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let mut nested = source.path().to_path_buf();
    while nested.to_string_lossy().len() < 280 {
        nested.push("long-path-segment-0123456789abcdef");
    }
    fs::create_dir_all(&nested).expect("long fixture directory");
    let source_path = nested.join("pixel.data");
    RgbaImage::from_pixel(4, 4, Rgba([80, 120, 200, 255]))
        .save_with_format(&source_path, ImageFormat::Png)
        .expect("long path fixture image");
    assert!(source_path.to_string_lossy().len() > 260);
    let original_bytes = fs::read(&source_path).expect("fixture bytes");
    let request = ScanRequest {
        scan_id: "long-path-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("completed long path scan");

    assert_eq!(
        fs::read(&source_path).expect("source after scan"),
        original_bytes,
    );
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 1,
            issue_count: 0,
            ..
        })
    ));
}

#[test]
fn interrupted_deep_scan_resumes_only_the_current_directory_without_duplicates() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let completed_directory = source.path().join("00-completed");
    let interrupted_directory = source.path().join("01-interrupted");
    fs::create_dir_all(&completed_directory).expect("completed fixture directory");
    fs::create_dir_all(&interrupted_directory).expect("interrupted fixture directory");
    for index in 0..2 {
        RgbaImage::from_pixel(2, 2, Rgba([index, 40, 80, 255]))
            .save(completed_directory.join(format!("{index:03}.png")))
            .expect("fixture image");
    }
    for index in 0..128 {
        RgbaImage::from_pixel(2, 2, Rgba([index as u8, 40, 80, 255]))
            .save(interrupted_directory.join(format!("{index:03}.png")))
            .expect("fixture image");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "recoverable-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let first_attempt = catch_unwind(AssertUnwindSafe(|| {
        let mut discovered = 0_u64;
        let _ = run_scan_with_storage(
            request.clone(),
            |event| {
                if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                    discovered += 1;
                    if discovered == 126 {
                        panic!("simulated process interruption");
                    }
                }
                true
            },
            StoragePaths {
                catalog_path: catalog_path.clone(),
                preview_root: preview_root.clone(),
                preview_budget_bytes: 64 * 1024 * 1024,
                settings_path: storage.path().join("settings.sqlite3"),
            },
        );
    }));
    assert!(first_attempt.is_err());

    let recoverable = SqliteCatalog::open(catalog_path.clone())
        .expect("interrupted catalog")
        .load_recoverable_scan()
        .expect("recoverable query")
        .expect("recoverable scan");
    assert_eq!(recoverable.scan_id, request.scan_id);
    assert_eq!(recoverable.visited_entries, 128);
    assert_eq!(recoverable.accepted_items, 126);
    let current_directory: String = Connection::open(catalog_path.clone())
        .expect("checkpoint catalog")
        .query_row(
            "SELECT current_directory_relative_path
                 FROM scan_runs WHERE id = 'recoverable-scan'",
            [],
            |row| row.get(0),
        )
        .expect("current directory frontier");
    assert_eq!(current_directory, "01-interrupted");

    let mut resumed_events = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            resumed_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("resumed scan");

    assert!(resumed_events.iter().any(|event| matches!(
        event,
        ScanEvent::Progress {
            visited_entries: 128,
            accepted_items: 126,
            ..
        }
    )));
    assert!(matches!(
        resumed_events.last(),
        Some(ScanEvent::Completed {
            asset_count: 130,
            issue_count: 0,
            ..
        })
    ));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let stored_locations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = 'recoverable-scan'",
            [],
            |row| row.get(0),
        )
        .expect("location count");
    assert_eq!(stored_locations, 130);
    assert_eq!(source.path().read_dir().expect("source entries").count(), 2,);
}

#[test]
fn disconnected_scan_consumer_leaves_a_running_scan_for_next_start() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "disconnected-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };

    run_scan_with_storage(
        request.clone(),
        |_| false,
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("detach scan consumer");

    let recoverable = SqliteCatalog::open(catalog_path)
        .expect("catalog")
        .load_recoverable_scan()
        .expect("recoverable scan query")
        .expect("disconnected scan remains recoverable");
    assert_eq!(recoverable.scan_id, request.scan_id);
}

#[test]
fn shutdown_suspend_keeps_foreground_full_scan_recoverable() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255]))
        .save(source.path().join("one.png"))
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let preview_root = storage.path().join("previews");
    let request = ScanRequest {
        scan_id: "shutdown-suspended-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };

    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(suspend_scan(&request.scan_id));
            }
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("suspend full scan");

    let recoverable = SqliteCatalog::open(catalog_path.clone())
        .expect("catalog")
        .load_recoverable_scan()
        .expect("recoverable scan query")
        .expect("suspended scan remains running");
    assert_eq!(recoverable.scan_id, request.scan_id);

    let mut resumed_events = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            resumed_events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("resume shutdown-suspended scan");

    assert!(matches!(
        resumed_events.last(),
        Some(ScanEvent::Completed { asset_count: 1, .. })
    ));
    assert!(
        SqliteCatalog::open(catalog_path)
            .expect("resumed catalog")
            .load_recoverable_scan()
            .expect("recoverable scan query")
            .is_none()
    );
}

#[test]
fn shutdown_suspend_does_not_override_user_pause_or_cancel() {
    let cancel_token = register_scan("shutdown-user-cancel").expect("register cancelled scan");
    let cancel_registration = ScanRegistration {
        scan_id: "shutdown-user-cancel".to_owned(),
    };
    assert!(cancel_scan("shutdown-user-cancel"));
    assert!(!suspend_scan("shutdown-user-cancel"));
    assert_eq!(cancel_token.load(Ordering::Relaxed), CONTROL_CANCEL);
    drop(cancel_registration);

    let pause_token = register_scan("shutdown-user-pause").expect("register paused scan");
    let pause_registration = ScanRegistration {
        scan_id: "shutdown-user-pause".to_owned(),
    };
    assert!(pause_scan("shutdown-user-pause"));
    assert!(!suspend_scan("shutdown-user-pause"));
    assert_eq!(pause_token.load(Ordering::Relaxed), CONTROL_PAUSE);
    drop(pause_registration);
}

#[test]
fn extremely_wide_directory_is_processed_through_bounded_entry_windows() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    for index in 0..1025 {
        fs::write(
            source.path().join(format!("ignored-{index:04}.txt")),
            b"not an image",
        )
        .expect("wide fixture entry");
    }
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "wide-directory-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("wide directory scan");

    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed {
            asset_count: 0,
            issue_count: 0,
            ..
        })
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Progress {
            visited_entries: 128,
            accepted_items: 0,
            issue_count: 0,
            ..
        }
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let (visited_entries, pending_entries): (i64, i64) = connection
        .query_row(
            "SELECT scans.visited_entries,
                        (SELECT COUNT(*) FROM scan_directory_entries)
                 FROM scan_runs AS scans WHERE scans.id = 'wide-directory-scan'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("wide scan state");
    assert_eq!(visited_entries, 1025);
    assert_eq!(pending_entries, 0);
    assert_eq!(
        source.path().read_dir().expect("source entries").count(),
        1025
    );
}

#[cfg(windows)]
#[test]
#[ignore = "manual synthetic large-library performance acceptance"]
fn synthetic_ten_thousand_file_scan_records_bounded_acceptance_evidence() {
    use std::time::{Duration, Instant};

    const FILE_COUNT: usize = 10_000;
    const CANCEL_AFTER: u64 = 512;

    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let template_path = source.path().join("template.png");
    RgbaImage::from_pixel(2, 2, Rgba([40, 80, 120, 255]))
        .save_with_format(&template_path, ImageFormat::Png)
        .expect("template image");
    let template_bytes = fs::read(&template_path).expect("template bytes");
    fs::remove_file(&template_path).expect("remove template");
    let fixture_started = Instant::now();
    for index in 0..FILE_COUNT {
        fs::write(
            source.path().join(format!("image-{index:05}.png")),
            &template_bytes,
        )
        .expect("synthetic image");
    }
    let fixture_elapsed = fixture_started.elapsed();
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let request = |scan_id: &str| ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };

    let cold_started = Instant::now();
    run_scan_with_storage(request("benchmark-cold"), |_| true, storage_paths.clone())
        .expect("cold scan");
    let cold_elapsed = cold_started.elapsed();

    let warm_started = Instant::now();
    run_scan_with_storage(request("benchmark-warm"), |_| true, storage_paths.clone())
        .expect("warm scan");
    let warm_elapsed = warm_started.elapsed();

    // Only an unpublished first import retains a foreground pause checkpoint.
    let resumable_storage = StoragePaths {
        catalog_path: storage.path().join("resumable").join("ame.sqlite3"),
        preview_root: storage.path().join("resumable-previews"),
        preview_budget_bytes: storage_paths.preview_budget_bytes,
        settings_path: storage.path().join("resumable-settings.sqlite3"),
    };
    let mut pause_accepted = 0_u64;
    let mut pause_requested = None;
    let mut did_pause = false;
    run_scan_with_storage(
        request("benchmark-resumed"),
        |event| {
            did_pause |= matches!(event, ScanEvent::Paused { .. });
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                pause_accepted += 1;
                if pause_accepted == CANCEL_AFTER {
                    pause_requested = Some(Instant::now());
                    assert!(pause_scan("benchmark-resumed"));
                }
            }
            true
        },
        resumable_storage.clone(),
    )
    .expect("paused benchmark scan");
    let pause_elapsed = pause_requested.expect("pause request").elapsed();
    assert!(did_pause);
    let paused_scan = SqliteCatalog::open(resumable_storage.catalog_path.clone())
        .expect("paused benchmark catalog")
        .load_paused_scan()
        .expect("paused benchmark state")
        .expect("paused benchmark scan");
    assert_eq!(paused_scan.scan_id, "benchmark-resumed");
    assert_eq!(paused_scan.accepted_items, CANCEL_AFTER);
    let paused_snapshot = load_test_snapshot(&resumable_storage);
    assert_eq!(paused_snapshot.roots.len(), 1);
    assert!(paused_snapshot.roots[0].active_scan_id.is_none());

    let resume_started = Instant::now();
    let mut did_complete_resume = false;
    resume_scan_with_storage(
        request("benchmark-resumed"),
        |event| {
            did_complete_resume |= matches!(event, ScanEvent::Completed { .. });
            true
        },
        resumable_storage.clone(),
    )
    .expect("resumed benchmark scan");
    let resume_elapsed = resume_started.elapsed();
    assert!(did_complete_resume);
    let resumed_connection =
        Connection::open(&resumable_storage.catalog_path).expect("resumed benchmark catalog");
    let (resumed_locations, unfinished_runs): (i64, i64) = resumed_connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_roots AS roots
                JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
                WHERE roots.active_scan_id = 'benchmark-resumed'),
               (SELECT COUNT(*) FROM scan_runs WHERE status IN ('running', 'paused'))",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("resumed benchmark publication");
    assert_eq!(resumed_locations, FILE_COUNT as i64);
    assert_eq!(unfinished_runs, 0);
    let resumed_catalog_bytes = fs::metadata(&resumable_storage.catalog_path)
        .expect("resumed catalog metadata")
        .len();

    let mut accepted = 0_u64;
    let mut cancellation_requested = None;
    run_scan_with_storage(
        request("benchmark-cancelled"),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                accepted += 1;
                if accepted == CANCEL_AFTER {
                    cancellation_requested = Some(Instant::now());
                    assert!(cancel_scan("benchmark-cancelled"));
                }
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("cancelled benchmark scan");
    let cancellation_elapsed = cancellation_requested
        .expect("cancellation request")
        .elapsed();

    let connection = Connection::open(&storage_paths.catalog_path).expect("benchmark catalog");
    let (active_locations, all_locations, assets, cancelled_locations): (i64, i64, i64, i64) =
        connection
            .query_row(
                "SELECT
                       (SELECT COUNT(*) FROM library_roots AS roots
                        JOIN asset_locations AS locations
                          ON locations.scan_id = roots.active_scan_id),
                       (SELECT COUNT(*) FROM asset_locations),
                       (SELECT COUNT(*) FROM assets),
                       (SELECT COUNT(*) FROM asset_locations
                        WHERE scan_id = 'benchmark-cancelled')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("benchmark row counts");
    let catalog_bytes = fs::metadata(&storage_paths.catalog_path)
        .expect("catalog metadata")
        .len();

    println!(
        "AME_SYNTHETIC_BENCHMARK files={FILE_COUNT} fixture_ms={} cold_ms={} warm_ms={} \
             pause_ms={} resume_ms={} cancel_ms={} catalog_bytes={catalog_bytes} \
             resumed_catalog_bytes={resumed_catalog_bytes}",
        fixture_elapsed.as_millis(),
        cold_elapsed.as_millis(),
        warm_elapsed.as_millis(),
        pause_elapsed.as_millis(),
        resume_elapsed.as_millis(),
        cancellation_elapsed.as_millis(),
    );

    assert_eq!(active_locations, FILE_COUNT as i64);
    assert_eq!(all_locations, FILE_COUNT as i64);
    assert_eq!(assets, FILE_COUNT as i64);
    assert_eq!(cancelled_locations, 0);
    assert!(cold_elapsed < Duration::from_secs(60));
    assert!(warm_elapsed < Duration::from_secs(60));
    assert!(pause_elapsed < Duration::from_secs(5));
    assert!(resume_elapsed < Duration::from_secs(60));
    assert!(cancellation_elapsed < Duration::from_secs(5));
    assert!(catalog_bytes < 64 * 1024 * 1024);
    assert!(resumed_catalog_bytes < 64 * 1024 * 1024);
    assert_eq!(
        fs::read(source.path().join("image-00000.png")).expect("first source bytes"),
        template_bytes,
    );
    assert_eq!(
        fs::read(source.path().join("image-09999.png")).expect("last source bytes"),
        template_bytes,
    );
    assert_eq!(
        source.path().read_dir().expect("source entries").count(),
        FILE_COUNT
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires current explicit user approval for one named source root"]
fn user_authorized_read_only_library_acceptance() {
    use std::time::Instant;

    const CONSENT: &str = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1";
    const SAMPLE_LIMIT: usize = 64;
    const SAMPLE_FILE_LIMIT: u64 = 64 * 1024 * 1024;

    let consent = std::env::var("CEDARFLAKE_AME_ACCEPTANCE_CONSENT")
        .expect("explicit acceptance consent is required");
    assert_eq!(consent, CONSENT, "acceptance consent does not match");
    let root = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_ROOT")
            .expect("an explicit acceptance root is required"),
    )
    .canonicalize()
    .expect("acceptance root must be available");
    let storage_root = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT")
            .expect("an explicit acceptance storage root is required"),
    )
    .canonicalize()
    .expect("acceptance storage root must be available");
    assert!(root.is_dir(), "acceptance root must be a directory");
    assert!(
        storage_root.is_dir(),
        "acceptance storage must be a directory"
    );
    assert!(
        !acceptance_paths_overlap(&root, &storage_root),
        "acceptance storage must remain outside the source root"
    );
    let report_path = PathBuf::from(
        std::env::var("CEDARFLAKE_AME_ACCEPTANCE_REPORT")
            .expect("an explicit acceptance report path is required"),
    );
    assert_eq!(
        report_path
            .parent()
            .expect("acceptance report parent")
            .canonicalize()
            .expect("acceptance report parent must be available"),
        storage_root,
        "acceptance report must remain directly inside acceptance storage"
    );

    let scan_id = std::env::var("CEDARFLAKE_AME_ACCEPTANCE_SCAN_ID")
        .expect("an explicit acceptance scan ID is required");
    assert!(!scan_id.trim().is_empty(), "acceptance scan ID is empty");
    let cancel_after = acceptance_optional_count("CEDARFLAKE_AME_ACCEPTANCE_CANCEL_AFTER");
    let pause_after = acceptance_optional_count("CEDARFLAKE_AME_ACCEPTANCE_PAUSE_AFTER");
    assert!(
        cancel_after.is_none() || pause_after.is_none(),
        "cancel and pause injection cannot be combined"
    );
    let storage = StoragePaths {
        catalog_path: storage_root.join("catalog").join("ame.sqlite3"),
        preview_root: storage_root.join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage_root.join("settings").join("storage.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: scan_id.clone(),
        root_path: root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    acceptance_record(
        &report_path,
        &format!(
            "AME_REAL_LIBRARY_BEGIN scan_id={scan_id:?} root={:?}",
            root.to_string_lossy()
        ),
    );

    let mut accepted_events = 0_u64;
    let mut next_progress_report = 10_000_u64;
    let mut terminal = None;
    let mut pause_requested = None;
    let mut cancel_requested = None;
    let mut samples = Vec::new();
    let mut sample_error = None;
    let scan_started = Instant::now();
    run_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::Progress {
                visited_entries, ..
            } = &event
                && cancel_after.is_some_and(|limit| *visited_entries >= limit)
                && cancel_requested.is_none()
            {
                cancel_requested = Some(Instant::now());
                assert!(
                    cancel_scan(&scan_id),
                    "acceptance cancellation request was rejected"
                );
            }
            match event {
                ScanEvent::Progress {
                    visited_entries,
                    accepted_items,
                    issue_count,
                    ..
                } if visited_entries >= next_progress_report => {
                    let line = format!(
                        "AME_ACCEPTANCE_PROGRESS visited={visited_entries} \
                             accepted={accepted_items} issues={issue_count}"
                    );
                    println!("{line}");
                    acceptance_record(&report_path, &line);
                    while next_progress_report <= visited_entries {
                        next_progress_report = next_progress_report.saturating_add(10_000);
                    }
                }
                ScanEvent::AssetDiscovered { asset, .. } => {
                    accepted_events += 1;
                    if samples.len() < SAMPLE_LIMIT
                        && asset.file_size <= SAMPLE_FILE_LIMIT
                        && acceptance_should_sample(&asset.relative_path, accepted_events)
                    {
                        let path = PathBuf::from(&asset.absolute_path);
                        match acceptance_hash_file(&path) {
                            Ok(hash) => samples.push((path, hash)),
                            Err(error) => {
                                sample_error = Some(format!(
                                    "Could not hash {} before scan completion: {error}",
                                    asset.absolute_path
                                ));
                            }
                        }
                    }
                    if pause_after.is_some_and(|limit| accepted_events == limit) {
                        pause_requested = Some(Instant::now());
                        assert!(
                            pause_scan(&scan_id),
                            "acceptance pause request was rejected"
                        );
                    }
                }
                ScanEvent::Completed { was_limited, .. } => {
                    assert!(
                        !was_limited,
                        "acceptance scan must not publish a limited result"
                    );
                    terminal = Some("completed");
                }
                ScanEvent::Cancelled { .. } => terminal = Some("cancelled"),
                ScanEvent::Paused { .. } => terminal = Some("paused"),
                ScanEvent::Stale { .. } => terminal = Some("stale"),
                _ => {}
            }
            true
        },
        storage.clone(),
    )
    .expect("read-only acceptance scan");
    let first_pass_elapsed = scan_started.elapsed();
    let pause_response_ms = pause_requested.map(|started| started.elapsed().as_millis());
    let cancel_response_ms = cancel_requested.map(|started| started.elapsed().as_millis());

    let mut resume_elapsed_ms = None;
    if pause_after.is_some() {
        assert!(pause_requested.is_some(), "pause threshold was not reached");
        assert_eq!(terminal, Some("paused"));
        let resume_started = Instant::now();
        let mut next_resume_progress_report = 10_000_u64;
        run_scan_with_storage(
            request,
            |event| {
                match event {
                    ScanEvent::Progress {
                        visited_entries,
                        accepted_items,
                        issue_count,
                        ..
                    } if visited_entries >= next_resume_progress_report => {
                        let line = format!(
                            "AME_ACCEPTANCE_RESUME_PROGRESS visited={visited_entries} \
                                 accepted={accepted_items} issues={issue_count}"
                        );
                        println!("{line}");
                        acceptance_record(&report_path, &line);
                        while next_resume_progress_report <= visited_entries {
                            next_resume_progress_report =
                                next_resume_progress_report.saturating_add(10_000);
                        }
                    }
                    ScanEvent::Completed { was_limited, .. } => {
                        assert!(
                            !was_limited,
                            "resumed acceptance scan must not publish a limited result"
                        );
                        terminal = Some("completed");
                    }
                    ScanEvent::Stale { .. } => terminal = Some("stale"),
                    _ => {}
                }
                true
            },
            storage.clone(),
        )
        .expect("resumed read-only acceptance scan");
        resume_elapsed_ms = Some(resume_started.elapsed().as_millis());
    } else if cancel_after.is_some() {
        assert!(
            cancel_requested.is_some(),
            "cancel threshold was not reached"
        );
        assert_eq!(terminal, Some("cancelled"));
    } else {
        assert_eq!(terminal, Some("completed"));
    }

    if let Some(error) = sample_error {
        panic!("{error}");
    }
    if accepted_events > 0 {
        assert!(
            !samples.is_empty(),
            "accepted source files did not produce an integrity sample"
        );
    }
    for (path, expected_hash) in &samples {
        assert_eq!(
            acceptance_hash_file(path).expect("post-scan sample hash"),
            *expected_hash,
            "source bytes changed during the acceptance scan: {}",
            path.display()
        );
    }

    let connection = Connection::open(&storage.catalog_path).expect("acceptance catalog");
    let (status, visited_entries, accepted_items, issue_count, scan_locations, is_active): (
        String,
        i64,
        i64,
        i64,
        i64,
        bool,
    ) = connection
        .query_row(
            "SELECT runs.status, runs.visited_entries, runs.accepted_items,
                        runs.issue_count,
                        (SELECT COUNT(*) FROM asset_locations WHERE scan_id = runs.id),
                        EXISTS(
                          SELECT 1 FROM library_roots WHERE active_scan_id = runs.id
                        )
                 FROM scan_runs AS runs WHERE runs.id = ?1",
            [&scan_id],
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
        .expect("acceptance scan metrics");
    if status == "completed" {
        assert!(is_active, "completed acceptance scan is not active");
        assert_eq!(scan_locations, accepted_items);
    } else if status == "cancelled" {
        assert!(!is_active, "cancelled acceptance scan was published");
        assert_eq!(scan_locations, 0, "cancelled scan left staged locations");
    }
    let (active_roots, active_locations_total): (i64, i64) = connection
        .query_row(
            "SELECT
                   (SELECT COUNT(*) FROM library_roots WHERE active_scan_id IS NOT NULL),
                   (SELECT COUNT(*) FROM library_roots AS roots
                    JOIN asset_locations AS locations
                      ON locations.scan_id = roots.active_scan_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("active acceptance catalog metrics");
    let issue_codes = {
        let mut statement = connection
            .prepare(
                "SELECT code, COUNT(*) FROM scan_issues
                     WHERE scan_id = ?1 GROUP BY code ORDER BY code",
            )
            .expect("acceptance issue-code statement");
        let rows = statement
            .query_map([&scan_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("acceptance issue-code query");
        let codes = rows
            .map(|row| {
                let (code, count) = row.expect("acceptance issue-code row");
                format!("{code}:{count}")
            })
            .collect::<Vec<_>>();
        if codes.is_empty() {
            "none".to_owned()
        } else {
            codes.join(",")
        }
    };
    let issue_evidence = {
        let mut statement = connection
            .prepare(
                "SELECT code, COUNT(*), MIN(message), MIN(COALESCE(path, ''))
                     FROM scan_issues WHERE scan_id = ?1
                     GROUP BY code ORDER BY code",
            )
            .expect("acceptance issue-evidence statement");
        statement
            .query_map([&scan_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .expect("acceptance issue-evidence query")
            .map(|row| row.expect("acceptance issue-evidence row"))
            .collect::<Vec<_>>()
    };
    let catalog_bytes = acceptance_catalog_bytes(&storage.catalog_path);
    let elapsed_ms = first_pass_elapsed.as_millis() + resume_elapsed_ms.unwrap_or_default();
    let throughput = if elapsed_ms == 0 {
        0.0
    } else {
        accepted_items as f64 * 1_000.0 / elapsed_ms as f64
    };

    let report = format!(
        "AME_REAL_LIBRARY_ACCEPTANCE status={status} scan_id={scan_id:?} root={:?} \
             visited={visited_entries} accepted={accepted_items} issues={issue_count} \
             scan_locations={scan_locations} is_active={is_active} \
             active_roots={active_roots} active_locations_total={active_locations_total} \
             issue_codes={issue_codes} \
             elapsed_ms={elapsed_ms} throughput_items_per_second={throughput:.2} \
             pause_response_ms={pause_response_ms:?} resume_ms={resume_elapsed_ms:?} \
             cancel_response_ms={cancel_response_ms:?} catalog_bytes={catalog_bytes} \
             source_hash_samples={}",
        root.to_string_lossy(),
        samples.len(),
    );
    println!("{report}");
    acceptance_record(&report_path, &report);
    for (code, count, message, sample_path) in issue_evidence {
        let evidence = format!(
            "AME_REAL_LIBRARY_ISSUE code={code:?} count={count} \
                 message={message:?} sample_path={sample_path:?}"
        );
        println!("{evidence}");
        acceptance_record(&report_path, &evidence);
    }
}

#[cfg(windows)]
fn acceptance_optional_count(name: &str) -> Option<u64> {
    std::env::var(name).ok().map(|value| {
        let value = value
            .parse::<u64>()
            .expect("acceptance count must be an integer");
        assert!(value > 0, "acceptance count must be positive");
        value
    })
}

#[cfg(windows)]
fn acceptance_paths_overlap(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    let left = normalize(left);
    let right = normalize(right);
    left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\"))
}

#[cfg(windows)]
fn acceptance_should_sample(relative_path: &str, accepted_items: u64) -> bool {
    if accepted_items == 1 {
        return true;
    }
    let hash = blake3::hash(relative_path.as_bytes());
    u16::from_le_bytes([hash.as_bytes()[0], hash.as_bytes()[1]]).is_multiple_of(1024)
}

#[cfg(windows)]
fn acceptance_hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

#[cfg(windows)]
fn acceptance_catalog_bytes(path: &Path) -> u64 {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return 0;
    };
    let Some(parent) = path.parent() else {
        return 0;
    };
    ["", "-wal", "-shm"]
        .iter()
        .filter_map(|suffix| fs::metadata(parent.join(format!("{file_name}{suffix}"))).ok())
        .map(|metadata| metadata.len())
        .sum()
}

#[cfg(windows)]
fn acceptance_record(path: &Path, line: &str) {
    use std::io::Write;

    let mut report = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("acceptance report");
    writeln!(report, "{line}").expect("acceptance report write");
    report.flush().expect("acceptance report flush");
}

#[test]
fn missing_first_import_checkpoint_position_rebuilds_on_explicit_resume() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(2, 2, Rgba([30, 60, 90, 255]))
        .save(source.path().join("present.png"))
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "missing-checkpoint-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let discovery = FileDiscovery::new(&request.root_path).expect("discovery");
    let canonical_root = discovery.canonical_root().expect("canonical root");
    let publication_root_identity = discovery
        .metadata_inventory_root_identity()
        .expect("publication root identity")
        .expect("Windows publication root identity");
    let root_path = canonical_root.to_string_lossy().into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            &root_id,
            &root_path,
            &publication_root_identity,
        )
        .expect("begin scan");
    catalog
        .checkpoint_scan(
            &request.scan_id,
            &crate::domain::ScanCheckpoint {
                last_visited_relative_path: Some("removed.png".to_owned()),
                visited_entries: 2,
                accepted_items: 1,
                issue_count: 0,
                requires_previous_snapshot: false,
            },
        )
        .expect("checkpoint");
    drop(catalog);

    let mut events = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("explicit first-import rebuild");

    assert!(!events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "scan_checkpoint_unavailable"
    )));
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert!(matches!(
        events.get(1),
        Some(ScanEvent::Progress {
            visited_entries: 0,
            accepted_items: 0,
            ..
        })
    ));
    let status: String = Connection::open(catalog_path)
        .expect("catalog database")
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'missing-checkpoint-scan'",
            [],
            |row| row.get(0),
        )
        .expect("scan status");
    assert_eq!(status, "completed");
}

#[test]
fn first_import_source_change_publishes_baseline_and_queues_catch_up() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("changing.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "stale-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                fs::write(&source_path, b"changed by another process")
                    .expect("external source change");
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("first import with concurrent source change");

    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_changed_during_scan"
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'stale-scan'",
            [],
            |row| row.get(0),
        )
        .expect("stale status");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    let pending_live_changes: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE lane.lane = 'p0_live' AND queue.status = 'pending'
               AND queue.relative_path = 'changing.png'",
            [],
            |row| row.get(0),
        )
        .expect("pending first-import catch-up");
    assert_eq!(status, "completed");
    assert_eq!(active_scan.as_deref(), Some("stale-scan"));
    assert_eq!(pending_live_changes, 1);
}

#[test]
fn foreground_update_drains_mid_scan_change_without_a_third_full_scan() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("changing.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "foreground-change-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    assert_eq!(before.assets.len(), 1);
    let before_asset = before.assets[0].clone();
    let root_id = before_asset.root_id.clone();
    let mut changed = false;
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "foreground-change-update".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !changed && matches!(event, ScanEvent::AssetDiscovered { .. }) {
                RgbaImage::from_pixel(9, 7, Rgba([10, 210, 90, 255]))
                    .save(&source_path)
                    .expect("mid-scan source replacement");
                changed = true;
            }
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("foreground update with a concurrent source change");

    assert!(changed);
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if matches!(
            code.as_str(),
            "source_changed_during_scan" | "source_replaced_during_scan"
        )
    )));
    let after = load_test_snapshot(&storage_paths);
    assert_eq!(after.assets.len(), 1);
    let after_asset = &after.assets[0];
    assert_eq!(after_asset.asset_id, before_asset.asset_id);
    assert_eq!((after_asset.width, after_asset.height), (9, 7));
    assert!(after_asset.source_generation > before_asset.source_generation);
    assert_ne!(after_asset.source_revision, before_asset.source_revision);

    let terminal_state: (i64, i64, i64) = Connection::open(&storage_paths.catalog_path)
        .expect("catalog database")
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM scan_runs WHERE root_id = ?1),
               (SELECT COUNT(*) FROM library_change_queue AS queue
                JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                WHERE queue.root_id = ?1 AND queue.scope = 'path'
                  AND queue.relative_path = 'changing.png'
                  AND lane.lane = 'p0_live' AND queue.status = 'completed'),
               (SELECT COUNT(*) FROM library_change_queue
                WHERE root_id = ?1 AND scope = 'path'
                  AND relative_path = 'changing.png'
                  AND status IN ('pending', 'leased', 'retry_wait'))",
            [&root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("terminal scan and P0 state");
    assert_eq!(terminal_state, (2, 1, 0));
}

#[test]
fn first_import_same_path_replacement_publishes_a_new_revision_through_live_catch_up() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("replaced.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut replaced = false;
    run_scan_with_storage(
        ScanRequest {
            scan_id: "same-path-first-import".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !replaced && matches!(event, ScanEvent::AssetDiscovered { .. }) {
                RgbaImage::from_pixel(9, 7, Rgba([10, 210, 90, 255]))
                    .save(&source_path)
                    .expect("same-path replacement");
                replaced = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("first import with same-path replacement");

    let before = load_test_snapshot(&storage_paths);
    let original_revision = before.assets[0]
        .source_revision
        .clone()
        .expect("baseline source revision");
    let canonical_root = FileDiscovery::new(&source.path().to_string_lossy())
        .expect("root discovery")
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    let mut catalog = SqliteCatalog::open(storage_paths.catalog_path.clone())
        .expect("first-import catch-up catalog");
    let report = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        current_unix_ms()
            .expect("catch-up clock")
            .saturating_add(1_000),
        LibraryChangeQueuePolicy::default(),
    )
    .expect("apply same-path catch-up");
    assert_eq!(report.applied_mutation_count, 1);
    drop(catalog);

    let after = load_test_snapshot(&storage_paths);
    assert_eq!(after.assets.len(), 1);
    assert_eq!((after.assets[0].width, after.assets[0].height), (9, 7));
    assert_ne!(
        after.assets[0].source_revision.as_ref(),
        Some(&original_revision)
    );
}

#[test]
fn first_import_commit_boundary_addition_publishes_then_converges_from_live_queue() {
    assert_first_import_commit_boundary_catch_up(LibraryChangeScope::Path);
}

#[test]
fn first_import_directory_changes_publish_then_converge_without_another_full_scan() {
    for scope in [LibraryChangeScope::Subtree, LibraryChangeScope::Root] {
        assert_first_import_commit_boundary_catch_up(scope);
    }
}

fn assert_first_import_commit_boundary_catch_up(scope: LibraryChangeScope) {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(4, 4, Rgba([40, 60, 80, 255]))
        .save(source.path().join("baseline.png"))
        .expect("baseline image");
    let root_path = source.path().to_string_lossy().into_owned();
    let canonical_root = FileDiscovery::new(&root_path)
        .expect("root discovery")
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &canonical_root);
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut enqueued = false;
    let mut completed = false;
    let mut finalizing_events = 0;
    run_scan_with_storage(
        ScanRequest {
            scan_id: format!("commit-boundary-first-import-{scope:?}"),
            root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if matches!(event, ScanEvent::Finalizing { validated_items, total_items, .. } if validated_items == total_items) {
                finalizing_events += 1;
                if finalizing_events > 1 {
                    return false;
                }
            }
            completed |= matches!(event, ScanEvent::Completed { .. });
            if !enqueued && matches!(event, ScanEvent::Finalizing { .. }) {
                fs::create_dir(source.path().join("incoming")).expect("incoming directory");
                RgbaImage::from_pixel(6, 5, Rgba([180, 20, 90, 255]))
                    .save(source.path().join("incoming").join("boundary.png"))
                    .expect("commit-boundary image");
                let observed_unix_ms = current_unix_ms().expect("observer clock");
                SqliteCatalog::open(storage_paths.catalog_path.clone())
                    .expect("observer queue catalog")
                    .enqueue_library_change_intents(
                        &[LibraryChangeIntent {
                            root_id: root_id.clone(),
                            root_generation: LibraryRootGeneration::initial(),
                            kind: LibraryChangeIntentKind::Reconcile,
                            scope,
                            relative_path: match scope {
                                LibraryChangeScope::Path => "incoming/boundary.png",
                                LibraryChangeScope::Subtree => "incoming",
                                LibraryChangeScope::Root => "",
                            }
                            .to_owned(),
                            previous_relative_path: None,
                            origin: LibraryChangeOrigin::LiveNotification,
                            first_observed_unix_ms: observed_unix_ms,
                            most_recent_observed_unix_ms: observed_unix_ms,
                            first_sequence: 1,
                            most_recent_sequence: 1,
                            coalesced_observation_count: 1,
                        }],
                        observed_unix_ms,
                        LibraryChangeQueuePolicy::default(),
                    )
                    .expect("enqueue commit-boundary change");
                enqueued = true;
            }
            true
        },
        storage_paths.clone(),
    )
    .expect("publish baseline before queued commit-boundary change");
    assert!(
        completed,
        "first import cannot wait for a live worker that needs its baseline"
    );

    let mut catalog = SqliteCatalog::open(storage_paths.catalog_path.clone())
        .expect("commit-boundary catch-up catalog");
    let now_unix_ms = current_unix_ms()
        .expect("catch-up clock")
        .saturating_add(1_000);
    let report = if scope == LibraryChangeScope::Path {
        crate::application::process_ready_library_changes_in_lane(
            &mut catalog,
            &root_id,
            LibraryRootGeneration::initial(),
            LibraryChangeLane::Live,
            now_unix_ms,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("apply commit-boundary path change")
    } else {
        let root = catalog
            .load_incremental_catalog_root(&root_id)
            .expect("published root")
            .expect("root remains configured");
        assert!(root.active_scan_id.is_some());
        assert!(!root.has_running_scan);
        let leased = catalog
            .lease_live_authoritative_library_change(
                &root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms,
                LibraryChangeQueuePolicy::default(),
            )
            .expect("live scope lease")
            .expect("unconsumed first-import directory change");
        crate::application::authoritative_library_changes::process_leased_authoritative_library_change_cancellable(
            &mut catalog, &root, &leased, now_unix_ms,
            LibraryChangeQueuePolicy::default(),
            crate::application::authoritative_library_changes::AuthoritativeRecoveryPolicy::default(),
            &std::sync::atomic::AtomicBool::new(false),
        ).expect("apply commit-boundary directory change").incremental
    };
    assert_eq!(report.applied_mutation_count, 1);
    drop(catalog);
    let snapshot = load_test_snapshot(&storage_paths);
    assert_eq!(snapshot.assets.len(), 2);
    assert!(
        snapshot
            .assets
            .iter()
            .any(|asset| asset.relative_path.replace('\\', "/") == "incoming/boundary.png")
    );
    let connection = Connection::open(&storage_paths.catalog_path).expect("scan count catalog");
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row
                .get::<_, i64>(0))
            .expect("scan count"),
        1
    );
}

#[test]
fn first_import_source_deletion_publishes_baseline_and_queues_catch_up() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("missing.png");
    RgbaImage::from_pixel(4, 4, Rgba([200, 30, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let request = ScanRequest {
        scan_id: "missing-scan".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut events = Vec::new();

    run_scan_with_storage(
        request,
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                fs::remove_file(&source_path).expect("external fixture deletion");
            }
            events.push(event);
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .expect("missing source scan");

    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_revalidation_failed"
    )));
    let connection = Connection::open(catalog_path).expect("catalog database");
    let active_scan: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("active scan state");
    let pending_live_changes: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE lane.lane = 'p0_live' AND queue.status = 'pending'
               AND queue.relative_path = 'missing.png'",
            [],
            |row| row.get(0),
        )
        .expect("pending deletion catch-up");
    assert_eq!(active_scan.as_deref(), Some("missing-scan"));
    assert_eq!(pending_live_changes, 1);
}

#[test]
fn corrupt_rescan_completes_and_removes_deterministically_invalid_media() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "trustworthy-initial-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    assert_eq!(before.assets.len(), 1);
    let corrupt_bytes = b"not a decodable image";
    fs::write(&source_path, corrupt_bytes).expect("controlled corruption");
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "trustworthy-corrupt-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("corrupt rescan remains recoverable");

    let after = load_test_snapshot(&storage_paths);
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert_eq!(after.revision, before.revision + 1);
    assert!(after.assets.is_empty());
    assert_eq!(
        fs::read(&source_path).expect("corrupt source bytes"),
        corrupt_bytes
    );
}

#[test]
fn authoritative_terminal_media_failures_publish_good_evidence_without_retries() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let retained_path = source.path().join("retained.png");
    let good_path = source.path().join("good.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&retained_path)
        .expect("retained fixture");
    RgbaImage::from_pixel(6, 4, Rgba([70, 90, 110, 255]))
        .save(&good_path)
        .expect("good fixture");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "authoritative-media-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let retained_before = before
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("retained published asset")
        .clone();
    let root_id = retained_before.root_id.clone();
    let retained_bytes = b"retained is no longer decodable";
    let new_bytes = b"new file is not decodable";
    fs::write(&retained_path, retained_bytes).expect("corrupt retained fixture");
    fs::write(source.path().join("new.png"), new_bytes).expect("new corrupt fixture");
    enqueue_root_freshness_unknown(&storage_paths, &root_id, 10_000);
    let mut events = Vec::new();

    begin_authoritative_checkpoint(
        &storage_paths,
        &ScanRequest {
            scan_id: "authoritative-media-recovery".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
    );
    run_scan_with_storage_reason(
        ScanRequest {
            scan_id: "authoritative-media-recovery".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("authoritative media recovery");

    let after = load_test_snapshot(&storage_paths);
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.assets.len(), 1);
    assert!(
        after
            .assets
            .iter()
            .any(|asset| asset.relative_path == "good.png")
    );
    assert!(
        after
            .assets
            .iter()
            .all(|asset| !matches!(asset.relative_path.as_str(), "new.png" | "retained.png"))
    );
    assert_eq!(
        fs::read(&retained_path).expect("retained bytes"),
        retained_bytes
    );
    assert_eq!(
        fs::read(source.path().join("new.png")).expect("new bytes"),
        new_bytes
    );
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let mut statement = connection
        .prepare(
            "SELECT relative_path, status
             FROM library_change_queue
             WHERE root_id = ?1 AND scope = 'path'
               AND status IN ('pending', 'leased', 'retry_wait')
             ORDER BY relative_path",
        )
        .expect("retry query");
    let retries = statement
        .query_map([&root_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("retry rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("retry evidence");
    assert!(retries.is_empty());
    let completed_root_gap = connection
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue
             WHERE root_id = ?1 AND scope = 'root' AND status = 'completed'",
            [&root_id],
            |row| row.get::<_, i64>(0),
        )
        .expect("completed root gap");
    assert_eq!(completed_root_gap, 1);
}

#[test]
fn authoritative_finalization_races_handoff_exact_paths_then_converge_without_another_scan() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let retained_path = source.path().join("retained.png");
    let stable_path = source.path().join("stable.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&retained_path)
        .expect("retained fixture");
    RgbaImage::from_pixel(6, 4, Rgba([70, 90, 110, 255]))
        .save(&stable_path)
        .expect("stable fixture");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "authoritative-race-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let retained_before = before
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("retained published asset")
        .clone();
    let root_id = retained_before.root_id.clone();
    let new_path = source.path().join("new.png");
    RgbaImage::from_pixel(5, 5, Rgba([130, 150, 170, 255]))
        .save(&new_path)
        .expect("new fixture");
    enqueue_root_freshness_unknown(&storage_paths, &root_id, 30_000);
    let mut changed_retained = false;
    let mut removed_new = false;
    let mut events = Vec::new();

    begin_authoritative_checkpoint(
        &storage_paths,
        &ScanRequest {
            scan_id: "authoritative-race-recovery".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
    );
    run_scan_with_storage_reason(
        ScanRequest {
            scan_id: "authoritative-race-recovery".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if let ScanEvent::AssetDiscovered { asset, .. } = &event {
                if asset.relative_path == "retained.png" && !changed_retained {
                    RgbaImage::from_pixel(11, 7, Rgba([200, 30, 60, 255]))
                        .save(&retained_path)
                        .expect("external retained change");
                    changed_retained = true;
                }
                if asset.relative_path == "new.png" && !removed_new {
                    fs::remove_file(&new_path).expect("external new file removal");
                    removed_new = true;
                }
            }
            events.push(event);
            true
        },
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("authoritative race recovery");

    assert!(changed_retained);
    assert!(removed_new);
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_changed_during_scan"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::Issue {
            issue: ScanIssue { code, .. },
            ..
        } if code == "source_revalidation_failed"
    )));
    let after = load_test_snapshot(&storage_paths);
    let retained_after = after
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("retained trustworthy asset");
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.assets.len(), 2);
    assert_eq!(retained_after.asset_id, retained_before.asset_id);
    assert_eq!(retained_after.width, retained_before.width);
    assert_eq!(retained_after.height, retained_before.height);
    assert!(
        after
            .assets
            .iter()
            .all(|asset| asset.relative_path != "new.png")
    );
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let mut statement = connection
        .prepare(
            "SELECT queue.relative_path, queue.status, queue.origin, lane.lane
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE queue.root_id = ?1 AND queue.scope = 'path'
               AND queue.status IN ('pending', 'leased', 'retry_wait')
             ORDER BY queue.relative_path",
        )
        .expect("retry query");
    let retries = statement
        .query_map([&root_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .expect("retry rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("retry evidence");
    assert_eq!(
        retries,
        vec![
            (
                "new.png".to_owned(),
                "pending".to_owned(),
                "live_notification".to_owned(),
                "p0_live".to_owned()
            ),
            (
                "retained.png".to_owned(),
                "pending".to_owned(),
                "live_notification".to_owned(),
                "p0_live".to_owned()
            ),
        ]
    );
    drop(statement);
    let recovery_scan_status: String = connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = 'authoritative-race-recovery'",
            [],
            |row| row.get(0),
        )
        .expect("recovery scan status");
    assert_eq!(recovery_scan_status, "completed");
    drop(connection);

    let mut catalog = SqliteCatalog::open(storage_paths.catalog_path.clone())
        .expect("finalization race P0 catalog");
    let report = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        current_unix_ms()
            .expect("P0 convergence clock")
            .saturating_add(1_000),
        LibraryChangeQueuePolicy::default(),
    )
    .expect("converge exact finalization paths through P0");
    assert_eq!(report.completed_count, 2);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 1);
    drop(catalog);

    let converged = load_test_snapshot(&storage_paths);
    let converged_retained = converged
        .assets
        .iter()
        .find(|asset| asset.relative_path == "retained.png")
        .expect("converged retained asset");
    assert_eq!(converged.assets.len(), 2);
    assert_eq!(converged_retained.asset_id, retained_before.asset_id);
    assert_eq!(
        (converged_retained.width, converged_retained.height),
        (11, 7)
    );
    assert_ne!(
        converged_retained.source_revision,
        retained_before.source_revision
    );
    assert!(
        converged_retained.source_generation > retained_before.source_generation,
        "same-path content replacement must advance source generation"
    );
    assert!(
        converged
            .assets
            .iter()
            .all(|asset| asset.relative_path != "new.png")
    );
    let terminal_state: (i64, i64) = Connection::open(&storage_paths.catalog_path)
        .expect("converged catalog database")
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM scan_runs WHERE root_id = ?1),
               (SELECT COUNT(*) FROM library_change_queue AS queue
                JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                WHERE queue.root_id = ?1 AND queue.scope = 'path'
                  AND queue.relative_path IN ('new.png', 'retained.png')
                  AND lane.lane = 'p0_live' AND queue.status = 'completed')",
            [&root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("terminal scan and P0 state");
    assert_eq!(terminal_state, (2, 2));
}

#[cfg(windows)]
#[test]
fn authoritative_scan_handoffs_temporarily_locked_media_to_p0_without_staling_snapshot() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("locked.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("locked-media fixture");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "locked-media-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial locked-media snapshot");
    let before = load_test_snapshot(&storage_paths);
    let retained = before.assets[0].clone();
    let root_id = retained.root_id.clone();
    enqueue_root_freshness_unknown(&storage_paths, &root_id, 40_000);
    let request = ScanRequest {
        scan_id: "locked-media-recovery".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    begin_authoritative_checkpoint(&storage_paths, &request);
    let mut exclusive_lock = None;
    let mut retryable_issue_code = None;
    let original_bytes = fs::read(&source_path).expect("original fixture bytes");
    let mut replacement_bytes = None;
    run_scan_with_storage_reason(
        request,
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                RgbaImage::from_pixel(11, 7, Rgba([200, 30, 60, 255]))
                    .save(&source_path)
                    .expect("changed source requires fresh media inspection");
                let bytes = fs::read(&source_path).expect("changed fixture bytes");
                assert_ne!(bytes, original_bytes);
                replacement_bytes = Some(bytes);
                exclusive_lock = Some(
                    OpenOptions::new()
                        .read(true)
                        .share_mode(0)
                        .open(&source_path)
                        .expect("exclusive media lock"),
                );
                assert_eq!(
                    fs::File::open(&source_path)
                        .expect_err("content read must be locked")
                        .raw_os_error(),
                    Some(32)
                );
            }
            if let ScanEvent::Issue {
                issue: ScanIssue { code, .. },
                ..
            } = &event
                && matches!(
                    code.as_str(),
                    "image_open_failed"
                        | "image_header_read_failed"
                        | "source_identity_unavailable"
                        | "source_became_unavailable"
                )
            {
                retryable_issue_code = Some(code.clone());
                exclusive_lock.take();
            }
            true
        },
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("authoritative scan with temporary media lock");
    drop(exclusive_lock);
    assert!(
        retryable_issue_code.is_some(),
        "the exclusive lock must produce retryable path evidence"
    );

    let preserved = load_test_snapshot(&storage_paths);
    assert_eq!(preserved.assets.len(), 1);
    assert_eq!(preserved.assets[0].asset_id, retained.asset_id);
    assert_eq!(
        preserved.assets[0].source_revision,
        retained.source_revision
    );
    let pending_state: (String, i64) = Connection::open(&storage_paths.catalog_path)
        .expect("locked-media handoff catalog")
        .query_row(
            "SELECT
               (SELECT status FROM scan_runs WHERE id = 'locked-media-recovery'),
               (SELECT COUNT(*) FROM library_change_queue AS queue
                JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                WHERE queue.root_id = ?1 AND queue.relative_path = 'locked.png'
                  AND queue.scope = 'path' AND queue.status = 'pending'
                  AND lane.lane = 'p0_live')",
            [&root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("locked-media handoff state");
    assert_eq!(pending_state, ("completed".to_owned(), 1));

    let mut catalog =
        SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("locked-media P0 catalog");
    let report = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        &root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        current_unix_ms()
            .expect("locked-media P0 clock")
            .saturating_add(1_000),
        LibraryChangeQueuePolicy::default(),
    )
    .expect("converge temporarily locked path");
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    drop(catalog);

    let converged = load_test_snapshot(&storage_paths);
    assert_eq!(converged.assets.len(), 1);
    assert_eq!(converged.assets[0].asset_id, retained.asset_id);
    assert_eq!(
        (converged.assets[0].width, converged.assets[0].height),
        (11, 7)
    );
    assert_ne!(
        converged.assets[0].source_revision,
        retained.source_revision
    );
    assert!(converged.assets[0].source_generation > retained.source_generation);
    assert_eq!(
        fs::read(&source_path).expect("source after recovery"),
        replacement_bytes.expect("external replacement bytes")
    );
    let terminal_state: (i64, i64) = Connection::open(&storage_paths.catalog_path)
        .expect("locked-media terminal catalog")
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM scan_runs WHERE root_id = ?1),
               (SELECT COUNT(*) FROM library_change_queue AS queue
                JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                WHERE queue.root_id = ?1 AND queue.relative_path = 'locked.png'
                  AND queue.scope = 'path' AND queue.status = 'completed'
                  AND lane.lane = 'p0_live')",
            [&root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("locked-media terminal state");
    assert_eq!(terminal_state, (2, 1));
}

#[test]
fn authoritative_resume_converts_legacy_terminal_media_staleness_without_retry() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let source_path = source.path().join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "legacy-authoritative-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let root_id = load_test_snapshot(&storage_paths)
        .assets
        .first()
        .expect("published asset")
        .root_id
        .clone();
    fs::write(&source_path, b"not decodable").expect("corrupt fixture");
    enqueue_root_freshness_unknown(&storage_paths, &root_id, 20_000);
    let request = ScanRequest {
        scan_id: "legacy-authoritative-recovery".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let canonical_root = FileDiscovery::new(&request.root_path)
        .expect("discovery")
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let mut catalog =
        SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("legacy catalog");
    let mut checkpoint = catalog
        .begin_authoritative_scan(&request, &root_id, &canonical_root)
        .expect("begin legacy authoritative scan");
    catalog
        .record_issue(
            &request.scan_id,
            &ScanIssue {
                path: Some(source_path.to_string_lossy().into_owned()),
                code: "image_dimensions_failed".to_owned(),
                message: "legacy prerelease media failure".to_owned(),
            },
        )
        .expect("legacy issue");
    checkpoint.issue_count = 1;
    checkpoint.requires_previous_snapshot = true;
    catalog
        .checkpoint_scan(&request.scan_id, &checkpoint)
        .expect("legacy stale checkpoint");
    drop(catalog);
    let mut events = Vec::new();

    run_scan_with_storage_reason(
        request,
        |event| {
            events.push(event);
            true
        },
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("resume legacy authoritative scan");

    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let (status, requires_previous_snapshot, retry_count): (String, bool, i64) = connection
        .query_row(
            "SELECT scans.status, scans.requires_previous_snapshot,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE root_id = ?2 AND scope = 'path'
                       AND relative_path = 'retained.png'
                       AND status IN ('pending', 'leased', 'retry_wait'))
             FROM scan_runs AS scans WHERE scans.id = ?1",
            rusqlite::params!["legacy-authoritative-recovery", root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("legacy recovery evidence");
    assert_eq!(status, "completed");
    assert!(!requires_previous_snapshot);
    assert_eq!(retry_count, 0);
}

#[cfg(windows)]
#[test]
fn authoritative_full_scan_with_new_placeholder_remains_stale_without_advancing_audit() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(source.path().join("published.png"))
        .expect("published fixture");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "placeholder-full-scan-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let before_audit: Option<i64> = connection
        .query_row(
            "SELECT last_consistency_audit_unix_ms FROM library_change_root_state LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("initial audit time");
    drop(connection);
    let placeholder_path = source.path().join("new-placeholder.png");
    RgbaImage::from_pixel(8, 6, Rgba([80, 100, 120, 255]))
        .save(&placeholder_path)
        .expect("placeholder fixture");
    set_scan_fixture_offline_attribute(&placeholder_path, true);
    let mut events = Vec::new();

    begin_authoritative_checkpoint(
        &storage_paths,
        &ScanRequest {
            scan_id: "sync-recovery-placeholder-full-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
    );
    run_scan_with_storage_reason(
        ScanRequest {
            scan_id: "sync-recovery-placeholder-full-scan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        || Ok(storage_paths.clone()),
        FullScanReason::ResumeAuthoritativeCheckpoint,
    )
    .expect("placeholder full scan remains recoverable");
    set_scan_fixture_offline_attribute(&placeholder_path, false);

    let after = load_test_snapshot(&storage_paths);
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    let (status, owner, after_audit): (String, String, Option<i64>) = connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, state.last_consistency_audit_unix_ms
             FROM scan_runs AS scans
             JOIN library_change_root_state AS state ON state.root_id = scans.root_id
             WHERE scans.id = 'sync-recovery-placeholder-full-scan'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("authoritative scan state");
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert_eq!(status, "stale");
    assert_eq!(owner, "authoritative_recovery");
    assert_eq!(after_audit, before_audit);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
}

#[cfg(windows)]
#[test]
fn migrated_v17_placeholder_preserves_the_normalized_legacy_location() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    let source_path = album.join("retained.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-normalization-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let before_asset = before.assets.first().expect("published asset");
    let old_location_id = before_asset.location_id.clone();
    let old_asset_id = before_asset.asset_id.clone();
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    remove_persistent_journal_v22_contract_for_test(&connection);
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
              DROP INDEX asset_locations_root_relative;
              DROP TABLE library_change_scan_handoff_items;
              DROP TABLE library_change_scan_handoff_lineage;
              DROP TABLE library_change_scan_handoff_batches;
              DROP TABLE library_change_queue_catch_up_lineage;
             DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             DROP TABLE scan_run_catch_up_lineage;
             DROP TABLE library_change_catch_up_handoffs;
             DROP INDEX scan_runs_one_active_root;
             ALTER TABLE library_change_queue DROP COLUMN authoritative_scan_id;
             ALTER TABLE library_change_root_state DROP COLUMN last_consistency_audit_unix_ms;
             ALTER TABLE scan_runs DROP COLUMN requires_previous_snapshot;
             ALTER TABLE scan_runs DROP COLUMN root_generation_at_start;
             ALTER TABLE scan_runs DROP COLUMN change_queue_high_watermark;
             ALTER TABLE scan_runs DROP COLUMN scan_owner;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_ownership_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN authoritative_recovery_complete;",
        )
        .expect("restore v17 table shape");
    connection
        .execute(
            "UPDATE preview_artifact_locations
             SET location_id = 'legacy-v17-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy preview owner");
    connection
        .execute(
            "UPDATE asset_locations
             SET relative_path = 'album\\retained.png',
                 location_id = 'legacy-v17-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy location identity");
    connection
        .execute_batch(
            "ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_handoff_batch_complete;
              ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_catch_up_lineage_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN change_catch_up_complete;
             DROP TABLE library_change_catch_up_state;
             UPDATE schema_info SET version = 17;",
        )
        .expect("restore v17 version");
    drop(connection);
    set_scan_fixture_offline_attribute(&source_path, true);
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-normalization-placeholder-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("migrated placeholder rescan remains recoverable");
    set_scan_fixture_offline_attribute(&source_path, false);

    let after = load_test_snapshot(&storage_paths);
    let retained = after.assets.first().expect("retained legacy location");
    let version: i64 = Connection::open(&storage_paths.catalog_path)
        .expect("migrated catalog")
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    assert!(matches!(events.last(), Some(ScanEvent::Stale { .. })));
    assert_eq!(version, 31);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
    assert_eq!(retained.location_id, "legacy-v17-location");
    assert_eq!(retained.relative_path, "album/retained.png");
    assert_eq!(retained.asset_id, old_asset_id);
}

#[cfg(windows)]
#[test]
fn migrated_v17_healthy_file_preserves_location_but_not_unproven_asset_identity() {
    let source = tempdir().expect("source directory");
    let storage = tempdir().expect("storage directory");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album directory");
    let source_path = album.join("healthy.png");
    RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
        .save(&source_path)
        .expect("fixture image");
    let storage_paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-healthy-initial".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage_paths.clone(),
    )
    .expect("initial scan");
    let before = load_test_snapshot(&storage_paths);
    let before_asset = before.assets.first().expect("published asset");
    let old_location_id = before_asset.location_id.clone();
    let old_asset_id = before_asset.asset_id.clone();
    let connection = Connection::open(&storage_paths.catalog_path).expect("catalog database");
    remove_persistent_journal_v22_contract_for_test(&connection);
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
              DROP INDEX asset_locations_root_relative;
              DROP TABLE library_change_scan_handoff_items;
              DROP TABLE library_change_scan_handoff_lineage;
              DROP TABLE library_change_scan_handoff_batches;
              DROP TABLE library_change_queue_catch_up_lineage;
             DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             DROP TABLE scan_run_catch_up_lineage;
             DROP TABLE library_change_catch_up_handoffs;
             DROP INDEX scan_runs_one_active_root;
             ALTER TABLE library_change_queue DROP COLUMN authoritative_scan_id;
             ALTER TABLE library_change_root_state DROP COLUMN last_consistency_audit_unix_ms;
             ALTER TABLE scan_runs DROP COLUMN requires_previous_snapshot;
             ALTER TABLE scan_runs DROP COLUMN root_generation_at_start;
             ALTER TABLE scan_runs DROP COLUMN change_queue_high_watermark;
             ALTER TABLE scan_runs DROP COLUMN scan_owner;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_ownership_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN authoritative_recovery_complete;",
        )
        .expect("restore v17 table shape");
    connection
        .execute(
            "UPDATE preview_artifact_locations
             SET location_id = 'legacy-v17-healthy-location'
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy preview owner");
    connection
        .execute(
            "UPDATE asset_locations
             SET relative_path = 'album\\healthy.png',
                 location_id = 'legacy-v17-healthy-location',
                 file_identity_scheme = NULL,
                 file_identity_value = NULL
             WHERE location_id = ?1",
            [&old_location_id],
        )
        .expect("restore legacy location without identity evidence");
    connection
        .execute_batch(
            "ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_handoff_batch_complete;
              ALTER TABLE library_change_queue_contract
               DROP COLUMN scan_catch_up_lineage_complete;
             ALTER TABLE library_change_queue_contract
               DROP COLUMN change_catch_up_complete;
             DROP TABLE library_change_catch_up_state;
             UPDATE schema_info SET version = 17;",
        )
        .expect("restore v17 version");
    drop(connection);
    let mut events = Vec::new();

    run_scan_with_storage(
        ScanRequest {
            scan_id: "v17-healthy-rescan".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage_paths.clone(),
    )
    .expect("healthy migrated rescan");

    let after = load_test_snapshot(&storage_paths);
    let retained = after.assets.first().expect("retained legacy location");
    assert!(matches!(events.last(), Some(ScanEvent::Completed { .. })));
    assert_eq!(after.assets.len(), 1);
    assert_eq!(retained.location_id, "legacy-v17-healthy-location");
    assert_eq!(retained.relative_path, "album/healthy.png");
    assert_ne!(retained.asset_id, old_asset_id);
    assert!(retained.file_identity.is_some());
    assert!(retained.source_revision.is_some());
    assert!(retained.source_generation > 0);
    assert!(!fs::read(&source_path).expect("source bytes").is_empty());
}

#[cfg(windows)]
fn set_scan_fixture_offline_attribute(path: &std::path::Path, is_offline: bool) {
    let status = std::process::Command::new("attrib.exe")
        .arg(if is_offline { "+O" } else { "-O" })
        .arg(path)
        .status()
        .expect("attrib executable");
    assert!(status.success());
}

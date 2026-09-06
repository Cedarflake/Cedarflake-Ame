use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusqlite::Connection;
use tempfile::tempdir;

use super::*;
use crate::adapters::SqliteCatalog;

fn usage(page_count: u64, freelist_count: u64) -> CatalogSpaceUsage {
    CatalogSpaceUsage {
        page_size: 4_096,
        page_count,
        freelist_count,
        auto_vacuum: CatalogAutoVacuumMode::Incremental,
    }
}

#[test]
fn reclamation_requires_both_absolute_and_ratio_thresholds() {
    assert!(should_start_reclamation(usage(100_000, 25_000)));
    assert!(!should_start_reclamation(usage(1_000_000, 20_000)));
    assert!(!should_start_reclamation(usage(10_000, 5_000)));
}

#[test]
fn incremental_reclamation_stops_at_the_low_watermark() {
    assert!(reclamation_target_reached(usage(100_000, 1_000)));
    assert!(reclamation_target_reached(usage(100_000, 5_000)));
    assert!(!reclamation_target_reached(usage(100_000, 6_000)));
}

#[test]
fn incremental_reclamation_keeps_the_256_page_write_window() {
    assert_eq!(INCREMENTAL_BATCH_PAGES, 256);
}

#[test]
fn large_catalog_model_converges_with_bounded_page_batches() {
    let mut current = usage(296_127, 294_956);
    let mut batches = 0_u32;

    while !reclamation_target_reached(current) {
        let reclaimed = current
            .freelist_count
            .min(u64::from(INCREMENTAL_BATCH_PAGES));
        current.page_count -= reclaimed;
        current.freelist_count -= reclaimed;
        batches += 1;
        assert!(batches <= 2_000, "bounded reclamation must converge");
    }

    assert_eq!(batches, 1_145);
    assert_eq!(current.page_count, 3_007);
    assert_eq!(current.freelist_count, 1_836);
    assert!(reclamation_target_reached(current));
}

#[test]
fn transient_maintenance_waits_past_the_old_retry_limit_and_cancels() {
    let operation = CatalogReclamationOperation::new("busy-until-cancelled".to_owned());
    let mut attempts = 0_u32;
    let mut delays = Vec::new();

    let result = retry_controlled_maintenance_with_wait(
        &operation,
        CatalogReclamationPhase::Reclaiming,
        |_| {
            attempts += 1;
            Ok::<_, ScanError>(CatalogMaintenanceAttempt::<()>::Busy)
        },
        |operation, delay| {
            delays.push(delay);
            if delays.len() == 10 {
                operation.cancel();
            }
            operation.is_cancelled()
        },
    )
    .expect("transient contention must not become a permanent failure");

    assert!(result.is_none());
    assert_eq!(attempts, 10);
    assert_eq!(
        delays,
        vec![
            Duration::from_millis(25),
            Duration::from_millis(50),
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(400),
            Duration::from_millis(800),
            Duration::from_millis(1_600),
            Duration::from_millis(3_200),
            Duration::from_millis(3_200),
            Duration::from_millis(3_200),
        ]
    );
    assert!(
        operation
            .finish(Ok(ReclamationCompletion::Cancelled))
            .expect("publish cancellation")
            .is_none()
    );
    assert_eq!(
        operation.snapshot().expect("cancelled snapshot").phase,
        CatalogReclamationPhase::Cancelled
    );
}

#[test]
fn non_transient_maintenance_failure_is_not_retried() {
    let operation = CatalogReclamationOperation::new("hard-failure".to_owned());
    let mut attempts = 0_u32;
    let error = retry_controlled_maintenance_with_wait::<()>(
        &operation,
        CatalogReclamationPhase::Reclaiming,
        |_| {
            attempts += 1;
            Err(ScanError::new(
                "catalog_reclamation_injected_failure",
                "Injected permanent failure",
            ))
        },
        |_, _| panic!("permanent failures must not enter backoff"),
    )
    .expect_err("permanent failure must propagate");

    assert_eq!(error.code, "catalog_reclamation_injected_failure");
    assert_eq!(attempts, 1);
}

#[test]
fn vacuum_capacity_uses_twice_the_main_database_plus_margin() {
    assert_eq!(
        required_vacuum_capacity(1_024).expect("bounded capacity"),
        2_048 + VACUUM_CAPACITY_MARGIN_BYTES,
    );
    assert_eq!(
        required_vacuum_capacity(u64::MAX).unwrap_err().code,
        "catalog_reclamation_capacity_overflow",
    );
}

#[test]
fn full_auto_vacuum_is_converted_before_reclamation_completes() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("ame.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
    Connection::open(&path)
        .expect("full-mode connection")
        .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = FULL; VACUUM;")
        .expect("enable full auto-vacuum");
    let operation = CatalogReclamationOperation::new("full-mode".to_owned());

    let result = run_reclamation_inner(&path, &operation);
    assert!(
        operation
            .finish(result)
            .expect("reclaim full-mode catalog")
            .is_none()
    );

    let maintenance = SqliteCatalogSpaceMaintenance::new(path);
    let after = match maintenance
        .inspect_catalog_space()
        .expect("inspect converted catalog")
    {
        CatalogMaintenanceAttempt::Completed(usage) => usage,
        CatalogMaintenanceAttempt::Busy => panic!("inspection unexpectedly busy"),
        CatalogMaintenanceAttempt::Interrupted => {
            panic!("inspection unexpectedly interrupted")
        }
    };
    assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::Incremental);
    assert_eq!(
        operation.snapshot().expect("completed snapshot").phase,
        CatalogReclamationPhase::Completed
    );
}

#[test]
fn maintenance_success_and_failure_invalidate_the_cached_catalog_session() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("ame.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
    Connection::open(&path)
        .expect("legacy-mode connection")
        .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE; VACUUM;")
        .expect("create legacy auto-vacuum fixture");
    super::super::catalog_session::reset_catalog_session(&path);
    crate::adapters::reset_full_schema_validation_count(&path);
    drop(
        super::super::catalog_session::open_catalog(
            &path,
            crate::domain::LibraryChangeLane::Recovery,
        )
        .expect("prime validated session"),
    );
    assert_eq!(crate::adapters::full_schema_validation_count(&path), 1);

    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    let operation = CatalogReclamationOperation::new("session-invalidation".to_owned());
    let converted = convert_with_retry(&path, &maintenance, &operation)
        .expect("convert legacy catalog")
        .expect("conversion was not cancelled");
    assert_eq!(converted.auto_vacuum, CatalogAutoVacuumMode::Incremental);
    drop(
        super::super::catalog_session::open_catalog(
            &path,
            crate::domain::LibraryChangeLane::Recovery,
        )
        .expect("revalidate after conversion"),
    );
    assert_eq!(crate::adapters::full_schema_validation_count(&path), 2);

    let injected = ScanError::new(
        "catalog_reclamation_injected_failure",
        "Injected maintenance failure",
    );
    assert_eq!(
        finish_maintenance_attempt::<()>(&path, Err(injected.clone()))
            .err()
            .expect("injected failure")
            .code,
        injected.code,
    );
    drop(
        super::super::catalog_session::open_catalog(
            &path,
            crate::domain::LibraryChangeLane::Recovery,
        )
        .expect("revalidate after failed maintenance"),
    );
    assert_eq!(crate::adapters::full_schema_validation_count(&path), 3);
}

#[test]
fn busy_maintenance_retains_the_validated_catalog_session() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("ame.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
    let writer = Connection::open(&path).expect("WAL writer");
    writer
        .execute_batch(
            "PRAGMA wal_autocheckpoint = 0;
         CREATE TABLE busy_checkpoint_fixture(value INTEGER);
         INSERT INTO busy_checkpoint_fixture VALUES (1);",
        )
        .expect("prepare WAL");
    let session = super::super::catalog_session::validated_catalog_session(&path)
        .expect("prime validated session");
    crate::adapters::reset_full_schema_validation_count(&path);
    let reader = Connection::open(&path).expect("WAL reader");
    reader.execute_batch("BEGIN;").expect("hold read snapshot");
    let _: i64 = reader
        .query_row("SELECT COUNT(*) FROM busy_checkpoint_fixture", [], |row| {
            row.get(0)
        })
        .expect("pin read snapshot");
    writer
        .execute("INSERT INTO busy_checkpoint_fixture VALUES (2)", [])
        .expect("append WAL");
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

    assert!(matches!(
        finish_maintenance_attempt(&path, maintenance.try_checkpoint_wal(&control))
            .expect("busy maintenance remains retryable"),
        CatalogMaintenanceAttempt::Busy,
    ));
    let current = super::super::catalog_session::validated_catalog_session(&path)
        .expect("reuse unchanged session");

    assert!(Arc::ptr_eq(&session, &current));
    drop(
        current
            .open_in_lane(crate::domain::LibraryChangeLane::Recovery)
            .expect("native identity and schema signature remain current"),
    );
    assert_eq!(crate::adapters::full_schema_validation_count(&path), 0);
    reader.execute_batch("ROLLBACK;").expect("release reader");
}

#[test]
fn busy_conversion_and_incremental_vacuum_preserve_the_validated_signature() {
    for mode in ["NONE", "FULL", "INCREMENTAL"] {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let writer = Connection::open(&path).expect("blocking writer");
        writer
            .execute_batch(&format!(
                "PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = {mode}; VACUUM;
             CREATE TABLE busy_vacuum_fixture(payload BLOB);
             INSERT INTO busy_vacuum_fixture VALUES (zeroblob(65536));
             DROP TABLE busy_vacuum_fixture;"
            ))
            .expect("prepare reclamation mode");
        let session = super::super::catalog_session::validated_catalog_session(&path)
            .expect("prime validated session");
        crate::adapters::reset_full_schema_validation_count(&path);
        let before: i64 = writer
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .expect("original mode");
        writer
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("hold SQLite writer");
        let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
        let attempt = if mode == "INCREMENTAL" {
            maintenance.try_reclaim_incremental(1, &control)
        } else {
            maintenance.try_convert_to_incremental(&control)
        };

        assert!(
            matches!(
                finish_maintenance_attempt(&path, attempt).expect("busy result"),
                CatalogMaintenanceAttempt::Busy
            ),
            "{mode}"
        );
        writer
            .execute_batch("ROLLBACK;")
            .expect("release SQLite writer");
        let current = super::super::catalog_session::validated_catalog_session(&path)
            .expect("reuse unchanged session");
        assert!(Arc::ptr_eq(&session, &current), "{mode}");
        drop(
            current
                .open_in_lane(crate::domain::LibraryChangeLane::Recovery)
                .expect("native identity and schema signature remain current"),
        );
        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            0,
            "{mode}"
        );
        let after: i64 = writer
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .expect("retained mode");
        assert_eq!(before, after, "{mode}");
    }
}

use super::*;
use crate::application::catalog_session;
use crate::application::scan_library::run_scan_with_storage;
use crate::application::storage::StoragePaths;
use crate::domain::{LibraryChangeLane, ScanEvent, ScanRequest};

#[test]
fn arriving_foreground_scan_preempts_structural_maintenance_before_using_renewed_proof() {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::sync::mpsc;

    let source = tempdir().expect("owned empty source");
    let storage = tempdir().expect("isolated storage");
    let paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    drop(SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog"));
    Connection::open(&paths.catalog_path)
        .expect("legacy fixture")
        .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE; VACUUM;")
        .expect("legacy mode");
    let previous =
        catalog_session::validated_catalog_session(&paths.catalog_path).expect("old proof");
    let operation = registry()
        .admit(
            reclamation_key(&paths.catalog_path),
            "maintenance-first".to_owned(),
            true,
        )
        .expect("reclamation admission")
        .worker
        .expect("worker ownership");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (interrupt_tx, interrupt_rx) = mpsc::channel();
    thread::scope(|scope| {
        let path = &paths.catalog_path;
        let worker = scope.spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let control = operation.begin_attempt().expect("attempt");
                let result = catalog_session::with_catalog_maintenance(path, || {
                    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
                    assert!(matches!(
                        maintenance
                            .try_convert_to_incremental(&control)
                            .expect("real structural conversion"),
                        CatalogMaintenanceAttempt::Completed(_)
                    ));
                    control.install_interrupt(Arc::new(move || {
                        let _ = interrupt_tx.send(());
                    }))?;
                    entered_tx
                        .send(())
                        .expect("maintenance holds admission after rewrite");
                    interrupt_rx
                        .recv_timeout(Duration::from_secs(5))
                        .expect("foreground scan preempts this exact attempt");
                    assert!(control.is_interrupted());
                    Ok::<_, ScanError>(CatalogMaintenanceAttempt::<()>::Interrupted)
                });
                operation.finish_attempt();
                assert!(matches!(
                    result.expect("maintenance outcome"),
                    CatalogMaintenanceAttempt::Interrupted
                ));
            }));
            operation
                .finish(Ok(ReclamationCompletion::Cancelled))
                .expect("retire fixture operation");
            if let Err(error) = result {
                resume_unwind(error);
            }
        });
        entered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("maintenance started first");
        let mut completed = false;
        run_scan_with_storage(
            ScanRequest {
                scan_id: "scan-after-maintenance".to_owned(),
                root_path: source.path().to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |event| {
                if matches!(event, ScanEvent::Started { .. }) {
                    let current = catalog_session::validated_catalog_session(&paths.catalog_path)
                        .expect("renewed proof");
                    assert!(!Arc::ptr_eq(&previous, &current));
                    drop(
                        catalog_session::open_catalog_reader(&paths.catalog_path)
                            .expect("scan remains readable"),
                    );
                }
                completed |= matches!(event, ScanEvent::Completed { .. });
                true
            },
            paths.clone(),
        )
        .expect("scan proceeds after preempted maintenance");
        assert!(completed);
        worker.join().expect("maintenance worker retires");
    });
    assert_eq!(
        fs::read_dir(source.path())
            .expect("source remains empty")
            .count(),
        0
    );
}

#[test]
fn checkpoint_preserves_catalog_reads_during_foreground_scan() {
    let source = tempdir().expect("owned empty source");
    let storage = tempdir().expect("isolated catalog storage");
    let paths = StoragePaths {
        catalog_path: storage.path().join("catalog.sqlite3"),
        preview_root: storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: storage.path().join("settings.sqlite3"),
    };
    let mut observed = false;
    run_scan_with_storage(
        ScanRequest {
            scan_id: format!("checkpoint-scan-{}", storage.path().display()),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if !matches!(event, ScanEvent::Started { .. }) {
                return true;
            }
            let session = catalog_session::validated_catalog_session(&paths.catalog_path)
                .expect("running scan shares valid session");
            drop(
                catalog_session::open_catalog_reader(&paths.catalog_path)
                    .expect("gallery reads work before checkpoint"),
            );
            crate::adapters::reset_full_schema_validation_count(&paths.catalog_path);
            let maintenance = SqliteCatalogSpaceMaintenance::new(paths.catalog_path.clone());
            let operation = CatalogReclamationOperation::new("checkpoint-scan".to_owned());
            assert!(
                checkpoint_with_retry(&maintenance, &operation)
                    .expect("real WAL checkpoint succeeds")
                    .is_some()
            );
            drop(
                session
                    .open_in_lane(LibraryChangeLane::Recovery)
                    .expect("original schema and file identity remain valid"),
            );
            for _ in 0..3 {
                drop(
                    catalog_session::open_catalog_reader(&paths.catalog_path)
                        .expect("checkpoint cannot disable reads for the running scan"),
                );
            }
            let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
            assert!(matches!(
                catalog_session::with_catalog_maintenance(&paths.catalog_path, || {
                    maintenance.try_convert_to_incremental(&control)
                })
                .expect("conversion must defer"),
                CatalogMaintenanceAttempt::Busy
            ));
            assert!(matches!(
                catalog_session::with_catalog_maintenance(&paths.catalog_path, || {
                    maintenance.try_reclaim_incremental(256, &control)
                })
                .expect("reclamation must defer"),
                CatalogMaintenanceAttempt::Busy
            ));
            drop(
                catalog_session::open_catalog_reader(&paths.catalog_path)
                    .expect("deferred structural work preserves the scan's proof"),
            );
            assert_eq!(
                crate::adapters::full_schema_validation_count(&paths.catalog_path),
                0
            );
            observed = true;
            true
        },
        paths.clone(),
    )
    .expect("scan still completes");
    assert!(observed);
    drop(
        catalog_session::open_catalog_reader(&paths.catalog_path)
            .expect("catalog remains readable after scan retires"),
    );
    assert_eq!(
        fs::read_dir(source.path())
            .expect("unchanged source")
            .count(),
        0
    );
}

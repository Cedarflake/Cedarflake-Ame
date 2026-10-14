use super::*;
use crate::application::catalog_session::{open_catalog_reader, validated_catalog_session};
use crate::domain::LibraryChangeLane;

#[test]
fn preempted_incremental_reclamation_preserves_interleaved_sync_reads() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let session = validated_catalog_session(&path).expect("validate catalog once");
    let retained = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("retain synchronization connection");
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    crate::adapters::reset_full_schema_validation_count(&path);

    for _ in 0..3 {
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
        control.preempt();
        assert!(matches!(
            crate::application::catalog_session::reclaim_catalog_pages(
                &path,
                &maintenance,
                INCREMENTAL_BATCH_PAGES,
                &control,
            )
            .expect("normal priority preemption"),
            CatalogMaintenanceAttempt::Interrupted
        ));
        session
            .revalidate_connection(&retained)
            .expect("retained connection remains valid after preemption");
        drop(open_catalog_reader(&path).expect("read without a full catalog validation"));
        assert!(Arc::ptr_eq(
            &session,
            &validated_catalog_session(&path).expect("reuse validated session")
        ));
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 0);
    }
}

#[test]
fn incremental_batches_preserve_validation_for_interleaved_catalog_reads() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
    Connection::open(&path)
        .expect("seed connection")
        .execute_batch(
            "CREATE TABLE reclamation_fixture(payload BLOB NOT NULL);
             WITH RECURSIVE rows(value) AS (
               SELECT 1 UNION ALL SELECT value + 1 FROM rows WHERE value < 1024
             )
             INSERT INTO reclamation_fixture(payload) SELECT zeroblob(4096) FROM rows;
             DELETE FROM reclamation_fixture;",
        )
        .expect("seed reclaimable derived pages");
    let session = validated_catalog_session(&path).expect("validate seeded catalog");
    let retained = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("retain idle synchronization connection");
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    let operation = CatalogReclamationOperation::new("validated-reclamation".to_owned());
    let mut previous = match maintenance.inspect_catalog_space().expect("initial usage") {
        CatalogMaintenanceAttempt::Completed(usage) => usage,
        _ => panic!("idle fixture inspection must complete"),
    };
    crate::adapters::reset_full_schema_validation_count(&path);

    for _ in 0..3 {
        let usage = reclaim_batch_with_retry(&path, &maintenance, &operation)
            .expect("reclaim a page batch")
            .expect("uncancelled batch");
        let reclaimed = previous.freelist_count - usage.freelist_count;
        assert!(reclaimed > 0 && reclaimed <= u64::from(INCREMENTAL_BATCH_PAGES));
        assert!(usage.page_count < previous.page_count);
        session
            .revalidate_connection(&retained)
            .expect("incremental reclamation preserves retained connection proof");
        drop(open_catalog_reader(&path).expect("read between page batches"));
        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            0,
            "space-only batches must not repeat the complete catalog audit"
        );
        assert!(Arc::ptr_eq(
            &session,
            &validated_catalog_session(&path).expect("reuse session")
        ));
        previous = usage;
    }

    Connection::open(&path)
        .expect("independent schema writer")
        .execute_batch("CREATE TABLE later_schema_change(value INTEGER);")
        .expect("schema change after reclamation");
    assert_eq!(
        session
            .revalidate_connection(&retained)
            .expect_err("schema mutation must still retire retained proof")
            .code,
        "catalog_validated_session_stale"
    );
    drop(retained);
    drop(open_catalog_reader(&path).expect("renew schema proof"));
    assert_eq!(crate::adapters::full_schema_validation_count(&path), 1);
    assert!(!Arc::ptr_eq(
        &session,
        &validated_catalog_session(&path).expect("new session")
    ));
}

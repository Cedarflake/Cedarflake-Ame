use super::super::tests::{path_intent, queue_catalog};
use super::*;

fn intents() -> Vec<LibraryChangeIntent> {
    ["one.png", "two.png"]
        .iter()
        .enumerate()
        .map(|(index, path)| {
            path_intent(
                "root-a",
                LibraryRootGeneration::initial(),
                index as u64 + 1,
                1_000,
                path,
            )
        })
        .collect()
}

fn assert_transaction_retired(catalog: &SqliteCatalog) {
    assert!(catalog.connection.is_autocommit());
    let timeout: u32 = catalog
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("restored timeout");
    assert_eq!(u128::from(timeout), SQLITE_BUSY_TIMEOUT.as_millis());
    assert!(
        catalog
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
            .is_some()
    );
}

#[test]
fn reserved_ingress_sql_failure_rolls_back_the_whole_batch_and_restores_connection() {
    let storage = tempfile::tempdir().expect("owned storage");
    let path = storage.path().join("catalog.sqlite3");
    let mut catalog = queue_catalog(path.clone());
    catalog.connection.execute_batch("CREATE TRIGGER reject_second_ingress BEFORE INSERT ON library_change_queue WHEN NEW.relative_path = 'two.png' BEGIN SELECT RAISE(ABORT, 'controlled second-row failure'); END;").expect("late batch failure");
    let mut reservation = catalog.reserve_change_ingress(&intents()).expect("reserve");
    let failure = catalog
        .try_enqueue_reserved_changes(&mut reservation, &intents(), 1_000, Default::default())
        .expect_err("second row fails");
    drop(reservation);
    assert_eq!(failure.code, "catalog_database_error");
    let rows: i64 = catalog
        .connection
        .query_row("SELECT count(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("rolled-back first row");
    assert_eq!(rows, 0);
    assert_transaction_retired(&catalog);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_second_ingress")
        .expect("remove failure");
    drop(catalog);
    SqliteCatalog::open(path).expect("FULL reopen after failed transaction");
}

#[test]
fn reserved_ingress_external_busy_keeps_priority_but_not_transaction_or_busy_timeout() {
    let storage = tempfile::tempdir().expect("owned storage");
    let path = storage.path().join("catalog.sqlite3");
    let mut catalog = queue_catalog(path.clone());
    let external = Connection::open(&path).expect("uncoordinated external writer");
    external
        .execute_batch("BEGIN IMMEDIATE")
        .expect("external lock");
    let mut reservation = catalog.reserve_change_ingress(&intents()).expect("reserve");
    let error = catalog
        .try_enqueue_reserved_changes(&mut reservation, &intents(), 1_000, Default::default())
        .expect_err("external writer");
    external
        .execute_batch("ROLLBACK")
        .expect("release external lock");
    assert_eq!(error.code, "catalog_database_busy");
    assert!(catalog.connection.is_autocommit());
    assert!(
        catalog
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
            .is_none(),
        "busy retains the live queue position"
    );
    let timeout: u32 = catalog
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("timeout after busy");
    assert_eq!(u128::from(timeout), SQLITE_BUSY_TIMEOUT.as_millis());
    catalog
        .try_enqueue_reserved_changes(&mut reservation, &intents(), 1_001, Default::default())
        .expect("same reservation commits after external writer release");
    drop(reservation);
    assert_transaction_retired(&catalog);
    drop(catalog);
    let reopened = SqliteCatalog::open(path).expect("FULL reopen");
    let rows: i64 = reopened
        .connection
        .query_row("SELECT count(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("exact publication");
    assert_eq!(rows, 2);
}

#[test]
fn reserved_ingress_rejects_other_catalog_root_generation_and_lane() {
    let storage = tempfile::tempdir().expect("owned storage");
    let mut catalog = queue_catalog(storage.path().join("one.sqlite3"));
    let mut other = queue_catalog(storage.path().join("two.sqlite3"));
    let mut reservation = catalog.reserve_change_ingress(&intents()).expect("reserve");
    let error = other
        .try_enqueue_reserved_changes(&mut reservation, &intents(), 1_000, Default::default())
        .expect_err("other catalog");
    assert_eq!(error.code, "change_ingress_reservation_mismatch");
    for changed in 0..3 {
        let mut batch = intents();
        for intent in &mut batch {
            match changed {
                0 => intent.root_id = "root-b".to_owned(),
                1 => {
                    intent.root_generation = LibraryRootGeneration::initial()
                        .next()
                        .expect("next generation")
                }
                _ => intent.origin = LibraryChangeOrigin::StartupCatchUp,
            }
        }
        let error = catalog
            .try_enqueue_reserved_changes(&mut reservation, &batch, 1_000, Default::default())
            .expect_err("changed binding");
        assert_eq!(error.code, "change_ingress_reservation_mismatch");
    }
    drop(reservation);
    assert_transaction_retired(&catalog);
    assert_transaction_retired(&other);
}

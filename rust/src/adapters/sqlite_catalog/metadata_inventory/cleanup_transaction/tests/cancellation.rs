use super::*;
use std::sync::atomic::AtomicUsize;

#[test]
fn bounded_cleanup_cancellation_during_sql_rolls_back_and_retires_its_handler() {
    let (_storage, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "CREATE TABLE cleanup_payload(value INTEGER);
        WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 2048)
        INSERT INTO cleanup_payload SELECT i FROM n",
        )
        .expect("bounded generated payload");
    let cancelled = Arc::new(AtomicBool::new(false));
    let observed = Arc::new(AtomicUsize::new(0));
    let signal = Arc::clone(&cancelled);
    let deletes = Arc::clone(&observed);
    catalog
        .connection
        .update_hook(Some(move |action, _: &str, table: &str, _| {
            if action == rusqlite::hooks::Action::SQLITE_DELETE && table == "cleanup_payload" {
                deletes.fetch_add(1, Ordering::Relaxed);
                signal.store(true, Ordering::Release);
            }
        }))
        .expect("observe actual SQL mutations");
    let error = try_cleanup_write(
        &mut catalog,
        InventoryCleanupControl::new(cancelled),
        |transaction| {
            transaction
                .execute("UPDATE cleanup_fixture SET value = 2", [])
                .map_err(database_error)?;
            transaction
                .execute("DELETE FROM cleanup_payload WHERE value > 0", [])
                .map_err(database_error)?;
            panic!("the production progress handler must stop SQL before all rows are visited");
        },
    )
    .expect_err("in-flight cancellation");
    catalog
        .connection
        .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
        .expect("retire SQL observer");
    assert_eq!(error.code, "metadata_inventory_cleanup_cancelled");
    assert!(observed.load(Ordering::Relaxed) > 0);
    assert!(observed.load(Ordering::Relaxed) < 2048);
    let remaining: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM cleanup_payload", [], |row| row.get(0))
        .expect("rollback evidence");
    assert_eq!(remaining, 2048);
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_pre_cancelled_read_and_write_do_not_admit_work() {
    let (_storage, mut catalog) = fixture();
    let control = InventoryCleanupControl::new(Arc::new(AtomicBool::new(true)));
    let epoch = catalog.completed_write_epoch();
    let read = read_candidates(&catalog.connection, &control, |_| panic!("cancelled read"));
    assert_eq!(
        read.expect_err("cancelled").code,
        "metadata_inventory_cleanup_cancelled"
    );
    let write = try_cleanup_write(&mut catalog, control, |_| panic!("cancelled write"));
    assert_eq!(
        write.expect_err("cancelled").code,
        "metadata_inventory_cleanup_cancelled"
    );
    assert_eq!(catalog.completed_write_epoch(), epoch);
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_cancellation_after_short_sql_cannot_commit() {
    let (_storage, mut catalog) = fixture();
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    catalog
        .connection
        .update_hook(Some(move |_, _: &str, table: &str, _| {
            if table == "cleanup_fixture" {
                signal.store(true, Ordering::Release);
            }
        }))
        .expect("observe short SQL mutation");
    let result = try_cleanup_write(
        &mut catalog,
        InventoryCleanupControl::new(cancelled),
        |transaction| {
            transaction
                .execute("UPDATE cleanup_fixture SET value = 2", [])
                .map_err(database_error)?;
            Ok(MetadataInventoryCleanupReport::default())
        },
    );
    catalog
        .connection
        .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
        .expect("retire observer");
    assert_eq!(
        result.expect_err("cancelled before commit").code,
        "metadata_inventory_cleanup_cancelled"
    );
    assert_restored(&catalog);
}

#[test]
fn bounded_cleanup_cancellation_interrupts_the_pre_admission_read() {
    let (_storage, catalog) = fixture();
    let cancelled = Arc::new(AtomicBool::new(false));
    let control = InventoryCleanupControl::new(Arc::clone(&cancelled));
    let result = read_candidates(&catalog.connection, &control, |connection| {
        cancelled.store(true, Ordering::Release);
        connection
            .query_row(
                "WITH RECURSIVE n(i) AS (
            SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 2048) SELECT SUM(i) > 0 FROM n",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)
    });
    assert_eq!(
        result.expect_err("cancelled read").code,
        "metadata_inventory_cleanup_cancelled"
    );
    assert_restored(&catalog);
}

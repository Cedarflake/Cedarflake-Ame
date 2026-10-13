use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

use crate::domain::{CatalogAutoVacuumMode, LibraryChangeLane};
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

use super::super::super::SqliteCatalogSpaceMaintenance;
use super::{SqliteCatalog, SqliteCatalogSession, fixture};

fn control() -> CatalogMaintenanceControl {
    CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)))
}

fn completed<T>(attempt: CatalogMaintenanceAttempt<T>) -> T {
    match attempt {
        CatalogMaintenanceAttempt::Completed(value) => value,
        CatalogMaintenanceAttempt::Busy => panic!("idle retained connection blocked maintenance"),
        CatalogMaintenanceAttempt::Interrupted => panic!("maintenance unexpectedly interrupted"),
    }
}

fn reclaimable_fixture(legacy_mode: bool) -> (TempDir, SqliteCatalogSession, SqliteCatalog) {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    SqliteCatalogSession::validate(path.clone()).expect("initialize catalog");
    let writer = Connection::open(&path).expect("seed reclaimable fixture");
    if legacy_mode {
        writer
            .execute_batch(
                "PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE;
                 VACUUM; PRAGMA journal_mode = WAL;",
            )
            .expect("prepare legacy auto-vacuum mode before retention");
    }
    writer
        .execute_batch(
            "CREATE TABLE reusable_reclamation_fixture(payload BLOB NOT NULL);
             WITH RECURSIVE rows(value) AS (
               SELECT 1 UNION ALL SELECT value + 1 FROM rows WHERE value < 64
             )
             INSERT INTO reusable_reclamation_fixture(payload) SELECT zeroblob(4096) FROM rows;
             DELETE FROM reusable_reclamation_fixture;",
        )
        .expect("seed bounded derived page debt");
    drop(writer);
    let session = SqliteCatalogSession::validate(path).expect("prove seeded catalog");
    let catalog = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("retain idle poll connection");
    (directory, session, catalog)
}

#[test]
fn reusable_connection_allows_wal_truncate_while_idle_and_still_alive() {
    let (_directory, session, catalog) = fixture();
    let writer = Connection::open(session.path()).expect("independent writer");
    writer
        .execute_batch(
            "PRAGMA wal_autocheckpoint = 0;
             UPDATE catalog_state SET revision = revision + 1;",
        )
        .expect("commit a WAL frame");
    drop(writer);
    session
        .revalidate_connection(&catalog)
        .expect("complete all poll reads before maintenance");
    let mut wal_path = session.path().as_os_str().to_os_string();
    wal_path.push("-wal");
    assert!(fs::metadata(&wal_path).expect("WAL evidence").len() > 0);
    let maintenance = SqliteCatalogSpaceMaintenance::new(session.path().to_path_buf());
    completed(
        maintenance
            .try_checkpoint_wal(&control())
            .expect("checkpoint with a live idle connection"),
    );
    assert_eq!(fs::metadata(wal_path).expect("truncated WAL").len(), 0);
    session
        .revalidate_connection(&catalog)
        .expect("checkpoint does not invalidate schema proof");
}

#[test]
fn reusable_connection_allows_bounded_reclamation_while_idle_and_still_alive() {
    let (_directory, session, catalog) = reclaimable_fixture(false);
    let maintenance = SqliteCatalogSpaceMaintenance::new(session.path().to_path_buf());
    let before = completed(maintenance.inspect_catalog_space().expect("page debt"));
    assert_eq!(before.auto_vacuum, CatalogAutoVacuumMode::Incremental);
    assert!(before.freelist_count > 8);
    session.revalidate_connection(&catalog).expect("idle proof");
    let after = completed(
        maintenance
            .try_reclaim_incremental(8, &control())
            .expect("bounded reclaim with a live idle connection"),
    );
    let reclaimed = before.freelist_count.saturating_sub(after.freelist_count);
    assert!(reclaimed > 0 && reclaimed <= 8);
    assert!(catalog.connection.is_autocommit());
    assert!(!catalog.connection.is_busy());
    let free_pages: i64 = catalog
        .connection
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .expect("retained connection observes reclamation");
    assert_eq!(u64::try_from(free_pages).unwrap(), after.freelist_count);
}

#[test]
fn reusable_connection_allows_legacy_vacuum_while_idle_and_then_rejects_old_proof() {
    let (_directory, session, catalog) = reclaimable_fixture(true);
    let maintenance = SqliteCatalogSpaceMaintenance::new(session.path().to_path_buf());
    let before = completed(
        maintenance
            .inspect_catalog_space()
            .expect("legacy page debt"),
    );
    assert_eq!(before.auto_vacuum, CatalogAutoVacuumMode::None);
    assert!(before.freelist_count > 0);
    session.revalidate_connection(&catalog).expect("idle proof");
    let after = completed(
        maintenance
            .try_convert_to_incremental(&control())
            .expect("VACUUM with a live idle connection"),
    );
    assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::Incremental);
    assert_eq!(after.freelist_count, 0);
    assert!(catalog.connection.is_autocommit());
    assert!(!catalog.connection.is_busy());
    super::assert_rejected(&session, &catalog, "catalog_validated_session_stale");
}

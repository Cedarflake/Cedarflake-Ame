use super::*;
use crate::domain::{LibraryChangeLane, ScanRequest};
use crate::ports::CatalogRepository;

fn begin_import(catalog: &mut SqliteCatalog, scan_id: &str, root_id: &str) {
    let root_path = format!("C:/fixture/{root_id}");
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: scan_id.to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            root_id,
            &root_path,
        )
        .expect("begin fixture import");
}

fn checkpoint_pages() -> i32 {
    crate::adapters::sqlite_catalog::operation_diagnostics::last_configured_checkpoint_pages()
        .expect("native threshold observed before the diagnostic hook")
}

#[test]
fn initial_import_window_follows_catalog_writers_and_retires_with_its_last_owner() {
    let directory = tempfile::tempdir().expect("isolated catalogs");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("scan catalog");
    let mut retained = SqliteCatalog::open(path.clone()).expect("retained writer");
    let mut unrelated =
        SqliteCatalog::open(directory.path().join("other.sqlite3")).expect("unrelated catalog");
    begin_import(&mut catalog, "scan", "root");
    let first = catalog
        .initial_import_write_window("scan")
        .unwrap()
        .unwrap();
    let second = catalog
        .initial_import_write_window("scan")
        .unwrap()
        .unwrap();

    retained.begin_write().unwrap().commit().unwrap();
    assert_eq!(checkpoint_pages(), INITIAL_IMPORT_CHECKPOINT_PAGES);
    let mut later = SqliteCatalog::open(path).expect("writer opened during import");
    later
        .begin_user_interactive_write()
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(checkpoint_pages(), INITIAL_IMPORT_CHECKPOINT_PAGES);
    unrelated.begin_write().unwrap().commit().unwrap();
    assert_eq!(checkpoint_pages(), AUTOMATIC_CHECKPOINT_PAGES);

    drop(first);
    retained
        .begin_preemptible_write_in_lane(LibraryChangeLane::Live, Arc::new(|| {}))
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(checkpoint_pages(), INITIAL_IMPORT_CHECKPOINT_PAGES);
    drop(second);
    retained.begin_write().unwrap().commit().unwrap();
    assert_eq!(checkpoint_pages(), AUTOMATIC_CHECKPOINT_PAGES);
    later
        .begin_user_interactive_write()
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(checkpoint_pages(), AUTOMATIC_CHECKPOINT_PAGES);
}

#[test]
fn paused_and_published_catalogs_do_not_admit_an_initial_import_window() {
    let directory = tempfile::tempdir().expect("isolated catalog");
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    begin_import(&mut catalog, "first", "root");
    assert!(
        catalog
            .initial_import_write_window("missing")
            .unwrap()
            .is_none()
    );
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET status = 'paused' WHERE id = 'first'",
            [],
        )
        .unwrap();
    assert!(
        catalog
            .initial_import_write_window("first")
            .unwrap()
            .is_none()
    );
    catalog
        .connection
        .execute_batch(
            "UPDATE scan_runs SET status = 'completed' WHERE id = 'first';
             UPDATE library_roots SET active_scan_id = 'first' WHERE id = 'root';",
        )
        .unwrap();
    begin_import(&mut catalog, "second", "another-root");
    assert!(
        catalog
            .initial_import_write_window("second")
            .unwrap()
            .is_none()
    );
}

#[test]
fn initial_import_window_keeps_full_durability_rollback_and_a_bounded_wal() {
    let directory = tempfile::tempdir().expect("isolated database");
    let path = directory.path().join("catalog.sqlite3");
    let mut connection = Connection::open(&path).unwrap();
    super::super::configure_catalog_connection(&connection).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE entries(id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
        )
        .unwrap();
    let policy = Arc::new(ImportCheckpointPolicy::default());
    let window = policy.enter();
    policy.apply(&connection).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA synchronous", [], |row| row.get::<_, i32>(0))
            .unwrap(),
        2,
    );
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i32>(0))
            .unwrap(),
        1,
    );
    for id in 0..600 {
        connection
            .execute("INSERT INTO entries VALUES (?1, zeroblob(8192))", [id])
            .unwrap();
    }
    let wal_bytes = std::fs::metadata(path.with_extension("sqlite3-wal"))
        .unwrap()
        .len();
    assert!(wal_bytes > 1024 * 1024);
    assert!(wal_bytes <= 16 * 1024 * 1024 + 64 * 1024);
    {
        let transaction = connection.transaction().unwrap();
        transaction.execute("DELETE FROM entries", []).unwrap();
    }
    let reader = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("independent reader");
    assert_eq!(
        reader
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        600,
    );
    drop(reader);
    drop(window);
    policy.apply(&connection).unwrap();
    assert_eq!(checkpoint_pages(), AUTOMATIC_CHECKPOINT_PAGES);
    drop(connection);
    let reopened = Connection::open(path).unwrap();
    assert_eq!(
        reopened
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        600,
    );
}

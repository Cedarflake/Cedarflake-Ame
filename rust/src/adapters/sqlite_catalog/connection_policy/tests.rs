use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};
use tempfile::TempDir;

use super::{SqliteCatalog, configure_catalog_connection, configure_catalog_read_connection};

fn fixture() -> (TempDir, PathBuf, Connection) {
    let directory = tempfile::tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("catalog connection");
    configure_catalog_connection(&connection).expect("catalog write policy");
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE entries(id INTEGER PRIMARY KEY, payload BLOB NOT NULL);",
        )
        .expect("catalog fixture");
    (directory, path, connection)
}

fn insert_entries(connection: &Connection, start: i64, count: i64) {
    for id in start..start + count {
        connection
            .execute(
                "INSERT INTO entries(id, payload) VALUES (?1, zeroblob(8192))",
                [id],
            )
            .expect("commit catalog entry");
    }
}

fn entry_count(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .expect("entry count")
}

#[test]
fn small_commits_reuse_a_one_mib_wal_window_without_losing_durable_rows() {
    let (_directory, path, mut connection) = fixture();
    assert_eq!(
        connection
            .query_row("PRAGMA synchronous", [], |row| row.get::<_, i64>(0))
            .expect("durability policy"),
        2,
    );
    insert_entries(&connection, 0, 320);
    let wal_bytes = std::fs::metadata(path.with_extension("sqlite3-wal"))
        .expect("active WAL")
        .len();
    assert!(
        wal_bytes <= 1024 * 1024 + 64 * 1024,
        "small commits accumulated {wal_bytes} WAL bytes before checkpointing",
    );
    {
        let transaction = connection.transaction().expect("rollback transaction");
        transaction
            .execute("DELETE FROM entries", [])
            .expect("uncommitted deletion");
    }
    let reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("independent reader");
    assert_eq!(entry_count(&reader), 320);
    drop(reader);
    drop(connection);
    let reopened = Connection::open(&path).expect("reopen committed catalog");
    assert_eq!(entry_count(&reopened), 320);
}

#[test]
fn pinned_read_snapshot_defers_checkpointing_without_blocking_commits() {
    let (_directory, path, connection) = fixture();
    insert_entries(&connection, 0, 1);
    let reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("snapshot reader");
    configure_catalog_read_connection(&reader).expect("catalog read policy");
    reader.execute_batch("BEGIN").expect("begin snapshot");
    assert_eq!(entry_count(&reader), 1);

    insert_entries(&connection, 1, 320);
    assert_eq!(entry_count(&reader), 1);
    assert_eq!(entry_count(&connection), 321);
    reader.execute_batch("COMMIT").expect("release snapshot");

    insert_entries(&connection, 321, 1);
    let (log_pages, checkpointed_pages): (i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
            Ok((row.get(1)?, row.get(2)?))
        })
        .expect("checkpoint after snapshot release");
    assert_eq!(log_pages, checkpointed_pages);
    assert_eq!(entry_count(&reader), 322);
    assert!(reader.execute("DELETE FROM entries", []).is_err());
}

#[test]
fn closing_the_retained_connection_preserves_the_committed_wal_without_rewriting_the_database() {
    let (_directory, path, mut catalog) = retained_fixture();
    catalog
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("publish fixture schema");
    insert_entries(&catalog.connection, 0, 8);
    {
        let transaction = catalog
            .connection
            .transaction()
            .expect("rollback transaction");
        transaction
            .execute("DELETE FROM entries", [])
            .expect("uncommitted deletion");
    }
    let wal_path = path.with_extension("sqlite3-wal");
    let database_before = std::fs::read(&path).expect("database before close");
    let wal_before = std::fs::read(&wal_path).expect("committed WAL before close");
    assert!(wal_before.len() > 32);

    drop(catalog);

    assert!(
        std::fs::read(&path).expect("database after close") == database_before,
        "retirement must not perform an implicit checkpoint",
    );
    assert!(
        std::fs::read(&wal_path).expect("committed WAL after close") == wal_before,
        "retirement must retain the committed WAL",
    );
    let reopened = Connection::open(&path).expect("reopen retained WAL");
    configure_catalog_connection(&reopened).expect("reopened write policy");
    assert_eq!(entry_count(&reopened), 8);
    assert_eq!(
        reopened
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            .expect("check retained catalog"),
        "ok",
    );
}

#[test]
fn explicit_checkpoint_reclaims_a_retained_wal_after_reopen() {
    let (_directory, path, catalog) = retained_fixture();
    insert_entries(&catalog.connection, 0, 8);
    drop(catalog);
    let wal_path = path.with_extension("sqlite3-wal");
    assert!(std::fs::metadata(&wal_path).expect("retained WAL").len() > 32);

    let reopened = Connection::open(&path).expect("reopen for maintenance");
    configure_catalog_connection(&reopened).expect("maintenance write policy");
    assert_eq!(entry_count(&reopened), 8);
    reopened
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("explicit checkpoint");
    assert_eq!(
        std::fs::metadata(&wal_path).expect("reclaimed WAL").len(),
        0
    );
    drop(reopened);
    let reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read checkpointed catalog");
    assert_eq!(entry_count(&reader), 8);
}

#[test]
fn ordinary_connections_finish_their_wal_on_close() {
    let (_directory, path, connection) = fixture();
    insert_entries(&connection, 0, 8);
    drop(connection);
    assert!(!path.with_extension("sqlite3-wal").exists());
    let reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read closed ordinary catalog");
    assert_eq!(entry_count(&reader), 8);
}

fn retained_fixture() -> (TempDir, PathBuf, SqliteCatalog) {
    let directory = tempfile::tempdir().expect("retained catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("retained catalog");
    catalog
        .retain_wal_on_close()
        .expect("retained close policy");
    catalog
        .connection
        .execute_batch("CREATE TABLE entries(id INTEGER PRIMARY KEY, payload BLOB NOT NULL)")
        .expect("retained catalog entries");
    (directory, path, catalog)
}

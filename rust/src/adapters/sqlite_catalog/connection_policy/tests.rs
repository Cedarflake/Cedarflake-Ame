use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};
use tempfile::TempDir;

use super::{configure_catalog_connection, configure_catalog_read_connection};

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

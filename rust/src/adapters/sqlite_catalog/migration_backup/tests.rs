use super::*;

fn fixture() -> (tempfile::TempDir, PathBuf, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE schema_info(version INTEGER NOT NULL);
             INSERT INTO schema_info VALUES (46);
             CREATE TABLE decisions(id TEXT PRIMARY KEY, album TEXT NOT NULL);
             INSERT INTO decisions VALUES ('image', 'retained album');",
        )
        .unwrap();
    (directory, path, connection)
}

#[test]
fn backup_recovers_committed_wal_data_after_the_live_catalog_changes() {
    let (directory, path, connection) = fixture();
    assert!(path.with_extension("sqlite3-wal").metadata().unwrap().len() > 0);
    let backup = prepare(&connection, &path, 47).unwrap().unwrap();
    connection
        .execute_batch(
            "UPDATE schema_info SET version = 47;
             UPDATE decisions SET album = 'later choice';",
        )
        .unwrap();
    let restored = directory.path().join("restored.sqlite3");
    std::fs::copy(&backup, &restored).unwrap();
    let recovery = Connection::open_with_flags(restored, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let actual: (i64, String) = recovery
        .query_row(
            "SELECT version, album FROM schema_info, decisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(actual, (46, "retained album".to_owned()));
    assert_eq!(
        std::fs::read_to_string(format!("{}.complete", backup.display())).unwrap(),
        "ame-catalog-backup-v1\nfrom=46\nto=47\n"
    );
}

#[test]
fn fresh_current_and_future_catalogs_do_not_create_backups() {
    let (directory, path, connection) = fixture();
    assert_eq!(prepare(&connection, &path, 46).unwrap(), None);
    assert_eq!(prepare(&connection, &path, 45).unwrap(), None);
    connection.execute_batch("DROP TABLE schema_info").unwrap();
    assert_eq!(prepare(&connection, &path, 47).unwrap(), None);
    assert!(std::fs::read_dir(directory.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("before-v")
    }));
}

#[test]
fn retry_keeps_the_previous_completed_snapshot() {
    let (_directory, path, connection) = fixture();
    let first = prepare(&connection, &path, 47).unwrap().unwrap();
    let original = std::fs::read(&first).unwrap();
    let second = prepare(&connection, &path, 47).unwrap().unwrap();
    assert_ne!(first, second);
    assert_eq!(std::fs::read(first).unwrap(), original);
}

#[test]
fn failed_backup_does_not_advance_the_schema_or_receive_a_completion_receipt() {
    let (directory, _path, connection) = fixture();
    let path = directory.path().join("interrupted.sqlite3");
    File::create(&path).unwrap();
    let error = create_snapshot(&connection, &path, 46, Duration::ZERO).unwrap_err();
    assert_eq!(error.code, "catalog_upgrade_backup_failed");
    let version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 46);
    assert!(!PathBuf::from(format!("{}.complete", path.display())).exists());
}

#[test]
fn an_unwritable_backup_destination_leaves_the_source_untouched() {
    let (directory, _path, connection) = fixture();
    let bad_path = directory.path().join("absent").join("catalog.sqlite3");
    assert_eq!(
        prepare(&connection, &bad_path, 47).unwrap_err().code,
        "catalog_upgrade_backup_failed"
    );
    assert_eq!(
        connection
            .query_row("SELECT album FROM decisions", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "retained album"
    );
}

#[test]
fn catalog_upgrade_retains_a_recoverable_snapshot_of_durable_duplicate_choices() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let catalog = super::super::SqliteCatalog::open(path.clone()).unwrap();
    super::super::migrations::downgrade_to_v46_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "INSERT INTO exact_duplicate_decisions VALUES (
           'blake3-256-full-v1',
           'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
           20, 'preferred-location', 'preferred-asset', 1, 123);
         DROP TRIGGER library_metadata_fallback_insert_guard;
         DROP TRIGGER library_metadata_fallback_update_guard;
         DROP TABLE library_metadata_fallbacks;
         UPDATE schema_info SET version = 45; PRAGMA user_version = 45;",
        )
        .unwrap();
    drop(catalog);
    let upgraded = super::super::SqliteCatalog::open(path).unwrap();
    let backup = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|entry| {
            let name = entry.file_name().unwrap().to_string_lossy();
            name.contains(&format!("before-v45-to-v{}", super::super::SCHEMA_VERSION))
                && name.ends_with(".sqlite3")
        })
        .unwrap();
    let snapshot = Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for database in [&upgraded.connection, &snapshot] {
        let actual: (String, bool) = database
            .query_row(
                "SELECT preferred_location_id, ignored FROM exact_duplicate_decisions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(actual, ("preferred-location".to_owned(), true));
    }
    assert_eq!(
        snapshot
            .query_row("SELECT version FROM schema_info", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        45
    );
}

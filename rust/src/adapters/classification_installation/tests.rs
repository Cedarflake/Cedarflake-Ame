use super::*;

#[test]
fn concurrent_startup_preserves_legacy_selection_and_pending_download() {
    use crate::ports::classification_acceleration::ClassificationAccelerationRepository;
    use std::sync::Barrier;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("classification.sqlite3");
    let selected = ClassificationInstallation {
        models: directory.path().join("models"),
        runtime: directory.path().join("runtime"),
    };
    let pending = DownloadIntent {
        generation: 7,
        package: "retained-model".into(),
    };
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE installation (id INTEGER PRIMARY KEY, models TEXT, runtime TEXT);
             CREATE TABLE download_intent (
                id INTEGER PRIMARY KEY, generation INTEGER, package TEXT, pending INTEGER
             );
             INSERT INTO download_intent VALUES (1, 7, 'retained-model', 1);
             PRAGMA user_version = 2;",
        )
        .unwrap();
    write_installation(&connection, &selected).unwrap();
    drop(connection);
    let startup = Barrier::new(8);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    startup.wait();
                    let store = ClassificationInstallationStore;
                    assert_eq!(store.load(&path).unwrap(), Some(selected.clone()));
                    assert!(store.gpu_enabled(&path).unwrap());
                    assert_eq!(
                        store.pending_download(&path, &pending.package).unwrap(),
                        Some(pending.clone())
                    );
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
    // Current-schema reads remain available while another connection owns a write.
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE;").unwrap();
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(selected)
    );
    connection.execute_batch("ROLLBACK;").unwrap();
}

#[test]
fn no_selection_leaves_storage_absent_and_explicit_selection_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("settings")
        .join("classification.sqlite3");
    assert_eq!(ClassificationInstallationStore.load(&path).unwrap(), None);
    assert!(!path.exists());
    let selected = ClassificationInstallation {
        models: directory.path().join("models"),
        runtime: directory.path().join("runtime"),
    };
    ClassificationInstallationStore
        .save(&path, &selected)
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(selected.clone())
    );
    let replacement = ClassificationInstallation {
        models: directory.path().join("replacement"),
        ..selected
    };
    ClassificationInstallationStore
        .save(&path, &replacement)
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(replacement)
    );
}

#[test]
fn an_unknown_storage_version_is_preserved_and_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("classification.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("PRAGMA user_version = 4;")
        .unwrap();
    assert!(ClassificationInstallationStore.load(&path).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        4
    );
}

#[test]
fn prepared_replacement_rejects_a_later_installation_selected_by_another_connection() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("classification.sqlite3");
    let first = ClassificationInstallation {
        models: directory.path().join("first"),
        runtime: directory.path().join("runtime"),
    };
    let second = ClassificationInstallation {
        models: directory.path().join("second"),
        ..first.clone()
    };
    let third = ClassificationInstallation {
        models: directory.path().join("third"),
        ..first.clone()
    };
    let cancelled = AtomicBool::new(false);
    ClassificationInstallationStore
        .save_if_unchanged(&path, None, &first, &cancelled)
        .unwrap();
    ClassificationInstallationStore
        .save(&path, &second)
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore
            .save_if_unchanged(&path, Some(&first), &third, &cancelled)
            .unwrap_err()
            .code,
        "classification_installation_changed"
    );
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(second.clone())
    );
    ClassificationInstallationStore
        .save_if_unchanged(&path, Some(&second), &third, &cancelled)
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(third)
    );
}

#[test]
fn cancellation_before_database_publication_preserves_the_saved_installation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("classification.sqlite3");
    let original = ClassificationInstallation {
        models: directory.path().join("original"),
        runtime: directory.path().join("runtime"),
    };
    ClassificationInstallationStore
        .save(&path, &original)
        .unwrap();
    let replacement = ClassificationInstallation {
        models: directory.path().join("replacement"),
        ..original.clone()
    };
    let result = ClassificationInstallationStore.save_if_unchanged(
        &path,
        Some(&original),
        &replacement,
        &AtomicBool::new(true),
    );
    assert_eq!(result.unwrap_err().code, "classification_cancelled");
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(original)
    );
}

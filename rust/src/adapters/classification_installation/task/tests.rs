use std::sync::atomic::AtomicBool;

use super::*;
use crate::adapters::{ClassificationInstallation, ClassificationInstallationStore};

#[test]
fn migration_preserves_version_one_selection_without_authorizing_a_download() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    assert!(
        DownloadIntent::pending(&path, "model-v1")
            .unwrap()
            .is_none()
    );
    assert!(!path.exists());
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE installation(id INTEGER PRIMARY KEY CHECK(id = 1), models TEXT NOT NULL, runtime TEXT NOT NULL) STRICT; PRAGMA user_version = 1;").unwrap();
    let selected = ClassificationInstallation {
        models: root.path().join("models"),
        runtime: root.path().join("runtime"),
    };
    connection
        .execute(
            "INSERT INTO installation VALUES(1, ?1, ?2)",
            params![
                selected.models.to_string_lossy(),
                selected.runtime.to_string_lossy()
            ],
        )
        .unwrap();
    drop(connection);
    assert!(
        DownloadIntent::pending(&path, "model-v1")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        ClassificationInstallationStore::load(&path).unwrap(),
        Some(selected)
    );
    assert_eq!(
        open(&path)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn reopened_intent_retains_authority_and_stale_cancel_cannot_retire_its_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    let first = DownloadIntent::authorize(&path, "model-v1").unwrap();
    assert_eq!(
        DownloadIntent::pending(&path, "model-v1").unwrap(),
        Some(first.clone())
    );
    assert_eq!(DownloadIntent::authorize(&path, "model-v1").unwrap(), first);
    assert!(
        DownloadIntent::pending(&path, "model-v2")
            .unwrap()
            .is_none()
    );
    first.cancel(&path).unwrap();
    let next = DownloadIntent::authorize(&path, "model-v1").unwrap();
    assert_ne!(first, next);
    first.cancel(&path).unwrap();
    assert!(first.require_current(&path).is_err());
    next.require_current(&path).unwrap();
}

#[test]
fn installation_selection_and_intent_completion_commit_atomically() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    let selected = ClassificationInstallation {
        models: root.path().join("models"),
        runtime: root.path().join("runtime"),
    };
    let first = DownloadIntent::authorize(&path, "model-v1").unwrap();
    let cancelled = AtomicBool::new(true);
    assert!(
        ClassificationInstallationStore::publish(&path, None, &selected, Some(&first), &cancelled)
            .is_err()
    );
    assert_eq!(ClassificationInstallationStore::load(&path).unwrap(), None);
    first.require_current(&path).unwrap();
    first.cancel(&path).unwrap();
    let next = DownloadIntent::authorize(&path, "model-v1").unwrap();
    let active = AtomicBool::new(false);
    assert!(
        ClassificationInstallationStore::publish(&path, None, &selected, Some(&first), &active)
            .is_err()
    );
    assert_eq!(ClassificationInstallationStore::load(&path).unwrap(), None);
    next.require_current(&path).unwrap();
    ClassificationInstallationStore::publish(&path, None, &selected, Some(&next), &active).unwrap();
    assert_eq!(
        ClassificationInstallationStore::load(&path).unwrap(),
        Some(selected)
    );
    assert!(
        DownloadIntent::pending(&path, "model-v1")
            .unwrap()
            .is_none()
    );
}

use std::sync::atomic::AtomicBool;

use super::super::ClassificationInstallationStore;
use super::*;
use crate::domain::classification::installation::ClassificationInstallation;
use crate::ports::classification_installation::ClassificationInstallationRepository;

#[test]
fn migration_preserves_version_one_selection_without_authorizing_a_download() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    assert!(
        ClassificationInstallationStore
            .pending_download(&path, "model-v1")
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
        ClassificationInstallationStore
            .pending_download(&path, "model-v1")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(selected)
    );
}

#[test]
fn reopened_intent_retains_authority_and_stale_cancel_cannot_retire_its_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    let first = ClassificationInstallationStore
        .authorize_download(&path, "model-v1")
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore
            .pending_download(&path, "model-v1")
            .unwrap(),
        Some(first.clone())
    );
    assert_eq!(
        ClassificationInstallationStore
            .authorize_download(&path, "model-v1")
            .unwrap(),
        first
    );
    assert!(
        ClassificationInstallationStore
            .pending_download(&path, "model-v2")
            .unwrap()
            .is_none()
    );
    ClassificationInstallationStore
        .cancel_download(&path, &first)
        .unwrap();
    let next = ClassificationInstallationStore
        .authorize_download(&path, "model-v1")
        .unwrap();
    assert_ne!(first, next);
    ClassificationInstallationStore
        .cancel_download(&path, &first)
        .unwrap();
    assert!(
        ClassificationInstallationStore
            .require_current_download(&path, &first)
            .is_err()
    );
    ClassificationInstallationStore
        .require_current_download(&path, &next)
        .unwrap();
}

#[test]
fn installation_selection_and_intent_completion_commit_atomically() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    let selected = ClassificationInstallation {
        models: root.path().join("models"),
        runtime: root.path().join("runtime"),
    };
    let first = ClassificationInstallationStore
        .authorize_download(&path, "model-v1")
        .unwrap();
    let cancelled = AtomicBool::new(true);
    assert!(
        ClassificationInstallationStore
            .publish(&path, None, &selected, Some(&first), &cancelled)
            .is_err()
    );
    assert_eq!(ClassificationInstallationStore.load(&path).unwrap(), None);
    ClassificationInstallationStore
        .require_current_download(&path, &first)
        .unwrap();
    ClassificationInstallationStore
        .cancel_download(&path, &first)
        .unwrap();
    let next = ClassificationInstallationStore
        .authorize_download(&path, "model-v1")
        .unwrap();
    let active = AtomicBool::new(false);
    assert!(
        ClassificationInstallationStore
            .publish(&path, None, &selected, Some(&first), &active)
            .is_err()
    );
    assert_eq!(ClassificationInstallationStore.load(&path).unwrap(), None);
    ClassificationInstallationStore
        .require_current_download(&path, &next)
        .unwrap();
    ClassificationInstallationStore
        .publish(&path, None, &selected, Some(&next), &active)
        .unwrap();
    assert_eq!(
        ClassificationInstallationStore.load(&path).unwrap(),
        Some(selected)
    );
    assert!(
        ClassificationInstallationStore
            .pending_download(&path, "model-v1")
            .unwrap()
            .is_none()
    );
}

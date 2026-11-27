use std::sync::atomic::AtomicBool;

use super::super::ClassificationInstallationStore;
use super::*;
use crate::domain::classification::installation::ClassificationInstallation;
use crate::ports::classification_installation::ClassificationInstallationRepository;

#[test]
fn unsupported_settings_preserve_selection_without_authorizing_a_download() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.sqlite3");
    assert!(
        ClassificationInstallationStore
            .pending_download(&path, "model-v1")
            .unwrap()
            .is_none()
    );
    assert!(!path.exists());
    for version in [1, 2, 4] {
        let path = root.path().join(format!("unsupported-{version}.sqlite3"));
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
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        assert!(
            ClassificationInstallationStore
                .pending_download(&path, "model-v1")
                .is_err()
        );
        assert_eq!(
            connection
                .query_row("SELECT models FROM installation", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            selected.models.to_string_lossy()
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            version
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'download_intent'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}

#[test]
fn an_unidentified_database_cannot_authorize_model_downloads() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("foreign.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE sqliteX_notes(value TEXT); INSERT INTO sqliteX_notes VALUES ('keep');",
        )
        .unwrap();
    assert!(
        ClassificationInstallationStore
            .authorize_download(&path, "model-v1")
            .is_err()
    );
    let objects: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_schema")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(objects, ["sqliteX_notes"]);
    assert_eq!(
        connection
            .query_row("SELECT value FROM sqliteX_notes", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "keep"
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

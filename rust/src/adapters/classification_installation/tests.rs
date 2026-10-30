use super::*;

#[test]
fn no_selection_leaves_storage_absent_and_explicit_selection_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("settings")
        .join("classification.sqlite3");
    assert_eq!(ClassificationInstallationStore::load(&path).unwrap(), None);
    assert!(!path.exists());
    let selected = ClassificationInstallation {
        models: directory.path().join("models"),
        runtime: directory.path().join("runtime"),
    };
    ClassificationInstallationStore::save(&path, &selected).unwrap();
    assert_eq!(
        ClassificationInstallationStore::load(&path).unwrap(),
        Some(selected.clone())
    );
    let replacement = ClassificationInstallation {
        models: directory.path().join("replacement"),
        ..selected
    };
    ClassificationInstallationStore::save(&path, &replacement).unwrap();
    assert_eq!(
        ClassificationInstallationStore::load(&path).unwrap(),
        Some(replacement)
    );
}

#[test]
fn an_unknown_storage_version_is_preserved_and_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("classification.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("PRAGMA user_version = 2;")
        .unwrap();
    assert!(ClassificationInstallationStore::load(&path).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
}

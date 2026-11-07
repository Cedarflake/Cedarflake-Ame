use std::io::Write;

use super::*;

#[test]
fn rebuilds_owned_corruption_and_interrupted_reset_without_touching_neighbor_data() {
    for damage in ["missing", "vector", "header", "interrupted reset"] {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("features");
        let catalog = root.path().join("catalog.sqlite3");
        std::fs::write(&catalog, b"retained catalog and album decisions").unwrap();
        let mut store = SqliteSemanticFeatureStore::open(&directory).unwrap();
        let model = model(2);
        store
            .publish_image(model, &content(1), &image(model))
            .unwrap();
        if damage == "vector" {
            store
                .connection
                .execute(
                    "UPDATE image_features SET vector = zeroblob(length(vector))",
                    [],
                )
                .unwrap();
        }
        drop(store);
        let path = directory.join(storage::FILE_NAME);
        if damage == "missing" {
            std::fs::remove_file(&path).unwrap();
        } else if damage == "header" {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .write_all(&[0; 72])
                .unwrap();
        } else if damage == "interrupted reset" {
            let connection = Connection::open(&path).unwrap();
            connection
                .set_db_config(
                    rusqlite::config::DbConfig::SQLITE_DBCONFIG_RESET_DATABASE,
                    true,
                )
                .unwrap();
            connection.execute_batch("VACUUM").unwrap();
        }
        SqliteSemanticFeatureStore::recover(&directory)
            .unwrap_or_else(|error| panic!("{damage}: {error:?}"));
        let mut rebuilt = SqliteSemanticFeatureStore::open(&directory).unwrap();
        assert_eq!(rebuilt.storage().unwrap().features, 0, "{damage}");
        assert_eq!(rebuilt.load_image(model, &content(1)).unwrap(), None);
        rebuilt
            .publish_image(model, &content(1), &image(model))
            .unwrap();
        drop(rebuilt);
        let reopened = SqliteSemanticFeatureStore::open(&directory).unwrap();
        assert_eq!(
            reopened.load_image(model, &content(1)).unwrap(),
            Some(image(model))
        );
        assert_eq!(
            std::fs::read(&catalog).unwrap(),
            b"retained catalog and album decisions"
        );
    }
}

#[test]
fn retry_preserves_healthy_features_and_refuses_a_locked_database() {
    let root = tempfile::tempdir().unwrap();
    let model = model(2);
    let mut store = SqliteSemanticFeatureStore::open(root.path()).unwrap();
    store
        .publish_image(model, &content(1), &image(model))
        .unwrap();
    let state = store.storage().unwrap();
    drop(store);
    SqliteSemanticFeatureStore::recover(root.path()).unwrap();
    let reopened = SqliteSemanticFeatureStore::open(root.path()).unwrap();
    assert_eq!(reopened.storage().unwrap(), state);
    drop(reopened);
    let connection = Connection::open(root.path().join(storage::FILE_NAME)).unwrap();
    connection.execute_batch("BEGIN EXCLUSIVE").unwrap();
    assert_eq!(
        SqliteSemanticFeatureStore::recover(root.path())
            .unwrap_err()
            .code,
        "semantic_storage_failed"
    );
    connection.execute_batch("ROLLBACK").unwrap();
    drop(connection);
    let reopened = SqliteSemanticFeatureStore::open(root.path()).unwrap();
    assert_eq!(
        reopened.load_image(model, &content(1)).unwrap(),
        Some(image(model))
    );
}

#[test]
fn recovery_refuses_a_substituted_file_even_when_the_old_ownership_receipt_exists() {
    let root = tempfile::tempdir().unwrap();
    drop(SqliteSemanticFeatureStore::open(root.path()).unwrap());
    let path = root.path().join(storage::FILE_NAME);
    std::fs::rename(&path, root.path().join("original.sqlite3")).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES ('keep')")
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        SqliteSemanticFeatureStore::recover(root.path())
            .unwrap_err()
            .code,
        "semantic_storage_invalid"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

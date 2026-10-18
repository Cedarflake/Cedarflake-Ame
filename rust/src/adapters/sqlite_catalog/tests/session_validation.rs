use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn validated_session_reopens_one_hundred_times_without_full_validation() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    reset_full_schema_validation_count(&path);
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    assert_eq!(full_schema_validation_count(&path), 1);

    for _ in 0..100 {
        session
            .open_in_lane(LibraryChangeLane::Live)
            .expect("O(1) validated reopen");
    }

    assert_eq!(full_schema_validation_count(&path), 1);
}

#[test]
fn validated_session_fails_closed_after_schema_cookie_changes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    let connection = Connection::open(path).expect("tamper connection");
    connection
        .execute_batch("CREATE TABLE unexpected_runtime_table(value INTEGER)")
        .expect("change schema cookie");

    let error = match session.open_in_lane(LibraryChangeLane::Live) {
        Ok(_) => panic!("a changed schema cookie must stale the runtime session"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_validated_session_stale");
}

#[test]
fn validated_session_fails_closed_after_header_identity_changes() {
    for pragma in ["PRAGMA application_id = 0", "PRAGMA user_version = 26"] {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.sqlite3");
        let session =
            SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
        let connection = Connection::open(path).expect("tamper connection");
        connection
            .execute_batch(pragma)
            .expect("change header identity");

        let error = match session.open_in_lane(LibraryChangeLane::Live) {
            Ok(_) => panic!("a changed catalog header must stale the runtime session"),
            Err(error) => error,
        };
        assert_eq!(error.code, "catalog_validated_session_stale");
    }
}

#[test]
fn validated_session_fails_closed_after_same_path_file_replacement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let displaced_path = directory.path().join("displaced.sqlite3");
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    fs::rename(&path, &displaced_path).expect("displace validated catalog");
    SqliteCatalogSession::validate(path.clone()).expect("create replacement catalog");

    let error = match session.open_in_lane(LibraryChangeLane::Live) {
        Ok(_) => panic!("a same-path file replacement must stale the runtime session"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_validated_session_stale");
}

#[test]
fn validated_session_ignores_normal_data_version_changes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    reset_full_schema_validation_count(&path);
    let session = SqliteCatalogSession::validate(path.clone()).expect("validate runtime catalog");
    let connection = Connection::open(path.clone()).expect("data writer");
    connection
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .expect("change ordinary catalog data");
    drop(connection);

    session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("ordinary data changes keep the validated session current");
    assert_eq!(full_schema_validation_count(&path), 1);
}

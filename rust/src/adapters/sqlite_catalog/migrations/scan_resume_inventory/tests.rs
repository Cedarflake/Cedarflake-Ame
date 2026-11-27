use tempfile::tempdir;

use crate::adapters::SqliteCatalog;

use super::*;

#[test]
fn missing_publication_guard_is_not_repaired_as_a_valid_current_catalog() {
    let directory = tempdir().expect("derived fixture");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
    let connection = Connection::open(&path).expect("fixture connection");
    connection
        .execute_batch("DROP TRIGGER scan_resume_publication_guard")
        .expect("damaged fixture");
    drop(connection);
    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("missing publication guard was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_scan_resume_inventory_unverifiable");
}

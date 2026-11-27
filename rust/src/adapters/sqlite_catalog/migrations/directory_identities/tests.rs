use tempfile::tempdir;

use crate::adapters::SqliteCatalog;

#[test]
fn missing_directory_identity_contract_is_rejected_on_reopen() {
    let storage = tempdir().expect("derived storage");
    let path = storage.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX scan_directory_identities_by_identity")
        .expect("damaged required lookup");
    drop(catalog);
    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("missing directory contract must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_directory_identity_contract_invalid");
}

use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::adapters::sqlite_catalog::migrations::current_schema;

#[test]
fn startup_rejects_missing_dispatch_state() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_admission", [])
        .unwrap();
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_state", [])
        .unwrap();
    assert!(current_schema::validate_current_schema_contract(&catalog.connection).is_err());
}

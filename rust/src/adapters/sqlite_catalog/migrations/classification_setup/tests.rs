use crate::adapters::SqliteCatalog;

use super::*;

#[test]
fn startup_rejects_a_missing_or_impossible_source_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute(
            "UPDATE classification_dispatch_state SET source_cursor = 1",
            [],
        )
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog
        .connection
        .execute(
            "UPDATE classification_dispatch_state SET source_cursor = 0",
            [],
        )
        .unwrap();
    catalog.connection.execute_batch(
        "UPDATE classification_dispatch_admission SET arrival_cursor = (SELECT next_source_generation FROM catalog_state);",
    ).unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_admission", [])
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
}

use rusqlite::Connection;

use super::*;

#[test]
fn first_import_capture_admission_failure_abandons_its_already_started_scan() {
    let source = tempfile::tempdir().expect("controlled empty source");
    let derived = tempfile::tempdir().expect("derived storage");
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
    let connection = Connection::open(&storage.catalog_path).expect("fault owner");
    connection.execute_batch(
        "CREATE TRIGGER withdraw_first_import_admission AFTER INSERT ON scan_runs
         WHEN NEW.id = 'withdrawn-first-import-admission'
         BEGIN UPDATE library_change_root_state SET is_active = 0 WHERE root_id = NEW.root_id; END;",
    ).expect("withdraw admission after durable begin");
    let mut events = Vec::new();
    let error = run_scan_with_storage(
        ScanRequest {
            scan_id: "withdrawn-first-import-admission".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect_err("post-begin admission failure");
    assert_eq!(error.code, "catalog_first_import_root_missing");
    assert!(
        events.is_empty(),
        "no observer-first scan starts without its owning root"
    );
    let evidence: (String, i64, i64, i64) = connection.query_row(
        "SELECT status,
                (SELECT COUNT(*) FROM scan_directory_frontier WHERE scan_id = scan.id),
                (SELECT COUNT(*) FROM asset_locations WHERE scan_id = scan.id),
                (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings WHERE scan_id = scan.id)
         FROM scan_runs AS scan WHERE id = 'withdrawn-first-import-admission'", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).expect("failed owner is settled");
    assert_eq!(evidence, ("failed".to_owned(), 0, 0, 0));
    connection
        .execute_batch("DROP TRIGGER withdraw_first_import_admission")
        .expect("remove fault");
    SqliteCatalog::open(storage.catalog_path).expect("full schema remains verifiable");
}

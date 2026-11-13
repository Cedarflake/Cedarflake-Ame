use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::ports::organization::OrganizationRepository;

#[test]
fn album_year_upgrade_preserves_history_and_rolls_back_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let album = catalog
        .create_organization_album("album", 0, "旅行", Default::default())
        .unwrap();
    let history = catalog.load_organization_history().unwrap();
    downgrade_for_test(&catalog.connection);
    catalog.connection.execute_batch(
        "INSERT INTO file_operation_exports
         (id, batch_id, authorized_unix_ms, action, rule, selected_files, root_path,
          identity_scheme, identity_value, report_state, report_path)
         VALUES (7, 'previous', 1700000000000, 'move', 'date', 2, 'C:\\Destination',
                 'fixture', 'root', 'written', 'C:\\Destination\\result.txt');
         INSERT INTO file_operation_export_skips VALUES ('previous', 1, 'C:\\Source\\图片.png', '没有可用的图库日期');
         CREATE TRIGGER reject_year_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 82 BEGIN SELECT RAISE(ABORT, 'injected'); END;"
    ).unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 81).unwrap();
    assert_history(&catalog.connection);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_year_upgrade;")
        .unwrap();
    drop(catalog);

    let catalog = SqliteCatalog::open(path).unwrap();
    current_schema::validate_current_schema_contract(&catalog.connection).unwrap();
    assert_history(&catalog.connection);
    assert_eq!(
        catalog
            .find_organization_album(&album.album_id)
            .unwrap()
            .unwrap()
            .name,
        "旅行"
    );
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    catalog.connection.execute_batch(
        "INSERT INTO file_operation_exports
         (batch_id, authorized_unix_ms, action, rule, selected_files, root_path, identity_scheme, identity_value)
         VALUES ('year-move', 1700000001000, 'move', 'album_year', 1, 'C:\\Destination', 'fixture', 'root');"
    ).unwrap();
    let id: i64 = catalog
        .connection
        .query_row(
            "SELECT id FROM file_operation_exports WHERE batch_id = 'year-move'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(id, 8);
    for sql in [
        "UPDATE file_operation_exports SET rule = 'album' WHERE batch_id = 'year-move'",
        "DELETE FROM file_operation_exports WHERE batch_id = 'year-move'",
        "DELETE FROM file_operation_export_skips WHERE batch_id = 'previous'",
        "INSERT INTO file_operation_export_skips VALUES ('unknown', 1, 'file.png', 'missing batch')",
    ] {
        assert!(catalog.connection.execute_batch(sql).is_err(), "{sql}");
    }
}

fn assert_history(connection: &Connection) {
    let row: (i64, String, i64, String, String) = connection.query_row(
        "SELECT id, rule, selected_files, report_state, report_path FROM file_operation_exports WHERE batch_id = 'previous'",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).unwrap();
    assert_eq!(
        row,
        (
            7,
            "date".into(),
            2,
            "written".into(),
            "C:\\Destination\\result.txt".into()
        )
    );
    let skipped: (i64, String) = connection
        .query_row(
            "SELECT ordinal, reason FROM file_operation_export_skips WHERE batch_id = 'previous'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(skipped, (1, "没有可用的图库日期".into()));
    let temporary: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name IN ('file_operation_exports_upgrade', 'file_operation_skips_upgrade')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(temporary, 0);
}

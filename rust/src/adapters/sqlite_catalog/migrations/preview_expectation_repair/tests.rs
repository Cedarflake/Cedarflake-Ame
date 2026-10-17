use tempfile::NamedTempFile;

use super::super::migrate_schema;
use super::*;

fn preview_fixture() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info VALUES (19);
             CREATE TABLE library_change_catch_up_handoffs (
                 preview_path TEXT, preview_status TEXT,
                 preview_issue_code TEXT, preview_issue_message TEXT
             );
             CREATE TABLE library_change_scan_handoff_items (
                 preview_path TEXT, preview_status TEXT,
                 preview_issue_code TEXT, preview_issue_message TEXT
             );
             CREATE TABLE library_roots (id TEXT PRIMARY KEY, active_scan_id TEXT);
             INSERT INTO library_roots VALUES ('root', 'active');
             CREATE TABLE asset_locations (
                 location_id TEXT PRIMARY KEY, root_id TEXT, scan_id TEXT,
                 preview_path TEXT, preview_status TEXT,
                 preview_issue_code TEXT, preview_issue_message TEXT
             );
             CREATE TABLE preview_artifacts (artifact_path TEXT, lifecycle_state TEXT);
             INSERT INTO preview_artifacts VALUES ('valid', 'ready'), ('unready', 'missing');
             CREATE TABLE preview_artifact_locations (location_id TEXT PRIMARY KEY);
             INSERT INTO library_change_catch_up_handoffs VALUES
                 ('missing', 'ready', 'old', 'old issue'),
                 ('unready', 'ready', NULL, NULL),
                 ('valid', 'ready', NULL, NULL);
             INSERT INTO library_change_scan_handoff_items
                 SELECT * FROM library_change_catch_up_handoffs;
             INSERT INTO asset_locations VALUES
                 ('missing', 'root', 'active', 'missing', 'ready', 'old', 'old issue'),
                 ('valid', 'root', 'active', 'valid', 'ready', NULL, NULL),
                 ('retained', 'root', 'old-scan', 'missing', 'ready', NULL, NULL),
                 ('pending', 'root', 'active', 'retry', 'pending', 'retry', 'retry issue');
             INSERT INTO preview_artifact_locations VALUES ('missing'), ('valid'), ('retained');",
        )
        .unwrap();
    connection
}

#[test]
fn repair_only_retires_missing_ready_expectations_in_the_current_publication() {
    let mut connection = preview_fixture();
    repair(&mut connection).unwrap();
    for table in [
        "library_change_catch_up_handoffs",
        "library_change_scan_handoff_items",
    ] {
        let retired: i64 = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {table}
                     WHERE preview_path = '' AND preview_status = 'pending'
                       AND preview_issue_code IS NULL AND preview_issue_message IS NULL"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retired, 2);
        let valid: i64 = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {table}
                     WHERE preview_path = 'valid' AND preview_status = 'ready'"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(valid, 1);
    }
    let states = connection
        .prepare(
            "SELECT location_id, preview_path, preview_status, preview_issue_code
             FROM asset_locations ORDER BY location_id",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        states,
        vec![
            ("missing".into(), "".into(), "pending".into(), None),
            (
                "pending".into(),
                "retry".into(),
                "pending".into(),
                Some("retry".into())
            ),
            ("retained".into(), "missing".into(), "ready".into(), None),
            ("valid".into(), "valid".into(), "ready".into(), None),
        ]
    );
    let retained_links: String = connection
        .query_row(
            "SELECT group_concat(location_id, ',') FROM (
                 SELECT location_id FROM preview_artifact_locations ORDER BY location_id
             )",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained_links, "retained,valid");
    assert!(marker_is_complete(&connection).unwrap());
    repair(&mut connection).unwrap();
    assert!(marker_is_complete(&connection).unwrap());
}

#[test]
fn repair_failure_rolls_back_handoffs_links_and_the_completion_marker() {
    let mut connection = preview_fixture();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_location_update BEFORE UPDATE ON asset_locations
             BEGIN SELECT RAISE(ABORT, 'controlled repair failure'); END;",
        )
        .unwrap();
    repair(&mut connection).expect_err("repair must retain its atomic boundary");
    assert!(!marker_is_complete(&connection).unwrap());
    for table in [
        "library_change_catch_up_handoffs",
        "library_change_scan_handoff_items",
    ] {
        let ready: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE preview_status = 'ready'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ready, 3);
    }
    let links: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(links, 3);
    connection
        .execute_batch("DROP TRIGGER reject_location_update;")
        .unwrap();
    repair(&mut connection).unwrap();
    assert!(marker_is_complete(&connection).unwrap());
}

#[test]
fn current_v19_malformed_preview_repair_marker_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v19 catalog");
    connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             CREATE TABLE library_change_preview_repair_contract(
               singleton INTEGER PRIMARY KEY
             );
             INSERT INTO library_change_preview_repair_contract(singleton) VALUES (1);",
        )
        .expect("malformed preview repair marker fixture");

    let error = migrate_schema(&mut connection).expect_err("malformed preview repair marker");

    assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
}

#[test]
fn concurrent_prerelease_v19_preview_repair_accepts_completed_marker() {
    let catalog = NamedTempFile::new().expect("temporary catalog");
    let mut first = Connection::open(catalog.path()).expect("first catalog connection");
    migrate_schema(&mut first).expect("fresh v19 catalog");
    first
        .execute_batch("DROP TABLE library_change_preview_repair_contract")
        .expect("restore prerelease preview repair marker");
    let mut second = Connection::open(catalog.path()).expect("second catalog connection");

    assert!(!marker_is_complete(&first).expect("first marker preflight"));
    assert!(!marker_is_complete(&second).expect("second marker preflight"));

    repair_missing_v19_preview_expectation_marker(&mut first)
        .expect("first connection preview repair");
    repair_missing_v19_preview_expectation_marker(&mut second)
        .expect("second connection accepts completed repair");

    assert!(marker_is_complete(&first).expect("first repaired marker"));
    assert!(marker_is_complete(&second).expect("second repaired marker"));
}

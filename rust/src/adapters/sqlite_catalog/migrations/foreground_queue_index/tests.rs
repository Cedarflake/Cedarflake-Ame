use std::collections::BTreeMap;

use rusqlite::types::Value;
use tempfile::tempdir;

use crate::adapters::SqliteCatalog;
use crate::domain::{LibraryChangeQueuePolicy, LibraryRootGeneration, ScanCheckpoint, ScanRequest};
use crate::ports::CatalogRepository;

use super::*;

#[test]
fn v33_migration_preserves_all_retained_rows_and_reopens() {
    for has_matching_index in [false, true] {
        let directory = tempdir().expect("derived fixture");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(path.clone()).expect("fresh catalog");
        let source = directory
            .path()
            .join("source")
            .to_string_lossy()
            .into_owned();
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: "retained".to_owned(),
                    root_path: source.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 128,
                },
                "root",
                &source,
            )
            .expect("retain scan authority");
        catalog
            .checkpoint_scan(
                "retained",
                &ScanCheckpoint {
                    visited_entries: 12,
                    ..ScanCheckpoint::default()
                },
            )
            .expect("retain checkpoint");
        catalog
            .connection
            .execute_batch(
                "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path, origin,
               first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES ('root', 1, 'reconcile', 'path', 'pending.jpg', 'live_notification',
               1, 1, '1', '1', 1, 'pending', 500, 0, 0, 1, 1);",
            )
            .expect("retain pending event with lane ownership");
        downgrade_to_v33_for_test(&catalog.connection);
        if has_matching_index {
            catalog
                .connection
                .execute_batch(INDEX_DDL)
                .expect("existing canonical access path");
        }
        let before = retained_rows(&catalog.connection);
        drop(catalog);
        for _ in 0..2 {
            let catalog = SqliteCatalog::open(path.clone()).expect("migrate and reopen");
            assert_eq!(
                versions(&catalog.connection),
                (super::super::SCHEMA_VERSION, super::super::SCHEMA_VERSION)
            );
            assert!(
                schema_object_sql_matches(&catalog.connection, "index", INDEX_NAME, INDEX_DDL)
                    .expect("new index definition")
            );
            let mut after = retained_rows(&catalog.connection);
            assert_eq!(
                after.remove("library_journal_scope_recoveries"),
                Some(Vec::new()),
            );
            assert_eq!(
                after.remove("library_media_admission_rechecks"),
                Some(Vec::new()),
            );
            for table in [
                "content_fingerprints",
                "exact_analysis_outcomes",
                "exact_analysis_runs",
                "exact_duplicate_decisions",
                "gallery_content_fingerprints",
                "gallery_retained_fingerprints",
                "library_metadata_fallbacks",
                "organization_albums",
                "organization_bindings",
                "organization_conflicts",
                "organization_operation_changes",
                "organization_operations",
                "organization_records",
                "organization_unassociated_records",
                "organization_sources",
                "organization_record_groups",
                "organization_album_retirements",
                "organization_album_operation_changes",
                "organization_operation_conflicts",
                "organization_operation_targets",
                "organization_admission_roots",
                "organization_admission_sources",
                "organization_admissions",
                "organization_review_bookmark",
                "organization_review_operations",
                "organization_review_scopes",
                "organization_review_session",
                "organization_archive_restores",
                "organization_archive_generations",
                "organization_recovery_roots",
                "organization_recovery_versions",
                "organization_recovery_admissions",
                "organization_recovery_admission_sources",
                "organization_recovery_admission_links",
                "organization_recovery_duplicate_decisions",
                "organization_recovery_duplicate_sources",
                "organization_restored_fields",
            ] {
                assert_eq!(after.remove(table), Some(Vec::new()), "{table}");
            }
            assert_eq!(
                after.remove("organization_state"),
                Some(vec![vec![Value::Integer(1), Value::Integer(0)]]),
            );
            let sequences = after
                .get_mut("sqlite_sequence")
                .expect("retained sequences");
            let admission_sequence = sequences
                .iter()
                .position(|row| row.first() == Some(&Value::Text("organization_admissions".into())))
                .expect("new admission sequence");
            assert_eq!(
                sequences.remove(admission_sequence),
                vec![
                    Value::Text("organization_admissions".into()),
                    Value::Integer(0)
                ],
            );
            assert_eq!(after, before);
        }
    }
}

#[test]
fn conflicting_index_rolls_back_without_changing_retained_data() {
    let directory = tempdir().expect("derived fixture");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
    let mut connection = Connection::open(path).expect("fixture connection");
    downgrade_to_v33_for_test(&connection);
    connection
        .execute_batch(
            "CREATE INDEX library_change_queue_active_origin ON library_change_queue(status)",
        )
        .expect("conflicting existing object");
    let before = retained_rows(&connection);
    let error = migrate(&mut connection).expect_err("conflict must abort");
    assert_eq!(error.code, "catalog_queue_index_conflict");
    assert_eq!(versions(&connection), (33, 33));
    assert_eq!(retained_rows(&connection), before);
    let columns: String = connection
        .query_row(
            "SELECT group_concat(name, ',') FROM pragma_index_info(?1)",
            [INDEX_NAME],
            |row| row.get(0),
        )
        .expect("original index survives");
    assert_eq!(columns, "status");
}

#[test]
fn corrupt_authority_is_not_admitted_by_index_migration() {
    let directory = tempdir().expect("derived fixture");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
    let mut connection = Connection::open(path).expect("fixture connection");
    downgrade_to_v33_for_test(&connection);
    connection
        .execute_batch("DROP TRIGGER scan_resume_publication_guard")
        .expect("damaged publication authority");
    let error = migrate(&mut connection).expect_err("authority must remain fail-closed");
    assert_eq!(error.code, "catalog_scan_resume_inventory_unverifiable");
    assert_eq!(versions(&connection), (33, 33));
    assert!(
        !schema_object_sql_matches(&connection, "index", INDEX_NAME, INDEX_DDL)
            .expect("no partial index publication")
    );
}

#[test]
fn optional_foreground_index_does_not_become_catalog_authority() {
    for has_wrong_definition in [false, true] {
        let directory = tempdir().expect("derived fixture");
        let path = directory.path().join("catalog.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
        let connection = Connection::open(&path).expect("fixture connection");
        connection
            .execute_batch("DROP INDEX library_change_queue_active_origin")
            .expect("remove optional access path");
        if has_wrong_definition {
            connection.execute_batch(
                "CREATE INDEX library_change_queue_active_origin ON library_change_queue(status)"
            ).expect("different optional index");
        }
        drop(connection);
        let catalog = SqliteCatalog::open(path).expect("authority remains independently verified");
        assert!(
            !catalog
                .has_ready_live_path_library_change(
                    "missing",
                    LibraryRootGeneration::initial(),
                    1_000,
                    LibraryChangeQueuePolicy::default()
                )
                .expect("readiness falls back without a hard index dependency")
        );
    }
}

fn versions(connection: &Connection) -> (i64, i64) {
    connection.query_row(
        "SELECT (SELECT version FROM schema_info), (SELECT user_version FROM pragma_user_version)",
        [], |row| Ok((row.get(0)?, row.get(1)?))
    ).expect("schema versions")
}

fn retained_rows(connection: &Connection) -> BTreeMap<String, Vec<Vec<Value>>> {
    let mut tables = connection.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name <> 'schema_info' ORDER BY name"
    ).expect("retained tables");
    let names = tables
        .query_map([], |row| row.get::<_, String>(0))
        .expect("table names")
        .collect::<Result<Vec<_>, _>>()
        .expect("all table names");
    names
        .into_iter()
        .map(|name| {
            let mut statement = connection
                .prepare(&format!("SELECT * FROM \"{name}\""))
                .expect("retained rows");
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|column| row.get(column))
                        .collect::<Result<Vec<Value>, _>>()
                })
                .expect("row values")
                .collect::<Result<Vec<_>, _>>()
                .expect("all row values");
            (name, rows)
        })
        .collect()
}

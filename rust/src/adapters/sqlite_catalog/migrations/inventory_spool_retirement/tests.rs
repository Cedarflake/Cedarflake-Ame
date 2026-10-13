use rusqlite::{Connection, params};

use super::super::{current_schema, migrate_schema};

mod identity;

const REVISION: &str = "windows-file-change-time-100ns-v1:0000000000000042";

fn v31_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().expect("isolated catalog");
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys enabled");
    migrate_schema(&mut connection).expect("production schema");
    super::downgrade_to_v31_for_test(&connection);
    for (id, state) in [(1, "completed"), (2, "enumerating")] {
        let root = format!("root-{id}");
        connection
            .execute(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
            VALUES (?1, ?2, ?1, 1)",
                params![root, format!("C:/generated-inventory-migration/{id}")],
            )
            .expect("root metadata only");
        connection.execute("INSERT INTO scan_runs(id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge)
            VALUES (?1, ?1, 'completed', 1, 2, 128)", [&root]).expect("baseline");
        connection.execute("INSERT INTO library_change_root_state(root_id, generation, is_active, updated_unix_ms)
            VALUES (?1, 1, 1, 2)", [&root]).expect("root generation");
        connection.execute("INSERT INTO library_persistent_journal_root_state(
            root_id, root_generation, protocol_version, contract_version, capability_state, continuity_state, updated_unix_ms)
            VALUES (?1, 1, 0, 1, 'unknown', 'baseline_required', 2)", [&root]).expect("journal state");
        connection.execute("INSERT INTO library_metadata_inventory_runs(
            id, root_id, root_generation, epoch, scope_kind, scope_relative_path, status, next_page_index,
            staged_entry_count, enumeration_complete, absence_authority, started_unix_ms, updated_unix_ms)
            VALUES (?1, ?1, 1, 1, 'subtree', 'album', 'running', 1, 0, 0, 0, 3, 3)", [&root]).expect("active run");
        connection.execute("INSERT INTO library_change_queue(
            id, root_id, root_generation, intent_kind, scope, relative_path, origin,
            first_observed_unix_ms, most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
            coalesced_observation_count, status, ready_unix_ms, catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms)
            VALUES (?1, ?2, 1, 'reconcile', 'subtree', 'album', 'consistency_audit', 3, 3, '1', '1', 1, 'pending', 3, 0, 3, 3)", params![id, root]).expect("queue authority");
        connection.execute("INSERT INTO library_recovery_authorities(change_id, run_id, root_id, root_generation, reason, authorized_unix_ms)
            VALUES (?1, ?2, ?2, 1, 'containment_failure', 3)", params![id, root]).expect("recovery authority");
        connection.execute("INSERT INTO library_metadata_inventory_spools(
            run_id, authority_change_id, root_id, root_generation, root_identity_scheme, root_identity_value,
            scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms)
            VALUES (?1, ?2, ?1, 1, 'windows-file-id-128-v1', '000000000000004d:ffffffffffffffffffffffffffffffff',
            'subtree', 'album', ?3, 3, 3)", params![root, id, if state == "completed" { "ready" } else { "enumerating" }]).expect("raw header");
        connection.execute("INSERT INTO library_metadata_inventory_spool_directories(
            run_id, ordinal, relative_directory, state, directory_identity_scheme, directory_identity_value,
            source_entry_count, created_unix_ms, updated_unix_ms)
            VALUES (?1, 0, 'album', ?2, 'fixture', 'directory', 3, 3, 3)", params![root, state]).expect("directory checkpoint");
        for index in 0..3 {
            connection.execute("INSERT INTO library_metadata_inventory_spool_entries(
                run_id, directory_relative_path, relative_path, entry_kind, file_size, modified_unix_ms,
                placeholder_state, is_reparse_point, staged_unix_ms, source_revision_token)
                VALUES (?1, 'album', ?2, 'file', 1, 3, 'available', 0, 3, ?3)",
                params![root, format!("album/file-{index}.png"), REVISION]).expect("raw revision");
        }
        connection.execute("INSERT INTO library_metadata_inventory_spool_entries(
            run_id, relative_path, entry_kind, modified_unix_ms, placeholder_state, is_reparse_point, staged_unix_ms)
            VALUES (?1, 'album', 'directory', 3, 'available', 0, 3)", [&root]).expect("nullable initial observation");
    }
    current_schema::validate_schema_version(&connection, 31)
        .expect("exact valid historical schema and rows");
    connection
}

fn raw_evidence(connection: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
    let mut evidence = Vec::new();
    for table in [
        "library_metadata_inventory_spools",
        "library_metadata_inventory_spool_directories",
        "library_metadata_inventory_spool_entries",
    ] {
        let mut query = connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2, 3"))
            .expect("raw evidence query");
        let width = query.column_count();
        evidence.extend(
            query
                .query_map([], |row| {
                    (0..width)
                        .map(|column| row.get(column))
                        .collect::<Result<Vec<_>, _>>()
                })
                .expect("raw evidence rows")
                .collect::<Result<Vec<_>, _>>()
                .expect("raw values"),
        );
    }
    evidence
}

#[test]
fn v32_migration_preserves_ready_incomplete_and_initial_observations_exactly() {
    let mut connection = v31_catalog();
    let before = raw_evidence(&connection);
    migrate_schema(&mut connection).expect("migrate active inventory");
    assert_eq!(raw_evidence(&connection), before);
    assert!(super::schema_matches(&connection).expect("exact v32 shape"));
    connection
        .execute_batch("PRAGMA query_only = ON")
        .expect("read-only reopen proof");
    migrate_schema(&mut connection).expect("idempotent current validation without writes");
}

#[test]
fn v32_migration_preserves_historical_orphans_for_bounded_cleanup() {
    let mut connection = v31_catalog();
    connection.execute_batch("INSERT INTO library_metadata_inventory_spool_entries(
        run_id, relative_path, entry_kind, modified_unix_ms, placeholder_state, is_reparse_point, staged_unix_ms)
        VALUES ('historical-orphan', 'album', 'directory', 3, 'available', 0, 3)").expect("old nullable-parent orphan");
    let before = raw_evidence(&connection);
    migrate_schema(&mut connection)
        .expect("preserve unowned raw evidence without inventing authority");
    assert_eq!(raw_evidence(&connection), before);
    let authority: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_recovery_authorities WHERE run_id = 'historical-orphan'",
            [],
            |row| row.get(0),
        )
        .expect("no authority");
    assert_eq!(authority, 0);
}

#[test]
fn v32_migration_rolls_back_copied_storage_on_final_version_failure() {
    let mut connection = v31_catalog();
    let before = raw_evidence(&connection);
    connection
        .execute_batch(
            "CREATE TRIGGER fixture_reject_v32 BEFORE UPDATE ON schema_info
        WHEN NEW.version = 32 BEGIN SELECT RAISE(ABORT, 'fixture_reject_v32'); END",
        )
        .expect("failure after graph replacement");
    let error = migrate_schema(&mut connection).expect_err("atomic migration failure");
    assert!(error.message.contains("fixture_reject_v32"), "{error:?}");
    assert_eq!(raw_evidence(&connection), before);
    current_schema::validate_schema_version(&connection, 31)
        .expect("original schema remains valid");
    let versions: (i64, i64) = connection.query_row("SELECT (SELECT version FROM schema_info), (SELECT user_version FROM pragma_user_version)", [], |row| Ok((row.get(0)?, row.get(1)?))).expect("version rollback");
    assert_eq!(versions, (31, 31));
    connection
        .execute_batch("DROP TRIGGER fixture_reject_v32")
        .expect("remove owned failure");
    migrate_schema(&mut connection).expect("retry migration");
    assert_eq!(raw_evidence(&connection), before);
}

#[test]
fn v32_migration_rejects_damaged_contract_without_replacing_storage() {
    let mut connection = v31_catalog();
    let before = raw_evidence(&connection);
    connection
        .execute_batch("DROP TRIGGER library_metadata_inventory_spool_binding_update_guard")
        .expect("damage exact schema contract");
    let error = migrate_schema(&mut connection).expect_err("reject malformed DDL");
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable"
    );
    assert_eq!(raw_evidence(&connection), before);
    let version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("unchanged version");
    assert_eq!(version, 31);
}

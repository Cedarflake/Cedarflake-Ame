use rusqlite::Connection;
use tempfile::NamedTempFile;

use super::migrate_schema;

pub(super) fn recovery_lifecycle_catalog(is_completed: bool) -> NamedTempFile {
    let catalog = NamedTempFile::new().expect("temporary recovery lifecycle catalog");
    let mut connection = Connection::open(catalog.path()).expect("recovery lifecycle catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('lifecycle-root', 'C:/lifecycle-source', 1);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('lifecycle-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'lifecycle-root', 1, 5, 1, 'supported', 'baseline_required', 1
             );
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               901, 'lifecycle-root', 1, 'freshness_unknown', 'root', '',
               'consistency_audit', 1, 1, '1', '1', 1,
               'pending', 1, 0, 1, 1
             );
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason,
               opening_journal_id, opening_next_usn, authorized_unix_ms
             ) VALUES (
               901, 'lifecycle-run', 'lifecycle-root', 1,
               'existing_root_baseline', '44', '10', 1
             );
             INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms
             ) VALUES (
               901, 'lifecycle-root', 1, 'lifecycle-volume', '77',
               3, X'01010101010101010101010101010101', '44',
               '10', 5, 1, 'inventory', 1, 1
             );",
        )
        .expect("valid active recovery lifecycle");
    migrate_schema(&mut connection).expect("validate active recovery lifecycle");
    if is_completed {
        connection
            .execute_batch(
                "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   updated_unix_ms
                 ) VALUES (
                   'lifecycle-root', 1, 'lifecycle-volume', '77',
                   3, X'01010101010101010101010101010101', '44',
                   '20', '20', 0, 5, 1, 'current', 3
                 );
                 UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = 3
                 WHERE root_id = 'lifecycle-root';
                 UPDATE library_persistent_journal_baselines
                 SET closing_next_usn = '20', phase = 'completed',
                     updated_unix_ms = 3, completed_unix_ms = 3
                 WHERE change_id = 901;
                 UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0,
                     updated_unix_ms = 3
                 WHERE id = 901;
                 UPDATE library_recovery_authorities
                 SET retired_unix_ms = 3 WHERE change_id = 901;",
            )
            .expect("valid completed recovery lifecycle");
        migrate_schema(&mut connection).expect("validate completed recovery lifecycle");
    }
    drop(connection);
    catalog
}

pub(super) fn active_frontier_catalog() -> NamedTempFile {
    let catalog = recovery_lifecycle_catalog(false);
    let mut connection = Connection::open(catalog.path()).expect("active frontier catalog");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('lifecycle-scan', 'lifecycle-root', 'completed', 1, 1, 128);
             UPDATE library_roots
             SET active_scan_id = 'lifecycle-scan' WHERE id = 'lifecycle-root';
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, enumeration_cursor, staged_entry_count,
               started_unix_ms, updated_unix_ms
             ) VALUES (
               'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
               'running', 2, 'album', 1, 1, 2
             );
             INSERT INTO library_metadata_inventory_entries(
               run_id, relative_path, entry_kind, file_size, modified_unix_ms,
               placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
             ) VALUES (
               'lifecycle-run', 'album', 'directory', NULL, 1,
               'available', 0, 1, 2
             );
             INSERT INTO library_metadata_inventory_frontier(
               run_id, ordinal, relative_directory, state,
               directory_identity_scheme, directory_identity_value,
               resume_after_relative_path, enumerated_entry_count, updated_unix_ms
             ) VALUES
               ('lifecycle-run', 0, '', 'enumerating',
                'windows-file-id-128-v1', 'volume:root', 'album', 1, 2),
               ('lifecycle-run', 1, 'album', 'pending',
                'windows-file-id-128-v1', 'volume:album', NULL, 0, 2);",
        )
        .expect("valid active frontier lifecycle");
    migrate_schema(&mut connection).expect("validate active frontier lifecycle");
    drop(connection);
    catalog
}

pub(super) fn reopen_recovery_lifecycle_catalog(
    catalog: &NamedTempFile,
) -> Result<(), crate::domain::ScanError> {
    let mut connection = Connection::open(catalog.path()).expect("reopen lifecycle catalog");
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("enable lifecycle foreign keys");
    migrate_schema(&mut connection)
}

pub(super) fn v27_root_proof_phase_catalog(phase: &str) -> NamedTempFile {
    let catalog = recovery_lifecycle_catalog(phase == "completed");
    let connection = Connection::open(catalog.path()).expect("v27 root-proof phase catalog");
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("enable v27 fixture foreign keys");
    connection
        .execute_batch(
            "INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('lifecycle-scan', 'lifecycle-root', 'completed', 1, 1, 128);
             UPDATE library_roots
             SET active_scan_id = 'lifecycle-scan' WHERE id = 'lifecycle-root';",
        )
        .expect("last trustworthy catalog fixture");
    match phase {
        "opening" => {
            connection
                .execute(
                    "DELETE FROM library_persistent_journal_baselines
                     WHERE change_id = 901",
                    [],
                )
                .expect("opening phase has not captured a baseline");
        }
        "inventory" => {
            connection
                .execute_batch(
                    "INSERT INTO library_metadata_inventory_runs(
                       id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                       status, next_page_index, started_unix_ms, updated_unix_ms
                     ) VALUES (
                       'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                       'running', 1, 1, 1
                     );",
                )
                .expect("v27 inventory run");
        }
        "replay" | "absence" => {
            let (next_unread, absence_authority) = if phase == "absence" {
                ("20", 1)
            } else {
                ("15", 0)
            };
            connection
                .execute(
                    "UPDATE library_persistent_journal_root_state
                     SET continuity_state = 'catching_up'
                     WHERE root_id = 'lifecycle-root'",
                    [],
                )
                .expect("v27 catch-up root state");
            connection
                .execute(
                    "UPDATE library_persistent_journal_baselines
                     SET phase = ?1, closing_next_usn = '20', updated_unix_ms = 2
                     WHERE change_id = 901",
                    [phase],
                )
                .expect("v27 replay or absence baseline");
            connection
                .execute(
                    "INSERT INTO library_persistent_journal_checkpoints(
                       root_id, root_generation, volume_guid, volume_serial,
                       root_reference_version, root_file_reference, journal_id,
                       next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                       protocol_version, contract_version, continuity_state,
                       updated_unix_ms
                     ) VALUES (
                       'lifecycle-root', 1, 'lifecycle-volume', '77',
                       3, X'01010101010101010101010101010101', '44',
                       ?1, '20', 0, 5, 1, 'catching_up', 2
                     )",
                    [next_unread],
                )
                .expect("v27 catch-up checkpoint");
            connection
                .execute(
                    "INSERT INTO library_metadata_inventory_runs(
                       id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                       status, next_page_index, comparison_cursor, staged_entry_count,
                       candidate_count, enumeration_complete, absence_authority,
                       started_unix_ms, updated_unix_ms
                     ) VALUES (
                       'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                       'comparing', 2, 'photo.jpg', 1, 1, 1, ?1, 1, 2
                     )",
                    [absence_authority],
                )
                .expect("v27 comparing run");
            connection
                .execute_batch(
                    "INSERT INTO library_metadata_inventory_entries(
                       run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                       placeholder_state, is_reparse_point, staged_page_index,
                       comparison_status, staged_unix_ms
                     ) VALUES (
                       'lifecycle-run', 'photo.jpg', 'file', 1, 1,
                       'available', 0, 1, 'enqueued', 1
                     );
                     INSERT INTO library_metadata_inventory_frontier(
                       run_id, ordinal, relative_directory, state,
                       directory_identity_scheme, directory_identity_value,
                       enumerated_entry_count, updated_unix_ms
                     ) VALUES (
                       'lifecycle-run', 0, '', 'completed',
                       'windows-file-id-128-v1', '77:root', 1, 2
                     );
                     INSERT INTO library_change_queue(
                       id, root_id, root_generation, intent_kind, scope, relative_path,
                       origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                       first_sequence, most_recent_sequence, coalesced_observation_count,
                       status, ready_unix_ms, catalog_revision_at_enqueue,
                       created_unix_ms, updated_unix_ms
                     ) VALUES (
                       902, 'lifecycle-root', 1, 'reconcile', 'path', 'photo.jpg',
                       'consistency_audit', 2, 2, '2', '2', 1,
                       'pending', 2, 0, 2, 2
                     );
                     INSERT INTO library_metadata_inventory_candidate_owners(
                       run_id, candidate_key, change_id, candidate_role,
                       relative_path, owned_unix_ms
                     ) VALUES (
                       'lifecycle-run', 'photo.jpg', 902, 'present', 'photo.jpg', 2
                     );",
                )
                .expect("v27 derived candidate work");
        }
        "completed" => {}
        other => panic!("unsupported v27 phase fixture: {other}"),
    }
    replace_current_spool_contract_with_v27_for_test(&connection);
    if phase == "inventory" {
        connection
            .execute_batch(
                "INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   scope_kind, scope_relative_path, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'lifecycle-run', 901, 'lifecycle-root', 1,
                   'root', '', 'enumerating', 1, 1
                 );
                 INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES ('lifecycle-run', 0, '', 'pending', 1, 1);",
            )
            .expect("v27 active source spool");
    }
    super::recovery_authority_contract::validate(&connection)
        .expect("valid v27 recovery authority");
    super::validate_persistent_journal_baseline_contract(&connection)
        .expect("valid v27 baseline phase");
    super::recovery_execution_contract::validate(&connection)
        .expect("valid v27 recovery execution");
    super::inventory_spool_contract::validate_version(&connection, 27, 1)
        .expect("valid v27 source spool");
    drop(connection);
    catalog
}

fn replace_current_spool_contract_with_v27_for_test(connection: &Connection) {
    remove_metadata_inventory_spool_v27_contract_for_test(connection);
    for sql in [
        super::METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
        super::METADATA_INVENTORY_SPOOL_TABLE_V27_DDL,
        super::METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
        super::METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
        super::METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL,
        super::METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
        super::METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL,
        super::METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    ] {
        connection
            .execute_batch(sql)
            .expect("create exact v27 spool shape");
    }
    connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spool_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1);
             PRAGMA user_version = 27;
             UPDATE schema_info SET version = 27;",
        )
        .expect("publish v27 spool fixture");
    super::inventory_spool_contract::validate_version(connection, 27, 1)
        .expect("validate exact v27 spool fixture");
}

pub(super) fn remove_metadata_inventory_spool_v27_contract_for_test(connection: &Connection) {
    remove_root_publication_namespace_v29_contract_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS library_metadata_inventory_spool_directory_complete_guard;
             DROP TRIGGER IF EXISTS library_metadata_inventory_spool_binding_update_guard;
             DROP INDEX IF EXISTS library_metadata_inventory_spool_entries_order;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_entries;
             DROP INDEX IF EXISTS library_metadata_inventory_spool_directories_state;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_directories;
             DROP TABLE IF EXISTS library_metadata_inventory_spools;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_contract;",
        )
        .expect("remove v27 inventory spool fixture");
}

pub(super) fn remove_root_publication_namespace_v29_contract_for_test(connection: &Connection) {
    super::downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
             DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
             DROP TABLE IF EXISTS library_live_gap_recovery_claims;
             DROP TABLE IF EXISTS library_live_gap_recovery_contract;
             DROP INDEX IF EXISTS library_scan_publication_namespace_root;
             DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
             DROP TABLE IF EXISTS library_root_publication_namespaces;
             DROP TABLE IF EXISTS library_root_publication_namespace_contract;",
        )
        .expect("remove v29 configured-root publication fixture");
}

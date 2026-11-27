use rusqlite::Connection;
use tempfile::NamedTempFile;

use super::initialize_schema;

pub(super) fn recovery_lifecycle_catalog(is_completed: bool) -> NamedTempFile {
    let catalog = NamedTempFile::new().expect("temporary recovery lifecycle catalog");
    let mut connection = Connection::open(catalog.path()).expect("recovery lifecycle catalog");
    initialize_schema(&mut connection).expect("fresh current catalog");
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
    initialize_schema(&mut connection).expect("validate active recovery lifecycle");
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
        initialize_schema(&mut connection).expect("validate completed recovery lifecycle");
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
    initialize_schema(&mut connection).expect("validate active frontier lifecycle");
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
    initialize_schema(&mut connection)
}

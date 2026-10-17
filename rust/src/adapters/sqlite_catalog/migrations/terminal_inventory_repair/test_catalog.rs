use rusqlite::Connection;
use tempfile::NamedTempFile;

use super::super::migrate_schema;

pub(in super::super) fn current_terminal_inventory_authority_catalog() -> NamedTempFile {
    let catalog = NamedTempFile::new().expect("current terminal inventory catalog");
    let mut connection =
        Connection::open(catalog.path()).expect("open current terminal inventory catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
               VALUES ('root-a', 'C:/source', 'published-scan', 1);
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 128);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('root-a', 1, 1, 2);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'root-a', 1, 0, 1, 'unknown', 'baseline_required', 2
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, enumeration_complete,
               absence_authority, started_unix_ms, updated_unix_ms, completed_unix_ms,
               last_issue_code, last_issue_message
             ) VALUES (
               'completed-inventory', 'root-a', 1, 1, 'root', '', 'completed', 2,
               1, 1, 1, 1, 2, 2, 'completed_inventory_note',
               'Completed inventory evidence must remain intact'
             );
             INSERT INTO library_metadata_inventory_entries(
               run_id, relative_path, entry_kind, file_size, modified_unix_ms,
               placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
             ) VALUES (
               'completed-inventory', 'completed.jpg', 'file', 123, 1,
               'available', 0, 1, 1
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, enumeration_complete,
               absence_authority, started_unix_ms, updated_unix_ms,
               last_issue_code, last_issue_message
             ) VALUES (
               'terminal-inventory', 'root-a', 1, 2, 'root', '', 'superseded', 2,
               1, 1, 1, 3, 4, 'metadata_inventory_newer_epoch',
               'A newer metadata inventory superseded this run'
             );
             INSERT INTO library_metadata_inventory_entries(
               run_id, relative_path, entry_kind, file_size, modified_unix_ms,
               placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
             ) VALUES (
               'terminal-inventory', 'retained.jpg', 'file', 456, 3,
               'available', 0, 1, 3
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, enumeration_complete,
               absence_authority, started_unix_ms, updated_unix_ms
             ) VALUES (
               'active-inventory', 'root-a', 1, 3, 'root', '', 'running', 1,
               0, 0, 0, 5, 5
             );
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               301, 'root-a', 1, 'freshness_unknown', 'root', '',
               'consistency_audit', 5, 5, '3', '3', 1, 'pending', 5, 0, 5, 5
             );
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (
               301, 'active-inventory', 'root-a', 1, 'containment_failure', 5
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, enumeration_complete,
               absence_authority, started_unix_ms, updated_unix_ms,
               last_issue_code, last_issue_message
             ) VALUES (
               'terminal-spool-inventory', 'root-a', 1, 4, 'root', '', 'failed', 2,
               0, 1, 1, 6, 7, 'legacy_terminal_spool',
               'Legacy terminal inventory retained a derived source spool'
             );
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               302, 'root-a', 1, 'freshness_unknown', 'root', '',
               'consistency_audit', 6, 6, '4', '4', 1, 'pending', 6, 0, 6, 6
             );
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (
               302, 'terminal-spool-inventory', 'root-a', 1, 'containment_failure', 6
             );
             INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value,
               scope_kind, scope_relative_path, state,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               'terminal-spool-inventory', 302, 'root-a', 1,
               'windows-file-id-128-v1',
               '000000000000004d:ffffffffffffffffffffffffffffffff',
               'root', '', 'enumerating', 6, 6
             );
             INSERT INTO library_metadata_inventory_spool_directories(
               run_id, ordinal, relative_directory, state,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               'terminal-spool-inventory', 0, '', 'pending', 6, 6
             );",
        )
        .expect("polluted current terminal inventory fixture");
    drop(connection);
    catalog
}

pub(in super::super) fn assert_current_terminal_inventory_repaired(connection: &Connection) {
    let terminal: (String, i64, Option<String>, Option<String>, i64) = connection
        .query_row(
            "SELECT
               (SELECT status FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT last_issue_code FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT last_issue_message FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT COUNT(*) FROM library_metadata_inventory_entries
                WHERE run_id = 'terminal-inventory' AND relative_path = 'retained.jpg')",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("retained terminal inventory evidence");
    assert_eq!(
        terminal,
        (
            "superseded".to_owned(),
            0,
            Some("metadata_inventory_newer_epoch".to_owned()),
            Some("A newer metadata inventory superseded this run".to_owned()),
            1,
        )
    );

    let completed: (String, i64, Option<String>, i64) = connection
        .query_row(
            "SELECT
               (SELECT status FROM library_metadata_inventory_runs
                WHERE id = 'completed-inventory'),
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'completed-inventory'),
               (SELECT last_issue_code FROM library_metadata_inventory_runs
                WHERE id = 'completed-inventory'),
               (SELECT COUNT(*) FROM library_metadata_inventory_entries
                WHERE run_id = 'completed-inventory' AND relative_path = 'completed.jpg')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("retained completed inventory evidence");
    assert_eq!(
        completed,
        (
            "completed".to_owned(),
            1,
            Some("completed_inventory_note".to_owned()),
            1,
        )
    );

    let active: (String, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT status FROM library_metadata_inventory_runs
                WHERE id = 'active-inventory'),
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'active-inventory'),
               (SELECT COUNT(*) FROM library_recovery_authorities
                WHERE run_id = 'active-inventory' AND retired_unix_ms IS NULL)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained active inventory evidence");
    assert_eq!(active, ("running".to_owned(), 0, 1));
    let terminal_spool: (String, i64, Option<String>, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT status FROM library_metadata_inventory_runs
                WHERE id = 'terminal-spool-inventory'),
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'terminal-spool-inventory'),
               (SELECT last_issue_code FROM library_metadata_inventory_runs
                WHERE id = 'terminal-spool-inventory'),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools
                WHERE run_id = 'terminal-spool-inventory' AND state <> 'retired'),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories
                WHERE run_id = 'terminal-spool-inventory' AND EXISTS(
                  SELECT 1 FROM library_metadata_inventory_spools
                  WHERE run_id = 'terminal-spool-inventory' AND state <> 'retired')),
               (SELECT COUNT(*) FROM library_recovery_authorities
                WHERE run_id = 'terminal-spool-inventory' AND retired_unix_ms IS NULL)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("retained terminal spool evidence");
    assert_eq!(
        terminal_spool,
        (
            "failed".to_owned(),
            1,
            Some("legacy_terminal_spool".to_owned()),
            0,
            0,
            1,
        )
    );
}

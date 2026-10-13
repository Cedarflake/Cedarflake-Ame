use rusqlite::Connection;
use tempfile::NamedTempFile;

use crate::adapters::SqliteCatalog;
use crate::ports::LibraryChangeQueue;

use super::{begin_successor, open, recovery_lifecycle_catalog};

fn retained_reset_history() -> NamedTempFile {
    let fixture = recovery_lifecycle_catalog(true);
    let connection = Connection::open(fixture.path()).expect("adapt legal baseline fixture");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             BEGIN IMMEDIATE;
             CREATE TEMP TABLE original_baseline AS
             SELECT * FROM library_persistent_journal_baselines WHERE change_id = 901;
             DELETE FROM library_persistent_journal_baselines WHERE change_id = 901;
             DELETE FROM library_recovery_authorities WHERE change_id = 901;
             UPDATE library_change_queue SET origin = 'metadata_inventory' WHERE id = 901;
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (901, 'lifecycle-run', 'lifecycle-root', 1, 'containment_failure', 1);
             INSERT INTO library_persistent_journal_baselines SELECT * FROM original_baseline;
             UPDATE library_recovery_authorities SET retired_unix_ms = 3 WHERE change_id = 901;
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue, superseded_by_change_id,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               900, 'lifecycle-root', 1, 'freshness_unknown', 'root', '',
               'live_notification', 1, 1, '1', '1', 1, 'superseded', 1, 0, 901, 1, 3
             );
             INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               recovery_change_id, created_unix_ms, consumed_unix_ms
             ) VALUES (900, 'lifecycle-root', 1, 'metadata_inventory_control', 901, 1, 3);
             COMMIT;",
        )
        .expect("bind completed control to consumed live-gap claim under exact guards");
    drop(connection);

    let mut catalog = open(&fixture);
    begin_successor(&mut catalog, true);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("finish retained journal reset");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             BEGIN IMMEDIATE;
             UPDATE library_persistent_journal_baselines
             SET closing_next_usn = '6', phase = 'completed',
                 updated_unix_ms = 11, completed_unix_ms = 11 WHERE change_id = 902;
             UPDATE library_change_queue
             SET status = 'completed', catalog_revision_at_success = 0,
                 updated_unix_ms = 11 WHERE id = 902;
             UPDATE library_recovery_authorities
             SET retired_unix_ms = 11 WHERE change_id = 902;
             UPDATE library_persistent_journal_checkpoints
             SET journal_id = '45', next_unread_usn = '6', captured_exclusive_end = '6',
                 continuity_state = 'current', last_failure_code = NULL,
                 last_failure_message = NULL, updated_unix_ms = 11;
             UPDATE library_persistent_journal_root_state
             SET continuity_state = 'current', updated_unix_ms = 11;
             COMMIT;",
        )
        .expect("complete replacement checkpoint and retired reset authority atomically");
    drop(connection);
    drop(open(&fixture));
    fixture
}

fn add_other_root_guards(catalog: &SqliteCatalog) {
    catalog
        .connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('other-root', 'C:/unvisited-cleanup-control', 1);
             INSERT INTO library_change_root_state(root_id, generation, is_active, updated_unix_ms)
             VALUES ('other-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES ('other-root', 1, 5, 1, 'supported', 'baseline_required', 1);
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               catalog_revision_at_success, created_unix_ms, updated_unix_ms
             ) VALUES
               (990, 'other-root', 1, 'freshness_unknown', 'root', '',
                'consistency_audit', 1, 1, '1', '1', 1, 'pending', 1, 0, NULL, 1, 1),
               (899, 'other-root', 1, 'reconcile', 'path', 'old.jpg',
                'live_notification', 1, 1, '1', '1', 1, 'completed', 1, 0, 0, 1, 3),
               (991, 'other-root', 1, 'reconcile', 'path', 'recent.jpg',
                'live_notification', 1, 1, '2', '2', 1, 'completed', 1, 0, 0, 1, 1000);
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason,
               opening_journal_id, opening_next_usn, authorized_unix_ms
             ) VALUES (990, 'other-run', 'other-root', 1, 'existing_root_baseline', '44', '10', 1);
             INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms
             ) SELECT 990, 'other-root', 1, volume_guid, volume_serial,
                      root_reference_version, root_file_reference, '44',
                      '10', protocol_version, contract_version, 'inventory', 1, 1
               FROM library_persistent_journal_baselines WHERE change_id = 901;
             COMMIT;",
        )
        .expect("add isolated active authority, eligible ordinary row and recent terminal row");
}

fn assert_other_root_guards(catalog: &SqliteCatalog) {
    let state: (i64, i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_roots WHERE id = 'other-root'),
               (SELECT COUNT(*) FROM library_change_queue WHERE id = 990 AND status = 'pending'),
               (SELECT COUNT(*) FROM library_recovery_authorities
                WHERE change_id = 990 AND retired_unix_ms IS NULL),
               (SELECT COUNT(*) FROM library_persistent_journal_baselines
                WHERE change_id = 990 AND phase = 'inventory'),
               (SELECT COUNT(*) FROM library_change_queue WHERE id = 991 AND updated_unix_ms = 1000)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .expect("inspect other-root retained authorities");
    assert_eq!(state, (1, 1, 1, 1, 1));
}

fn assert_history(catalog: &SqliteCatalog, expected: &[i64]) {
    let mut statement = catalog
        .connection
        .prepare(
            "SELECT change_id FROM library_persistent_journal_baselines
             WHERE root_id = 'lifecycle-root'
             ORDER BY authorized_unix_ms, change_id",
        )
        .expect("read retained baseline history");
    let actual = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("query retained history")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("decode retained history");
    assert_eq!(actual, expected);
    let foreign_key_failures: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("verify foreign keys after queue pruning");
    assert_eq!(foreign_key_failures, 0);
}

#[test]
fn terminal_cleanup_preserves_claim_retained_reset_history_and_reopens_each_prefix_batch() {
    let fixture = retained_reset_history();
    let catalog = open(&fixture);
    add_other_root_guards(&catalog);
    drop(catalog);
    let mut catalog = open(&fixture);
    assert_history(&catalog, &[901, 902]);
    assert_other_root_guards(&catalog);

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(100, 16)
            .expect("first batch"),
        2
    );
    assert_history(&catalog, &[901, 902]);
    let released: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM library_change_queue WHERE id IN (899, 900)),
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims WHERE gap_change_id = 900)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("observe consumed gap retirement");
    assert_eq!(released, (0, 0));
    drop(catalog);
    let mut catalog = open(&fixture);
    assert_other_root_guards(&catalog);

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(100, 16)
            .expect("older prefix batch"),
        1
    );
    assert_history(&catalog, &[902]);
    drop(catalog);
    let mut catalog = open(&fixture);
    assert_other_root_guards(&catalog);

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(100, 16)
            .expect("reset prefix batch"),
        1
    );
    assert_history(&catalog, &[]);
    drop(catalog);
    let mut catalog = open(&fixture);
    assert_other_root_guards(&catalog);
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(100, 16)
            .expect("converged batch"),
        0
    );
    drop(catalog);
    drop(open(&fixture));
}

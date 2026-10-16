use crate::ports::LibraryChangeQueue;

use super::super::super::persistent_journal_range_history;
use super::interrupted_checkpoint::closed_interrupted_recovery;
use super::*;

fn complete_reset(fixture: &NamedTempFile) {
    let connection = Connection::open(fixture.path()).unwrap();
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             BEGIN IMMEDIATE;
             UPDATE library_persistent_journal_baselines
             SET phase = 'completed', completed_unix_ms = 15, updated_unix_ms = 15
             WHERE journal_id = '45';
             UPDATE library_recovery_authorities
             SET retired_unix_ms = 15 WHERE reason = 'journal_reset';
             UPDATE library_change_queue
             SET status = 'completed', catalog_revision_at_success = 0, updated_unix_ms = 15
             WHERE id IN (
               SELECT change_id FROM library_recovery_authorities WHERE reason = 'journal_reset'
             );
             UPDATE library_persistent_journal_checkpoints
             SET next_unread_usn = captured_exclusive_end, continuity_state = 'current',
                 updated_unix_ms = 15;
             UPDATE library_persistent_journal_root_state
             SET continuity_state = 'current', updated_unix_ms = 15;
             COMMIT;",
        )
        .expect("complete replacement journal while retaining old progress");
}

#[test]
fn reset_range_history_requires_a_closed_matching_whole_root_authority() {
    for mutation in [
        "DELETE FROM library_change_queue WHERE id IN (
           SELECT change_id FROM library_recovery_authorities WHERE reason = 'journal_reset'
         )",
        "DROP TRIGGER library_recovery_authority_update_guard;
         UPDATE library_recovery_authorities SET reason = 'containment_failure'
         WHERE reason = 'journal_reset'",
        "UPDATE library_persistent_journal_source_ranges SET checkpointed_unix_ms = 14",
        "UPDATE library_persistent_journal_baselines
         SET phase = 'inventory', closing_next_usn = NULL WHERE journal_id = '45'",
        "DROP TRIGGER library_persistent_journal_baseline_update_guard;
         UPDATE library_persistent_journal_baselines
         SET root_file_reference = X'02020202020202020202020202020202'
         WHERE journal_id = '45'",
        "DROP TRIGGER library_persistent_journal_baseline_update_guard;
         UPDATE library_persistent_journal_baselines
         SET volume_serial = '78' WHERE journal_id = '45'",
        "DROP TRIGGER library_persistent_journal_baseline_update_guard;
         UPDATE library_persistent_journal_baselines
         SET protocol_version = 6 WHERE journal_id = '45'",
        "UPDATE library_change_queue SET scope = 'subtree', relative_path = 'nested'
         WHERE id IN (
           SELECT change_id FROM library_recovery_authorities WHERE reason = 'journal_reset'
         )",
    ] {
        let (fixture, _) = closed_interrupted_recovery(true);
        drop(open(&fixture));
        let connection = Connection::open(fixture.path()).unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .unwrap();
        connection.execute_batch(mutation).expect(mutation);
        let error = persistent_journal_range_history::validate(&connection)
            .expect_err("reject old progress without matching reset proof");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        drop(connection);
        assert!(SqliteCatalog::open(fixture.path().to_path_buf()).is_err());
    }
}

#[test]
fn reset_range_history_keeps_same_journal_coverage_and_volume_checks() {
    for mutation in [
        "UPDATE library_persistent_journal_checkpoints
         SET journal_id = '44', next_unread_usn = '5'",
        "UPDATE library_persistent_journal_checkpoints SET volume_guid = 'unrelated-volume'",
        "UPDATE library_persistent_journal_checkpoints SET volume_serial = '78'",
    ] {
        let (fixture, _) = closed_interrupted_recovery(true);
        let connection = Connection::open(fixture.path()).unwrap();
        connection.execute_batch(mutation).expect(mutation);
        assert!(persistent_journal_range_history::validate(&connection).is_err());
    }
}

#[test]
fn reset_range_history_survives_cleanup_until_its_old_progress_is_retired() {
    let (fixture, _) = closed_interrupted_recovery(true);
    complete_reset(&fixture);
    let mut catalog = open(&fixture);
    assert_eq!(
        catalog.cleanup_terminal_library_changes(100, 16).unwrap(),
        1
    );
    drop(catalog);
    let mut catalog = open(&fixture);
    assert_eq!(
        catalog.cleanup_terminal_library_changes(100, 16).unwrap(),
        0
    );
    drop(catalog);

    let connection = Connection::open(fixture.path()).unwrap();
    let (ranges, resets): (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
                    (SELECT COUNT(*) FROM library_persistent_journal_baselines)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((ranges, resets), (1, 1));
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             DELETE FROM library_persistent_journal_source_ranges;",
        )
        .expect("retire the completed old range and its derived children");
    drop(connection);
    let mut catalog = open(&fixture);
    assert_eq!(
        catalog.cleanup_terminal_library_changes(100, 16).unwrap(),
        1
    );
    drop(catalog);
    drop(open(&fixture));
}

#[test]
fn reset_range_history_survives_another_journal_replacement() {
    let (fixture, _) = closed_interrupted_recovery(true);
    complete_reset(&fixture);
    let mut catalog = open(&fixture);
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    let journal_id = JournalIdentifier::new(46).unwrap();
    let recovery = catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: checkpoint.root_id,
                root_generation: checkpoint.root_generation,
                kind: PersistentJournalRootFailureKind::JournalReset,
                failure: PersistentJournalFailure {
                    code: "journal_reset".to_owned(),
                    message: "Journal replaced again".to_owned(),
                },
                opening_boundary: Some(LibraryRecoveryOpeningBoundary {
                    volume: checkpoint.volume.clone(),
                    root_file_reference: checkpoint.root_file_reference.clone(),
                    journal_id,
                    next_usn: JournalUsn::new(3).unwrap(),
                    protocol_version: checkpoint.protocol_version,
                    contract_version: checkpoint.contract_version,
                }),
            },
            20,
            LibraryChangeQueuePolicy::default(),
        )
        .unwrap()
        .unwrap();
    catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &PersistentJournalBaselineClosingBoundary {
                change_id: recovery,
                volume: checkpoint.volume,
                root_file_reference: checkpoint.root_file_reference,
                journal_id,
                closing_next_usn: JournalUsn::new(4).unwrap(),
                protocol_version: checkpoint.protocol_version,
                captured_unix_ms: 21,
            },
        )
        .unwrap();
    drop(catalog);
    drop(open(&fixture));
}

use rusqlite::Connection;
use tempfile::NamedTempFile;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    JournalIdentifier, JournalUsn, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRecoveryOpeningBoundary, LibraryRootGeneration,
    PersistentJournalBaselineClosingBoundary, PersistentJournalContinuityState,
    PersistentJournalEnrollmentBatch, PersistentJournalFailure, PersistentJournalRangeState,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind, PersistentJournalSourceRange,
    PersistentJournalVolumeBatch, PersistentJournalVolumePage, persistent_journal_batch_id,
};
use crate::ports::PersistentJournalRepository;

use super::super::tests::recovery_lifecycle_catalog;
use super::validate_persistent_journal_baseline_contract;

mod terminal_cleanup;

fn open(catalog: &NamedTempFile) -> SqliteCatalog {
    SqliteCatalog::open(catalog.path().to_path_buf()).expect("FULL-open baseline catalog")
}

fn reject(catalog: &NamedTempFile) {
    let connection = Connection::open(catalog.path()).expect("inspect baseline catalog");
    let error = validate_persistent_journal_baseline_contract(&connection)
        .expect_err("reject unproven completed history");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
    drop(connection);
    assert!(SqliteCatalog::open(catalog.path().to_path_buf()).is_err());
}

fn publish_followup(catalog: &mut SqliteCatalog, is_complete: bool, has_candidate: bool) {
    let mut checkpoint = catalog
        .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
        .expect("load completed checkpoint")
        .expect("completed checkpoint");
    let mut enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: String::new(),
            root_id: checkpoint.root_id.clone(),
            root_generation: checkpoint.root_generation,
            volume: checkpoint.volume.clone(),
            journal_id: checkpoint.journal_id,
            requested_start_usn: checkpoint.next_unread_usn,
            requested_end_usn: JournalUsn::new(if is_complete { 21 } else { 22 }).expect("end"),
            covered_until_usn: JournalUsn::new(21).expect("coverage"),
            is_complete,
            protocol_version: checkpoint.protocol_version,
            contract_version: checkpoint.contract_version,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 10,
            checkpointed_unix_ms: None,
        },
        intents: if has_candidate {
            vec![LibraryChangeIntent {
                root_id: checkpoint.root_id.clone(),
                root_generation: checkpoint.root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "changed.jpg".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 10,
                most_recent_observed_unix_ms: 10,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }]
        } else {
            Vec::new()
        },
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    enrollment.range.batch_id = persistent_journal_batch_id(&enrollment);
    checkpoint.next_unread_usn = enrollment.range.covered_until_usn;
    checkpoint.captured_exclusive_end = enrollment.range.requested_end_usn;
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    checkpoint.updated_unix_ms = 10;
    catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment,
                    checkpoint,
                }],
            },
            10,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("publish exact post-baseline range through production port");
}

fn begin_successor(catalog: &mut SqliteCatalog, resets_journal: bool) {
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
        .expect("load historical checkpoint")
        .expect("historical checkpoint");
    let boundary = LibraryRecoveryOpeningBoundary {
        volume: checkpoint.volume,
        root_file_reference: checkpoint.root_file_reference,
        journal_id: JournalIdentifier::new(if resets_journal { 45 } else { 44 })
            .expect("successor journal"),
        next_usn: JournalUsn::new(if resets_journal { 5 } else { 20 }).expect("opening"),
        protocol_version: checkpoint.protocol_version,
        contract_version: checkpoint.contract_version,
    };
    catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: checkpoint.root_id,
                root_generation: checkpoint.root_generation,
                kind: if resets_journal {
                    PersistentJournalRootFailureKind::JournalReset
                } else {
                    PersistentJournalRootFailureKind::ContainmentFailure
                },
                failure: PersistentJournalFailure {
                    code: "baseline_history_successor".to_owned(),
                    message: "Controlled subsequent continuity failure".to_owned(),
                },
                opening_boundary: Some(boundary),
            },
            10,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("persist genuine successor authority and window")
        .expect("successor control");
}

#[test]
fn completed_history_reopens_after_exact_current_checkpoint_advancement() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, true, false);
    drop(catalog);
    let reopened = open(&fixture);
    let checkpoint = reopened
        .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
        .expect("load advanced checkpoint")
        .expect("advanced checkpoint");
    assert_eq!(checkpoint.next_unread_usn.value(), 21);
    assert_eq!(
        checkpoint.continuity,
        PersistentJournalContinuityState::Current
    );
}

#[test]
fn completed_history_reopens_when_range_publication_precedes_the_final_receipt() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, true, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("complete the later recovery receipt");
    connection
        .execute_batch(
            "UPDATE library_persistent_journal_baselines
         SET updated_unix_ms = 11, completed_unix_ms = 11;
         UPDATE library_recovery_authorities SET retired_unix_ms = 11;
         UPDATE library_change_queue SET updated_unix_ms = 11 WHERE id = 901;",
        )
        .expect("P1 coverage may commit before the P2 final receipt");
    drop(connection);
    drop(open(&fixture));
}

#[test]
fn completed_history_reopens_during_partial_and_pending_incremental_ranges() {
    for (is_complete, has_candidate) in [(false, false), (true, true)] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        publish_followup(&mut catalog, is_complete, has_candidate);
        drop(catalog);
        let reopened = open(&fixture);
        let checkpoint = reopened
            .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
            .expect("load catching-up checkpoint")
            .expect("catching-up checkpoint");
        assert_eq!(checkpoint.next_unread_usn.value(), 21);
        assert_eq!(
            checkpoint.continuity,
            PersistentJournalContinuityState::CatchingUp
        );
    }
}

#[test]
fn completed_history_rejects_checkpoint_regression_identity_drift_and_unproven_advance() {
    for mutation in [
        "next_unread_usn = '19', captured_exclusive_end = '19'",
        "next_unread_usn = '21', captured_exclusive_end = '21'",
        "volume_guid = 'unrelated-volume'",
        "volume_serial = '78'",
        "root_file_reference = X'02020202020202020202020202020202'",
        "journal_id = '45'",
        "protocol_version = 6",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let connection = Connection::open(fixture.path()).expect("mutate checkpoint");
        connection
            .execute_batch(&format!(
                "UPDATE library_persistent_journal_checkpoints SET {mutation}"
            ))
            .expect("persist isolated checkpoint corruption");
        drop(connection);
        reject(&fixture);
    }
}

#[test]
fn completed_history_rejects_missing_or_unfinished_current_range_proof() {
    for remove_range in [false, true] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        publish_followup(&mut catalog, true, false);
        drop(catalog);
        let connection = Connection::open(fixture.path()).expect("mutate range proof");
        connection
            .execute_batch(if remove_range {
                "PRAGMA foreign_keys = ON;
                 DELETE FROM library_persistent_journal_source_ranges"
            } else {
                "UPDATE library_persistent_journal_range_lifecycle
                 SET lifecycle_state = 'pending', completed_unix_ms = NULL"
            })
            .expect("remove exact current coverage proof");
        drop(connection);
        reject(&fixture);
    }
}

#[test]
fn completed_history_rejects_catching_up_without_range_or_consistent_projection() {
    for mutation in [
        "UPDATE library_persistent_journal_root_state SET continuity_state = 'catching_up'",
        "UPDATE library_persistent_journal_checkpoints SET continuity_state = 'catching_up'",
        "UPDATE library_persistent_journal_root_state SET continuity_state = 'catching_up';
         UPDATE library_persistent_journal_checkpoints SET continuity_state = 'catching_up'",
        "DELETE FROM library_persistent_journal_checkpoints",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let connection = Connection::open(fixture.path()).expect("mutate projection");
        connection
            .execute_batch(mutation)
            .expect("persist missing proof");
        drop(connection);
        reject(&fixture);
    }
}

#[test]
fn completed_history_reopens_with_genuine_recovery_and_reset_successors() {
    for resets_journal in [false, true] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        begin_successor(&mut catalog, resets_journal);
        drop(catalog);
        drop(open(&fixture));
    }
}

#[test]
fn completed_history_reopens_after_successor_captures_reset_closing_boundary() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    begin_successor(&mut catalog, true);
    let baseline = catalog
        .load_persistent_journal_baselines()
        .expect("load reset window")
        .into_iter()
        .next()
        .expect("reset window");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &PersistentJournalBaselineClosingBoundary {
                change_id: baseline.change_id,
                volume: baseline.volume,
                root_file_reference: baseline.root_file_reference,
                journal_id: baseline.journal_id,
                closing_next_usn: JournalUsn::new(6).expect("closing"),
                protocol_version: baseline.protocol_version,
                captured_unix_ms: 11,
            },
        )
        .expect("capture authorized replacement journal");
    drop(catalog);
    drop(open(&fixture));
}

#[test]
fn completed_history_rejects_successor_authorized_before_prior_terminal_evidence() {
    for mutation in [
        "UPDATE library_recovery_authorities SET retired_unix_ms = 11 WHERE change_id = 901",
        "UPDATE library_change_queue SET updated_unix_ms = 11 WHERE id = 901",
        "UPDATE library_persistent_journal_baselines
         SET updated_unix_ms = 11, completed_unix_ms = 11 WHERE change_id = 901",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        begin_successor(&mut catalog, false);
        drop(catalog);
        let connection = Connection::open(fixture.path()).expect("mutate terminal chronology");
        connection
            .execute_batch(mutation)
            .expect("persist reversed chronology");
        drop(connection);
        reject(&fixture);
    }
}

#[test]
fn completed_history_rejects_forged_successor_namespace_and_journal() {
    for mutation in [
        "volume_guid = 'unrelated-volume'",
        "root_file_reference = X'02020202020202020202020202020202'",
        "journal_id = '45'",
        "opening_next_usn = '19'",
        "protocol_version = 6",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        begin_successor(&mut catalog, false);
        drop(catalog);
        let connection = Connection::open(fixture.path()).expect("mutate successor");
        connection
            .execute_batch(&format!(
                "DROP TRIGGER library_persistent_journal_baseline_update_guard;
                 UPDATE library_persistent_journal_baselines SET {mutation} WHERE change_id <> 901"
            ))
            .expect("persist forged successor identity");
        connection
            .execute_batch(super::PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL)
            .expect("restore exact schema guard");
        drop(connection);
        reject(&fixture);
    }
}

#[test]
fn completed_history_cannot_validate_an_unfinished_original_baseline() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, true, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("mutate original phase");
    connection
        .execute_batch(
            "UPDATE library_persistent_journal_baselines
             SET phase = 'inventory', closing_next_usn = NULL, completed_unix_ms = NULL",
        )
        .expect("restore an unfinished phase without its authority");
    drop(connection);
    reject(&fixture);
}

#[test]
fn completed_history_reopens_after_successor_becomes_completed_history() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    begin_successor(&mut catalog, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("complete successor fixture");
    connection
        .execute_batch(
            "UPDATE library_persistent_journal_baselines
             SET closing_next_usn = '20', phase = 'completed',
                 updated_unix_ms = 11, completed_unix_ms = 11 WHERE change_id <> 901;
             UPDATE library_change_queue
             SET status = 'completed', catalog_revision_at_success = 0,
                 updated_unix_ms = 11 WHERE id <> 901;
             UPDATE library_recovery_authorities
             SET retired_unix_ms = 11 WHERE change_id <> 901;
             UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'current', last_failure_code = NULL,
                 last_failure_message = NULL, updated_unix_ms = 11;
             UPDATE library_persistent_journal_root_state
             SET continuity_state = 'current', updated_unix_ms = 11;",
        )
        .expect("persist completed successor with exact terminal evidence");
    drop(connection);
    drop(open(&fixture));
}

#[test]
fn completed_history_rejects_nonterminal_control_even_with_valid_successor() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    begin_successor(&mut catalog, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("mutate retired control");
    connection
        .execute_batch(
            "UPDATE library_change_queue SET status = 'pending',
                 catalog_revision_at_success = NULL WHERE id = 901",
        )
        .expect("make terminal proof inconsistent");
    drop(connection);
    reject(&fixture);
}

#[test]
fn completed_history_rejects_successor_from_another_generation() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    begin_successor(&mut catalog, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("mutate successor generation");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP TRIGGER library_persistent_journal_baseline_update_guard;
             UPDATE library_persistent_journal_baselines SET root_generation = 2
             WHERE change_id <> 901",
        )
        .expect("persist isolated contradictory successor ownership");
    connection
        .execute_batch(super::PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL)
        .expect("restore exact schema guard");
    drop(connection);
    reject(&fixture);
}

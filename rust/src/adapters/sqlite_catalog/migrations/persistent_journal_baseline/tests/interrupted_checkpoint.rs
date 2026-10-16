use super::*;

fn checkpoint(catalog: &SqliteCatalog) -> crate::domain::PersistentJournalCheckpoint {
    catalog
        .load_persistent_journal_checkpoint("lifecycle-root", LibraryRootGeneration::initial())
        .expect("load retained checkpoint")
        .expect("retained checkpoint")
}

fn interrupt(catalog: &mut SqliteCatalog, kind: PersistentJournalRootFailureKind) {
    let current = checkpoint(catalog);
    let recovery = catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: current.root_id,
                root_generation: current.root_generation,
                kind,
                failure: PersistentJournalFailure {
                    code: "identity_journal_hardlinks_not_supported".to_owned(),
                    message: "Windows metadata operation failed: 50".to_owned(),
                },
                opening_boundary: None,
            },
            12,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("persist interruption through the production failure port");
    assert_eq!(recovery.is_some(), kind.recovery_reason().is_some());
}

#[test]
fn completed_history_reopens_after_interrupted_baseline_and_incremental_checkpoints() {
    for followup in [
        None,
        Some((true, false)),
        Some((false, false)),
        Some((true, true)),
    ] {
        for kind in [
            PersistentJournalRootFailureKind::NonRecoverable,
            PersistentJournalRootFailureKind::JournalReconstructionFailure,
        ] {
            let fixture = recovery_lifecycle_catalog(true);
            let mut catalog = open(&fixture);
            if let Some((is_complete, has_candidate)) = followup {
                publish_followup(&mut catalog, is_complete, has_candidate);
            }
            let mut expected = checkpoint(&catalog);
            interrupt(&mut catalog, kind);
            expected.continuity = PersistentJournalContinuityState::RecoveryRequired;
            expected.failure = Some(PersistentJournalFailure {
                code: "identity_journal_hardlinks_not_supported".to_owned(),
                message: "Windows metadata operation failed: 50".to_owned(),
            });
            expected.updated_unix_ms = 12;
            assert_eq!(checkpoint(&catalog), expected);
            drop(catalog);

            let reopened = open(&fixture);
            assert_eq!(checkpoint(&reopened), expected);
            drop(reopened);
            assert_eq!(checkpoint(&open(&fixture)), expected);
        }
    }
}

#[test]
fn interrupted_history_reopens_before_enrolled_coverage_advances_the_checkpoint() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, false, false);
    drop(catalog);
    let connection = Connection::open(fixture.path()).expect("retain split enrollment boundary");
    // Legacy enrollment can commit before its separate checkpoint publication.
    connection
        .execute_batch(
            "UPDATE library_persistent_journal_source_ranges
             SET status = 'enrolled', checkpointed_unix_ms = NULL;
             UPDATE library_persistent_journal_range_lifecycle
             SET lifecycle_state = 'pending', completed_unix_ms = NULL;
             UPDATE library_persistent_journal_checkpoints SET next_unread_usn = '20';",
        )
        .expect("persist enrolled range without advancing coverage");
    drop(connection);
    let mut catalog = open(&fixture);
    interrupt(
        &mut catalog,
        PersistentJournalRootFailureKind::NonRecoverable,
    );
    let expected = checkpoint(&catalog);
    drop(catalog);
    assert_eq!(checkpoint(&open(&fixture)), expected);
    assert_eq!(expected.next_unread_usn.value(), 20);
    assert_eq!(expected.captured_exclusive_end.value(), 22);
}

fn begin_recovery_after_reopen(
    fixture: &NamedTempFile,
    resets_journal: bool,
) -> crate::domain::LibraryChangeId {
    let mut reopened = open(fixture);
    let retained = checkpoint(&reopened);
    reopened
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: retained.root_id,
                root_generation: retained.root_generation,
                kind: if resets_journal {
                    PersistentJournalRootFailureKind::JournalReset
                } else {
                    PersistentJournalRootFailureKind::JournalReconstructionFailure
                },
                failure: retained.failure.expect("recorded interruption"),
                opening_boundary: Some(LibraryRecoveryOpeningBoundary {
                    volume: retained.volume,
                    root_file_reference: retained.root_file_reference,
                    journal_id: if resets_journal {
                        JournalIdentifier::new(45).expect("replacement journal")
                    } else {
                        retained.journal_id
                    },
                    next_usn: if resets_journal {
                        JournalUsn::new(5).expect("replacement cursor")
                    } else {
                        retained.captured_exclusive_end
                    },
                    protocol_version: retained.protocol_version,
                    contract_version: retained.contract_version,
                }),
            },
            13,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("admit recovery after reopening")
        .expect("new recovery authority")
}

#[test]
fn reopened_interruption_can_admit_recovery_and_reset_without_discarding_partial_progress() {
    for resets_journal in [false, true] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        publish_followup(&mut catalog, false, false);
        interrupt(
            &mut catalog,
            PersistentJournalRootFailureKind::NonRecoverable,
        );
        let retained = checkpoint(&catalog);
        drop(catalog);

        let recovery = begin_recovery_after_reopen(&fixture, resets_journal);
        let reopened = open(&fixture);
        let after = checkpoint(&reopened);
        assert_eq!(after.next_unread_usn, retained.next_unread_usn);
        assert_eq!(
            after.captured_exclusive_end,
            retained.captured_exclusive_end
        );
        let baselines = reopened
            .load_persistent_journal_baselines()
            .expect("load recovery window");
        assert!(
            baselines
                .iter()
                .any(|baseline| baseline.change_id == recovery)
        );
    }
}

#[test]
fn recovery_opening_rejects_checkpoint_coverage_beyond_its_authorized_boundary() {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, false, false);
    interrupt(
        &mut catalog,
        PersistentJournalRootFailureKind::NonRecoverable,
    );
    drop(catalog);
    begin_recovery_after_reopen(&fixture, false);
    let connection = Connection::open(fixture.path()).expect("mutate recovery checkpoint");
    connection
        .execute_batch(
            "UPDATE library_persistent_journal_checkpoints SET captured_exclusive_end = '23'",
        )
        .expect("persist coverage beyond the recovery opening");
    drop(connection);
    reject(&fixture);
}

pub(super) fn closed_interrupted_recovery(
    resets_journal: bool,
) -> (NamedTempFile, crate::domain::PersistentJournalCheckpoint) {
    let fixture = recovery_lifecycle_catalog(true);
    let mut catalog = open(&fixture);
    publish_followup(&mut catalog, false, false);
    interrupt(
        &mut catalog,
        PersistentJournalRootFailureKind::NonRecoverable,
    );
    drop(catalog);
    let recovery = begin_recovery_after_reopen(&fixture, resets_journal);
    let mut catalog = open(&fixture);
    let baseline = catalog
        .load_persistent_journal_baselines()
        .unwrap()
        .into_iter()
        .find(|baseline| baseline.change_id == recovery)
        .unwrap();
    let closing = JournalUsn::new(baseline.opening_next_usn.value() + 2).unwrap();
    catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &crate::domain::PersistentJournalBaselineClosingBoundary {
                change_id: recovery,
                volume: baseline.volume.clone(),
                root_file_reference: baseline.root_file_reference.clone(),
                journal_id: baseline.journal_id,
                closing_next_usn: closing,
                protocol_version: baseline.protocol_version,
                captured_unix_ms: 14,
            },
        )
        .expect("close a recovery after an interrupted partial batch");
    let after = checkpoint(&catalog);
    assert_eq!(after.next_unread_usn, baseline.opening_next_usn);
    assert_eq!(after.captured_exclusive_end, closing);
    assert_eq!(
        after.continuity,
        PersistentJournalContinuityState::CatchingUp
    );
    assert!(after.failure.is_none());
    drop(catalog);
    (fixture, after)
}

#[test]
fn interrupted_recovery_closes_from_the_new_opening_and_reopens_for_replay() {
    for resets_journal in [false, true] {
        let (fixture, after) = closed_interrupted_recovery(resets_journal);
        assert_eq!(checkpoint(&open(&fixture)), after);
    }
}

#[test]
fn interrupted_history_requires_explicit_failure_and_unchanged_proven_identity() {
    for mutation in [
        "last_failure_code = NULL, last_failure_message = NULL",
        "last_failure_code = ''",
        "last_failure_message = ' '",
        "next_unread_usn = '19', captured_exclusive_end = '19'",
        "next_unread_usn = '21', captured_exclusive_end = '21'",
        "captured_exclusive_end = '21'",
        "volume_guid = 'unrelated-volume'",
        "volume_serial = '78'",
        "root_file_reference = X'02020202020202020202020202020202'",
        "journal_id = '45'",
        "protocol_version = 6",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        interrupt(
            &mut catalog,
            PersistentJournalRootFailureKind::NonRecoverable,
        );
        drop(catalog);
        let connection = Connection::open(fixture.path()).expect("mutate interrupted checkpoint");
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
fn interrupted_history_rejects_missing_range_or_inconsistent_root_projection() {
    for mutation in [
        "PRAGMA foreign_keys = ON; DELETE FROM library_persistent_journal_source_ranges",
        "DELETE FROM library_persistent_journal_range_lifecycle",
        "UPDATE library_persistent_journal_root_state SET continuity_state = 'current'",
        "UPDATE library_persistent_journal_checkpoints
         SET next_unread_usn = '20', captured_exclusive_end = '21'",
    ] {
        let fixture = recovery_lifecycle_catalog(true);
        let mut catalog = open(&fixture);
        publish_followup(&mut catalog, true, false);
        interrupt(
            &mut catalog,
            PersistentJournalRootFailureKind::NonRecoverable,
        );
        drop(catalog);
        let connection = Connection::open(fixture.path()).expect("mutate interrupted range proof");
        connection
            .execute_batch(mutation)
            .expect("persist missing coverage proof");
        drop(connection);
        reject(&fixture);
    }
}

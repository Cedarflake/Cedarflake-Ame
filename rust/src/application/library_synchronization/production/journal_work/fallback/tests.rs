use super::*;
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRecoveryOpeningBoundary,
    LibraryRootGeneration, PersistentJournalContinuityState, PersistentJournalVolumeIdentity,
};

fn checkpoint() -> PersistentJournalCheckpoint {
    PersistentJournalCheckpoint {
        root_id: "root-a".into(),
        root_generation: LibraryRootGeneration::initial(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "volume".into(),
            volume_serial: 1,
        },
        root_file_reference: JournalFileReference::V2([1; 8]),
        journal_id: JournalIdentifier::new(1).unwrap(),
        next_unread_usn: JournalUsn::new(10).unwrap(),
        captured_exclusive_end: JournalUsn::new(30).unwrap(),
        covered_catalog_revision: 1,
        protocol_version: 1,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: 1,
    }
}

fn timeout(checkpoint: &PersistentJournalCheckpoint) -> PersistentJournalReadFailure {
    PersistentJournalReadFailure {
        kind: PersistentJournalRootFailureKind::Transient,
        failure: PersistentJournalFailure {
            code: "identity_journal_timeout".into(),
            message: "Read timed out".into(),
        },
        opening_boundary: Some(Box::new(LibraryRecoveryOpeningBoundary {
            volume: checkpoint.volume.clone(),
            root_file_reference: checkpoint.root_file_reference.clone(),
            journal_id: checkpoint.journal_id,
            next_usn: checkpoint.captured_exclusive_end,
            protocol_version: checkpoint.protocol_version,
            contract_version: checkpoint.contract_version,
        })),
    }
}

#[test]
fn repeated_failed_reads_fall_back_but_checkpoint_progress_resets_the_budget() {
    let failures = Arc::new(JournalReadFailures::default());
    let mut checkpoint = checkpoint();
    for _ in 0..2 {
        let mut failure = timeout(&checkpoint);
        failures.classify(&checkpoint, &mut failure);
        assert_eq!(failure.kind, PersistentJournalRootFailureKind::Transient);
    }
    checkpoint.next_unread_usn = JournalUsn::new(20).unwrap();
    for attempt in 1..=3 {
        let mut failure = timeout(&checkpoint);
        failures.classify(&checkpoint, &mut failure);
        if attempt < 3 {
            assert_eq!(failure.kind, PersistentJournalRootFailureKind::Transient);
        } else {
            assert_eq!(
                failure.kind,
                PersistentJournalRootFailureKind::AutomaticMetadataFallback(
                    crate::domain::LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
                )
            );
            assert_eq!(failure.failure.code, "persistent_journal_read_stalled");
            assert!(failure.failure.message.contains("identity_journal_timeout"));
        }
    }
}

struct ReadResult(Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure>);

impl PersistentJournalVolumeReader for ReadResult {
    fn read_volume(
        &self,
        _: &[PersistentJournalCheckpoint],
        _: &[PersistentJournalPendingRename],
        _: i64,
        _: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        self.0.clone()
    }
}

#[test]
fn empty_success_cancel_and_new_generation_never_inherit_a_stalled_read() {
    let failures = Arc::new(JournalReadFailures::default());
    let mut checkpoint = checkpoint();
    let cancelled = AtomicBool::new(false);
    for reset in ["empty", "cancel", "generation"] {
        for _ in 0..2 {
            failures.classify(&checkpoint, &mut timeout(&checkpoint));
        }
        if reset == "generation" {
            checkpoint.root_generation = LibraryRootGeneration::new(2).unwrap();
        } else {
            let outcome = PersistentJournalRootReadOutcome {
                root_id: checkpoint.root_id.clone(),
                root_generation: checkpoint.root_generation,
                opening_boundary: None,
                page: Ok(None),
            };
            cancelled.store(reset == "cancel", Ordering::Release);
            let reader = FallbackReader {
                reader: ReadResult(Ok(vec![outcome])),
                failures: Arc::clone(&failures),
            };
            reader
                .read_volume(&[checkpoint.clone()], &[], 1, &cancelled)
                .unwrap();
            cancelled.store(false, Ordering::Release);
        }
        let mut failure = timeout(&checkpoint);
        failures.classify(&checkpoint, &mut failure);
        assert_eq!(failure.kind, PersistentJournalRootFailureKind::Transient);
        failures.reset(&checkpoint.root_id);
    }
}

#[test]
fn shared_failure_keeps_root_boundaries_separate_and_admits_broker_recovery() {
    let failures = Arc::new(JournalReadFailures::default());
    let first = checkpoint();
    let mut second = first.clone();
    second.root_id = "root-b".into();
    second.root_file_reference = JournalFileReference::V2([2; 8]);
    let reader = FallbackReader {
        reader: ReadResult(Err(timeout(&first))),
        failures: Arc::clone(&failures),
    };
    for _ in 0..2 {
        reader
            .read_volume(
                &[first.clone(), second.clone()],
                &[],
                1,
                &AtomicBool::new(false),
            )
            .unwrap();
    }
    let outcomes = reader
        .read_volume(&[first, second], &[], 2, &AtomicBool::new(false))
        .unwrap();
    let first = outcomes[0].page.as_ref().unwrap_err();
    let second = outcomes[1].page.as_ref().unwrap_err();
    assert_eq!(
        first.kind,
        PersistentJournalRootFailureKind::AutomaticMetadataFallback(
            crate::domain::LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
        )
    );
    assert_eq!(
        second.kind,
        PersistentJournalRootFailureKind::AutomaticMetadataFallback(
            crate::domain::LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
        )
    );
    assert!(second.opening_boundary.is_none());
}

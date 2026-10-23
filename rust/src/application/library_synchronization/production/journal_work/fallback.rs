use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::{
    PersistentJournalCheckpoint, PersistentJournalFailure, PersistentJournalPendingRename,
    PersistentJournalReadFailure, PersistentJournalRootFailureKind,
    PersistentJournalRootReadOutcome,
};
use crate::ports::PersistentJournalVolumeReader;

const MAX_FAILED_READS: u32 = 3;

#[derive(Default)]
pub(in super::super) struct JournalReadFailures {
    roots: Mutex<BTreeMap<String, FailedRead>>,
}

struct FailedRead {
    checkpoint: PersistentJournalCheckpoint,
    attempts: u32,
}

pub(in super::super) struct FallbackReader<R> {
    pub(in super::super) reader: R,
    pub(in super::super) failures: Arc<JournalReadFailures>,
}

impl JournalReadFailures {
    pub(in super::super) fn retain_roots(&self, roots: impl Fn(&str) -> bool) {
        if let Ok(mut failures) = self.roots.lock() {
            failures.retain(|root, _| roots(root));
        }
    }

    pub(super) fn classify(
        &self,
        checkpoint: &PersistentJournalCheckpoint,
        failure: &mut PersistentJournalReadFailure,
    ) {
        let Ok(mut roots) = self.roots.lock() else {
            return;
        };
        if failure.kind != PersistentJournalRootFailureKind::Transient {
            roots.remove(&checkpoint.root_id);
            if let Some(reason) = failure.kind.recovery_reason() {
                failure.kind = PersistentJournalRootFailureKind::AutomaticMetadataFallback(reason);
            }
            return;
        }
        let entry = roots
            .entry(checkpoint.root_id.clone())
            .or_insert_with(|| FailedRead {
                checkpoint: checkpoint.clone(),
                attempts: 0,
            });
        if !same_position(&entry.checkpoint, checkpoint) {
            entry.checkpoint = checkpoint.clone();
            entry.attempts = 0;
        }
        entry.attempts = entry.attempts.saturating_add(1);
        if entry.attempts < MAX_FAILED_READS {
            return;
        }
        failure.kind = PersistentJournalRootFailureKind::AutomaticMetadataFallback(
            crate::domain::LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure,
        );
        failure.failure = PersistentJournalFailure {
            code: "persistent_journal_read_stalled".into(),
            message: format!(
                "Journal reads failed {MAX_FAILED_READS} times at the same checkpoint: {}",
                failure.failure.code,
            ),
        };
    }

    pub(super) fn handle_closing_failure(
        &self,
        error: crate::domain::ScanError,
        root: &(String, crate::domain::LibraryRootGeneration),
        session: &crate::adapters::SqliteCatalogSession,
        now: i64,
        policy: crate::domain::LibraryChangeQueuePolicy,
        cancelled: &AtomicBool,
    ) -> Result<(), crate::domain::ScanError> {
        use crate::ports::PersistentJournalRepository;
        if cancelled.load(Ordering::Acquire)
            || !matches!(
                error.code.as_str(),
                "persistent_journal_boundary_unavailable"
                    | "persistent_journal_baseline_closing_unavailable"
            )
        {
            return Err(error);
        }
        let mut catalog = session.open_in_lane(crate::domain::LibraryChangeLane::Journal)?;
        let Some(checkpoint) = catalog.load_persistent_journal_checkpoint(&root.0, root.1)? else {
            return Err(error);
        };
        let mut failure = PersistentJournalReadFailure {
            kind: PersistentJournalRootFailureKind::Transient,
            failure: PersistentJournalFailure {
                code: error.code.clone(),
                message: error.message.clone(),
            },
            opening_boundary: None,
        };
        self.classify(&checkpoint, &mut failure);
        if !matches!(
            failure.kind,
            PersistentJournalRootFailureKind::AutomaticMetadataFallback(_)
        ) {
            return Err(error);
        }
        catalog.persist_persistent_journal_root_failure(
            &crate::domain::PersistentJournalRootFailure {
                root_id: root.0.clone(),
                root_generation: root.1,
                kind: failure.kind,
                failure: failure.failure,
                opening_boundary: None,
            },
            now,
            policy,
        )?;
        Ok(())
    }

    fn reset(&self, root_id: &str) {
        if let Ok(mut roots) = self.roots.lock() {
            roots.remove(root_id);
        }
    }
}

fn same_position(a: &PersistentJournalCheckpoint, b: &PersistentJournalCheckpoint) -> bool {
    a.root_generation == b.root_generation
        && a.volume == b.volume
        && a.root_file_reference == b.root_file_reference
        && a.journal_id == b.journal_id
        && a.next_unread_usn == b.next_unread_usn
        && a.captured_exclusive_end == b.captured_exclusive_end
        && a.protocol_version == b.protocol_version
        && a.contract_version == b.contract_version
}

impl<R: PersistentJournalVolumeReader> PersistentJournalVolumeReader for FallbackReader<R> {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        // Count completed failed reads, never scheduling delays, capacity yields or slow I/O.
        let result = self
            .reader
            .read_volume(checkpoints, pending, observed_unix_ms, cancelled);
        if cancelled.load(Ordering::Acquire) {
            for checkpoint in checkpoints {
                self.failures.reset(&checkpoint.root_id);
            }
            return result;
        }
        let mut outcomes = match result {
            Ok(outcomes) => outcomes,
            Err(failure) => checkpoints
                .iter()
                .map(|checkpoint| {
                    let mut failure = failure.clone();
                    if failure.opening_boundary.as_ref().is_some_and(|opening| {
                        opening.volume != checkpoint.volume
                            || opening.root_file_reference != checkpoint.root_file_reference
                    }) {
                        failure.opening_boundary = None;
                    }
                    PersistentJournalRootReadOutcome {
                        root_id: checkpoint.root_id.clone(),
                        root_generation: checkpoint.root_generation,
                        opening_boundary: failure.opening_boundary.as_deref().cloned(),
                        page: Err(failure),
                    }
                })
                .collect(),
        };
        for outcome in &mut outcomes {
            let Some(checkpoint) = checkpoints.iter().find(|checkpoint| {
                checkpoint.root_id == outcome.root_id
                    && checkpoint.root_generation == outcome.root_generation
            }) else {
                continue;
            };
            match &mut outcome.page {
                Err(failure) => self.failures.classify(checkpoint, failure),
                Ok(_) => {
                    self.failures.reset(&checkpoint.root_id);
                    if outcome.page.as_ref().is_ok_and(Option::is_none)
                        && checkpoint.continuity
                            == crate::domain::PersistentJournalContinuityState::CatchingUp
                        && outcome.opening_boundary.as_ref().is_some_and(|opening| {
                            opening.volume == checkpoint.volume
                                && opening.root_file_reference == checkpoint.root_file_reference
                                && opening.journal_id == checkpoint.journal_id
                                && opening.next_usn == checkpoint.next_unread_usn
                                && opening.next_usn == checkpoint.captured_exclusive_end
                        })
                    {
                        outcome.page =
                            Ok(Some(crate::domain::PersistentJournalReadPage::EmptyReplay(
                                checkpoint.clone(),
                            )));
                    }
                }
            }
        }
        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests;

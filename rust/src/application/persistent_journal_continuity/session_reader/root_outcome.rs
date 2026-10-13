use crate::domain::{
    JournalIdentifier, JournalUsn, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeScope, PERSISTENT_JOURNAL_CONTRACT_VERSION,
    PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalEnrollmentBatch, PersistentJournalRangeState, PersistentJournalReadFailure,
    PersistentJournalSourceRange, ScanError, persistent_journal_batch_id,
};
use crate::journal_broker::{
    BrokerCandidate, CandidateKind, CandidateScope, PROTOCOL_VERSION, SharedJournalRootOutcome,
};

use super::{
    RootPage, containment_failure, invalid, map_broker_failure_code, reconstruction_failure,
    reconstruction_from_error, recovery_opening_boundary,
};

pub(super) fn translate_root_outcome(
    previous: &PersistentJournalCheckpoint,
    outcome: SharedJournalRootOutcome,
    requested_end_usn: i64,
    observed_unix_ms: i64,
) -> Result<Option<RootPage>, PersistentJournalReadFailure> {
    let opening_boundary =
        recovery_opening_boundary(previous, previous.journal_id.value(), requested_end_usn)?;
    if let Some(failure) = outcome.failure {
        return Err(map_broker_failure_code(
            failure.code,
            Some(opening_boundary),
        ));
    }
    if outcome.binding.root_id != previous.root_id
        || outcome.binding.root_generation != previous.root_generation.value()
        || outcome.binding.volume_id != previous.volume.volume_guid
        || outcome.requested_start_usn != previous.next_unread_usn.value()
    {
        return Err(containment_failure(
            "broker root outcome identity is invalid",
            Some(opening_boundary),
        ));
    }
    let covered = outcome.covered_until_usn.ok_or_else(|| {
        reconstruction_failure(
            "broker root outcome proof is missing",
            Some(opening_boundary.clone()),
        )
    })?;
    if previous.next_unread_usn.value() == requested_end_usn {
        if covered != requested_end_usn || !outcome.is_complete || !outcome.candidates.is_empty() {
            return Err(reconstruction_failure(
                "broker empty replay boundary is invalid",
                Some(opening_boundary),
            ));
        }
        return Ok(None);
    }
    let intents = outcome
        .candidates
        .into_iter()
        .map(|candidate| candidate_to_intent(previous, candidate, observed_unix_ms))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?;
    let range = PersistentJournalSourceRange {
        batch_id: "pending-content-id".to_owned(),
        root_id: previous.root_id.clone(),
        root_generation: previous.root_generation,
        volume: previous.volume.clone(),
        journal_id: JournalIdentifier::new(previous.journal_id.value())
            .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?,
        requested_start_usn: previous.next_unread_usn,
        requested_end_usn: JournalUsn::new(requested_end_usn)
            .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?,
        covered_until_usn: JournalUsn::new(covered)
            .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?,
        is_complete: outcome.is_complete,
        protocol_version: PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        state: PersistentJournalRangeState::Enrolled,
        enrolled_unix_ms: observed_unix_ms,
        checkpointed_unix_ms: None,
    };
    let checkpoint = PersistentJournalCheckpoint {
        root_id: previous.root_id.clone(),
        root_generation: previous.root_generation,
        volume: previous.volume.clone(),
        root_file_reference: previous.root_file_reference.clone(),
        journal_id: previous.journal_id,
        next_unread_usn: JournalUsn::new(covered)
            .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?,
        captured_exclusive_end: JournalUsn::new(requested_end_usn)
            .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?,
        covered_catalog_revision: previous.covered_catalog_revision,
        protocol_version: PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: observed_unix_ms,
    };
    let mut batch = PersistentJournalEnrollmentBatch {
        range,
        intents,
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
        directory_changes: Vec::new(),
    };
    batch.range.batch_id = persistent_journal_batch_id(&batch);
    batch
        .validate()
        .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?;
    checkpoint
        .validate()
        .map_err(|error| reconstruction_from_error(error, Some(opening_boundary)))?;
    Ok(Some((batch, checkpoint)))
}

fn candidate_to_intent(
    checkpoint: &PersistentJournalCheckpoint,
    candidate: BrokerCandidate,
    observed_unix_ms: i64,
) -> Result<LibraryChangeIntent, ScanError> {
    let (scope, relative_path) = match candidate.scope {
        CandidateScope::Root => (LibraryChangeScope::Root, String::new()),
        CandidateScope::RelativePath(path) => (
            if candidate.kind == CandidateKind::Subtree {
                LibraryChangeScope::Subtree
            } else {
                LibraryChangeScope::Path
            },
            path,
        ),
    };
    Ok(LibraryChangeIntent {
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        kind: if candidate.kind == CandidateKind::Rename {
            LibraryChangeIntentKind::RenameCandidate
        } else {
            LibraryChangeIntentKind::Reconcile
        },
        scope,
        relative_path,
        previous_relative_path: candidate.previous_scope.and_then(|scope| match scope {
            CandidateScope::Root => None,
            CandidateScope::RelativePath(path) => Some(path),
        }),
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: observed_unix_ms,
        most_recent_observed_unix_ms: observed_unix_ms,
        first_sequence: u64::try_from(candidate.usn)
            .map_err(|_| invalid("broker candidate USN is invalid"))?,
        most_recent_sequence: u64::try_from(candidate.usn)
            .map_err(|_| invalid("broker candidate USN is invalid"))?,
        coalesced_observation_count: 1,
    })
}

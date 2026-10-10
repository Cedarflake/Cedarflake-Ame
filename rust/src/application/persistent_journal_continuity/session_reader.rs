use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeScope,
    LibraryRecoveryOpeningBoundary, LibraryRootGeneration, PERSISTENT_JOURNAL_CONTRACT_VERSION,
    PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalCrossRootLineage, PersistentJournalEnrollmentBatch, PersistentJournalFailure,
    PersistentJournalLineageState, PersistentJournalPendingRename, PersistentJournalRangeState,
    PersistentJournalReadFailure, PersistentJournalRootFailureKind,
    PersistentJournalRootReadOutcome, PersistentJournalSourceRange, ScanError,
    persistent_journal_batch_id, persistent_journal_pending_rename_id,
};
use crate::journal_broker::{
    BrokerCandidate, BrokerFailureCode, BrokerResponse, CallerClaim, CandidateKind, CandidateScope,
    JournalCapability, PROTOCOL_VERSION, PersistentChangeJournal,
    PersistentChangeJournalConnection, PersistentChangeJournalOperationError,
    PersistentChangeJournalSession, QueryJournalRequest, ReadJournalVolumeRequest,
    RegisterRootRequest, RootAuthorization, SharedJournalPendingRename,
    SharedJournalPendingRenameRequest, SharedJournalRootOutcome, SharedJournalRootRequest,
};
use crate::ports::PersistentJournalVolumeReader;

const DEFAULT_MAX_RECORDS: u32 = 64;
const DEFAULT_MAX_EVIDENCE_BYTES: u32 = 512 * 1_024;
const DEFAULT_TIMEOUT_MS: u32 = 10_000;

type RootKey = (String, LibraryRootGeneration);
type RootPage = (
    PersistentJournalEnrollmentBatch,
    PersistentJournalCheckpoint,
);
type CollectedRootOutcome = (
    RootKey,
    Result<Option<RootPage>, PersistentJournalReadFailure>,
);

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct PersistentJournalBrokerRoot {
    pub(crate) authorization: RootAuthorization,
    pub(crate) client_root_handle: u64,
    pub(crate) volume_serial: u64,
}

pub(crate) struct SessionBackedPersistentJournalVolumeReader {
    connection: SessionConnection,
    caller: CallerClaim,
    roots: HashMap<(String, LibraryRootGeneration), PersistentJournalBrokerRoot>,
}

enum SessionConnection {
    Factory(Arc<dyn PersistentChangeJournal>),
    Retained(Arc<dyn PersistentChangeJournalSession>),
}

impl SessionBackedPersistentJournalVolumeReader {
    pub(crate) fn new(
        journal: Arc<dyn PersistentChangeJournal>,
        caller: CallerClaim,
        roots: Vec<PersistentJournalBrokerRoot>,
    ) -> Result<Self, ScanError> {
        let mut indexed = HashMap::with_capacity(roots.len());
        for root in roots {
            let generation = LibraryRootGeneration::new(root.authorization.root_generation)
                .ok_or_else(|| invalid("root generation is zero"))?;
            if root.client_root_handle == 0
                || root.authorization.root_identity.len() != 16
                || indexed
                    .insert((root.authorization.root_id.clone(), generation), root)
                    .is_some()
            {
                return Err(invalid("broker root registration is invalid or duplicated"));
            }
        }
        if indexed.is_empty() || indexed.len() > 8 {
            return Err(invalid("broker root registration count is outside 1..=8"));
        }
        Ok(Self {
            connection: SessionConnection::Factory(journal),
            caller,
            roots: indexed,
        })
    }

    pub(crate) fn with_retained_session(
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: CallerClaim,
        roots: Vec<PersistentJournalBrokerRoot>,
    ) -> Result<Self, ScanError> {
        let mut reader = Self::new(Arc::new(UnavailablePersistentChangeJournal), caller, roots)?;
        reader.connection = SessionConnection::Retained(session);
        Ok(reader)
    }
}

struct UnavailablePersistentChangeJournal;

impl PersistentChangeJournal for UnavailablePersistentChangeJournal {
    fn connect(&self) -> PersistentChangeJournalConnection {
        PersistentChangeJournalConnection::LiveOnly(
            crate::journal_broker::PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
        )
    }
}

impl PersistentJournalVolumeReader for SessionBackedPersistentJournalVolumeReader {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending_renames: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_failure(
                "persistent journal volume read was cancelled",
            ));
        }
        if checkpoints.is_empty() || checkpoints.len() > 8 || observed_unix_ms < 0 {
            return Err(nonrecoverable_failure(
                "persistent journal volume read input is invalid",
            ));
        }
        let first = &checkpoints[0];
        if checkpoints.iter().any(|checkpoint| {
            checkpoint.volume != first.volume || checkpoint.journal_id != first.journal_id
        }) {
            return Err(nonrecoverable_failure(
                "persistent journal roots do not share one volume and journal",
            ));
        }
        match &self.connection {
            SessionConnection::Retained(session) => self.read_connected(
                session.as_ref(),
                checkpoints,
                pending_renames,
                observed_unix_ms,
                cancelled,
            ),
            SessionConnection::Factory(journal) => {
                let session = match journal.connect() {
                    PersistentChangeJournalConnection::Connected(session) => session,
                    PersistentChangeJournalConnection::LiveOnly(_) => {
                        return Err(broker_after_current_failure(
                            "persistent journal broker is unavailable",
                        ));
                    }
                };
                let result = self.read_connected(
                    session.as_ref(),
                    checkpoints,
                    pending_renames,
                    observed_unix_ms,
                    cancelled,
                );
                let close = session.close().map_err(map_operation_failure);
                match (result, close) {
                    (Ok(value), Ok(())) => Ok(value),
                    (Err(error), _) => Err(error),
                    (Ok(_), Err(error)) => Err(error),
                }
            }
        }
    }
}

impl SessionBackedPersistentJournalVolumeReader {
    fn read_connected(
        &self,
        session: &dyn crate::journal_broker::PersistentChangeJournalSession,
        checkpoints: &[PersistentJournalCheckpoint],
        pending_renames: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        let mut outcomes = Vec::with_capacity(checkpoints.len());
        let mut admitted = Vec::with_capacity(checkpoints.len());
        let mut registered = HashMap::with_capacity(checkpoints.len());
        let mut openings = HashMap::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_failure(
                    "persistent journal volume read was cancelled between roots",
                ));
            }
            checkpoint.validate().map_err(nonrecoverable_from_error)?;
            let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
            let Some(root) = self.roots.get(&key) else {
                outcomes.push((
                    key,
                    Err(containment_failure(
                        "broker root registration is missing",
                        None,
                    )),
                ));
                continue;
            };
            if root.authorization.volume_id != checkpoint.volume.volume_guid
                || root.volume_serial != checkpoint.volume.volume_serial
                || root.authorization.root_generation != checkpoint.root_generation.value()
            {
                outcomes.push((
                    key,
                    Err(containment_failure(
                        "broker root identity does not match checkpoint",
                        None,
                    )),
                ));
                continue;
            }
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_failure(
                    "persistent journal volume read was cancelled before root registration",
                ));
            }
            let capability = match session.register_root(RegisterRootRequest {
                caller: self.caller.clone(),
                root: root.authorization.clone(),
                client_root_handle: root.client_root_handle,
                timeout_ms: DEFAULT_TIMEOUT_MS,
            }) {
                Ok(capability) => capability,
                Err(error) => {
                    outcomes.push((key, Err(map_operation_failure(error))));
                    continue;
                }
            };
            registered.insert(key.clone(), capability);
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_failure(
                    "persistent journal volume read was cancelled before journal query",
                ));
            }
            let query = session.query_journal(QueryJournalRequest {
                caller: self.caller.clone(),
                root: root.authorization.clone(),
                root_capability: capability,
                timeout_ms: DEFAULT_TIMEOUT_MS,
            });
            match query {
                Ok(BrokerResponse::Journal {
                    capability: JournalCapability::Supported,
                    journal_id: Some(journal_id),
                    first_usn: Some(first_usn),
                    next_usn: Some(next_usn),
                    ..
                }) => {
                    let opening = recovery_opening_boundary(checkpoint, journal_id, next_usn)?;
                    if journal_id != checkpoint.journal_id.value() {
                        outcomes.push((
                            key,
                            Err(read_failure(
                                PersistentJournalRootFailureKind::JournalReset,
                                "persistent_journal_reset",
                                "The journal identity changed after the last current checkpoint",
                                Some(opening),
                            )),
                        ));
                    } else if first_usn > checkpoint.next_unread_usn.value() {
                        outcomes.push((
                            key,
                            Err(read_failure(
                                PersistentJournalRootFailureKind::JournalTrim,
                                "persistent_journal_trimmed",
                                "The retained journal begins after the last current checkpoint",
                                Some(opening),
                            )),
                        ));
                    } else if next_usn < checkpoint.next_unread_usn.value() {
                        outcomes.push((
                            key,
                            Err(read_failure(
                                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                                "persistent_journal_boundary_regressed",
                                "The journal boundary regressed behind the current checkpoint",
                                Some(opening),
                            )),
                        ));
                    } else {
                        openings.insert(key.clone(), opening.clone());
                        admitted.push((checkpoint, root, capability, next_usn, opening));
                    }
                }
                Ok(BrokerResponse::Journal { .. }) => outcomes.push((
                    key,
                    Err(nonrecoverable_failure(
                        "broker journal capability response cannot provide persistent continuity",
                    )),
                )),
                Ok(_) => outcomes.push((
                    key,
                    Err(reconstruction_failure(
                        "broker query response is invalid",
                        None,
                    )),
                )),
                Err(error) => outcomes.push((key, Err(map_operation_failure(error)))),
            }
        }
        if admitted.is_empty() {
            return Ok(order_outcomes(checkpoints, outcomes, openings));
        }
        let end_usn = admitted
            .iter()
            .map(|(_, _, _, next_usn, _)| *next_usn)
            .min()
            .ok_or_else(|| {
                reconstruction_failure("broker did not capture a journal boundary", None)
            })?;
        let mut progressing = Vec::with_capacity(admitted.len());
        for admitted_root in admitted {
            if admitted_root.0.next_unread_usn.value() < end_usn {
                progressing.push(admitted_root);
            } else {
                outcomes.push((
                    (
                        admitted_root.0.root_id.clone(),
                        admitted_root.0.root_generation,
                    ),
                    Ok(None),
                ));
            }
        }
        if progressing.is_empty() {
            return Ok(order_outcomes(checkpoints, outcomes, openings));
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_failure(
                "persistent journal volume read was cancelled before carry reconstruction",
            ));
        }
        let carried = pending_renames
            .iter()
            .map(|pending| {
                if cancelled.load(Ordering::Acquire) {
                    return Err(ScanError::new(
                        "persistent_journal_cancelled",
                        "Persistent journal carry reconstruction was cancelled",
                    ));
                }
                pending.validate()?;
                if pending.volume != checkpoints[0].volume
                    || pending.journal_id != checkpoints[0].journal_id
                {
                    return Err(invalid(
                        "pending rename carry does not match the requested volume",
                    ));
                }
                let key = (
                    pending.previous_root_id.clone(),
                    pending.previous_root_generation,
                );
                let root = self
                    .roots
                    .get(&key)
                    .ok_or_else(|| invalid("pending rename root registration is missing"))?;
                Ok(SharedJournalPendingRenameRequest {
                    carry_id: pending.carry_id.clone(),
                    source_range_id: pending.source_range_id.clone(),
                    root: root.authorization.clone(),
                    root_capability: registered.get(&key).copied(),
                    file_reference: pending.file_reference.as_bytes().to_vec(),
                    old_usn: pending.old_usn.value(),
                    previous_relative_path: pending.previous_relative_path.clone(),
                    is_directory: pending.is_directory,
                })
            })
            .collect::<Result<Vec<_>, ScanError>>()
            .map_err(|error| {
                if error.code == "persistent_journal_cancelled" {
                    cancelled_from_error(error)
                } else {
                    reconstruction_from_error(error, None)
                }
            })?;
        let request = ReadJournalVolumeRequest {
            caller: self.caller.clone(),
            roots: progressing
                .iter()
                .map(
                    |(checkpoint, root, capability, _, _)| SharedJournalRootRequest {
                        root: root.authorization.clone(),
                        root_capability: *capability,
                        start_usn: checkpoint.next_unread_usn.value(),
                    },
                )
                .collect(),
            pending_renames: carried,
            journal_id: progressing[0].0.journal_id.value(),
            end_usn,
            max_records: DEFAULT_MAX_RECORDS,
            max_evidence_bytes: DEFAULT_MAX_EVIDENCE_BYTES,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        };
        let request_binding = request.clone();
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_failure(
                "persistent journal volume read was cancelled before physical read",
            ));
        }
        let pending = session
            .begin_read_volume(request)
            .map_err(map_operation_failure)?;
        if cancelled.load(Ordering::Acquire) {
            pending
                .cancel(self.caller.clone())
                .map_err(map_operation_failure)?;
        }
        let response = pending.wait().map_err(map_operation_failure)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_failure(
                "persistent journal volume read completed after cancellation",
            ));
        }
        let BrokerResponse::ReadVolume {
            client_instance,
            volume_id,
            journal_id,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            outcomes: broker_outcomes,
            handoffs,
            pending_renames: broker_pending_renames,
            ..
        } = response
        else {
            return Err(reconstruction_failure(
                "broker shared read response is invalid",
                None,
            ));
        };
        if client_instance != self.caller.client_instance
            || volume_id != checkpoints[0].volume.volume_guid
            || journal_id != progressing[0].0.journal_id.value()
            || requested_end_usn != end_usn
            || max_records != DEFAULT_MAX_RECORDS
            || max_evidence_bytes != DEFAULT_MAX_EVIDENCE_BYTES
            || broker_outcomes.len() != progressing.len()
        {
            return Err(reconstruction_failure(
                "broker shared read boundary is invalid",
                None,
            ));
        }
        for (broker, (checkpoint, _, _, _, _opening)) in
            broker_outcomes.into_iter().zip(progressing)
        {
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_failure(
                    "persistent journal volume read was cancelled between root outcomes",
                ));
            }
            let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
            outcomes.push((
                key,
                translate_root_outcome(checkpoint, broker, end_usn, observed_unix_ms).map(Some),
            ));
        }
        attach_pending_renames(
            &mut outcomes,
            &broker_pending_renames,
            &self.caller,
            &checkpoints[0].volume,
            journal_id,
            observed_unix_ms,
        )
        .map_err(|error| reconstruction_from_error(error, None))?;
        attach_cross_root_lineage(
            &mut outcomes,
            &handoffs,
            pending_renames,
            &request_binding,
            &checkpoints[0].volume,
            journal_id,
        )
        .map_err(|error| reconstruction_from_error(error, None))?;
        finalize_batch_ids(&mut outcomes)
            .map_err(|error| reconstruction_from_error(error, None))?;
        Ok(order_outcomes(checkpoints, outcomes, openings))
    }
}

fn attach_pending_renames(
    outcomes: &mut [CollectedRootOutcome],
    pending_renames: &[SharedJournalPendingRename],
    caller: &CallerClaim,
    volume: &crate::domain::PersistentJournalVolumeIdentity,
    journal_id: u64,
    observed_unix_ms: i64,
) -> Result<(), ScanError> {
    let mut seen = HashSet::new();
    for pending in pending_renames {
        if pending.binding.client_instance != caller.client_instance
            || pending.binding.volume_id != volume.volume_guid
        {
            return Err(invalid("broker pending rename binding is invalid"));
        }
        let root_generation = LibraryRootGeneration::new(pending.binding.root_generation)
            .ok_or_else(|| invalid("broker pending rename generation is invalid"))?;
        let key = (pending.binding.root_id.clone(), root_generation);
        let index = outcomes
            .iter()
            .position(|(outcome_key, page)| outcome_key == &key && matches!(page, Ok(Some(_))))
            .ok_or_else(|| invalid("broker pending rename has no successful root proof"))?;
        let Some((batch, _)) = outcomes[index].1.as_mut().ok().and_then(Option::as_mut) else {
            return Err(invalid("broker pending rename root proof disappeared"));
        };
        if pending.old_usn < batch.range.requested_start_usn.value()
            || pending.old_usn >= batch.range.covered_until_usn.value()
        {
            return Err(invalid(
                "broker pending rename USN is outside its root proof",
            ));
        }
        let mut durable = PersistentJournalPendingRename {
            carry_id: "pending-carry-id".to_owned(),
            source_range_id: batch.range.batch_id.clone(),
            volume: volume.clone(),
            journal_id: JournalIdentifier::new(journal_id)?,
            file_reference: JournalFileReference::from_bytes(&pending.file_reference)?,
            old_usn: JournalUsn::new(pending.old_usn)?,
            previous_root_id: key.0,
            previous_root_generation: key.1,
            previous_relative_path: pending.previous_relative_path.clone(),
            is_directory: pending.is_directory,
            enrolled_unix_ms: observed_unix_ms,
        };
        durable.carry_id = persistent_journal_pending_rename_id(&durable);
        if !seen.insert(durable.carry_id.clone())
            || batch
                .pending_renames
                .iter()
                .any(|existing| existing.carry_id == durable.carry_id)
        {
            return Err(invalid("broker pending rename is duplicated"));
        }
        durable.validate()?;
        batch.pending_renames.push(durable);
    }
    Ok(())
}

fn attach_cross_root_lineage(
    outcomes: &mut [CollectedRootOutcome],
    handoffs: &[crate::journal_broker::SharedJournalHandoff],
    pending_renames: &[PersistentJournalPendingRename],
    request: &ReadJournalVolumeRequest,
    volume: &crate::domain::PersistentJournalVolumeIdentity,
    journal_id: u64,
) -> Result<(), ScanError> {
    let mut consumed = HashSet::new();
    for handoff in handoffs {
        if let Some(carry_id) = &handoff.previous_carry_id {
            if !consumed.insert(carry_id.as_str()) {
                return Err(invalid("broker consumed a pending rename more than once"));
            }
            attach_carried_cross_root_lineage(
                outcomes,
                handoff,
                carry_id,
                pending_renames,
                request,
                volume,
                journal_id,
            )?;
            continue;
        }
        if !binding_matches_requested_root(&handoff.previous_binding, request)
            || !binding_matches_requested_root(&handoff.current_binding, request)
        {
            return Err(invalid(
                "broker handoff is not bound to the original root request",
            ));
        }
        let file_reference = JournalFileReference::from_bytes(&handoff.file_reference)?;
        let previous_key = (
            handoff.previous_binding.root_id.clone(),
            LibraryRootGeneration::new(handoff.previous_binding.root_generation)
                .ok_or_else(|| invalid("broker handoff previous generation is invalid"))?,
        );
        let current_key = (
            handoff.current_binding.root_id.clone(),
            LibraryRootGeneration::new(handoff.current_binding.root_generation)
                .ok_or_else(|| invalid("broker handoff current generation is invalid"))?,
        );
        if previous_key == current_key {
            return Err(invalid("broker handoff endpoints are not distinct"));
        }
        let previous_index = outcomes
            .iter()
            .position(|(key, page)| key == &previous_key && matches!(page, Ok(Some(_))))
            .ok_or_else(|| invalid("broker handoff previous endpoint has no successful proof"))?;
        let current_index = outcomes
            .iter()
            .position(|(key, page)| key == &current_key && matches!(page, Ok(Some(_))))
            .ok_or_else(|| invalid("broker handoff current endpoint has no successful proof"))?;
        let endpoint_range_accepts = |index: usize| {
            outcomes[index]
                .1
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .is_some_and(|(batch, _)| {
                    handoff.usn >= batch.range.requested_start_usn.value()
                        && handoff.usn < batch.range.covered_until_usn.value()
                })
        };
        if !endpoint_range_accepts(previous_index) || !endpoint_range_accepts(current_index) {
            return Err(invalid("broker handoff USN is outside an endpoint proof"));
        }
        let range_identity = |index: usize| {
            outcomes[index]
                .1
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .map(|(batch, _)| {
                    (
                        batch.range.requested_start_usn.value(),
                        batch.range.requested_end_usn.value(),
                        batch.range.covered_until_usn.value(),
                        batch.range.protocol_version,
                        batch.range.contract_version,
                    )
                })
        };
        let previous_range = range_identity(previous_index)
            .ok_or_else(|| invalid("broker handoff previous range proof disappeared"))?;
        let current_range = range_identity(current_index)
            .ok_or_else(|| invalid("broker handoff current range proof disappeared"))?;
        let lineage_id = super::super::scan_library::stable_id(
            "persistent-journal-handoff-v4",
            &format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                volume.volume_guid,
                volume.volume_serial,
                journal_id,
                file_reference.to_canonical_text(),
                handoff.usn,
                previous_key.0,
                previous_key.1.value(),
                handoff.previous_relative_path,
                current_key.0,
                current_key.1.value(),
                handoff.current_relative_path,
                u8::from(handoff.is_directory),
                previous_range.0,
                previous_range.1,
                previous_range.2,
                previous_range.3,
                previous_range.4,
                current_range.0,
                current_range.1,
                current_range.2,
                current_range.3,
                current_range.4,
            ),
        );
        for (index, owner_key) in [
            (previous_index, &previous_key),
            (current_index, &current_key),
        ] {
            let Some((batch, _)) = outcomes[index].1.as_mut().ok().and_then(Option::as_mut) else {
                return Err(invalid("broker handoff endpoint proof disappeared"));
            };
            batch
                .cross_root_lineage
                .push(PersistentJournalCrossRootLineage {
                    lineage_id: lineage_id.clone(),
                    owner_source_range_id: batch.range.batch_id.clone(),
                    volume: volume.clone(),
                    journal_id: JournalIdentifier::new(journal_id)?,
                    file_reference: file_reference.clone(),
                    old_usn: None,
                    new_usn: None,
                    previous_carry_id: None,
                    previous_root_id: previous_key.0.clone(),
                    previous_root_generation: previous_key.1,
                    previous_relative_path: handoff.previous_relative_path.clone(),
                    current_root_id: current_key.0.clone(),
                    current_root_generation: current_key.1,
                    current_relative_path: handoff.current_relative_path.clone(),
                    state: PersistentJournalLineageState::Pending,
                });
            if batch.range.root_id != owner_key.0 || batch.range.root_generation != owner_key.1 {
                return Err(invalid("broker handoff endpoint owner is mismatched"));
            }
        }
    }
    Ok(())
}

fn attach_carried_cross_root_lineage(
    outcomes: &mut [CollectedRootOutcome],
    handoff: &crate::journal_broker::SharedJournalHandoff,
    carry_id: &str,
    pending_renames: &[PersistentJournalPendingRename],
    request: &ReadJournalVolumeRequest,
    volume: &crate::domain::PersistentJournalVolumeIdentity,
    journal_id: u64,
) -> Result<(), ScanError> {
    let carry = pending_renames
        .iter()
        .find(|pending| pending.carry_id == carry_id)
        .ok_or_else(|| invalid("broker consumed an unknown pending rename"))?;
    let requested = request
        .pending_renames
        .iter()
        .find(|pending| pending.carry_id == carry_id)
        .ok_or_else(|| invalid("broker carried handoff is absent from the original request"))?;
    if handoff.previous_binding.client_instance != request.caller.client_instance
        || handoff.previous_binding.root_id != carry.previous_root_id
        || handoff.previous_binding.root_generation != carry.previous_root_generation.value()
        || handoff.previous_binding.volume_id != volume.volume_guid
        || requested.source_range_id != carry.source_range_id
        || requested.file_reference != carry.file_reference.as_bytes()
        || handoff.file_reference != carry.file_reference.as_bytes()
        || handoff.previous_relative_path != carry.previous_relative_path
        || handoff.is_directory != carry.is_directory
        || handoff.usn <= carry.old_usn.value()
        || !binding_matches_requested_root(&handoff.current_binding, request)
    {
        return Err(invalid(
            "broker carried handoff does not match durable request evidence",
        ));
    }
    let current_generation = LibraryRootGeneration::new(handoff.current_binding.root_generation)
        .ok_or_else(|| invalid("broker carried handoff generation is invalid"))?;
    let current_key = (handoff.current_binding.root_id.clone(), current_generation);
    if current_key
        == (
            carry.previous_root_id.clone(),
            carry.previous_root_generation,
        )
    {
        return Err(invalid("broker carried handoff endpoints are not distinct"));
    }
    let current_index = outcomes
        .iter()
        .position(|(key, page)| key == &current_key && matches!(page, Ok(Some(_))))
        .ok_or_else(|| invalid("broker carried handoff current endpoint has no proof"))?;
    let Some((batch, _)) = outcomes[current_index]
        .1
        .as_mut()
        .ok()
        .and_then(Option::as_mut)
    else {
        return Err(invalid("broker carried handoff current proof disappeared"));
    };
    if handoff.usn < batch.range.requested_start_usn.value()
        || handoff.usn >= batch.range.covered_until_usn.value()
    {
        return Err(invalid(
            "broker carried handoff USN is outside current proof",
        ));
    }
    let file_reference = JournalFileReference::from_bytes(&handoff.file_reference)?;
    let lineage_id = super::super::scan_library::stable_id(
        "persistent-journal-carried-handoff-v1",
        &[
            volume.volume_guid.clone(),
            volume.volume_serial.to_string(),
            journal_id.to_string(),
            carry.source_range_id.clone(),
            carry.carry_id.clone(),
            file_reference.to_canonical_text(),
            carry.old_usn.value().to_string(),
            handoff.usn.to_string(),
            carry.previous_root_id.clone(),
            carry.previous_root_generation.value().to_string(),
            carry.previous_relative_path.clone(),
            current_key.0.clone(),
            current_key.1.value().to_string(),
            handoff.current_relative_path.clone(),
            u8::from(handoff.is_directory).to_string(),
            batch.range.requested_start_usn.value().to_string(),
            batch.range.requested_end_usn.value().to_string(),
            batch.range.covered_until_usn.value().to_string(),
        ]
        .join("\0"),
    );
    let lineage = PersistentJournalCrossRootLineage {
        lineage_id,
        owner_source_range_id: carry.source_range_id.clone(),
        volume: volume.clone(),
        journal_id: JournalIdentifier::new(journal_id)?,
        file_reference,
        old_usn: Some(carry.old_usn),
        new_usn: Some(JournalUsn::new(handoff.usn)?),
        previous_carry_id: Some(carry.carry_id.clone()),
        previous_root_id: carry.previous_root_id.clone(),
        previous_root_generation: carry.previous_root_generation,
        previous_relative_path: carry.previous_relative_path.clone(),
        current_root_id: current_key.0,
        current_root_generation: current_key.1,
        current_relative_path: handoff.current_relative_path.clone(),
        state: PersistentJournalLineageState::Pending,
    };
    batch.carried_cross_root_lineage.push(lineage.clone());
    batch
        .cross_root_lineage
        .push(PersistentJournalCrossRootLineage {
            owner_source_range_id: batch.range.batch_id.clone(),
            ..lineage
        });
    batch
        .consumed_pending_rename_ids
        .push(carry.carry_id.clone());
    Ok(())
}

fn binding_matches_requested_root(
    binding: &crate::journal_broker::ResponseBinding,
    request: &ReadJournalVolumeRequest,
) -> bool {
    binding.client_instance == request.caller.client_instance
        && request.roots.iter().any(|requested| {
            requested.root.root_id == binding.root_id
                && requested.root.root_generation == binding.root_generation
                && requested.root.volume_id == binding.volume_id
        })
}

fn finalize_batch_ids(outcomes: &mut [CollectedRootOutcome]) -> Result<(), ScanError> {
    for (_, page) in outcomes {
        let Some((batch, _)) = page.as_mut().ok().and_then(Option::as_mut) else {
            continue;
        };
        let previous_id = batch.range.batch_id.clone();
        let content_id = persistent_journal_batch_id(batch);
        batch.range.batch_id = content_id.clone();
        for lineage in &mut batch.cross_root_lineage {
            if lineage.owner_source_range_id == previous_id {
                lineage.owner_source_range_id.clone_from(&content_id);
            }
        }
        for pending in &mut batch.pending_renames {
            if pending.source_range_id == previous_id {
                pending.source_range_id.clone_from(&content_id);
            }
        }
        batch.validate()?;
    }
    Ok(())
}

fn order_outcomes(
    checkpoints: &[PersistentJournalCheckpoint],
    outcomes: Vec<CollectedRootOutcome>,
    mut openings: HashMap<RootKey, LibraryRecoveryOpeningBoundary>,
) -> Vec<PersistentJournalRootReadOutcome> {
    let mut indexed = outcomes.into_iter().collect::<HashMap<_, _>>();
    checkpoints
        .iter()
        .map(|checkpoint| PersistentJournalRootReadOutcome {
            root_id: checkpoint.root_id.clone(),
            root_generation: checkpoint.root_generation,
            opening_boundary: openings
                .remove(&(checkpoint.root_id.clone(), checkpoint.root_generation)),
            page: indexed
                .remove(&(checkpoint.root_id.clone(), checkpoint.root_generation))
                .unwrap_or_else(|| {
                    Err(reconstruction_failure(
                        "broker omitted a requested root outcome",
                        None,
                    ))
                }),
        })
        .collect()
}

fn translate_root_outcome(
    previous: &PersistentJournalCheckpoint,
    outcome: SharedJournalRootOutcome,
    requested_end_usn: i64,
    observed_unix_ms: i64,
) -> Result<RootPage, PersistentJournalReadFailure> {
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
    };
    batch.range.batch_id = persistent_journal_batch_id(&batch);
    batch
        .validate()
        .map_err(|error| reconstruction_from_error(error, Some(opening_boundary.clone())))?;
    checkpoint
        .validate()
        .map_err(|error| reconstruction_from_error(error, Some(opening_boundary)))?;
    Ok((batch, checkpoint))
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

fn recovery_opening_boundary(
    checkpoint: &PersistentJournalCheckpoint,
    journal_id: u64,
    next_usn: i64,
) -> Result<LibraryRecoveryOpeningBoundary, PersistentJournalReadFailure> {
    let boundary = LibraryRecoveryOpeningBoundary {
        volume: checkpoint.volume.clone(),
        root_file_reference: checkpoint.root_file_reference.clone(),
        journal_id: JournalIdentifier::new(journal_id)
            .map_err(|error| reconstruction_from_error(error, None))?,
        next_usn: JournalUsn::new(next_usn)
            .map_err(|error| reconstruction_from_error(error, None))?,
        protocol_version: checkpoint.protocol_version,
        contract_version: checkpoint.contract_version,
    };
    boundary
        .validate()
        .map_err(|error| reconstruction_from_error(error, None))?;
    Ok(boundary)
}

fn map_operation_failure(
    error: PersistentChangeJournalOperationError,
) -> PersistentJournalReadFailure {
    let message = error.to_string();
    match error {
        PersistentChangeJournalOperationError::Remote(code) => map_broker_failure_code(code, None),
        PersistentChangeJournalOperationError::Cancelled => cancelled_failure(&message),
        PersistentChangeJournalOperationError::ProtocolMismatch
        | PersistentChangeJournalOperationError::InvalidRequest => read_failure(
            PersistentJournalRootFailureKind::NonRecoverable,
            if matches!(
                error,
                PersistentChangeJournalOperationError::ProtocolMismatch
            ) {
                "persistent_journal_protocol_mismatch"
            } else {
                "persistent_journal_request_invalid"
            },
            &message,
            None,
        ),
        PersistentChangeJournalOperationError::CapacityExceeded
        | PersistentChangeJournalOperationError::RequestNotActive => read_failure(
            PersistentJournalRootFailureKind::Transient,
            if matches!(
                error,
                PersistentChangeJournalOperationError::CapacityExceeded
            ) {
                "persistent_journal_capacity_exceeded"
            } else {
                "persistent_journal_request_not_active"
            },
            &message,
            None,
        ),
        PersistentChangeJournalOperationError::ResponseIntegrity => {
            reconstruction_failure(&message, None)
        }
        PersistentChangeJournalOperationError::Closed
        | PersistentChangeJournalOperationError::CloseInProgress
        | PersistentChangeJournalOperationError::TransportUnavailable => {
            broker_after_current_failure(&message)
        }
    }
}

pub(crate) fn classify_persistent_journal_operation_failure(
    error: PersistentChangeJournalOperationError,
) -> PersistentJournalReadFailure {
    map_operation_failure(error)
}

fn map_broker_failure_code(
    code: BrokerFailureCode,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    let kind = match code {
        BrokerFailureCode::CallerRejected
        | BrokerFailureCode::RootUnauthorized
        | BrokerFailureCode::RootIdentityMismatch
        | BrokerFailureCode::VolumeMismatch => PersistentJournalRootFailureKind::ContainmentFailure,
        BrokerFailureCode::JournalDiscontinuous | BrokerFailureCode::RecordUnsupported => {
            PersistentJournalRootFailureKind::JournalReconstructionFailure
        }
        BrokerFailureCode::JournalUnavailable
        | BrokerFailureCode::BackendUnavailable
        | BrokerFailureCode::Internal => {
            PersistentJournalRootFailureKind::BrokerAfterCurrentFailure
        }
        BrokerFailureCode::EvidenceLimitExceeded
        | BrokerFailureCode::TimedOut
        | BrokerFailureCode::RequestNotActive
        | BrokerFailureCode::ActiveLimitExceeded
        | BrokerFailureCode::TerminalDeliveryBackpressure => {
            PersistentJournalRootFailureKind::Transient
        }
        BrokerFailureCode::Cancelled => PersistentJournalRootFailureKind::Cancelled,
        BrokerFailureCode::ProtocolMismatch
        | BrokerFailureCode::MalformedFrame
        | BrokerFailureCode::RequestTooLarge => PersistentJournalRootFailureKind::NonRecoverable,
    };
    read_failure(
        kind,
        remote_error_code(code),
        &format!("The persistent journal broker returned {code:?}"),
        opening_boundary,
    )
}

fn remote_error_code(code: BrokerFailureCode) -> &'static str {
    match code {
        BrokerFailureCode::RootUnauthorized | BrokerFailureCode::RootIdentityMismatch => {
            "persistent_journal_root_unauthorized"
        }
        BrokerFailureCode::VolumeMismatch => "persistent_journal_volume_mismatch",
        BrokerFailureCode::JournalDiscontinuous => "persistent_journal_discontinuous",
        BrokerFailureCode::EvidenceLimitExceeded => "persistent_journal_evidence_limit",
        BrokerFailureCode::Cancelled => "persistent_journal_cancelled",
        _ => "persistent_journal_broker_failure",
    }
}

fn read_failure(
    kind: PersistentJournalRootFailureKind,
    code: &str,
    message: &str,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    PersistentJournalReadFailure {
        kind,
        failure: PersistentJournalFailure {
            code: code.to_owned(),
            message: message.to_owned(),
        },
        opening_boundary: opening_boundary.map(Box::new),
    }
}

fn containment_failure(
    message: &str,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::ContainmentFailure,
        "persistent_journal_containment_failure",
        message,
        opening_boundary,
    )
}

fn reconstruction_failure(
    message: &str,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::JournalReconstructionFailure,
        "persistent_journal_reconstruction_failure",
        message,
        opening_boundary,
    )
}

fn broker_after_current_failure(message: &str) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
        "persistent_journal_broker_after_current_failure",
        message,
        None,
    )
}

fn nonrecoverable_failure(message: &str) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::NonRecoverable,
        "persistent_journal_nonrecoverable",
        message,
        None,
    )
}

fn cancelled_failure(message: &str) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::Cancelled,
        "persistent_journal_cancelled",
        message,
        None,
    )
}

fn reconstruction_from_error(
    error: ScanError,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::JournalReconstructionFailure,
        &error.code,
        &error.message,
        opening_boundary,
    )
}

fn nonrecoverable_from_error(error: ScanError) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::NonRecoverable,
        &error.code,
        &error.message,
        None,
    )
}

fn cancelled_from_error(error: ScanError) -> PersistentJournalReadFailure {
    read_failure(
        PersistentJournalRootFailureKind::Cancelled,
        &error.code,
        &error.message,
        None,
    )
}

fn invalid(message: &str) -> ScanError {
    ScanError::new("persistent_journal_session_invalid", message)
}

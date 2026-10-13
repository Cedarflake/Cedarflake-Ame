use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    JournalIdentifier, JournalUsn, LibraryRecoveryOpeningBoundary, PersistentJournalCheckpoint,
    PersistentJournalFailure, PersistentJournalPendingRename, PersistentJournalReadFailure,
    PersistentJournalRootFailureKind, PersistentJournalRootReadOutcome, ScanError,
};
use crate::journal_broker::{
    BrokerResponse, CallerClaim, JournalCapability, PersistentChangeJournalSession,
    QueryJournalRequest, RegisterRootRequest,
};
use crate::ports::{
    IdentityJournalRead, IdentityJournalRoot, PersistentIdentityJournalSource,
    PersistentJournalNamespaceReader, PersistentJournalVolumeReader, identity_for_reference,
};

use super::session_reader::PersistentJournalBrokerRoot;
#[cfg(test)]
mod tests;
mod translation;

pub(crate) struct IdentityPersistentJournalVolumeReader {
    pub(crate) session: Arc<dyn PersistentChangeJournalSession>,
    pub(crate) caller: CallerClaim,
    pub(crate) roots: Vec<PersistentJournalBrokerRoot>,
    pub(crate) source: Arc<dyn PersistentIdentityJournalSource>,
    pub(crate) namespace: Arc<dyn PersistentJournalNamespaceReader>,
    pub(super) read_windows: super::read_boundary::JournalReadWindows,
}

struct PreparedRoot<'a> {
    checkpoint: &'a PersistentJournalCheckpoint,
    root: IdentityJournalRoot,
    opening: LibraryRecoveryOpeningBoundary,
    read_end: i64,
}

impl IdentityPersistentJournalVolumeReader {
    fn prepare<'a>(
        &self,
        checkpoint: &'a PersistentJournalCheckpoint,
    ) -> Result<PreparedRoot<'a>, PersistentJournalReadFailure> {
        let registration = self
            .roots
            .iter()
            .find(|root| {
                root.authorization.root_id == checkpoint.root_id
                    && root.authorization.root_generation == checkpoint.root_generation.value()
            })
            .ok_or_else(|| {
                failure(
                    PersistentJournalRootFailureKind::ContainmentFailure,
                    "identity_journal_root_missing",
                    None,
                )
            })?;
        if registration.authorization.root_identity.as_slice()
            != checkpoint.root_file_reference.as_bytes()
            || registration.authorization.volume_id != checkpoint.volume.volume_guid
        {
            return Err(failure(
                PersistentJournalRootFailureKind::ContainmentFailure,
                "identity_journal_root_mismatch",
                None,
            ));
        }
        let capability = self
            .session
            .register_root(RegisterRootRequest {
                caller: self.caller.clone(),
                root: registration.authorization.clone(),
                client_root_handle: registration.client_root_handle,
                timeout_ms: 10_000,
            })
            .map_err(|_| {
                failure(
                    PersistentJournalRootFailureKind::Transient,
                    "identity_journal_registration_failed",
                    None,
                )
            })?;
        let root = IdentityJournalRoot {
            root_id: checkpoint.root_id.clone(),
            root_generation: checkpoint.root_generation,
        };
        let volume = self
            .source
            .volume_identity(&root)
            .map_err(|error| from_error(error, None))?;
        let response = self
            .session
            .query_journal(QueryJournalRequest {
                caller: self.caller.clone(),
                root: registration.authorization.clone(),
                root_capability: capability,
                timeout_ms: 10_000,
            })
            .map_err(|_| {
                failure(
                    PersistentJournalRootFailureKind::Transient,
                    "identity_journal_query_failed",
                    None,
                )
            })?;
        let BrokerResponse::Journal {
            binding,
            capability: JournalCapability::Supported,
            journal_id: Some(journal),
            first_usn: Some(first),
            next_usn: Some(next),
            ..
        } = response
        else {
            return Err(failure(
                PersistentJournalRootFailureKind::Transient,
                "identity_journal_unavailable",
                None,
            ));
        };
        let opening = LibraryRecoveryOpeningBoundary {
            volume,
            root_file_reference: checkpoint.root_file_reference.clone(),
            journal_id: JournalIdentifier::new(journal).map_err(|error| from_error(error, None))?,
            next_usn: JournalUsn::new(next).map_err(|error| from_error(error, None))?,
            protocol_version: checkpoint.protocol_version,
            contract_version: checkpoint.contract_version,
        };
        if binding.client_instance != self.caller.client_instance
            || binding.root_id != root.root_id
            || binding.root_generation != root.root_generation.value()
            || binding.volume_id != checkpoint.volume.volume_guid
            || opening.volume != checkpoint.volume
        {
            return Err(failure(
                PersistentJournalRootFailureKind::ContainmentFailure,
                "identity_journal_volume_mismatch",
                Some(&opening),
            ));
        }
        if journal != checkpoint.journal_id.value() {
            return Err(failure(
                PersistentJournalRootFailureKind::JournalReset,
                "identity_journal_reset",
                Some(&opening),
            ));
        }
        if first > checkpoint.next_unread_usn.value() {
            return Err(failure(
                PersistentJournalRootFailureKind::JournalTrim,
                "identity_journal_trim",
                Some(&opening),
            ));
        }
        if first < 0 || next < checkpoint.next_unread_usn.value() {
            return Err(failure(
                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                "identity_journal_regressed",
                Some(&opening),
            ));
        }
        if !self
            .namespace
            .has_snapshot(&root)
            .map_err(|error| from_error(error, Some(&opening)))?
        {
            return Err(failure(
                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                "identity_journal_namespace_baseline_required",
                Some(&opening),
            ));
        }
        let read_end = self
            .read_windows
            .end(checkpoint, next)
            .map_err(|error| from_error(error, Some(&opening)))?;
        Ok(PreparedRoot {
            checkpoint,
            root,
            opening,
            read_end,
        })
    }
}

impl PersistentJournalVolumeReader for IdentityPersistentJournalVolumeReader {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        let mut outcomes = Vec::new();
        let mut ready = Vec::new();
        for checkpoint in checkpoints {
            if cancelled.load(Ordering::Acquire) {
                return Err(failure(
                    PersistentJournalRootFailureKind::Cancelled,
                    "identity_journal_cancelled",
                    None,
                ));
            }
            match self.prepare(checkpoint) {
                Ok(root) => {
                    if pending.iter().any(|carry| {
                        carry.previous_root_id == checkpoint.root_id
                            && carry.previous_root_generation == checkpoint.root_generation
                    }) {
                        outcomes.push(outcome(
                            checkpoint,
                            Some(root.opening.clone()),
                            Err(failure(
                                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                                "identity_journal_previous_adapter_carry",
                                Some(&root.opening),
                            )),
                        ));
                    } else if self.read_windows.blocks_next_window(checkpoint) {
                        outcomes.push(outcome(checkpoint, Some(root.opening), Ok(None)));
                    } else {
                        ready.push(root);
                    }
                }
                Err(error) => outcomes.push(outcome(
                    checkpoint,
                    error.opening_boundary.as_deref().cloned(),
                    Err(error),
                )),
            }
        }
        if ready.is_empty() {
            return Ok(outcomes);
        }
        let end = ready
            .iter()
            .map(|root| root.read_end)
            .min()
            .expect("nonempty prepared roots");
        let mut admitted = Vec::new();
        for root in ready {
            if root.checkpoint.next_unread_usn.value() < end
                && self.read_windows.admits_shared_end(root.checkpoint, end)
            {
                admitted.push(root);
            } else {
                outcomes.push(outcome(root.checkpoint, Some(root.opening), Ok(None)));
            }
        }
        let ready = admitted;
        if ready.is_empty() {
            return Ok(outcomes);
        }
        let start = ready
            .iter()
            .map(|root| root.checkpoint.next_unread_usn.value())
            .min()
            .expect("nonempty admitted roots");
        let request = IdentityJournalRead {
            volume: ready[0].checkpoint.volume.clone(),
            journal_id: ready[0].checkpoint.journal_id.value(),
            start_usn: start,
            end_usn: end,
            max_records: 64,
        };
        let page = self
            .source
            .read_volume(&request, cancelled)
            .map_err(|error| from_error(error, Some(&ready[0].opening)))?;
        if page.covered_until_usn <= start
            || page.covered_until_usn > end
            || page.records.len() > 64
            || page
                .records
                .iter()
                .any(|record| record.usn < start || record.usn >= page.covered_until_usn)
            || page
                .records
                .windows(2)
                .any(|pair| pair[0].usn >= pair[1].usn)
        {
            return Err(failure(
                PersistentJournalRootFailureKind::JournalReconstructionFailure,
                "identity_journal_page_invalid",
                Some(&ready[0].opening),
            ));
        }
        let references: Vec<_> = page
            .records
            .iter()
            .flat_map(|record| {
                [
                    record.file_reference.clone(),
                    record.parent_reference.clone(),
                ]
            })
            .map(|reference| (reference.to_canonical_text(), reference))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        let identities: Vec<_> = references
            .iter()
            .map(|reference| identity_for_reference(request.volume.volume_serial, reference))
            .collect();
        let mut locations = HashMap::new();
        for root in ready {
            if root.checkpoint.next_unread_usn.value() >= page.covered_until_usn {
                outcomes.push(outcome(root.checkpoint, Some(root.opening), Ok(None)));
                continue;
            }
            let captured = (|| {
                let current = self
                    .source
                    .current_locations(&root.root, &references, cancelled)?;
                // Read historical hints after native resolution so a concurrent completed
                // inventory's catalog paths participate in the same enrolled final-state work.
                let previous = self.namespace.previous_locations(&root.root, &identities)?;
                let batch = translation::translate(
                    root.checkpoint,
                    &page,
                    end,
                    observed_unix_ms,
                    &previous,
                    &current,
                )?;
                Ok::<_, ScanError>((batch, previous, current))
            })();
            match captured {
                Ok((batch, previous, current)) => {
                    locations.insert(root.checkpoint.root_id.clone(), (previous, current));
                    outcomes.push(outcome(
                        root.checkpoint,
                        Some(root.opening),
                        Ok(Some(batch)),
                    ));
                }
                Err(error) => outcomes.push(outcome(
                    root.checkpoint,
                    Some(root.opening.clone()),
                    Err(from_error(error, Some(&root.opening))),
                )),
            }
        }
        translation::attach_handoffs(&mut outcomes, &page, &locations)
            .map_err(|error| from_error(error, None))?;
        if cancelled.load(Ordering::Acquire) {
            return Err(failure(
                PersistentJournalRootFailureKind::Cancelled,
                "identity_journal_cancelled",
                None,
            ));
        }
        Ok(outcomes)
    }
}

type RootPage = (
    crate::domain::PersistentJournalEnrollmentBatch,
    PersistentJournalCheckpoint,
);

fn outcome(
    checkpoint: &PersistentJournalCheckpoint,
    opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
    page: Result<Option<RootPage>, PersistentJournalReadFailure>,
) -> PersistentJournalRootReadOutcome {
    PersistentJournalRootReadOutcome {
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        opening_boundary,
        page,
    }
}

fn failure(
    kind: PersistentJournalRootFailureKind,
    code: &str,
    opening: Option<&LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    PersistentJournalReadFailure {
        kind,
        failure: PersistentJournalFailure {
            code: code.to_owned(),
            message: "The identity journal cannot prove complete root continuity".to_owned(),
        },
        opening_boundary: opening.cloned().map(Box::new),
    }
}

fn from_error(
    error: ScanError,
    opening: Option<&LibraryRecoveryOpeningBoundary>,
) -> PersistentJournalReadFailure {
    let kind = match error.code.as_str() {
        "identity_journal_cancelled" => PersistentJournalRootFailureKind::Cancelled,
        "identity_journal_discontinuous" => PersistentJournalRootFailureKind::JournalTrim,
        "identity_journal_identity_invalid"
        | "identity_journal_record_invalid"
        | "persistent_journal_baseline_read_window_invalid" => {
            PersistentJournalRootFailureKind::JournalReconstructionFailure
        }
        _ => PersistentJournalRootFailureKind::Transient,
    };
    PersistentJournalReadFailure {
        kind,
        failure: PersistentJournalFailure {
            code: error.code,
            message: error.message,
        },
        opening_boundary: opening.cloned().map(Box::new),
    }
}

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    AssetLocationView, LibraryRecoveryAuthorityReason, MetadataInventoryEntry,
    MetadataInventoryEntryKind, MetadataInventoryPlaceholderState, TerminalMediaEvidence,
};
use crate::ports::MediaInspector;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InventoryReusePolicy {
    ObservedRevision,
    WindowsRecovery,
    Reinspect,
}

impl InventoryReusePolicy {
    pub(super) const fn for_recovery(reason: LibraryRecoveryAuthorityReason) -> Self {
        match reason {
            LibraryRecoveryAuthorityReason::ExistingRootBaseline
            | LibraryRecoveryAuthorityReason::FirstImportBoundary => Self::ObservedRevision,
            LibraryRecoveryAuthorityReason::ContainmentFailure => Self::Reinspect,
            LibraryRecoveryAuthorityReason::JournalGap
            | LibraryRecoveryAuthorityReason::JournalReset
            | LibraryRecoveryAuthorityReason::JournalTrim
            | LibraryRecoveryAuthorityReason::JournalReconstructionFailure
            | LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
            | LibraryRecoveryAuthorityReason::WatcherUncoveredGap => Self::WindowsRecovery,
        }
    }

    fn allows_reuse(self, entry: &MetadataInventoryEntry) -> bool {
        match self {
            Self::ObservedRevision => true,
            Self::Reinspect => false,
            Self::WindowsRecovery => has_windows_recovery_evidence(entry),
        }
    }
}

pub(super) enum EntryComparison {
    Unchanged,
    Candidate { previous_path: Option<String> },
}

pub(super) fn compare_entry(
    entry: &MetadataInventoryEntry,
    path_prior: Option<&AssetLocationView>,
    terminal_media_evidence: Option<&TerminalMediaEvidence>,
    media_inspector: &impl MediaInspector,
    previous_paths: &BTreeMap<(String, String), String>,
    claimed_identities: &mut BTreeSet<(String, String)>,
    reuse: InventoryReusePolicy,
) -> EntryComparison {
    if entry.kind != MetadataInventoryEntryKind::File {
        return EntryComparison::Unchanged;
    }
    if reuse.allows_reuse(entry) {
        if path_prior.is_some_and(|prior| inventory_matches_location(entry, prior)) {
            return EntryComparison::Unchanged;
        }
        if terminal_media_evidence.is_some_and(|evidence| {
            inventory_matches_terminal_media_evidence(entry, evidence, media_inspector)
        }) {
            return EntryComparison::Unchanged;
        }
    }
    if path_prior.is_none()
        && let Some(identity) = entry.file_identity.as_ref()
    {
        let identity_key = (identity.scheme.clone(), identity.value.clone());
        if !claimed_identities.contains(&identity_key)
            && let Some(previous_path) = previous_paths.get(&identity_key)
        {
            claimed_identities.insert(identity_key);
            return EntryComparison::Candidate {
                previous_path: Some(previous_path.clone()),
            };
        }
    }
    EntryComparison::Candidate {
        previous_path: None,
    }
}

fn has_windows_recovery_evidence(entry: &MetadataInventoryEntry) -> bool {
    if entry.is_reparse_point
        || entry.placeholder_state != MetadataInventoryPlaceholderState::Available
    {
        return false;
    }
    let Some(identity) = entry.file_identity.as_ref() else {
        return false;
    };
    let Some(revision) = entry.source_revision.as_ref() else {
        return false;
    };
    let identity_bytes = identity.value.as_bytes();
    identity.scheme == "windows-file-id-128-v1"
        && identity_bytes.len() == 49
        && identity_bytes[16] == b':'
        && identity_bytes.iter().enumerate().all(|(index, byte)| {
            index == 16 || byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
        })
        && revision.scheme == "windows-file-change-time-100ns-v1"
        && revision.value.len() == 16
        && revision
            .value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && u64::from_str_radix(&revision.value, 16).is_ok_and(|value| value > 0)
}

pub(super) fn inventory_matches_terminal_media_evidence(
    entry: &MetadataInventoryEntry,
    evidence: &TerminalMediaEvidence,
    media_inspector: &impl MediaInspector,
) -> bool {
    entry.file_size == Some(evidence.file_size)
        && entry.modified_unix_ms == evidence.modified_unix_ms
        && entry.source_revision.is_some()
        && entry.source_revision == evidence.source_revision
        && entry.placeholder_state == MetadataInventoryPlaceholderState::Available
        && entry.file_identity == evidence.file_identity
        && evidence.inspection_engine_id == media_inspector.inspection_engine_id()
        && evidence.inspection_engine_version == media_inspector.inspection_engine_version()
}

pub(in super::super) fn inventory_matches_location(
    entry: &MetadataInventoryEntry,
    prior: &AssetLocationView,
) -> bool {
    if entry.file_size != Some(prior.file_size) || entry.modified_unix_ms != prior.modified_unix_ms
    {
        return false;
    }
    if entry.placeholder_state != MetadataInventoryPlaceholderState::Available {
        return entry.file_identity.is_none();
    }
    if entry.source_revision.is_none() || entry.source_revision != prior.source_revision {
        return false;
    }
    match (&entry.file_identity, &prior.file_identity) {
        (Some(current), Some(previous)) => current == previous,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests;

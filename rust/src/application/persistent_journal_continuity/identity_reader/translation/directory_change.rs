use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    FileIdentityEvidence, LibraryChangeIntent, PersistentJournalCheckpoint,
    PersistentJournalDirectoryChange, ScanError,
};
use crate::ports::{IdentityJournalRecord, IdentityJournalResolution};

use super::{IntentKey, add_intent, invalid};

pub(super) struct DirectoryNamespaceEvidence<'a> {
    pub(super) identity: FileIdentityEvidence,
    pub(super) previous_paths: &'a BTreeSet<String>,
    pub(super) current_paths: &'a BTreeSet<String>,
    pub(super) resolution: &'a IdentityJournalResolution,
}

pub(super) fn translate(
    checkpoint: &PersistentJournalCheckpoint,
    record: &IdentityJournalRecord,
    evidence: DirectoryNamespaceEvidence<'_>,
    intents: &mut BTreeMap<IntentKey, LibraryChangeIntent>,
    observed_unix_ms: i64,
) -> Result<Option<PersistentJournalDirectoryChange>, ScanError> {
    let DirectoryNamespaceEvidence {
        identity,
        previous_paths: old,
        current_paths: live,
        resolution: current,
    } = evidence;
    let paired = match (old.len(), live.len(), old.first(), live.first()) {
        (1, 1, Some(previous), Some(path))
            if previous != path && !previous.is_empty() && !path.is_empty() =>
        {
            Some((previous.as_str(), path.as_str()))
        }
        _ => None,
    };
    if let Some((previous, path)) = paired {
        add_intent(
            intents,
            checkpoint,
            path,
            Some(previous),
            true,
            record.usn,
            observed_unix_ms,
        )?;
    } else {
        for path in old.union(live).filter(|path| !path.is_empty()) {
            add_intent(
                intents,
                checkpoint,
                path,
                None,
                true,
                record.usn,
                observed_unix_ms,
            )?;
        }
    }
    let unresolved = current
        .access_denied
        .iter()
        .chain(&current.unresolved)
        .any(|unresolved| unresolved == &identity);
    if unresolved && !record.is_deleted {
        return Ok(None);
    }
    if live.len() > 1 {
        return Err(invalid());
    }
    let previous_relative_paths: Vec<_> = old
        .iter()
        .filter(|path| !path.is_empty())
        .cloned()
        .collect();
    let current_relative_path = live.iter().find(|path| !path.is_empty()).cloned();
    if previous_relative_paths.is_empty() && current_relative_path.is_none() {
        return Ok(None);
    }
    Ok(Some(PersistentJournalDirectoryChange {
        identity,
        previous_relative_paths,
        current_relative_path,
    }))
}

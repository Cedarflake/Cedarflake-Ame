use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::domain::{
    JournalUsn, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeScope, PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalCrossRootLineage, PersistentJournalEnrollmentBatch,
    PersistentJournalLineageState, PersistentJournalRangeState, PersistentJournalReadPage,
    PersistentJournalRootReadOutcome, PersistentJournalSourceRange, ScanError,
    persistent_journal_batch_id,
};
use crate::ports::{
    IdentityJournalLocation, IdentityJournalPage, IdentityJournalResolution, identity_for_reference,
};

mod directory_change;

type IntentKey = (u8, String, Option<String>);
type Locations = HashMap<String, (Vec<IdentityJournalLocation>, IdentityJournalResolution)>;

pub(super) fn translate(
    previous_checkpoint: &PersistentJournalCheckpoint,
    page: &IdentityJournalPage,
    requested_end: i64,
    observed_unix_ms: i64,
    previous: &[IdentityJournalLocation],
    current: &IdentityJournalResolution,
) -> Result<
    (
        PersistentJournalEnrollmentBatch,
        PersistentJournalCheckpoint,
    ),
    ScanError,
> {
    let mut intents = BTreeMap::<IntentKey, LibraryChangeIntent>::new();
    let mut directory_changes = BTreeMap::new();
    for record in page
        .records
        .iter()
        .filter(|record| record.usn >= previous_checkpoint.next_unread_usn.value())
    {
        let identity = identity_for_reference(
            previous_checkpoint.volume.volume_serial,
            &record.file_reference,
        );
        let old = paths(previous, &identity.value, record.is_directory);
        let live = paths(&current.locations, &identity.value, record.is_directory);
        if current
            .access_denied
            .iter()
            .any(|denied| denied == &identity)
            && old.is_empty()
        {
            let parent = identity_for_reference(
                previous_checkpoint.volume.volume_serial,
                &record.parent_reference,
            );
            for relative_path in paths(previous, &parent.value, true).union(&paths(
                &current.locations,
                &parent.value,
                true,
            )) {
                add_intent(
                    &mut intents,
                    previous_checkpoint,
                    relative_path,
                    None,
                    true,
                    record.usn,
                    observed_unix_ms,
                )?;
            }
            continue;
        }
        if record.is_directory {
            let evidence = directory_change::DirectoryNamespaceEvidence {
                identity,
                previous_paths: &old,
                current_paths: &live,
                resolution: current,
            };
            if let Some(change) = directory_change::translate(
                previous_checkpoint,
                record,
                evidence,
                &mut intents,
                observed_unix_ms,
            )? {
                directory_changes.insert(change.identity.value.clone(), change);
            }
            continue;
        }
        let removed: Vec<_> = old.difference(&live).collect();
        let added: Vec<_> = live.difference(&old).collect();
        let paired = match (removed.as_slice(), added.as_slice()) {
            ([old], [new]) => Some((old.as_str(), new.as_str())),
            _ => None,
        };
        if let Some((old, new)) = paired {
            add_intent(
                &mut intents,
                previous_checkpoint,
                new,
                Some(old),
                false,
                record.usn,
                observed_unix_ms,
            )?;
        }
        for path in old.union(&live) {
            if paired.is_some_and(|(old, new)| path == old || path == new) {
                continue;
            }
            add_intent(
                &mut intents,
                previous_checkpoint,
                path,
                None,
                false,
                record.usn,
                observed_unix_ms,
            )?;
        }
    }
    if intents.len() > crate::domain::persistent_journal::MAX_PERSISTENT_JOURNAL_BATCH_INTENTS {
        return Err(invalid());
    }
    let mut batch = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "pending-content-id".to_owned(),
            root_id: previous_checkpoint.root_id.clone(),
            root_generation: previous_checkpoint.root_generation,
            volume: previous_checkpoint.volume.clone(),
            journal_id: previous_checkpoint.journal_id,
            requested_start_usn: previous_checkpoint.next_unread_usn,
            requested_end_usn: JournalUsn::new(requested_end)?,
            covered_until_usn: JournalUsn::new(page.covered_until_usn)?,
            is_complete: page.covered_until_usn == requested_end,
            protocol_version: previous_checkpoint.protocol_version,
            contract_version: previous_checkpoint.contract_version,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: observed_unix_ms,
            checkpointed_unix_ms: None,
        },
        intents: intents.into_values().collect(),
        directory_changes: directory_changes.into_values().collect(),
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    batch.range.batch_id = persistent_journal_batch_id(&batch);
    batch.validate()?;
    let mut checkpoint = previous_checkpoint.clone();
    checkpoint.next_unread_usn = batch.range.covered_until_usn;
    checkpoint.captured_exclusive_end = batch.range.requested_end_usn;
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    checkpoint.failure = None;
    checkpoint.updated_unix_ms = observed_unix_ms;
    checkpoint.validate()?;
    Ok((batch, checkpoint))
}

fn paths(
    locations: &[IdentityJournalLocation],
    identity: &str,
    is_directory: bool,
) -> BTreeSet<String> {
    locations
        .iter()
        .filter(|location| {
            location.identity.value == identity && location.is_directory == is_directory
        })
        .map(|location| location.relative_path.clone())
        .collect()
}

fn add_intent(
    intents: &mut BTreeMap<IntentKey, LibraryChangeIntent>,
    checkpoint: &PersistentJournalCheckpoint,
    path: &str,
    previous: Option<&str>,
    directory: bool,
    usn: i64,
    observed: i64,
) -> Result<(), ScanError> {
    let scope = if path.is_empty() {
        LibraryChangeScope::Root
    } else if directory {
        LibraryChangeScope::Subtree
    } else {
        LibraryChangeScope::Path
    };
    let key = (
        u8::from(directory),
        path.to_owned(),
        previous.map(str::to_owned),
    );
    let sequence = u64::try_from(usn).map_err(|_| invalid())?;
    if let Some(existing) = intents.get_mut(&key) {
        existing.most_recent_sequence = sequence;
        existing.coalesced_observation_count =
            existing.coalesced_observation_count.saturating_add(1);
    } else {
        intents.insert(
            key,
            LibraryChangeIntent {
                root_id: checkpoint.root_id.clone(),
                root_generation: checkpoint.root_generation,
                kind: if previous.is_some() {
                    LibraryChangeIntentKind::RenameCandidate
                } else {
                    LibraryChangeIntentKind::Reconcile
                },
                scope,
                relative_path: path.to_owned(),
                previous_relative_path: previous.map(str::to_owned),
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: observed,
                most_recent_observed_unix_ms: observed,
                first_sequence: sequence,
                most_recent_sequence: sequence,
                coalesced_observation_count: 1,
            },
        );
    }
    Ok(())
}

pub(super) fn attach_handoffs(
    outcomes: &mut [PersistentJournalRootReadOutcome],
    page: &IdentityJournalPage,
    locations: &Locations,
) -> Result<(), ScanError> {
    let mut references = BTreeMap::new();
    for record in &page.records {
        references.insert(record.file_reference.to_canonical_text(), record);
    }
    for record in references.into_values() {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        for (index, outcome) in outcomes.iter().enumerate() {
            let Ok(Some(PersistentJournalReadPage::Enrolled { batch, .. })) = &outcome.page else {
                continue;
            };
            if record.usn < batch.range.requested_start_usn.value() {
                continue;
            }
            let Some((previous, current)) = locations.get(&outcome.root_id) else {
                continue;
            };
            let identity =
                identity_for_reference(batch.range.volume.volume_serial, &record.file_reference);
            let old = paths(previous, &identity.value, record.is_directory);
            let live = paths(&current.locations, &identity.value, record.is_directory);
            removed.extend(
                old.difference(&live)
                    .filter(|path| !path.is_empty())
                    .map(|path| (index, path.clone())),
            );
            added.extend(
                live.difference(&old)
                    .filter(|path| !path.is_empty())
                    .map(|path| (index, path.clone())),
            );
        }
        let ([(old_index, old_path)], [(new_index, new_path)]) =
            (removed.as_slice(), added.as_slice())
        else {
            continue;
        };
        if old_index == new_index {
            continue;
        }
        let previous_root = (
            outcomes[*old_index].root_id.clone(),
            outcomes[*old_index].root_generation,
        );
        let current_root = (
            outcomes[*new_index].root_id.clone(),
            outcomes[*new_index].root_generation,
        );
        let id = crate::application::scan_library::stable_id(
            "identity-journal-handoff-v1",
            &format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                record.file_reference.to_canonical_text(),
                record.usn,
                previous_root.0,
                previous_root.1.value(),
                old_path,
                current_root.0,
                current_root.1.value(),
                new_path
            ),
        );
        for index in [*old_index, *new_index] {
            let Ok(Some(PersistentJournalReadPage::Enrolled { batch, .. })) =
                &mut outcomes[index].page
            else {
                return Err(invalid());
            };
            batch
                .cross_root_lineage
                .push(PersistentJournalCrossRootLineage {
                    lineage_id: id.clone(),
                    owner_source_range_id: batch.range.batch_id.clone(),
                    volume: batch.range.volume.clone(),
                    journal_id: batch.range.journal_id,
                    file_reference: record.file_reference.clone(),
                    old_usn: None,
                    new_usn: None,
                    previous_carry_id: None,
                    previous_root_id: previous_root.0.clone(),
                    previous_root_generation: previous_root.1,
                    previous_relative_path: old_path.clone(),
                    current_root_id: current_root.0.clone(),
                    current_root_generation: current_root.1,
                    current_relative_path: new_path.clone(),
                    state: PersistentJournalLineageState::Pending,
                });
        }
    }
    for outcome in outcomes {
        if let Ok(Some(PersistentJournalReadPage::Enrolled { batch, .. })) = &mut outcome.page {
            let previous = batch.range.batch_id.clone();
            let current = persistent_journal_batch_id(batch);
            for lineage in &mut batch.cross_root_lineage {
                if lineage.owner_source_range_id == previous {
                    lineage.owner_source_range_id.clone_from(&current);
                }
            }
            batch.range.batch_id = current;
            batch.validate()?;
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "identity_journal_translation_invalid",
        "Identity journal reconstruction exceeded its verified ownership or resource bounds",
    )
}

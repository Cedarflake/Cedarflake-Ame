use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    LibraryChangeQueuePolicy, PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalCrossRootLineage, PersistentJournalEnrollmentBatch,
    PersistentJournalEnrollmentReport, PersistentJournalFailure, PersistentJournalPendingRename,
    PersistentJournalReadFailure, PersistentJournalRootFailure, PersistentJournalRootFailureKind,
    PersistentJournalVolumeBatch, PersistentJournalVolumePage, ScanError,
};
use crate::ports::{PersistentJournalRepository, PersistentJournalVolumeReader};

mod session_reader;

pub(crate) use session_reader::{
    PersistentJournalBrokerRoot, SessionBackedPersistentJournalVolumeReader,
    classify_persistent_journal_operation_failure,
};

pub(crate) fn catch_up_persistent_journal_volume<Repository, Reader>(
    repository: &mut Repository,
    reader: &Reader,
    checkpoints: &[PersistentJournalCheckpoint],
    observed_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<PersistentJournalEnrollmentReport, ScanError>
where
    Repository: PersistentJournalRepository,
    Reader: PersistentJournalVolumeReader,
{
    validate_checkpoint_set(checkpoints)?;
    let capabilities = repository.load_persistent_journal_capabilities()?;
    if checkpoints.iter().any(|checkpoint| {
        !capabilities.iter().any(|capability| {
            capability.root_id == checkpoint.root_id
                && capability.root_generation == checkpoint.root_generation
                && capability.state == PersistentJournalCapabilityState::Supported
        })
    }) {
        return Err(invalid_page(
            "Every requested root must own supported persistent journal authority",
        ));
    }
    let initial = checkpoints
        .iter()
        .map(|checkpoint| {
            (
                (checkpoint.root_id.as_str(), checkpoint.root_generation),
                checkpoint,
            )
        })
        .collect::<HashMap<_, _>>();
    let pending_renames = repository.load_persistent_journal_pending_renames(
        &checkpoints[0].volume,
        checkpoints[0].journal_id,
    )?;
    let pages = match reader.read_volume(checkpoints, &pending_renames, observed_unix_ms, cancelled)
    {
        Ok(pages) => pages,
        Err(failure) => {
            let mut report = PersistentJournalEnrollmentReport::default();
            for checkpoint in checkpoints {
                record_root_failure(
                    repository,
                    &mut report,
                    checkpoint,
                    failure.clone(),
                    observed_unix_ms,
                    policy,
                )?;
            }
            return Ok(report);
        }
    };
    let mut seen_roots = HashSet::with_capacity(pages.len());
    let mut report = PersistentJournalEnrollmentReport::default();
    let mut publishable = Vec::new();
    for outcome in pages {
        if cancelled.load(Ordering::Acquire) {
            return Ok(report);
        }
        let outcome_key = (outcome.root_id.as_str(), outcome.root_generation);
        if !initial.contains_key(&outcome_key) {
            report.failed_root_count = report.failed_root_count.saturating_add(1);
            continue;
        }
        if !seen_roots.insert((outcome.root_id.clone(), outcome.root_generation)) {
            let checkpoint = initial[&outcome_key];
            record_root_failure(
                repository,
                &mut report,
                checkpoint,
                reconstruction_failure(
                    "persistent_journal_reader_duplicate_root",
                    "The journal reader returned a root more than once",
                ),
                observed_unix_ms,
                policy,
            )?;
            continue;
        }
        if let Some(boundary) = &outcome.opening_boundary {
            repository.attach_persistent_journal_recovery_opening_boundary(
                &outcome.root_id,
                outcome.root_generation,
                boundary,
                observed_unix_ms,
            )?;
        }
        let (batch, checkpoint) = match outcome.page {
            Ok(Some(page)) => page,
            Ok(None) => continue,
            Err(failure) => {
                record_root_failure(
                    repository,
                    &mut report,
                    initial[&outcome_key],
                    failure,
                    observed_unix_ms,
                    policy,
                )?;
                continue;
            }
        };
        let root_key = (batch.range.root_id.as_str(), batch.range.root_generation);
        let Some(previous) = initial.get(&root_key).copied() else {
            report.failed_root_count = report.failed_root_count.saturating_add(1);
            continue;
        };
        if root_key != outcome_key {
            record_root_failure(
                repository,
                &mut report,
                previous,
                reconstruction_failure(
                    "persistent_journal_reader_root_mismatch",
                    "The journal page owner does not match its requested root",
                ),
                observed_unix_ms,
                policy,
            )?;
            continue;
        }
        if let Err(error) = validate_page(previous, &batch, &checkpoint) {
            record_root_failure(
                repository,
                &mut report,
                previous,
                PersistentJournalReadFailure {
                    kind: PersistentJournalRootFailureKind::JournalReconstructionFailure,
                    failure: PersistentJournalFailure {
                        code: error.code,
                        message: error.message,
                    },
                    opening_boundary: None,
                },
                observed_unix_ms,
                policy,
            )?;
            continue;
        }
        publishable.push(PersistentJournalVolumePage {
            enrollment: batch,
            checkpoint,
        });
    }
    for checkpoint in checkpoints {
        if !seen_roots.contains(&(checkpoint.root_id.clone(), checkpoint.root_generation)) {
            record_root_failure(
                repository,
                &mut report,
                checkpoint,
                reconstruction_failure(
                    "persistent_journal_reader_omitted_root",
                    "The journal reader omitted a requested root",
                ),
                observed_unix_ms,
                policy,
            )?;
        }
    }
    if !publishable.is_empty() {
        let volume_batch = PersistentJournalVolumeBatch { pages: publishable };
        if validate_volume_batch(&volume_batch, &pending_renames).is_err() {
            for page in &volume_batch.pages {
                let previous = initial[&(
                    page.checkpoint.root_id.as_str(),
                    page.checkpoint.root_generation,
                )];
                record_root_failure(
                    repository,
                    &mut report,
                    previous,
                    reconstruction_failure(
                        "persistent_journal_volume_reconstruction_invalid",
                        "The shared journal volume page set could not be reconstructed",
                    ),
                    observed_unix_ms,
                    policy,
                )?;
            }
        } else {
            for component in partition_atomic_volume_components(&volume_batch) {
                if cancelled.load(Ordering::Acquire) {
                    return Ok(report);
                }
                match repository.publish_persistent_journal_volume_batch(
                    &component,
                    observed_unix_ms,
                    policy,
                ) {
                    Ok(publication) => {
                        report.enrolled_root_count = report
                            .enrolled_root_count
                            .saturating_add(publication.enrolled_root_count);
                        report.observation_count = report
                            .observation_count
                            .saturating_add(publication.observation_count);
                        report.advanced_checkpoint_count = report
                            .advanced_checkpoint_count
                            .saturating_add(publication.advanced_checkpoint_count);
                    }
                    Err(error) => {
                        for page in &component.pages {
                            let previous = initial[&(
                                page.checkpoint.root_id.as_str(),
                                page.checkpoint.root_generation,
                            )];
                            record_root_failure(
                                repository,
                                &mut report,
                                previous,
                                PersistentJournalReadFailure {
                                    kind: PersistentJournalRootFailureKind::Transient,
                                    failure: PersistentJournalFailure {
                                        code: error.code.clone(),
                                        message: error.message.clone(),
                                    },
                                    opening_boundary: None,
                                },
                                observed_unix_ms,
                                policy,
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}

fn partition_atomic_volume_components(
    batch: &PersistentJournalVolumeBatch,
) -> Vec<PersistentJournalVolumeBatch> {
    let mut parents = (0..batch.pages.len()).collect::<Vec<_>>();
    let mut lineage_owner = HashMap::<&str, usize>::new();
    for (index, page) in batch.pages.iter().enumerate() {
        for endpoint in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            if let Some(owner) = lineage_owner.insert(endpoint.lineage_id.as_str(), index) {
                union_components(&mut parents, owner, index);
            }
        }
    }
    let mut groups = Vec::<(usize, Vec<PersistentJournalVolumePage>)>::new();
    for (index, page) in batch.pages.iter().cloned().enumerate() {
        let root = find_component(&mut parents, index);
        if let Some((_, pages)) = groups.iter_mut().find(|(owner, _)| *owner == root) {
            pages.push(page);
        } else {
            groups.push((root, vec![page]));
        }
    }
    groups
        .into_iter()
        .map(|(_, pages)| PersistentJournalVolumeBatch { pages })
        .collect()
}

fn union_components(parents: &mut [usize], left: usize, right: usize) {
    let left = find_component(parents, left);
    let right = find_component(parents, right);
    if left != right {
        parents[right] = left;
    }
}

fn find_component(parents: &mut [usize], mut index: usize) -> usize {
    let mut root = index;
    while parents[root] != root {
        root = parents[root];
    }
    while parents[index] != index {
        let next = parents[index];
        parents[index] = root;
        index = next;
    }
    root
}

fn record_root_failure<Repository: PersistentJournalRepository>(
    repository: &mut Repository,
    report: &mut PersistentJournalEnrollmentReport,
    checkpoint: &PersistentJournalCheckpoint,
    failure: PersistentJournalReadFailure,
    failed_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    failure.validate()?;
    let failure = PersistentJournalRootFailure {
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        kind: failure.kind,
        failure: failure.failure,
        opening_boundary: failure.opening_boundary.map(|boundary| *boundary),
    };
    failure.validate()?;
    if failure.kind != PersistentJournalRootFailureKind::Cancelled {
        repository.persist_persistent_journal_root_failure(&failure, failed_unix_ms, policy)?;
    }
    report.failed_root_count = report.failed_root_count.saturating_add(1);
    report.root_failures.push(failure);
    Ok(())
}

fn reconstruction_failure(code: &str, message: &str) -> PersistentJournalReadFailure {
    PersistentJournalReadFailure {
        kind: PersistentJournalRootFailureKind::JournalReconstructionFailure,
        failure: PersistentJournalFailure {
            code: code.to_owned(),
            message: message.to_owned(),
        },
        opening_boundary: None,
    }
}

fn validate_volume_batch(
    batch: &PersistentJournalVolumeBatch,
    pending_renames: &[PersistentJournalPendingRename],
) -> Result<(), ScanError> {
    batch.validate()?;
    let ranges = batch
        .pages
        .iter()
        .map(|page| {
            (
                page.enrollment.range.batch_id.as_str(),
                &page.enrollment.range,
            )
        })
        .collect::<HashMap<_, _>>();
    let pending = pending_renames
        .iter()
        .map(|carry| (carry.carry_id.as_str(), carry))
        .collect::<HashMap<_, _>>();
    let mut consumed = HashSet::new();
    let mut lineage = HashMap::<&str, Vec<&PersistentJournalCrossRootLineage>>::new();
    let mut carried_owner_counts = HashMap::<&str, usize>::new();
    for page in &batch.pages {
        for carry_id in &page.enrollment.consumed_pending_rename_ids {
            if !pending.contains_key(carry_id.as_str()) || !consumed.insert(carry_id.as_str()) {
                return Err(invalid_page(
                    "A pending rename carry was consumed without unique durable ownership",
                ));
            }
        }
        for endpoint in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            if let Some(carry_id) = endpoint.previous_carry_id.as_deref() {
                *carried_owner_counts.entry(carry_id).or_default() += 1;
            }
            lineage
                .entry(endpoint.lineage_id.as_str())
                .or_default()
                .push(endpoint);
        }
    }
    for endpoints in lineage.values() {
        let [first, second] = endpoints.as_slice() else {
            return Err(invalid_page(
                "A cross-root handoff must publish both durable owners together",
            ));
        };
        if first.owner_source_range_id == second.owner_source_range_id
            || !same_lineage(first, second)
        {
            return Err(invalid_page(
                "A cross-root handoff contains mismatched or duplicated owners",
            ));
        }
        if let Some(carry_id) = first.previous_carry_id.as_deref() {
            let carry = pending.get(carry_id).copied().ok_or_else(|| {
                invalid_page("A carried handoff has no durable pending rename predecessor")
            })?;
            if !consumed.contains(carry_id)
                || first.volume != carry.volume
                || first.journal_id != carry.journal_id
                || first.file_reference != carry.file_reference
                || first.old_usn != Some(carry.old_usn)
                || first.previous_root_id != carry.previous_root_id
                || first.previous_root_generation != carry.previous_root_generation
                || first.previous_relative_path != carry.previous_relative_path
            {
                return Err(invalid_page(
                    "A consumed carry does not exactly match its carried handoff",
                ));
            }
            let new_usn = first
                .new_usn
                .ok_or_else(|| invalid_page("A carried handoff is missing its NEW USN"))?;
            let previous_owner_count = [first, second]
                .into_iter()
                .filter(|endpoint| endpoint.owner_source_range_id == carry.source_range_id)
                .count();
            let current_owner_count = [first, second]
                .into_iter()
                .filter(|endpoint| {
                    ranges
                        .get(endpoint.owner_source_range_id.as_str())
                        .is_some_and(|range| {
                            endpoint.current_root_id == range.root_id
                                && endpoint.current_root_generation == range.root_generation
                                && new_usn >= range.requested_start_usn
                                && new_usn < range.covered_until_usn
                        })
                })
                .count();
            if previous_owner_count != 1 || current_owner_count != 1 {
                return Err(invalid_page(
                    "A consumed carry must publish one exact previous and current owner",
                ));
            }
        }
        for endpoint in [*first, *second] {
            let expected = ranges
                .get(endpoint.owner_source_range_id.as_str())
                .map(|range| (range.root_id.as_str(), range.root_generation))
                .or_else(|| {
                    pending_renames
                        .iter()
                        .find(|carry| carry.source_range_id == endpoint.owner_source_range_id)
                        .map(|carry| {
                            (
                                carry.previous_root_id.as_str(),
                                carry.previous_root_generation,
                            )
                        })
                })
                .ok_or_else(|| invalid_page("A handoff owner has no durable source range"))?;
            let is_previous = expected
                == (
                    endpoint.previous_root_id.as_str(),
                    endpoint.previous_root_generation,
                );
            let is_current = expected
                == (
                    endpoint.current_root_id.as_str(),
                    endpoint.current_root_generation,
                );
            if !is_previous && !is_current {
                return Err(invalid_page(
                    "A handoff owner does not match either endpoint authority",
                ));
            }
        }
    }
    if carried_owner_counts.len() != consumed.len()
        || carried_owner_counts
            .iter()
            .any(|(carry_id, owner_count)| *owner_count != 2 || !consumed.contains(carry_id))
    {
        return Err(invalid_page(
            "Consumed pending renames and carried owners are not one-to-one",
        ));
    }
    Ok(())
}

fn same_lineage(
    left: &PersistentJournalCrossRootLineage,
    right: &PersistentJournalCrossRootLineage,
) -> bool {
    left.lineage_id == right.lineage_id
        && left.volume == right.volume
        && left.journal_id == right.journal_id
        && left.file_reference == right.file_reference
        && left.old_usn == right.old_usn
        && left.new_usn == right.new_usn
        && left.previous_carry_id == right.previous_carry_id
        && left.previous_root_id == right.previous_root_id
        && left.previous_root_generation == right.previous_root_generation
        && left.previous_relative_path == right.previous_relative_path
        && left.current_root_id == right.current_root_id
        && left.current_root_generation == right.current_root_generation
        && left.current_relative_path == right.current_relative_path
        && left.state == right.state
}

fn validate_checkpoint_set(checkpoints: &[PersistentJournalCheckpoint]) -> Result<(), ScanError> {
    let Some(first) = checkpoints.first() else {
        return Err(invalid_page(
            "A volume read requires at least one root checkpoint",
        ));
    };
    let mut roots = HashSet::with_capacity(checkpoints.len());
    for checkpoint in checkpoints {
        checkpoint.validate()?;
        if checkpoint.volume != first.volume
            || checkpoint.journal_id != first.journal_id
            || !roots.insert((checkpoint.root_id.as_str(), checkpoint.root_generation))
        {
            return Err(invalid_page(
                "A physical volume read must contain unique roots from one journal",
            ));
        }
    }
    Ok(())
}

fn validate_page(
    previous: &PersistentJournalCheckpoint,
    batch: &PersistentJournalEnrollmentBatch,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    batch.validate()?;
    checkpoint.validate()?;
    if batch.range.root_id != checkpoint.root_id
        || batch.range.root_generation != checkpoint.root_generation
        || batch.range.volume != checkpoint.volume
        || batch.range.journal_id != checkpoint.journal_id
        || batch.range.requested_start_usn != previous.next_unread_usn
        || batch.range.requested_end_usn != checkpoint.captured_exclusive_end
        || batch.range.covered_until_usn != checkpoint.next_unread_usn
        || checkpoint.root_file_reference != previous.root_file_reference
        || checkpoint.volume != previous.volume
        || checkpoint.journal_id != previous.journal_id
        || batch.range.protocol_version != previous.protocol_version
        || checkpoint.protocol_version != previous.protocol_version
        || batch.range.contract_version != previous.contract_version
        || checkpoint.contract_version != previous.contract_version
        || checkpoint.next_unread_usn < previous.next_unread_usn
    {
        return Err(invalid_page(
            "The volume reader returned a root page with a gap or mismatched identity",
        ));
    }
    Ok(())
}

fn invalid_page(message: &str) -> ScanError {
    ScanError::new("persistent_journal_volume_page_invalid", message)
}

#[cfg(test)]
mod tests;

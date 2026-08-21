use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::LocalMetadataInventory;
use crate::domain::{
    AssetLocationView, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, MetadataInventoryComparisonStatus,
    MetadataInventoryComparisonUpdate, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, MetadataInventoryReport, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MetadataInventoryRepository,
    MetadataInventorySource,
};

const MAX_INVENTORY_PAGE_ENTRIES: u32 = 4_096;

pub fn run_local_metadata_inventory<Repository>(
    repository: &mut Repository,
    request: &MetadataInventoryRunRequest,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &AtomicBool,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    let root = repository
        .load_incremental_catalog_root(&request.root_id)?
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_root_missing",
                "The metadata inventory root is no longer registered",
            )
        })?;
    if root.root_generation != request.root_generation {
        return Err(ScanError::new(
            "metadata_inventory_root_stale",
            "The metadata inventory root generation changed",
        ));
    }
    let mut source = LocalMetadataInventory::new(&root.root_path, &request.scope)?;
    run_metadata_inventory(
        repository,
        &mut source,
        request,
        observed_unix_ms,
        page_limit,
        queue_policy,
        cancellation,
    )
}

fn run_metadata_inventory<Repository, Source>(
    repository: &mut Repository,
    source: &mut Source,
    request: &MetadataInventoryRunRequest,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &AtomicBool,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
    Source: MetadataInventorySource,
{
    validate_request(request, page_limit, queue_policy)?;
    repository.begin_metadata_inventory(request)?;
    let result = run_started_metadata_inventory(
        repository,
        source,
        request,
        observed_unix_ms,
        page_limit,
        queue_policy,
        cancellation,
    );
    if let Err(error) = &result {
        terminate_failed(repository, &request.run_id, error, observed_unix_ms);
    }
    result
}

fn run_started_metadata_inventory<Repository, Source>(
    repository: &mut Repository,
    source: &mut Source,
    request: &MetadataInventoryRunRequest,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &AtomicBool,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
    Source: MetadataInventorySource,
{
    let mut report = MetadataInventoryReport::default();
    loop {
        if cancellation.load(Ordering::Relaxed) {
            terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
            report.is_cancelled = true;
            return Ok(report);
        }
        let page = match source.next_page(page_limit, cancellation) {
            Ok(page) => page,
            Err(error) if error.code == "metadata_inventory_cancelled" => {
                terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
                report.is_cancelled = true;
                return Ok(report);
            }
            Err(error) => return Err(error),
        };
        let is_complete = page.is_complete;
        let run =
            repository.stage_metadata_inventory_page(&request.run_id, &page, observed_unix_ms)?;
        report.staged_entry_count = run.staged_entry_count;
        if is_complete {
            break;
        }
    }
    if cancellation.load(Ordering::Relaxed) {
        terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
        report.is_cancelled = true;
        return Ok(report);
    }
    repository.authorize_metadata_inventory_absence(&request.run_id, observed_unix_ms)?;

    let mut sequence = 1_u64;
    let mut claimed_identities = BTreeSet::new();
    loop {
        if cancellation.load(Ordering::Relaxed) {
            terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
            report.is_cancelled = true;
            return Ok(report);
        }
        let entries =
            repository.load_pending_metadata_inventory_entries(&request.run_id, page_limit)?;
        if entries.is_empty() {
            break;
        }
        let mut intents = Vec::new();
        let mut updates = Vec::with_capacity(entries.len());
        for entry in entries {
            let comparison = compare_entry(repository, request, &entry, &mut claimed_identities)?;
            match comparison {
                EntryComparison::Unchanged => {
                    report.unchanged_count = checked_add(
                        report.unchanged_count,
                        1,
                        "metadata inventory unchanged count",
                    )?;
                    updates.push(MetadataInventoryComparisonUpdate {
                        relative_path: entry.relative_path,
                        status: MetadataInventoryComparisonStatus::Unchanged,
                        candidate_previous_relative_path: None,
                    });
                }
                EntryComparison::Candidate { previous_path } => {
                    let intent = candidate_intent(
                        request,
                        &entry.relative_path,
                        previous_path.as_deref(),
                        observed_unix_ms,
                        sequence,
                    );
                    sequence = sequence.checked_add(1).ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_sequence_overflow",
                            "The metadata inventory candidate sequence overflowed",
                        )
                    })?;
                    intents.push(intent);
                    updates.push(MetadataInventoryComparisonUpdate {
                        relative_path: entry.relative_path,
                        status: MetadataInventoryComparisonStatus::Enqueued,
                        candidate_previous_relative_path: previous_path,
                    });
                }
            }
        }
        enqueue_candidates(
            repository,
            &intents,
            observed_unix_ms,
            queue_policy,
            &mut report,
        )?;
        let run = repository.record_metadata_inventory_comparisons(
            &request.run_id,
            &updates,
            observed_unix_ms,
        )?;
        report.candidate_count = run.candidate_count;
    }

    let mut absence_cursor = None;
    loop {
        if cancellation.load(Ordering::Relaxed) {
            terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
            report.is_cancelled = true;
            return Ok(report);
        }
        let paths = repository.load_metadata_inventory_absence_candidates(
            &request.run_id,
            absence_cursor.as_deref(),
            page_limit,
        )?;
        if paths.is_empty() {
            break;
        }
        let mut intents = Vec::with_capacity(paths.len());
        for path in &paths {
            intents.push(candidate_intent(
                request,
                path,
                None,
                observed_unix_ms,
                sequence,
            ));
            sequence = sequence.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_sequence_overflow",
                    "The metadata inventory candidate sequence overflowed",
                )
            })?;
        }
        enqueue_candidates(
            repository,
            &intents,
            observed_unix_ms,
            queue_policy,
            &mut report,
        )?;
        let next_cursor = paths.last().expect("non-empty absence page").clone();
        let count = u64::try_from(paths.len()).map_err(|_| {
            ScanError::new(
                "metadata_inventory_candidate_count_overflow",
                "The metadata inventory absence count exceeded the supported range",
            )
        })?;
        report.absence_candidate_count = checked_add(
            report.absence_candidate_count,
            count,
            "metadata inventory absence count",
        )?;
        let run = repository.advance_metadata_inventory_absence_cursor(
            &request.run_id,
            absence_cursor.as_deref(),
            &next_cursor,
            count,
            observed_unix_ms,
        )?;
        report.candidate_count = run.candidate_count;
        absence_cursor = Some(next_cursor);
    }

    let run = repository.complete_metadata_inventory(&request.run_id, observed_unix_ms)?;
    report.staged_entry_count = run.staged_entry_count;
    report.candidate_count = run.candidate_count;
    report.is_complete = true;
    Ok(report)
}

fn compare_entry<Repository>(
    repository: &Repository,
    request: &MetadataInventoryRunRequest,
    entry: &MetadataInventoryEntry,
    claimed_identities: &mut BTreeSet<(String, String)>,
) -> Result<EntryComparison, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository,
{
    if entry.kind != MetadataInventoryEntryKind::File {
        return Ok(EntryComparison::Unchanged);
    }
    let path_prior = repository
        .load_incremental_location_by_relative_path(&request.root_id, &entry.relative_path)?;
    if path_prior
        .as_ref()
        .is_some_and(|prior| inventory_matches_location(entry, prior))
    {
        return Ok(EntryComparison::Unchanged);
    }
    if path_prior.is_none()
        && let Some(identity) = entry.file_identity.as_ref()
    {
        let identity_key = (identity.scheme.clone(), identity.value.clone());
        if !claimed_identities.contains(&identity_key)
            && let Some(previous_path) =
                repository.load_metadata_inventory_previous_path(&request.run_id, identity)?
        {
            claimed_identities.insert(identity_key);
            return Ok(EntryComparison::Candidate {
                previous_path: Some(previous_path),
            });
        }
    }
    Ok(EntryComparison::Candidate {
        previous_path: None,
    })
}

fn inventory_matches_location(entry: &MetadataInventoryEntry, prior: &AssetLocationView) -> bool {
    if entry.placeholder_state != MetadataInventoryPlaceholderState::Available
        || entry.is_reparse_point
        || entry.file_size != Some(prior.file_size)
        || entry.modified_unix_ms != prior.modified_unix_ms
    {
        return false;
    }
    match (&entry.file_identity, &prior.file_identity) {
        (Some(current), Some(previous)) => current == previous,
        (None, None) => true,
        _ => false,
    }
}

fn candidate_intent(
    request: &MetadataInventoryRunRequest,
    relative_path: &str,
    previous_relative_path: Option<&str>,
    observed_unix_ms: i64,
    sequence: u64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: request.root_id.clone(),
        root_generation: request.root_generation,
        kind: if previous_relative_path.is_some() {
            LibraryChangeIntentKind::RenameCandidate
        } else {
            LibraryChangeIntentKind::Reconcile
        },
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: previous_relative_path.map(str::to_owned),
        origin: LibraryChangeOrigin::MetadataInventory,
        first_observed_unix_ms: observed_unix_ms,
        most_recent_observed_unix_ms: observed_unix_ms,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: 1,
    }
}

fn enqueue_candidates<Repository>(
    repository: &mut Repository,
    intents: &[LibraryChangeIntent],
    observed_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    report: &mut MetadataInventoryReport,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    if intents.is_empty() {
        return Ok(());
    }
    let enqueue =
        repository.enqueue_library_change_intents(intents, observed_unix_ms, queue_policy)?;
    report.enqueued_count = checked_add(
        report.enqueued_count,
        u64::from(enqueue.inserted_count),
        "metadata inventory enqueued count",
    )?;
    report.coalesced_count = checked_add(
        report.coalesced_count,
        u64::from(enqueue.coalesced_count),
        "metadata inventory coalesced count",
    )?;
    report.superseded_count = checked_add(
        report.superseded_count,
        u64::from(enqueue.superseded_count),
        "metadata inventory superseded count",
    )?;
    Ok(())
}

fn terminate_cancelled<Repository>(
    repository: &mut Repository,
    run_id: &str,
    updated_unix_ms: i64,
) -> Result<(), ScanError>
where
    Repository: MetadataInventoryRepository,
{
    repository.terminate_metadata_inventory(
        run_id,
        MetadataInventoryRunStatus::Cancelled,
        None,
        updated_unix_ms,
    )?;
    Ok(())
}

fn terminate_failed<Repository>(
    repository: &mut Repository,
    run_id: &str,
    error: &ScanError,
    updated_unix_ms: i64,
) where
    Repository: MetadataInventoryRepository,
{
    let _ = repository.terminate_metadata_inventory(
        run_id,
        MetadataInventoryRunStatus::Failed,
        Some((&error.code, &error.message)),
        updated_unix_ms,
    );
}

fn validate_request(
    request: &MetadataInventoryRunRequest,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    if request.run_id.is_empty()
        || request.root_id.is_empty()
        || request.epoch == 0
        || page_limit == 0
        || page_limit > MAX_INVENTORY_PAGE_ENTRIES
        || !queue_policy.is_valid()
    {
        return Err(ScanError::new(
            "metadata_inventory_request_invalid",
            "Metadata inventory identity, paging, or queue policy is invalid",
        ));
    }
    Ok(())
}

fn checked_add(value: u64, addend: u64, field: &str) -> Result<u64, ScanError> {
    value.checked_add(addend).ok_or_else(|| {
        ScanError::new(
            "metadata_inventory_count_overflow",
            format!("The {field} overflowed"),
        )
    })
}

enum EntryComparison {
    Unchanged,
    Candidate { previous_path: Option<String> },
}

#[cfg(test)]
mod tests;

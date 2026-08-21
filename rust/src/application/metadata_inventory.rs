use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::LocalMetadataInventory;
use crate::domain::{
    AssetLocationView, IncrementalCatalogRoot, IncrementalLibraryChangeReport, LeasedLibraryChange,
    LibraryChangeFailure, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, MetadataInventoryComparisonStatus, MetadataInventoryComparisonUpdate,
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryPlaceholderState,
    MetadataInventoryReport, MetadataInventoryRunRequest, MetadataInventoryRunStatus,
    MetadataInventoryScope, MetadataInventoryStartRequest, ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MetadataInventoryRepository,
    MetadataInventorySource,
};

const MAX_INVENTORY_PAGE_ENTRIES: u32 = 4_096;
const MAX_INVENTORY_CLEANUP_RUNS: u32 = 128;
const INVENTORY_TERMINAL_RETENTION_MILLIS: i64 = 7 * 24 * 60 * 60 * 1_000;
const TERMINATION_ATTEMPTS: usize = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MetadataInventoryRecoveryReport {
    pub incremental: IncrementalLibraryChangeReport,
    pub inventory: MetadataInventoryReport,
}

#[derive(Clone, Copy)]
struct InventoryExecution<'a> {
    authority: Option<&'a LeasedLibraryChange>,
    yield_after_work_page: bool,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &'a AtomicBool,
}

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

pub(crate) fn run_next_local_metadata_inventory<Repository>(
    repository: &mut Repository,
    request: &MetadataInventoryStartRequest,
    authority: &LeasedLibraryChange,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &AtomicBool,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    validate_start_request(request, page_limit, queue_policy)?;
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
    drain_terminal_inventory_cleanup(repository, observed_unix_ms)?;
    let run = repository.begin_next_metadata_inventory(request)?;
    finish_started_metadata_inventory(
        repository,
        &mut source,
        &run.request,
        InventoryExecution {
            authority: Some(authority),
            yield_after_work_page: true,
            observed_unix_ms,
            page_limit,
            queue_policy,
            cancellation,
        },
    )
}

pub(crate) fn leased_change_requires_metadata_inventory(leased: &LeasedLibraryChange) -> bool {
    leased.change.intent.kind == LibraryChangeIntentKind::FreshnessUnknown
        || matches!(
            leased.change.intent.origin,
            LibraryChangeOrigin::StartupCatchUp | LibraryChangeOrigin::ConsistencyAudit
        )
        || leased
            .change
            .last_failure
            .as_ref()
            .is_some_and(|failure| failure.code == "metadata_inventory_required")
}

pub(crate) fn process_leased_metadata_inventory_change<Repository>(
    repository: &mut Repository,
    root: &IncrementalCatalogRoot,
    leased: &LeasedLibraryChange,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &AtomicBool,
) -> Result<MetadataInventoryRecoveryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    if leased.change.intent.root_id != root.root_id
        || leased.change.intent.root_generation != root.root_generation
    {
        return Err(ScanError::new(
            "metadata_inventory_lease_root_mismatch",
            "The metadata inventory lease does not belong to the selected catalog root",
        ));
    }
    let request = MetadataInventoryStartRequest {
        run_id: metadata_inventory_run_id(leased),
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        scope: metadata_inventory_scope(leased)?,
        started_unix_ms: observed_unix_ms,
    };
    let inventory = match run_next_local_metadata_inventory(
        repository,
        &request,
        leased,
        observed_unix_ms,
        page_limit,
        queue_policy,
        cancellation,
    ) {
        Ok(report) => report,
        Err(error) => {
            let authoritative = super::retry_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                LibraryChangeFailure {
                    code: error.code,
                    message: error.message,
                },
                observed_unix_ms,
                queue_policy,
            )?;
            return Ok(MetadataInventoryRecoveryReport {
                incremental: authoritative.incremental,
                ..MetadataInventoryRecoveryReport::default()
            });
        }
    };
    if inventory.is_cancelled || cancellation.load(Ordering::Acquire) {
        let authoritative = super::defer_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            observed_unix_ms,
        )?;
        return Ok(MetadataInventoryRecoveryReport {
            incremental: authoritative.incremental,
            inventory,
        });
    }
    if !inventory.is_complete {
        let authoritative = super::defer_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            observed_unix_ms,
        )?;
        return Ok(MetadataInventoryRecoveryReport {
            incremental: authoritative.incremental,
            inventory,
        });
    }
    let catalog_revision = repository
        .load_incremental_catalog_root(&root.root_id)?
        .map_or(root.catalog_revision, |current| current.catalog_revision);
    let mut incremental = IncrementalLibraryChangeReport {
        leased_count: 1,
        catalog_revision,
        ..IncrementalLibraryChangeReport::default()
    };
    match repository.complete_library_change(
        leased.change.id,
        leased.lease_generation,
        catalog_revision,
        observed_unix_ms,
    )? {
        LibraryChangeLeaseUpdateOutcome::Applied => incremental.completed_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => incremental.superseded_count = 1,
    }
    Ok(MetadataInventoryRecoveryReport {
        incremental,
        inventory,
    })
}

fn metadata_inventory_run_id(leased: &LeasedLibraryChange) -> String {
    let intent = &leased.change.intent;
    super::scan_library::stable_id(
        "metadata-inventory-run-v1",
        &format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            intent.root_id,
            intent.root_generation.value(),
            leased.change.id.value(),
            leased.change.attempt_count,
            intent.most_recent_observed_unix_ms,
            intent.most_recent_sequence,
            intent.coalesced_observation_count,
            inventory_origin_key(intent.origin),
        ),
    )
}

const fn inventory_origin_key(origin: LibraryChangeOrigin) -> &'static str {
    match origin {
        LibraryChangeOrigin::LiveNotification => "live",
        LibraryChangeOrigin::StartupCatchUp => "startup",
        LibraryChangeOrigin::ConsistencyAudit => "audit",
        LibraryChangeOrigin::MetadataInventory => "inventory",
        LibraryChangeOrigin::UserRefresh => "user-refresh",
    }
}

fn metadata_inventory_scope(
    leased: &LeasedLibraryChange,
) -> Result<MetadataInventoryScope, ScanError> {
    let intent = &leased.change.intent;
    if intent.kind == LibraryChangeIntentKind::FreshnessUnknown
        || intent.scope == LibraryChangeScope::Root
    {
        return Ok(MetadataInventoryScope::Root);
    }
    if intent.scope != LibraryChangeScope::Subtree {
        return Err(ScanError::new(
            "metadata_inventory_scope_invalid",
            "Only root or subtree authority can start a metadata inventory",
        ));
    }
    let relative_path = intent.previous_relative_path.as_deref().map_or_else(
        || intent.relative_path.clone(),
        |previous| common_relative_ancestor(&intent.relative_path, previous),
    );
    if relative_path.is_empty() {
        Ok(MetadataInventoryScope::Root)
    } else {
        Ok(MetadataInventoryScope::Subtree { relative_path })
    }
}

fn common_relative_ancestor(left: &str, right: &str) -> String {
    left.split('/')
        .zip(right.split('/'))
        .take_while(|(left, right)| left == right)
        .map(|(segment, _)| segment)
        .collect::<Vec<_>>()
        .join("/")
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
    drain_terminal_inventory_cleanup(repository, observed_unix_ms)?;
    repository.begin_metadata_inventory(request)?;
    finish_started_metadata_inventory(
        repository,
        source,
        request,
        InventoryExecution {
            authority: None,
            yield_after_work_page: false,
            observed_unix_ms,
            page_limit,
            queue_policy,
            cancellation,
        },
    )
}

fn finish_started_metadata_inventory<Repository, Source>(
    repository: &mut Repository,
    source: &mut Source,
    request: &MetadataInventoryRunRequest,
    execution: InventoryExecution<'_>,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
    Source: MetadataInventorySource,
{
    let observed_unix_ms = execution.observed_unix_ms;
    let result = run_started_metadata_inventory(repository, source, request, execution);
    match result {
        Ok(mut report) => {
            if drain_terminal_inventory_cleanup(repository, observed_unix_ms).is_err() {
                report.cleanup_pending = true;
            }
            Ok(report)
        }
        Err(error) => {
            if let Err(terminal_error) =
                terminate_failed(repository, &request.run_id, &error, observed_unix_ms)
            {
                return Err(combined_inventory_error(
                    "metadata_inventory_termination_failed",
                    &error,
                    &terminal_error,
                ));
            }
            if let Err(cleanup_error) =
                drain_terminal_inventory_cleanup(repository, observed_unix_ms)
            {
                return Err(combined_inventory_error(
                    "metadata_inventory_cleanup_failed",
                    &error,
                    &cleanup_error,
                ));
            }
            Err(error)
        }
    }
}

fn run_started_metadata_inventory<Repository, Source>(
    repository: &mut Repository,
    source: &mut Source,
    request: &MetadataInventoryRunRequest,
    execution: InventoryExecution<'_>,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
    Source: MetadataInventorySource,
{
    let InventoryExecution {
        authority,
        yield_after_work_page,
        observed_unix_ms,
        page_limit,
        queue_policy,
        cancellation,
    } = execution;
    let mut report = MetadataInventoryReport::default();
    let persisted = repository
        .load_metadata_inventory_run(&request.run_id)?
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_run_missing",
                "The metadata inventory continuation no longer exists",
            )
        })?;
    report.staged_entry_count = persisted.staged_entry_count;
    report.candidate_count = persisted.candidate_count;
    if !persisted.enumeration_complete {
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
            let run = repository.stage_metadata_inventory_page(
                &request.run_id,
                &page,
                observed_unix_ms,
            )?;
            report.staged_entry_count = run.staged_entry_count;
            if is_complete {
                break;
            }
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
        match enqueue_candidates(
            repository,
            authority,
            &intents,
            observed_unix_ms,
            queue_policy,
            &mut report,
        )? {
            CandidateEnqueueOutcome::Applied => {}
            CandidateEnqueueOutcome::Backpressured => {
                report.is_backpressured = true;
                return Ok(report);
            }
            CandidateEnqueueOutcome::AuthoritySuperseded => {
                repository.terminate_metadata_inventory(
                    &request.run_id,
                    MetadataInventoryRunStatus::Superseded,
                    None,
                    observed_unix_ms,
                )?;
                report.is_cancelled = true;
                return Ok(report);
            }
        }
        let run = repository.record_metadata_inventory_comparisons(
            &request.run_id,
            &updates,
            observed_unix_ms,
        )?;
        report.candidate_count = run.candidate_count;
        if yield_after_work_page {
            return Ok(report);
        }
    }

    let mut absence_cursor = persisted.absence_cursor;
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
        match enqueue_candidates(
            repository,
            authority,
            &intents,
            observed_unix_ms,
            queue_policy,
            &mut report,
        )? {
            CandidateEnqueueOutcome::Applied => {}
            CandidateEnqueueOutcome::Backpressured => {
                report.is_backpressured = true;
                return Ok(report);
            }
            CandidateEnqueueOutcome::AuthoritySuperseded => {
                repository.terminate_metadata_inventory(
                    &request.run_id,
                    MetadataInventoryRunStatus::Superseded,
                    None,
                    observed_unix_ms,
                )?;
                report.is_cancelled = true;
                return Ok(report);
            }
        }
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
        if yield_after_work_page {
            return Ok(report);
        }
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
    if entry.file_size != Some(prior.file_size) || entry.modified_unix_ms != prior.modified_unix_ms
    {
        return false;
    }
    if entry.placeholder_state != MetadataInventoryPlaceholderState::Available {
        return entry.file_identity.is_none();
    }
    if entry.is_reparse_point {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateEnqueueOutcome {
    Applied,
    Backpressured,
    AuthoritySuperseded,
}

fn enqueue_candidates<Repository>(
    repository: &mut Repository,
    authority: Option<&LeasedLibraryChange>,
    intents: &[LibraryChangeIntent],
    observed_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    report: &mut MetadataInventoryReport,
) -> Result<CandidateEnqueueOutcome, ScanError>
where
    Repository: LibraryChangeQueue,
{
    if intents.is_empty() {
        return Ok(CandidateEnqueueOutcome::Applied);
    }
    let enqueue = if let Some(authority) = authority {
        match repository.enqueue_metadata_inventory_candidates(
            authority,
            intents,
            observed_unix_ms,
            queue_policy,
        ) {
            Ok(Some(report)) => report,
            Ok(None) => return Ok(CandidateEnqueueOutcome::AuthoritySuperseded),
            Err(error) if error.code == "metadata_inventory_backpressure" => {
                return Ok(CandidateEnqueueOutcome::Backpressured);
            }
            Err(error) => return Err(error),
        }
    } else {
        repository.enqueue_library_change_intents(intents, observed_unix_ms, queue_policy)?
    };
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
    Ok(CandidateEnqueueOutcome::Applied)
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
) -> Result<(), ScanError>
where
    Repository: MetadataInventoryRepository,
{
    let issue_code = bounded_text(&error.code, 128);
    let issue_message = bounded_text(&error.message, 4_096);
    retry_terminalization(|| {
        repository
            .terminate_metadata_inventory(
                run_id,
                MetadataInventoryRunStatus::Failed,
                Some((&issue_code, &issue_message)),
                updated_unix_ms,
            )
            .map(|_| ())
    })
}

fn retry_terminalization(
    mut terminalize: impl FnMut() -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let mut last_error = None;
    for attempt in 0..TERMINATION_ATTEMPTS {
        match terminalize() {
            Ok(()) => return Ok(()),
            Err(termination_error)
                if attempt + 1 < TERMINATION_ATTEMPTS
                    && is_catalog_contention(&termination_error.code) =>
            {
                last_error = Some(termination_error);
            }
            Err(termination_error) => return Err(termination_error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        ScanError::new(
            "metadata_inventory_termination_failed",
            "Metadata inventory terminalization did not complete",
        )
    }))
}

fn drain_terminal_inventory_cleanup<Repository>(
    repository: &mut Repository,
    observed_unix_ms: i64,
) -> Result<(), ScanError>
where
    Repository: MetadataInventoryRepository,
{
    let terminal_before_unix_ms =
        observed_unix_ms.saturating_sub(INVENTORY_TERMINAL_RETENTION_MILLIS);
    loop {
        let cleanup = repository.cleanup_terminal_metadata_inventories(
            terminal_before_unix_ms,
            MAX_INVENTORY_PAGE_ENTRIES,
            MAX_INVENTORY_CLEANUP_RUNS,
        )?;
        if !cleanup.has_more {
            return Ok(());
        }
    }
}

fn is_catalog_contention(code: &str) -> bool {
    matches!(code, "catalog_database_busy" | "catalog_database_locked")
}

fn combined_inventory_error(code: &str, primary: &ScanError, secondary: &ScanError) -> ScanError {
    ScanError::new(
        code,
        bounded_text(
            &format!(
                "Primary failure [{}]: {}; follow-up failure [{}]: {}",
                primary.code, primary.message, secondary.code, secondary.message
            ),
            4_096,
        ),
    )
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
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

fn validate_start_request(
    request: &MetadataInventoryStartRequest,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError> {
    validate_request(
        &MetadataInventoryRunRequest {
            run_id: request.run_id.clone(),
            root_id: request.root_id.clone(),
            root_generation: request.root_generation,
            epoch: 1,
            scope: request.scope.clone(),
            started_unix_ms: request.started_unix_ms,
        },
        page_limit,
        queue_policy,
    )
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

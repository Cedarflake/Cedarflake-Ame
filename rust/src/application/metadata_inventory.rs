#[cfg(test)]
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery};
use crate::domain::{
    AssetLocationView, IncrementalCatalogRoot, IncrementalLibraryChangeReport, LeasedLibraryChange,
    LibraryChangeFailure, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRecoveryAuthorityReason, MetadataInventoryComparisonStatus,
    MetadataInventoryComparisonUpdate, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, MetadataInventoryReport, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, MetadataInventoryScope, MetadataInventoryStartRequest, ScanError,
    TerminalMediaEvidence,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector,
    MetadataInventoryAbsencePublicationRequest, MetadataInventoryRepository,
    MetadataInventorySource, MetadataInventorySourcePreparation,
};

const MAX_INVENTORY_PAGE_ENTRIES: u32 = 4_096;
const MAX_INVENTORY_CLEANUP_RUNS: u32 = 128;
const INVENTORY_TERMINAL_RETENTION_MILLIS: i64 = 7 * 24 * 60 * 60 * 1_000;
const TERMINATION_ATTEMPTS: usize = 2;

#[cfg(test)]
thread_local! {
    static BEFORE_METADATA_INVENTORY_FINALIZATION_HOOK: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
}

#[cfg(test)]
fn set_before_metadata_inventory_finalization_hook(hook: impl FnOnce() + 'static) {
    BEFORE_METADATA_INVENTORY_FINALIZATION_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
}

#[cfg(test)]
fn run_before_metadata_inventory_finalization_hook() {
    BEFORE_METADATA_INVENTORY_FINALIZATION_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MetadataInventoryRecoveryReport {
    pub incremental: IncrementalLibraryChangeReport,
    pub inventory: MetadataInventoryReport,
}

pub(crate) struct RetainedMetadataInventorySource {
    run_id: String,
    source: Box<dyn MetadataInventorySource>,
}

impl RetainedMetadataInventorySource {
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        run_id: impl Into<String>,
        source: Box<dyn MetadataInventorySource>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            source,
        }
    }
}

pub(crate) struct MetadataInventoryRecoveryPage {
    pub report: MetadataInventoryRecoveryReport,
    pub retained_source: Option<RetainedMetadataInventorySource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetadataInventoryProgressPhase {
    Enumeration,
    Comparison,
    QueuePublication,
}

#[derive(Clone, Copy)]
pub(crate) struct MetadataInventoryWorkerControl<'a> {
    cancellation: &'a AtomicBool,
    progress: Option<&'a dyn Fn(MetadataInventoryProgressPhase)>,
    page_yield: Option<&'a dyn Fn(MetadataInventoryProgressPhase)>,
}

impl<'a> MetadataInventoryWorkerControl<'a> {
    pub(crate) const fn with_progress_and_page_yield(
        cancellation: &'a AtomicBool,
        progress: &'a dyn Fn(MetadataInventoryProgressPhase),
        page_yield: &'a dyn Fn(MetadataInventoryProgressPhase),
    ) -> Self {
        Self {
            cancellation,
            progress: Some(progress),
            page_yield: Some(page_yield),
        }
    }

    #[cfg(test)]
    const fn without_progress(cancellation: &'a AtomicBool) -> Self {
        Self {
            cancellation,
            progress: None,
            page_yield: None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MetadataInventoryRecoveryExecution<'a> {
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    control: MetadataInventoryWorkerControl<'a>,
}

impl<'a> MetadataInventoryRecoveryExecution<'a> {
    pub(crate) const fn new(
        observed_unix_ms: i64,
        page_limit: u32,
        queue_policy: LibraryChangeQueuePolicy,
        control: MetadataInventoryWorkerControl<'a>,
    ) -> Self {
        Self {
            observed_unix_ms,
            page_limit,
            queue_policy,
            control,
        }
    }

    #[cfg(test)]
    pub(crate) const fn without_progress(
        observed_unix_ms: i64,
        page_limit: u32,
        queue_policy: LibraryChangeQueuePolicy,
        cancellation: &'a AtomicBool,
    ) -> Self {
        Self::new(
            observed_unix_ms,
            page_limit,
            queue_policy,
            MetadataInventoryWorkerControl::without_progress(cancellation),
        )
    }
}

#[derive(Clone, Copy)]
struct InventoryExecution<'a> {
    authority: Option<&'a LeasedLibraryChange>,
    invalidates_unchanged_sources: bool,
    yield_after_work_page: bool,
    observed_unix_ms: i64,
    page_limit: u32,
    queue_policy: LibraryChangeQueuePolicy,
    control: MetadataInventoryWorkerControl<'a>,
}

#[cfg(test)]
pub(crate) fn run_metadata_inventory_with_source_for_test<Repository, Source>(
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
    run_metadata_inventory(
        repository,
        source,
        request,
        observed_unix_ms,
        page_limit,
        queue_policy,
        cancellation,
    )
}

fn run_next_local_metadata_inventory<Repository>(
    repository: &mut Repository,
    request: &MetadataInventoryStartRequest,
    authority: &LeasedLibraryChange,
    execution: MetadataInventoryRecoveryExecution<'_>,
    retained_source: Option<RetainedMetadataInventorySource>,
) -> Result<
    (
        MetadataInventoryReport,
        Option<RetainedMetadataInventorySource>,
    ),
    ScanError,
>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    let MetadataInventoryRecoveryExecution {
        observed_unix_ms,
        page_limit,
        queue_policy,
        control,
    } = execution;
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
    let recovery_authority = repository
        .load_metadata_inventory_recovery_authority(authority.change.id)?
        .filter(|current| {
            current.root_id == root.root_id
                && current.root_generation == root.root_generation
                && current.retired_unix_ms.is_none()
        })
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_not_authorized",
                "The P2 source requires its current durable recovery authority",
            )
        })?;
    if root.publication_root_identity.is_none()
        && !matches!(
            recovery_authority.reason,
            LibraryRecoveryAuthorityReason::ExistingRootBaseline
                | LibraryRecoveryAuthorityReason::FirstImportBoundary
        )
    {
        return Err(ScanError::new(
            "root_publication_namespace_unproven",
            "Only a first-authority baseline may capture a source without an existing publication namespace proof",
        ));
    }
    let expected_publication_identity = root.publication_root_identity.clone();
    let opening_guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &root.root_path,
        expected_publication_identity.as_ref(),
    )?;
    let expected_source_identity = opening_guard
        .metadata_inventory_root_identity()?
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_root_identity_unavailable",
                "The recovery source root has no durable Windows file identity",
            )
        })?;
    drain_terminal_inventory_cleanup(repository, observed_unix_ms)?;
    let run = repository.begin_next_metadata_inventory(request)?;
    let mut retained_source = if run.enumeration_complete {
        None
    } else if let Some(retained) = retained_source
        && retained.run_id == run.request.run_id
    {
        Some(retained)
    } else {
        Some(RetainedMetadataInventorySource {
            run_id: run.request.run_id.clone(),
            source: repository.open_metadata_inventory_source(
                &root.root_path,
                &request.scope,
                &run,
                authority,
                expected_publication_identity.as_ref(),
                &expected_source_identity,
            )?,
        })
    };
    let execution = InventoryExecution {
        authority: Some(authority),
        invalidates_unchanged_sources: recovery_reason_invalidates_unchanged_sources(
            recovery_authority.reason,
        ),
        yield_after_work_page: true,
        observed_unix_ms,
        page_limit,
        queue_policy,
        control,
    };
    let report = match retained_source.as_mut() {
        Some(retained) => finish_started_metadata_inventory(
            repository,
            Some(retained.source.as_mut()),
            &run.request,
            execution,
        )?,
        None => finish_started_metadata_inventory(repository, None, &run.request, execution)?,
    };
    Ok((report, retained_source))
}

pub(crate) fn leased_change_requires_metadata_inventory(leased: &LeasedLibraryChange) -> bool {
    metadata_inventory_recovery_reason(leased).is_some()
}

#[cfg(test)]
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
    process_leased_metadata_inventory_change_internal(
        repository,
        root,
        leased,
        MetadataInventoryRecoveryExecution::new(
            observed_unix_ms,
            page_limit,
            queue_policy,
            MetadataInventoryWorkerControl::without_progress(cancellation),
        ),
        None,
    )
    .map(|page| page.report)
}

pub(crate) fn process_leased_metadata_inventory_change_with_retained_source<Repository>(
    repository: &mut Repository,
    root: &IncrementalCatalogRoot,
    leased: &LeasedLibraryChange,
    execution: MetadataInventoryRecoveryExecution<'_>,
    retained_source: Option<RetainedMetadataInventorySource>,
) -> Result<MetadataInventoryRecoveryPage, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_leased_metadata_inventory_change_internal(
        repository,
        root,
        leased,
        execution,
        retained_source,
    )
}

fn process_leased_metadata_inventory_change_internal<Repository>(
    repository: &mut Repository,
    root: &IncrementalCatalogRoot,
    leased: &LeasedLibraryChange,
    execution: MetadataInventoryRecoveryExecution<'_>,
    retained_source: Option<RetainedMetadataInventorySource>,
) -> Result<MetadataInventoryRecoveryPage, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    let MetadataInventoryRecoveryExecution {
        observed_unix_ms,
        page_limit: _,
        queue_policy,
        control,
    } = execution;
    if leased.change.intent.root_id != root.root_id
        || leased.change.intent.root_generation != root.root_generation
    {
        return Err(ScanError::new(
            "metadata_inventory_lease_root_mismatch",
            "The metadata inventory lease does not belong to the selected catalog root",
        ));
    }
    let authority = repository
        .load_metadata_inventory_recovery_authority(leased.change.id)?
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_recovery_not_authorized",
                "The P2 worker requires an unretired durable recovery authority",
            )
        })?;
    if authority.root_id != root.root_id
        || authority.root_generation != root.root_generation
        || authority.retired_unix_ms.is_some()
        || authority.run_id.is_empty()
    {
        return Err(ScanError::new(
            "metadata_inventory_recovery_authority_conflict",
            "The leased recovery no longer owns its durable root authority",
        ));
    }
    let run_id = authority.run_id;
    let request = MetadataInventoryStartRequest {
        run_id,
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        scope: metadata_inventory_scope(leased)?,
        started_unix_ms: observed_unix_ms,
    };
    let (inventory, retained_source) = match run_next_local_metadata_inventory(
        repository,
        &request,
        leased,
        execution,
        retained_source,
    ) {
        Ok(report) => report,
        Err(error) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "[Ame sync] metadata inventory page failed root={} change={:?} code={}",
                root.root_id, leased.change.id, error.code
            );
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
            return Ok(MetadataInventoryRecoveryPage {
                report: MetadataInventoryRecoveryReport {
                    incremental: authoritative.incremental,
                    ..MetadataInventoryRecoveryReport::default()
                },
                retained_source: None,
            });
        }
    };
    if inventory.is_cancelled || control.cancellation.load(Ordering::Acquire) {
        let authoritative = super::defer_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            observed_unix_ms,
        )?;
        return Ok(MetadataInventoryRecoveryPage {
            report: MetadataInventoryRecoveryReport {
                incremental: authoritative.incremental,
                inventory,
            },
            retained_source: None,
        });
    }
    if !inventory.is_complete {
        let authoritative = super::defer_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            observed_unix_ms,
        )?;
        return Ok(MetadataInventoryRecoveryPage {
            report: MetadataInventoryRecoveryReport {
                incremental: authoritative.incremental,
                inventory,
            },
            retained_source,
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
    #[cfg(test)]
    run_before_metadata_inventory_finalization_hook();
    let completion = match repository.finish_metadata_inventory_recovery(
        leased.change.id,
        leased.lease_generation,
        catalog_revision,
        observed_unix_ms,
    )? {
        Some(outcome) => outcome,
        None => {
            let outcome = repository.defer_library_change(
                leased.change.id,
                leased.lease_generation,
                observed_unix_ms,
            )?;
            if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
                incremental.superseded_count = 1;
            }
            return Ok(MetadataInventoryRecoveryPage {
                report: MetadataInventoryRecoveryReport {
                    incremental,
                    inventory,
                },
                retained_source: None,
            });
        }
    };
    match completion {
        LibraryChangeLeaseUpdateOutcome::Applied => incremental.completed_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => incremental.superseded_count = 1,
    }
    Ok(MetadataInventoryRecoveryPage {
        report: MetadataInventoryRecoveryReport {
            incremental,
            inventory,
        },
        retained_source: None,
    })
}

fn metadata_inventory_recovery_reason(
    leased: &LeasedLibraryChange,
) -> Option<LibraryRecoveryAuthorityReason> {
    if leased
        .change
        .last_failure
        .as_ref()
        .is_some_and(|failure| failure.code == "metadata_inventory_required")
    {
        return match leased.change.intent.origin {
            LibraryChangeOrigin::ConsistencyAudit => {
                Some(LibraryRecoveryAuthorityReason::JournalReconstructionFailure)
            }
            LibraryChangeOrigin::MetadataInventory => {
                Some(LibraryRecoveryAuthorityReason::WatcherUncoveredGap)
            }
            LibraryChangeOrigin::LiveNotification
            | LibraryChangeOrigin::StartupCatchUp
            | LibraryChangeOrigin::UserRefresh => None,
        };
    }
    let intent = &leased.change.intent;
    match (intent.origin, intent.kind) {
        (LibraryChangeOrigin::MetadataInventory, _) if intent.scope != LibraryChangeScope::Path => {
            Some(LibraryRecoveryAuthorityReason::WatcherUncoveredGap)
        }
        (LibraryChangeOrigin::ConsistencyAudit, _) => {
            Some(LibraryRecoveryAuthorityReason::ContainmentFailure)
        }
        _ => None,
    }
}

const fn recovery_reason_invalidates_unchanged_sources(
    reason: LibraryRecoveryAuthorityReason,
) -> bool {
    matches!(
        reason,
        LibraryRecoveryAuthorityReason::JournalGap
            | LibraryRecoveryAuthorityReason::JournalReset
            | LibraryRecoveryAuthorityReason::JournalTrim
            | LibraryRecoveryAuthorityReason::JournalReconstructionFailure
            | LibraryRecoveryAuthorityReason::ContainmentFailure
            | LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
            | LibraryRecoveryAuthorityReason::WatcherUncoveredGap
    )
}

#[cfg(test)]
fn metadata_inventory_run_id(leased: &LeasedLibraryChange) -> String {
    let intent = &leased.change.intent;
    super::scan_library::stable_id(
        "metadata-inventory-run-v2",
        &format!(
            "{}:{}:{}:{}:{}:{}:{}",
            intent.root_id,
            intent.root_generation.value(),
            leased.change.id.value(),
            intent.most_recent_observed_unix_ms,
            intent.most_recent_sequence,
            intent.coalesced_observation_count,
            inventory_origin_key(intent.origin),
        ),
    )
}

#[cfg(test)]
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

#[cfg(test)]
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
        Some(source),
        request,
        InventoryExecution {
            authority: None,
            invalidates_unchanged_sources: false,
            yield_after_work_page: false,
            observed_unix_ms,
            page_limit,
            queue_policy,
            control: MetadataInventoryWorkerControl::without_progress(cancellation),
        },
    )
}

fn finish_started_metadata_inventory<Repository>(
    repository: &mut Repository,
    source: Option<&mut dyn MetadataInventorySource>,
    request: &MetadataInventoryRunRequest,
    execution: InventoryExecution<'_>,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    let observed_unix_ms = execution.observed_unix_ms;
    let preserves_recovery_frontier = execution.authority.is_some();
    let result = run_started_metadata_inventory(repository, source, request, execution);
    match result {
        Ok(mut report) => {
            if drain_terminal_inventory_cleanup(repository, observed_unix_ms).is_err() {
                report.cleanup_pending = true;
            }
            Ok(report)
        }
        Err(error) => {
            if !preserves_recovery_frontier
                && let Err(terminal_error) =
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

fn run_started_metadata_inventory<Repository>(
    repository: &mut Repository,
    source: Option<&mut dyn MetadataInventorySource>,
    request: &MetadataInventoryRunRequest,
    execution: InventoryExecution<'_>,
) -> Result<MetadataInventoryReport, ScanError>
where
    Repository: MetadataInventoryRepository + IncrementalCatalogRepository + LibraryChangeQueue,
{
    let InventoryExecution {
        authority,
        invalidates_unchanged_sources,
        yield_after_work_page,
        observed_unix_ms,
        page_limit,
        queue_policy,
        control,
    } = execution;
    let cancellation = control.cancellation;
    let progress = control.progress;
    let page_yield = control.page_yield;
    let candidate_page_limit =
        page_limit.min(queue_policy.lane_capacity(LibraryChangeLane::Recovery));
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
        let source = source.ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_source_missing",
                "The active metadata inventory lost its retained source frontier",
            )
        })?;
        report_inventory_progress(progress, MetadataInventoryProgressPhase::Enumeration);
        loop {
            if cancellation.load(Ordering::Relaxed) {
                if authority.is_none() {
                    terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
                }
                report.is_cancelled = true;
                return Ok(report);
            }
            match source.prepare_next_page(page_limit, cancellation) {
                Ok(MetadataInventorySourcePreparation::Ready) => {}
                Ok(MetadataInventorySourcePreparation::Yielded) => {
                    report_inventory_progress(
                        progress,
                        MetadataInventoryProgressPhase::Enumeration,
                    );
                    yield_inventory_page(page_yield, MetadataInventoryProgressPhase::Enumeration);
                    return Ok(report);
                }
                Err(error) if error.code == "metadata_inventory_cancelled" => {
                    if authority.is_none() {
                        terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
                    }
                    report.is_cancelled = true;
                    return Ok(report);
                }
                Err(error) => return Err(error),
            }
            let page = match source.next_page(page_limit, cancellation) {
                Ok(page) => page,
                Err(error) if error.code == "metadata_inventory_cancelled" => {
                    if authority.is_none() {
                        terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
                    }
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
            report_inventory_progress(progress, MetadataInventoryProgressPhase::Enumeration);
            if is_complete {
                if yield_after_work_page {
                    yield_inventory_page(page_yield, MetadataInventoryProgressPhase::Enumeration);
                    return Ok(report);
                }
                break;
            }
            yield_inventory_page(page_yield, MetadataInventoryProgressPhase::Enumeration);
            if yield_after_work_page {
                return Ok(report);
            }
        }
    }
    report_inventory_progress(progress, MetadataInventoryProgressPhase::Comparison);
    if cancellation.load(Ordering::Relaxed) {
        if authority.is_none() {
            terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
        }
        report.is_cancelled = true;
        return Ok(report);
    }
    let mut sequence = 1_u64;
    let mut claimed_identities = BTreeSet::new();
    let media_inspector = LocalMediaInspector::new();
    loop {
        if cancellation.load(Ordering::Relaxed) {
            if authority.is_none() {
                terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
            }
            report.is_cancelled = true;
            return Ok(report);
        }
        let entries = repository
            .load_pending_metadata_inventory_entries(&request.run_id, candidate_page_limit)?;
        if entries.is_empty() {
            break;
        }
        let relative_paths = entries
            .iter()
            .filter(|entry| entry.kind == MetadataInventoryEntryKind::File)
            .map(|entry| entry.relative_path.clone())
            .collect::<Vec<_>>();
        let path_priors = repository
            .load_incremental_locations_by_relative_paths(&request.root_id, &relative_paths)?
            .into_iter()
            .map(|location| (location.relative_path.clone(), location))
            .collect::<BTreeMap<_, _>>();
        let terminal_media_evidence = repository
            .load_terminal_media_evidence_by_relative_paths(&request.root_id, &relative_paths)?
            .into_iter()
            .map(|evidence| (evidence.relative_path.clone(), evidence))
            .collect::<BTreeMap<_, _>>();
        let identities = entries
            .iter()
            .filter(|entry| {
                entry.kind == MetadataInventoryEntryKind::File
                    && !path_priors.contains_key(&entry.relative_path)
            })
            .filter_map(|entry| entry.file_identity.as_ref())
            .filter(|identity| {
                !claimed_identities.contains(&(identity.scheme.clone(), identity.value.clone()))
            })
            .map(|identity| {
                (
                    (identity.scheme.clone(), identity.value.clone()),
                    identity.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        let previous_paths = repository
            .load_metadata_inventory_previous_paths(&request.run_id, &identities)?
            .into_iter()
            .map(|(identity, previous_path)| ((identity.scheme, identity.value), previous_path))
            .collect::<BTreeMap<_, _>>();
        let mut intents = Vec::new();
        let mut updates = Vec::with_capacity(entries.len());
        for entry in entries {
            let comparison = compare_entry(
                &entry,
                path_priors.get(&entry.relative_path),
                terminal_media_evidence.get(&entry.relative_path),
                &media_inspector,
                &previous_paths,
                &mut claimed_identities,
            );
            let comparison = match comparison {
                EntryComparison::Unchanged
                    if invalidates_unchanged_sources
                        && entry.kind == MetadataInventoryEntryKind::File
                        && path_priors.contains_key(&entry.relative_path) =>
                {
                    EntryComparison::Candidate {
                        previous_path: None,
                    }
                }
                comparison => comparison,
            };
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
        report_inventory_progress(progress, MetadataInventoryProgressPhase::QueuePublication);
        let run = if let Some(authority) = authority {
            match repository.publish_metadata_inventory_comparison_candidates(
                authority,
                &request.run_id,
                &intents,
                &updates,
                observed_unix_ms,
                queue_policy,
            ) {
                Ok(Some((enqueue, run))) => {
                    apply_enqueue_report(&mut report, enqueue)?;
                    run
                }
                Ok(None) => {
                    repository.terminate_metadata_inventory(
                        &request.run_id,
                        MetadataInventoryRunStatus::Superseded,
                        None,
                        observed_unix_ms,
                    )?;
                    report.is_cancelled = true;
                    return Ok(report);
                }
                Err(error) if error.code == "metadata_inventory_backpressure" => {
                    report.is_backpressured = true;
                    return Ok(report);
                }
                Err(error) => return Err(error),
            }
        } else {
            match enqueue_candidates(
                repository,
                None,
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
                    return Err(ScanError::new(
                        "metadata_inventory_authority_state_invalid",
                        "A manual inventory cannot lose recovery authority",
                    ));
                }
            }
            repository.record_metadata_inventory_comparisons(
                &request.run_id,
                &updates,
                observed_unix_ms,
            )?
        };
        report.candidate_count = run.candidate_count;
        report_inventory_progress(progress, MetadataInventoryProgressPhase::Comparison);
        if yield_after_work_page {
            yield_inventory_page(page_yield, MetadataInventoryProgressPhase::Comparison);
            return Ok(report);
        }
    }

    let recovery_root_binding = if authority.is_some() {
        let expected_identity = repository
            .load_metadata_inventory_root_identity(&request.run_id)?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_root_identity_missing",
                    "Recovery absence authority lacks its durable pinned-root identity",
                )
            })?;
        let root = repository
            .load_incremental_catalog_root(&request.root_id)?
            .filter(|root| root.root_generation == request.root_generation)
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_root_stale",
                    "The recovery root changed before absence authorization",
                )
            })?;
        Some((root.root_path, expected_identity))
    } else {
        None
    };
    if let Some(authority) = authority
        && !repository.metadata_inventory_recovery_allows_absence(authority.change.id)?
    {
        report.awaiting_closing_boundary = true;
        return Ok(report);
    }
    {
        let _publication_guard = recovery_root_binding
            .as_ref()
            .map(|(root_path, expected_identity)| {
                PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                    root_path,
                    expected_identity,
                )
            })
            .transpose()?;
        repository.authorize_metadata_inventory_absence(&request.run_id, observed_unix_ms)?;
    }

    let mut absence_cursor = persisted.absence_cursor;
    loop {
        if cancellation.load(Ordering::Relaxed) {
            if authority.is_none() {
                terminate_cancelled(repository, &request.run_id, observed_unix_ms)?;
            }
            report.is_cancelled = true;
            return Ok(report);
        }
        let paths = repository.load_metadata_inventory_absence_candidates(
            &request.run_id,
            absence_cursor.as_deref(),
            candidate_page_limit,
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
        report_inventory_progress(progress, MetadataInventoryProgressPhase::QueuePublication);
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
        let _publication_guard = recovery_root_binding
            .as_ref()
            .map(|(root_path, expected_identity)| {
                PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                    root_path,
                    expected_identity,
                )
            })
            .transpose()?;
        let run = if let Some(authority) = authority {
            match repository.publish_metadata_inventory_absence_candidates(
                authority,
                MetadataInventoryAbsencePublicationRequest {
                    run_id: &request.run_id,
                    expected_cursor: absence_cursor.as_deref(),
                    next_cursor: &next_cursor,
                    intents: &intents,
                    updated_unix_ms: observed_unix_ms,
                    policy: queue_policy,
                },
            ) {
                Ok(Some((enqueue, run))) => {
                    apply_enqueue_report(&mut report, enqueue)?;
                    run
                }
                Ok(None) => {
                    repository.terminate_metadata_inventory(
                        &request.run_id,
                        MetadataInventoryRunStatus::Superseded,
                        None,
                        observed_unix_ms,
                    )?;
                    report.is_cancelled = true;
                    return Ok(report);
                }
                Err(error) if error.code == "metadata_inventory_backpressure" => {
                    report.is_backpressured = true;
                    return Ok(report);
                }
                Err(error) => return Err(error),
            }
        } else {
            match enqueue_candidates(
                repository,
                None,
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
                    return Err(ScanError::new(
                        "metadata_inventory_authority_state_invalid",
                        "A manual inventory cannot lose recovery authority",
                    ));
                }
            }
            repository.advance_metadata_inventory_absence_cursor(
                &request.run_id,
                absence_cursor.as_deref(),
                &next_cursor,
                count,
                observed_unix_ms,
            )?
        };
        report.candidate_count = run.candidate_count;
        absence_cursor = Some(next_cursor);
        report_inventory_progress(progress, MetadataInventoryProgressPhase::Comparison);
        if yield_after_work_page {
            yield_inventory_page(page_yield, MetadataInventoryProgressPhase::QueuePublication);
            return Ok(report);
        }
    }

    let run = {
        let _publication_guard = recovery_root_binding
            .as_ref()
            .map(|(root_path, expected_identity)| {
                PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                    root_path,
                    expected_identity,
                )
            })
            .transpose()?;
        repository.complete_metadata_inventory(&request.run_id, observed_unix_ms)?
    };
    report.staged_entry_count = run.staged_entry_count;
    report.candidate_count = run.candidate_count;
    report.is_complete = true;
    Ok(report)
}

fn report_inventory_progress(
    progress: Option<&dyn Fn(MetadataInventoryProgressPhase)>,
    phase: MetadataInventoryProgressPhase,
) {
    if let Some(progress) = progress {
        progress(phase);
    }
}

fn yield_inventory_page(
    page_yield: Option<&dyn Fn(MetadataInventoryProgressPhase)>,
    phase: MetadataInventoryProgressPhase,
) {
    if let Some(page_yield) = page_yield {
        page_yield(phase);
    }
    std::thread::yield_now();
}

fn compare_entry(
    entry: &MetadataInventoryEntry,
    path_prior: Option<&AssetLocationView>,
    terminal_media_evidence: Option<&TerminalMediaEvidence>,
    media_inspector: &impl MediaInspector,
    previous_paths: &BTreeMap<(String, String), String>,
    claimed_identities: &mut BTreeSet<(String, String)>,
) -> EntryComparison {
    if entry.kind != MetadataInventoryEntryKind::File {
        return EntryComparison::Unchanged;
    }
    if path_prior.is_some_and(|prior| inventory_matches_location(entry, prior)) {
        return EntryComparison::Unchanged;
    }
    if terminal_media_evidence.is_some_and(|evidence| {
        inventory_matches_terminal_media_evidence(entry, evidence, media_inspector)
    }) {
        return EntryComparison::Unchanged;
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

fn inventory_matches_terminal_media_evidence(
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

pub(super) fn inventory_matches_location(
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
    apply_enqueue_report(report, enqueue)?;
    Ok(CandidateEnqueueOutcome::Applied)
}

fn apply_enqueue_report(
    report: &mut MetadataInventoryReport,
    enqueue: crate::domain::LibraryChangeEnqueueReport,
) -> Result<(), ScanError> {
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
mod inspection_revision_tests;
#[cfg(test)]
mod tests;

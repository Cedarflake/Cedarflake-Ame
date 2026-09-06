use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{
    FileVisitOutcome, LocalMediaInspector, PublicationGuardedFileDiscovery, user_visible_path,
};
use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, DiscoveredFile, ExpectedFileState, FileIdentityEvidence,
    IncrementalLibraryChangeReport, IncrementalReconciliationDecision,
    IncrementalReconciliationOutcome, LeasedLibraryChange, LibraryChangeCatchUpEvidence,
    LibraryChangeCompletion, LibraryChangeFailure, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, PreviewStatus, ReconciliationFileEvidence,
    ReconciliationObservedState, RetainedPreviewExpectation, ScanError, ScanIssue,
    TerminalMediaEvidence, TerminalMediaEvidenceUpdate,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MediaInspectionFailureKind, MediaInspector,
};

use super::directory_synchronization::reconcile_path_evidence;
use super::metadata_inventory::inventory_matches_location;
use super::scan_library::{stable_id, stable_location_id};

const MAX_CATALOG_REVISION_REBASE_ATTEMPTS: usize = 2;

struct PreparedChange {
    completion: LibraryChangeCompletion,
    mutations: Vec<CatalogDeltaMutation>,
    terminal_media_evidence: Vec<TerminalMediaEvidenceUpdate>,
    revalidation: Vec<RevalidationTarget>,
}

struct PathChangeContext<'a> {
    relative_path: &'a str,
    observed: Option<InspectedPath>,
    candidate_prior: Option<AssetLocationView>,
    may_remove_candidate_prior: bool,
    removals: Vec<String>,
}

enum RevalidationTarget {
    Present {
        relative_path: String,
        expected: ExpectedFileState,
    },
    CatalogAbsent(String),
}

enum InspectedPath {
    File(DiscoveredFile),
    TerminalMedia {
        file: DiscoveredFile,
        issue: LibraryChangeFailure,
        report_issue: bool,
    },
    CatalogAbsent,
    PreservedIssue(LibraryChangeFailure),
    Retry(LibraryChangeFailure),
}

pub fn process_ready_library_changes<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_ready_library_changes_internal(
        repository,
        root_id,
        root_generation,
        PathLeaseSelection::Any,
        now_unix_ms,
        policy,
        None,
    )
}

pub(crate) fn process_ready_library_changes_in_lane<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    lane: LibraryChangeLane,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_ready_library_changes_internal(
        repository,
        root_id,
        root_generation,
        PathLeaseSelection::Lane(lane),
        now_unix_ms,
        policy,
        None,
    )
}

pub(crate) fn process_ready_library_changes_in_lane_cancellable<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    lane: LibraryChangeLane,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_ready_library_changes_internal(
        repository,
        root_id,
        root_generation,
        PathLeaseSelection::Lane(lane),
        now_unix_ms,
        policy,
        Some(cancelled),
    )
}

pub(crate) fn process_ready_metadata_inventory_recovery_candidates_cancellable<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_ready_library_changes_internal(
        repository,
        root_id,
        root_generation,
        PathLeaseSelection::OwnedRecovery,
        now_unix_ms,
        policy,
        Some(cancelled),
    )
}

pub(crate) fn process_ready_unowned_recovery_paths_cancellable<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_ready_library_changes_internal(
        repository,
        root_id,
        root_generation,
        PathLeaseSelection::UnownedRecovery,
        now_unix_ms,
        policy,
        Some(cancelled),
    )
}

#[derive(Clone, Copy)]
enum PathLeaseSelection {
    Any,
    Lane(LibraryChangeLane),
    OwnedRecovery,
    UnownedRecovery,
}

fn process_ready_library_changes_internal<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    lease_selection: PathLeaseSelection,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: Option<&AtomicBool>,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    validate_request(root_id, policy)?;
    let mut report = IncrementalLibraryChangeReport::default();
    if cancellation_requested(cancelled) {
        return Ok(report);
    }
    let Some(root) = repository.load_incremental_catalog_root(root_id)? else {
        return Ok(report);
    };
    report.catalog_revision = root.catalog_revision;
    if root.root_generation != root_generation {
        return Ok(report);
    }
    if root.active_scan_id.is_none() {
        return Ok(report);
    }
    let can_publish_during_scan = matches!(
        lease_selection,
        PathLeaseSelection::Lane(LibraryChangeLane::Live)
    );
    if root.has_running_scan && !can_publish_during_scan {
        return Ok(report);
    }
    if cancellation_requested(cancelled) {
        return Ok(report);
    }
    let leased = match lease_selection {
        PathLeaseSelection::Lane(lane) => repository.lease_path_library_changes_in_lane(
            root_id,
            root_generation,
            lane,
            now_unix_ms,
            policy,
        )?,
        PathLeaseSelection::OwnedRecovery => repository
            .lease_metadata_inventory_recovery_candidates(
                root_id,
                root_generation,
                now_unix_ms,
                policy,
            )?,
        PathLeaseSelection::UnownedRecovery => repository
            .lease_unowned_recovery_path_library_changes(
                root_id,
                root_generation,
                now_unix_ms,
                policy,
            )?,
        PathLeaseSelection::Any => {
            repository.lease_path_library_changes(root_id, root_generation, now_unix_ms, policy)?
        }
    };
    report.leased_count = bounded_count(leased.len(), "leased change count")?;
    if leased.is_empty() {
        return Ok(report);
    }
    let expected_root_identity = if matches!(lease_selection, PathLeaseSelection::OwnedRecovery) {
        let identity = repository
            .load_metadata_inventory_candidate_root_identity(root_id, root_generation)?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_candidate_root_identity_missing",
                    "Owned recovery candidates lack their durable pinned-root identity",
                )
            })?;
        if root
            .publication_root_identity
            .as_ref()
            .is_some_and(|established| established != &identity)
        {
            let issue = failure(
                "root_publication_namespace_conflict",
                "Recovery evidence disagrees with the established configured-root namespace",
            );
            retry_changes(
                repository,
                &leased,
                &issue,
                now_unix_ms,
                policy,
                &mut report,
            )?;
            return Ok(report);
        }
        identity
    } else if let Some(identity) = root.publication_root_identity.clone() {
        identity
    } else {
        let issue = failure(
            "root_publication_namespace_unproven",
            "The configured root has no trustworthy persistent namespace identity",
        );
        retry_changes(
            repository,
            &leased,
            &issue,
            now_unix_ms,
            policy,
            &mut report,
        )?;
        return Ok(report);
    };
    let discovery = match PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        &expected_root_identity,
    ) {
        Ok(discovery) => discovery,
        Err(error) => {
            let issue = scan_failure(error);
            retry_changes(
                repository,
                &leased,
                &issue,
                now_unix_ms,
                policy,
                &mut report,
            )?;
            return Ok(report);
        }
    };
    if cancellation_requested(cancelled) {
        defer_cancelled_leases(repository, &leased, now_unix_ms, &mut report)?;
        return Ok(report);
    }
    let inspector = LocalMediaInspector::new();
    let mut prepared = Vec::new();
    let mut retries = Vec::new();
    for change in &leased {
        if cancellation_requested(cancelled) {
            defer_cancelled_leases(repository, &leased, now_unix_ms, &mut report)?;
            return Ok(report);
        }
        match prepare_change(repository, &discovery, &inspector, change) {
            Ok(change) => prepared.push(change),
            Err(issue) => retries.push((change.clone(), issue)),
        }
    }
    let mut ready = Vec::new();
    for change in prepared {
        if cancellation_requested(cancelled) {
            defer_cancelled_leases(repository, &leased, now_unix_ms, &mut report)?;
            return Ok(report);
        }
        match revalidate_change(&discovery, &change) {
            Ok(()) => ready.push(change),
            Err(issue) => {
                let leased = leased
                    .iter()
                    .find(|leased| leased.change.id == change.completion.change_id)
                    .expect("prepared changes originate from the leased batch")
                    .clone();
                retries.push((leased, issue));
            }
        }
    }
    let mut expected_catalog_revision = root.catalog_revision;
    let mut catalog_revision_rebase_attempts = 0_usize;
    while !ready.is_empty() {
        if cancellation_requested(cancelled) {
            defer_cancelled_leases(repository, &leased, now_unix_ms, &mut report)?;
            return Ok(report);
        }
        discovery.require_metadata_inventory_root_identity(&expected_root_identity)?;
        let batch = CatalogDeltaBatch {
            root_id: root_id.to_owned(),
            root_generation,
            expected_catalog_revision,
            mutations: ready
                .iter()
                .flat_map(|change| change.mutations.clone())
                .collect(),
            terminal_media_evidence: ready
                .iter()
                .flat_map(|change| change.terminal_media_evidence.clone())
                .collect(),
            completions: ready
                .iter()
                .map(|change| change.completion.clone())
                .collect(),
        };
        let publication = match repository.publish_catalog_delta(&batch, now_unix_ms) {
            Ok(publication) => publication,
            Err(error) if is_publication_namespace_failure(&error) => {
                let issue = failure(error.code, error.message);
                for change in &ready {
                    let leased = leased
                        .iter()
                        .find(|leased| leased.change.id == change.completion.change_id)
                        .expect("prepared changes originate from the leased batch");
                    retry_changes(
                        repository,
                        std::slice::from_ref(leased),
                        &issue,
                        now_unix_ms,
                        policy,
                        &mut report,
                    )?;
                }
                return Ok(report);
            }
            Err(error) => return Err(error),
        };
        report.catalog_revision = publication.catalog_revision;
        match publication.status {
            CatalogDeltaPublicationStatus::Applied => {
                report.completed_count = report
                    .completed_count
                    .checked_add(publication.completed_change_count)
                    .ok_or_else(|| count_overflow("completed change count"))?;
                report.applied_mutation_count = report
                    .applied_mutation_count
                    .checked_add(publication.applied_mutation_count)
                    .ok_or_else(|| count_overflow("applied mutation count"))?;
                break;
            }
            CatalogDeltaPublicationStatus::RootScanInProgress
            | CatalogDeltaPublicationStatus::NoPublishedCatalog => {
                defer_changes(repository, &ready, now_unix_ms, &mut report)?;
                break;
            }
            CatalogDeltaPublicationStatus::StaleCatalogRevision
                if catalog_revision_rebase_attempts < MAX_CATALOG_REVISION_REBASE_ATTEMPTS =>
            {
                catalog_revision_rebase_attempts += 1;
                let Some(latest_root) = repository.load_incremental_catalog_root(root_id)? else {
                    let issue =
                        publication_failure(CatalogDeltaPublicationStatus::RootGenerationChanged);
                    for change in &ready {
                        let leased = leased
                            .iter()
                            .find(|leased| leased.change.id == change.completion.change_id)
                            .expect("prepared changes originate from the leased batch")
                            .clone();
                        retries.push((leased, issue.clone()));
                    }
                    break;
                };
                if latest_root.root_generation != root_generation
                    || latest_root.root_path != root.root_path
                    || latest_root.active_scan_id.is_none()
                    || latest_root.has_running_scan && !can_publish_during_scan
                {
                    let status = if latest_root.root_generation != root_generation
                        || latest_root.root_path != root.root_path
                    {
                        CatalogDeltaPublicationStatus::RootGenerationChanged
                    } else if latest_root.has_running_scan && !can_publish_during_scan {
                        CatalogDeltaPublicationStatus::RootScanInProgress
                    } else {
                        CatalogDeltaPublicationStatus::NoPublishedCatalog
                    };
                    let issue = publication_failure(status);
                    for change in &ready {
                        let leased = leased
                            .iter()
                            .find(|leased| leased.change.id == change.completion.change_id)
                            .expect("prepared changes originate from the leased batch")
                            .clone();
                        retries.push((leased, issue.clone()));
                    }
                    break;
                }
                expected_catalog_revision = latest_root.catalog_revision;
                let mut refreshed_ready = Vec::with_capacity(ready.len());
                for change in &ready {
                    if cancellation_requested(cancelled) {
                        defer_cancelled_leases(repository, &leased, now_unix_ms, &mut report)?;
                        return Ok(report);
                    }
                    let leased = leased
                        .iter()
                        .find(|leased| leased.change.id == change.completion.change_id)
                        .expect("prepared changes originate from the leased batch")
                        .clone();
                    match prepare_change(repository, &discovery, &inspector, &leased).and_then(
                        |prepared| {
                            revalidate_change(&discovery, &prepared)?;
                            Ok(prepared)
                        },
                    ) {
                        Ok(prepared) => refreshed_ready.push(prepared),
                        Err(issue) => retries.push((leased, issue)),
                    }
                }
                ready = refreshed_ready;
            }
            status => {
                let issue = publication_failure(status);
                for change in &ready {
                    let leased = leased
                        .iter()
                        .find(|leased| leased.change.id == change.completion.change_id)
                        .expect("prepared changes originate from the leased batch")
                        .clone();
                    retries.push((leased, issue.clone()));
                }
                break;
            }
        }
    }
    for (change, issue) in retries {
        retry_changes(
            repository,
            std::slice::from_ref(&change),
            &issue,
            now_unix_ms,
            policy,
            &mut report,
        )?;
    }
    Ok(report)
}

fn cancellation_requested(cancelled: Option<&AtomicBool>) -> bool {
    cancelled.is_some_and(|cancelled| cancelled.load(Ordering::Acquire))
}

fn defer_cancelled_leases<Repository>(
    repository: &mut Repository,
    leased: &[LeasedLibraryChange],
    now_unix_ms: i64,
    report: &mut IncrementalLibraryChangeReport,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    let identities = leased
        .iter()
        .map(crate::domain::LibraryChangeLeaseIdentity::from)
        .collect::<Vec<_>>();
    let outcomes = repository.defer_library_changes(&identities, now_unix_ms)?;
    if outcomes.len() != identities.len() {
        return Err(ScanError::new(
            "change_queue_deferral_outcome_invalid",
            "The queue must return exactly one outcome for each deferred lease",
        ));
    }
    for outcome in outcomes {
        record_lease_deferral(outcome, report)?;
    }
    Ok(())
}

pub(super) struct AuthoritativePathSetContext<'a> {
    pub(super) root_id: &'a str,
    pub(super) root_generation: LibraryRootGeneration,
    pub(super) expected_catalog_revision: u64,
    pub(super) expected_root_identity: &'a FileIdentityEvidence,
    pub(super) leased: &'a LeasedLibraryChange,
    pub(super) relative_paths: &'a [String],
    pub(super) now_unix_ms: i64,
    pub(super) queue_policy: LibraryChangeQueuePolicy,
    pub(super) cancellation: &'a AtomicBool,
}

pub(super) fn process_authoritative_path_set<Repository>(
    repository: &mut Repository,
    context: AuthoritativePathSetContext<'_>,
    discovery: &PublicationGuardedFileDiscovery,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    process_authoritative_path_set_guarded(
        repository,
        AuthoritativePathSetRequest {
            root_id: context.root_id,
            root_generation: context.root_generation,
            expected_catalog_revision: context.expected_catalog_revision,
            expected_root_identity: context.expected_root_identity,
            discovery,
            leased: context.leased,
            relative_paths: context.relative_paths,
            now_unix_ms: context.now_unix_ms,
            queue_policy: context.queue_policy,
            cancellation: context.cancellation,
        },
    )
}

struct AuthoritativePathSetRequest<'a> {
    root_id: &'a str,
    root_generation: LibraryRootGeneration,
    expected_catalog_revision: u64,
    expected_root_identity: &'a FileIdentityEvidence,
    discovery: &'a PublicationGuardedFileDiscovery,
    leased: &'a LeasedLibraryChange,
    relative_paths: &'a [String],
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    cancellation: &'a AtomicBool,
}

fn process_authoritative_path_set_guarded<Repository>(
    repository: &mut Repository,
    request: AuthoritativePathSetRequest<'_>,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    validate_request(request.root_id, request.queue_policy)?;
    if request.cancellation.load(Ordering::Relaxed) {
        return defer_authoritative_path_set(repository, &request);
    }
    let can_publish_during_scan =
        request.leased.change.intent.origin.lane() == LibraryChangeLane::Live;
    let root_is_current = matches!(
        repository.load_incremental_catalog_root(request.root_id)?,
        Some(root)
            if root.root_generation == request.root_generation
                && root.active_scan_id.is_some()
                && (!root.has_running_scan || can_publish_during_scan)
                && root.publication_root_identity.as_ref()
                    == Some(request.expected_root_identity)
    );
    if !root_is_current {
        let mut report = IncrementalLibraryChangeReport {
            leased_count: 1,
            catalog_revision: request.expected_catalog_revision,
            ..IncrementalLibraryChangeReport::default()
        };
        defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
        return Ok(report);
    }
    if let Err(error) = request
        .discovery
        .require_metadata_inventory_root_identity(request.expected_root_identity)
    {
        let mut report = IncrementalLibraryChangeReport {
            leased_count: 1,
            catalog_revision: request.expected_catalog_revision,
            ..IncrementalLibraryChangeReport::default()
        };
        retry_changes(
            repository,
            std::slice::from_ref(request.leased),
            &failure(error.code, error.message),
            request.now_unix_ms,
            request.queue_policy,
            &mut report,
        )?;
        return Ok(report);
    }
    let inspector = LocalMediaInspector::new();
    let mut combined = PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: request.leased.change.id,
            lease_generation: request.leased.lease_generation,
            issue: None,
        },
        mutations: Vec::new(),
        terminal_media_evidence: Vec::new(),
        revalidation: Vec::new(),
    };
    for relative_path in request.relative_paths {
        if request.cancellation.load(Ordering::Relaxed) {
            return defer_authoritative_path_set(repository, &request);
        }
        let prepared = match prepare_path_change(
            repository,
            request.discovery,
            &inspector,
            request.leased,
            PathChangeContext {
                relative_path,
                observed: None,
                candidate_prior: None,
                may_remove_candidate_prior: false,
                removals: Vec::new(),
            },
        ) {
            Ok(prepared) => prepared,
            Err(issue) => {
                let mut report = IncrementalLibraryChangeReport {
                    leased_count: 1,
                    catalog_revision: request.expected_catalog_revision,
                    ..IncrementalLibraryChangeReport::default()
                };
                retry_changes(
                    repository,
                    std::slice::from_ref(request.leased),
                    &issue,
                    request.now_unix_ms,
                    request.queue_policy,
                    &mut report,
                )?;
                return Ok(report);
            }
        };
        if combined.completion.issue.is_none() {
            combined.completion.issue = prepared.completion.issue;
        }
        combined.mutations.extend(prepared.mutations);
        combined
            .terminal_media_evidence
            .extend(prepared.terminal_media_evidence);
        combined.revalidation.extend(prepared.revalidation);
    }
    let mut report = IncrementalLibraryChangeReport {
        leased_count: 1,
        catalog_revision: request.expected_catalog_revision,
        ..IncrementalLibraryChangeReport::default()
    };
    if request.cancellation.load(Ordering::Relaxed) {
        defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
        return Ok(report);
    }
    if let Err(issue) = revalidate_change(request.discovery, &combined) {
        retry_changes(
            repository,
            std::slice::from_ref(request.leased),
            &issue,
            request.now_unix_ms,
            request.queue_policy,
            &mut report,
        )?;
        return Ok(report);
    }
    if request.cancellation.load(Ordering::Relaxed) {
        defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
        return Ok(report);
    }
    if let Err(error) = request
        .discovery
        .require_metadata_inventory_root_identity(request.expected_root_identity)
    {
        retry_changes(
            repository,
            std::slice::from_ref(request.leased),
            &failure(error.code, error.message),
            request.now_unix_ms,
            request.queue_policy,
            &mut report,
        )?;
        return Ok(report);
    }
    let batch = CatalogDeltaBatch {
        root_id: request.root_id.to_owned(),
        root_generation: request.root_generation,
        expected_catalog_revision: request.expected_catalog_revision,
        mutations: combined.mutations.clone(),
        terminal_media_evidence: combined.terminal_media_evidence.clone(),
        completions: vec![combined.completion.clone()],
    };
    let publication = match repository.publish_catalog_delta(&batch, request.now_unix_ms) {
        Ok(publication) => publication,
        Err(error) if is_publication_namespace_failure(&error) => {
            retry_changes(
                repository,
                std::slice::from_ref(request.leased),
                &failure(error.code, error.message),
                request.now_unix_ms,
                request.queue_policy,
                &mut report,
            )?;
            return Ok(report);
        }
        Err(error) => return Err(error),
    };
    report.catalog_revision = publication.catalog_revision;
    match publication.status {
        CatalogDeltaPublicationStatus::Applied => {
            report.completed_count = publication.completed_change_count;
            report.applied_mutation_count = publication.applied_mutation_count;
        }
        CatalogDeltaPublicationStatus::RootScanInProgress
        | CatalogDeltaPublicationStatus::NoPublishedCatalog => {
            defer_changes(repository, &[combined], request.now_unix_ms, &mut report)?;
        }
        status => {
            retry_changes(
                repository,
                std::slice::from_ref(request.leased),
                &publication_failure(status),
                request.now_unix_ms,
                request.queue_policy,
                &mut report,
            )?;
        }
    }
    Ok(report)
}

fn defer_authoritative_path_set<Repository>(
    repository: &mut Repository,
    request: &AuthoritativePathSetRequest<'_>,
) -> Result<IncrementalLibraryChangeReport, ScanError>
where
    Repository: LibraryChangeQueue,
{
    let mut report = IncrementalLibraryChangeReport {
        leased_count: 1,
        catalog_revision: request.expected_catalog_revision,
        ..IncrementalLibraryChangeReport::default()
    };
    defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
    Ok(report)
}

fn defer_leased_change<Repository>(
    repository: &mut Repository,
    leased: &LeasedLibraryChange,
    now_unix_ms: i64,
    report: &mut IncrementalLibraryChangeReport,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    record_lease_deferral(
        repository.defer_library_change(leased.change.id, leased.lease_generation, now_unix_ms)?,
        report,
    )
}

fn record_lease_deferral(
    outcome: LibraryChangeLeaseUpdateOutcome,
    report: &mut IncrementalLibraryChangeReport,
) -> Result<(), ScanError> {
    match outcome {
        LibraryChangeLeaseUpdateOutcome::Applied => {
            report.deferred_count = report
                .deferred_count
                .checked_add(1)
                .ok_or_else(|| count_overflow("deferred change count"))?;
        }
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => {
            report.superseded_count = report
                .superseded_count
                .checked_add(1)
                .ok_or_else(|| count_overflow("superseded change count"))?;
        }
    }
    Ok(())
}

fn prepare_change<Repository>(
    repository: &Repository,
    discovery: &PublicationGuardedFileDiscovery,
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
) -> Result<PreparedChange, LibraryChangeFailure>
where
    Repository: IncrementalCatalogRepository,
{
    let intent = &leased.change.intent;
    if intent.kind == LibraryChangeIntentKind::FreshnessUnknown
        || intent.scope != LibraryChangeScope::Path
    {
        return Err(failure(
            "incremental_scope_requires_reconciliation",
            "Subtree and root freshness work remains pending for authoritative reconciliation",
        ));
    }
    match intent.kind {
        LibraryChangeIntentKind::Reconcile if intent.previous_relative_path.is_none() => {
            prepare_path_change(
                repository,
                discovery,
                inspector,
                leased,
                PathChangeContext {
                    relative_path: &intent.relative_path,
                    observed: None,
                    candidate_prior: None,
                    may_remove_candidate_prior: false,
                    removals: Vec::new(),
                },
            )
        }
        LibraryChangeIntentKind::Reconcile | LibraryChangeIntentKind::RenameCandidate => {
            let previous_path = intent.previous_relative_path.as_deref().ok_or_else(|| {
                failure(
                    "incremental_rename_previous_path_missing",
                    "A paired rename requires its previous relative path",
                )
            })?;
            let previous_prior = repository
                .load_incremental_location_by_relative_path(&intent.root_id, previous_path)
                .map_err(scan_failure)?;
            let previous_observed = inspect_path(discovery, previous_path);
            let previous_identity = match &previous_observed {
                InspectedPath::File(file) => file.file_identity.clone(),
                InspectedPath::TerminalMedia { file, .. } => file.file_identity.clone(),
                _ => None,
            };
            let previous_is_absent = match &previous_observed {
                InspectedPath::CatalogAbsent => true,
                InspectedPath::File(_) | InspectedPath::TerminalMedia { .. } => false,
                InspectedPath::PreservedIssue(issue) | InspectedPath::Retry(issue) => {
                    return Err(issue.clone());
                }
            };
            let mut previous = prepare_path_change(
                repository,
                discovery,
                inspector,
                leased,
                PathChangeContext {
                    relative_path: previous_path,
                    observed: Some(previous_observed),
                    candidate_prior: None,
                    may_remove_candidate_prior: false,
                    removals: Vec::new(),
                },
            )?;
            let mut current = prepare_path_change(
                repository,
                discovery,
                inspector,
                leased,
                PathChangeContext {
                    relative_path: &intent.relative_path,
                    observed: None,
                    candidate_prior: previous_prior.clone(),
                    may_remove_candidate_prior: previous_is_absent,
                    removals: Vec::new(),
                },
            )?;
            let current_identity = current
                .mutations
                .iter()
                .find_map(|mutation| mutation.upsert_location.as_ref())
                .and_then(|location| location.file_identity.clone());
            if previous_is_absent
                && current
                    .mutations
                    .iter()
                    .any(|mutation| mutation.upsert_location.is_some())
            {
                previous.mutations.clear();
            }
            if windows_case_alias(previous_path, &intent.relative_path)
                && previous_identity.is_some()
                && previous_identity == current_identity
            {
                previous.mutations.clear();
                if let Some(prior) = &previous_prior {
                    for mutation in &mut current.mutations {
                        if mutation.upsert_location.is_some() {
                            push_unique(
                                &mut mutation.remove_location_ids,
                                prior.location_id.clone(),
                            );
                        }
                    }
                }
            }
            previous.mutations.append(&mut current.mutations);
            previous.revalidation.append(&mut current.revalidation);
            if previous.completion.issue.is_none() {
                previous.completion.issue = current.completion.issue;
            }
            Ok(previous)
        }
        LibraryChangeIntentKind::FreshnessUnknown => unreachable!("handled above"),
    }
}

fn prepare_path_change<Repository>(
    repository: &Repository,
    discovery: &PublicationGuardedFileDiscovery,
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
    context: PathChangeContext<'_>,
) -> Result<PreparedChange, LibraryChangeFailure>
where
    Repository: IncrementalCatalogRepository,
{
    let PathChangeContext {
        relative_path,
        observed,
        candidate_prior,
        may_remove_candidate_prior,
        mut removals,
    } = context;
    let intent = &leased.change.intent;
    let catch_up_lineage = leased_catch_up_lineage(leased)?;
    let path_prior = repository
        .load_incremental_location_by_relative_path(&intent.root_id, relative_path)
        .map_err(scan_failure)?;
    if observed.is_none()
        && intent.origin == crate::domain::LibraryChangeOrigin::MetadataInventory
        && intent.kind == crate::domain::LibraryChangeIntentKind::Reconcile
        && intent.scope == crate::domain::LibraryChangeScope::Path
        && intent.previous_relative_path.is_none()
        && let Some(prior) = path_prior.as_ref()
        && inspection_metadata_is_compatible(prior, inspector)
        && let Ok(entry) = discovery.metadata_inventory_entry(relative_path)
        && let Some(change) =
            metadata_inventory_dirty_path_change(leased, prior, &entry, removals.clone())
    {
        return Ok(change);
    }
    let observed = observed.unwrap_or_else(|| inspect_path(discovery, relative_path));
    if let InspectedPath::TerminalMedia {
        file,
        issue,
        report_issue,
    } = observed
    {
        let identity_prior = file
            .file_identity
            .as_ref()
            .map(|identity| {
                repository.load_incremental_location_by_file_identity(identity, catch_up_lineage)
            })
            .transpose()
            .map_err(scan_failure)?
            .flatten();
        let selected_prior = select_prior(
            path_prior.as_ref(),
            identity_prior.as_ref(),
            candidate_prior.as_ref(),
            &file,
        )
        .cloned();
        if let Some(prior) = path_prior.as_ref() {
            push_unique(&mut removals, prior.location_id.clone());
        }
        if may_remove_candidate_prior && let Some(prior) = candidate_prior.as_ref() {
            push_unique(&mut removals, prior.location_id.clone());
        }
        return Ok(terminal_media_change(
            inspector,
            leased,
            file,
            selected_prior.as_ref(),
            issue,
            report_issue,
            removals,
        ));
    }
    let mut completion_issue = None;
    let mut revalidation = Vec::new();
    let (decision, current_file, selected_prior) = match observed {
        InspectedPath::File(file) => {
            let identity_prior = file
                .file_identity
                .as_ref()
                .map(|identity| {
                    repository
                        .load_incremental_location_by_file_identity(identity, catch_up_lineage)
                })
                .transpose()
                .map_err(scan_failure)?
                .flatten();
            let selected_prior = select_prior(
                path_prior.as_ref(),
                identity_prior.as_ref(),
                candidate_prior.as_ref(),
                &file,
            )
            .cloned();
            if candidate_prior
                .as_ref()
                .and_then(|prior| prior.file_identity.as_ref())
                .is_some()
                && file.file_identity.is_none()
            {
                return Err(failure(
                    "incremental_current_identity_unavailable",
                    "A paired rename cannot publish while current file identity is unavailable",
                ));
            }
            let mut decision = reconcile_path_evidence(
                selected_prior.as_ref().map(location_evidence).as_ref(),
                ReconciliationObservedState::Present(file_evidence(&file)),
            );
            let dirty_reconcile_outcome = decision.outcome
                == IncrementalReconciliationOutcome::Unchanged
                || intent.previous_relative_path.is_some()
                    && decision.outcome == IncrementalReconciliationOutcome::RenamedOrMoved;
            if selected_prior.is_some()
                && matches!(
                    intent.origin,
                    crate::domain::LibraryChangeOrigin::LiveNotification
                        | crate::domain::LibraryChangeOrigin::StartupCatchUp
                        | crate::domain::LibraryChangeOrigin::MetadataInventory
                )
                && intent.kind == crate::domain::LibraryChangeIntentKind::Reconcile
                && intent.scope == crate::domain::LibraryChangeScope::Path
                && dirty_reconcile_outcome
            {
                decision.outcome = IncrementalReconciliationOutcome::Modified;
                decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
                if let Some(current) = decision.current.as_mut() {
                    current.source_generation = 0;
                }
            }
            if intent.origin == crate::domain::LibraryChangeOrigin::MetadataInventory
                && intent.kind == crate::domain::LibraryChangeIntentKind::RenameCandidate
                && decision.outcome == IncrementalReconciliationOutcome::RenamedOrMoved
            {
                decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
                if let Some(current) = decision.current.as_mut() {
                    current.source_generation = 0;
                }
            }
            if selected_prior
                .as_ref()
                .is_some_and(|prior| !inspection_metadata_is_compatible(prior, inspector))
            {
                match decision.outcome {
                    IncrementalReconciliationOutcome::Unchanged => {
                        decision.outcome = IncrementalReconciliationOutcome::Modified;
                        decision.evidence_disposition =
                            DerivedEvidenceDisposition::InvalidateDerived;
                    }
                    IncrementalReconciliationOutcome::RenamedOrMoved => {
                        decision.evidence_disposition =
                            DerivedEvidenceDisposition::InvalidateDerived;
                    }
                    _ => {}
                }
            }
            (decision, Some(file), selected_prior)
        }
        InspectedPath::CatalogAbsent => {
            let decision = reconcile_path_evidence(
                path_prior.as_ref().map(location_evidence).as_ref(),
                ReconciliationObservedState::Missing {
                    relative_path: relative_path.to_owned(),
                    is_authoritative: true,
                },
            );
            revalidation.push(RevalidationTarget::CatalogAbsent(relative_path.to_owned()));
            (decision, None, path_prior.clone())
        }
        InspectedPath::TerminalMedia { .. } => unreachable!("handled above"),
        InspectedPath::PreservedIssue(issue) => return Err(issue),
        InspectedPath::Retry(issue) => return Err(issue),
    };
    match decision.outcome {
        IncrementalReconciliationOutcome::RetryableFailure => {
            return Err(failure(
                decision
                    .issue_code
                    .clone()
                    .unwrap_or_else(|| "incremental_reconciliation_retry".to_owned()),
                "Incremental reconciliation requires newer trustworthy evidence",
            ));
        }
        IncrementalReconciliationOutcome::TerminalIssue => {
            completion_issue = Some(failure(
                decision
                    .issue_code
                    .clone()
                    .unwrap_or_else(|| "incremental_reconciliation_issue".to_owned()),
                "Incremental reconciliation preserved the last trustworthy catalog",
            ));
        }
        _ => {}
    }

    let mutation = match decision.outcome {
        IncrementalReconciliationOutcome::Unchanged => {
            let identity_backfill = current_file.as_ref().is_some_and(|file| {
                file.file_identity.is_some()
                    && selected_prior.as_ref().is_some_and(|prior| {
                        prior.file_identity.is_none() && prior.relative_path == file.relative_path
                    })
            });
            if identity_backfill {
                let file = current_file.as_ref().ok_or_else(|| {
                    failure(
                        "incremental_current_file_missing",
                        "Identity backfill requires current file evidence",
                    )
                })?;
                let prior = selected_prior.as_ref().ok_or_else(|| {
                    failure(
                        "incremental_identity_backfill_prior_missing",
                        "Identity backfill requires the prior catalog location",
                    )
                })?;
                let mut built = match build_location(
                    discovery,
                    inspector,
                    leased,
                    file,
                    &decision,
                    Some(prior),
                ) {
                    Ok(built) => built,
                    Err(BuildLocationFailure::Retry(issue)) => return Err(issue),
                    Err(BuildLocationFailure::Terminal(issue)) => {
                        return Ok(terminal_media_change(
                            inspector,
                            leased,
                            file.clone(),
                            selected_prior.as_ref(),
                            issue,
                            true,
                            removals,
                        ));
                    }
                };
                built.location.location_id.clone_from(&prior.location_id);
                revalidation.push(RevalidationTarget::Present {
                    relative_path: relative_path.to_owned(),
                    expected: expected_state(file),
                });
                Some(CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: decision.outcome,
                    evidence_disposition: decision.evidence_disposition,
                    remove_location_ids: removals,
                    upsert_location: Some(built.location),
                    retained_preview_expectation: selected_prior
                        .as_ref()
                        .map(retained_preview_expectation),
                })
            } else {
                (!removals.is_empty()).then_some(CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Removed,
                    evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
                    remove_location_ids: removals,
                    upsert_location: None,
                    retained_preview_expectation: None,
                })
            }
        }
        IncrementalReconciliationOutcome::Skipped
        | IncrementalReconciliationOutcome::RetryableFailure
        | IncrementalReconciliationOutcome::TerminalIssue => None,
        IncrementalReconciliationOutcome::Removed => {
            if let Some(prior) = selected_prior.as_ref().or(path_prior.as_ref()) {
                push_unique(&mut removals, prior.location_id.clone());
            }
            Some(CatalogDeltaMutation {
                change_id: leased.change.id,
                outcome: decision.outcome,
                evidence_disposition: decision.evidence_disposition,
                remove_location_ids: removals,
                upsert_location: None,
                retained_preview_expectation: None,
            })
        }
        IncrementalReconciliationOutcome::Added
        | IncrementalReconciliationOutcome::Modified
        | IncrementalReconciliationOutcome::RenamedOrMoved
        | IncrementalReconciliationOutcome::Replaced => {
            let file = current_file.as_ref().ok_or_else(|| {
                failure(
                    "incremental_current_file_missing",
                    "A catalog upsert requires current file evidence",
                )
            })?;
            if let Some(path_prior) = &path_prior {
                push_unique(&mut removals, path_prior.location_id.clone());
            }
            if may_remove_candidate_prior && let Some(candidate) = &candidate_prior {
                push_unique(&mut removals, candidate.location_id.clone());
            }
            let built = match build_location(
                discovery,
                inspector,
                leased,
                file,
                &decision,
                selected_prior.as_ref(),
            ) {
                Ok(built) => built,
                Err(BuildLocationFailure::Retry(issue)) => return Err(issue),
                Err(BuildLocationFailure::Terminal(issue)) => {
                    return Ok(terminal_media_change(
                        inspector,
                        leased,
                        file.clone(),
                        selected_prior.as_ref(),
                        issue,
                        true,
                        removals,
                    ));
                }
            };
            if completion_issue.is_none() {
                completion_issue = built.issue;
            }
            revalidation.push(RevalidationTarget::Present {
                relative_path: relative_path.to_owned(),
                expected: expected_state(file),
            });
            Some(CatalogDeltaMutation {
                change_id: leased.change.id,
                outcome: decision.outcome,
                evidence_disposition: decision.evidence_disposition,
                remove_location_ids: removals,
                upsert_location: Some(built.location),
                retained_preview_expectation: (decision.evidence_disposition
                    == DerivedEvidenceDisposition::RetainCompatible)
                    .then(|| selected_prior.as_ref().map(retained_preview_expectation))
                    .flatten(),
            })
        }
    };
    Ok(PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: leased.change.id,
            lease_generation: leased.lease_generation,
            issue: completion_issue,
        },
        mutations: mutation.into_iter().collect(),
        terminal_media_evidence: Vec::new(),
        revalidation,
    })
}

fn terminal_media_change(
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
    file: DiscoveredFile,
    prior: Option<&AssetLocationView>,
    issue: LibraryChangeFailure,
    report_issue: bool,
    removals: Vec<String>,
) -> PreparedChange {
    let relative_path = file.relative_path.clone();
    let expected = expected_state(&file);
    let retains_asset_identity = prior.is_some_and(|prior| {
        file.file_identity.is_some() && prior.file_identity == file.file_identity
    });
    let terminal_location = AssetLocationView {
        asset_id: if retains_asset_identity {
            prior
                .expect("retained identity requires a prior asset")
                .asset_id
                .clone()
        } else {
            incremental_asset_id(leased, &file)
        },
        location_id: stable_location_id(&leased.change.intent.root_id, &file.relative_path),
        root_id: leased.change.intent.root_id.clone(),
        scan_id: prior.map_or_else(String::new, |prior| prior.scan_id.clone()),
        absolute_path: file.absolute_path.clone(),
        display_path: user_visible_path(&file.absolute_path),
        relative_path: file.relative_path.clone(),
        preview_path: String::new(),
        file_size: file.file_size,
        created_unix_ms: file.created_unix_ms,
        modified_unix_ms: file.modified_unix_ms,
        file_identity: file.file_identity.clone(),
        source_revision: file.source_revision.clone(),
        source_generation: 0,
        width: 0,
        height: 0,
        preview_status: PreviewStatus::Failed,
        preview_issue_code: Some(issue.code.clone()),
        preview_issue_message: Some(issue.message.clone()),
        metadata_engine_id: inspector.inspection_engine_id().to_owned(),
        metadata_engine_version: inspector.inspection_engine_version().to_string(),
        capture_time: None,
    };
    let mutation = CatalogDeltaMutation {
        change_id: leased.change.id,
        outcome: IncrementalReconciliationOutcome::TerminalIssue,
        evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
        remove_location_ids: removals,
        upsert_location: Some(terminal_location),
        retained_preview_expectation: None,
    };
    PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: leased.change.id,
            lease_generation: leased.lease_generation,
            issue: report_issue.then(|| issue.clone()),
        },
        mutations: vec![mutation],
        terminal_media_evidence: vec![TerminalMediaEvidenceUpdate {
            change_id: leased.change.id,
            evidence: TerminalMediaEvidence {
                relative_path: file.relative_path.clone(),
                file_size: file.file_size,
                modified_unix_ms: file.modified_unix_ms,
                file_identity: file.file_identity.clone(),
                source_revision: file.source_revision.clone(),
                source_generation: 0,
                inspection_engine_id: inspector.inspection_engine_id().to_owned(),
                inspection_engine_version: inspector.inspection_engine_version(),
                issue,
            },
        }],
        revalidation: vec![RevalidationTarget::Present {
            relative_path,
            expected,
        }],
    }
}

fn metadata_inventory_dirty_path_change(
    leased: &LeasedLibraryChange,
    prior: &AssetLocationView,
    entry: &MetadataInventoryEntry,
    mut removals: Vec<String>,
) -> Option<PreparedChange> {
    if entry.kind != MetadataInventoryEntryKind::File
        || entry.placeholder_state != MetadataInventoryPlaceholderState::Available
        || !inventory_matches_location(entry, prior)
    {
        return None;
    }
    let file_size = entry.file_size?;
    let mut current = prior.clone();
    current.file_size = file_size;
    current.modified_unix_ms = entry.modified_unix_ms;
    current.file_identity.clone_from(&entry.file_identity);
    current.source_revision.clone_from(&entry.source_revision);
    current.source_generation = 0;
    current.metadata_engine_id = super::INVALIDATED_MEDIA_METADATA_ENGINE_ID.to_owned();
    current.metadata_engine_version = super::INVALIDATED_MEDIA_METADATA_ENGINE_VERSION.to_owned();
    current.capture_time = None;
    current.preview_path.clear();
    current.preview_status = PreviewStatus::Pending;
    current.preview_issue_code = None;
    current.preview_issue_message = None;
    push_unique(&mut removals, prior.location_id.clone());
    let expected = ExpectedFileState {
        absolute_path: prior.absolute_path.clone(),
        file_size,
        modified_unix_ms: entry.modified_unix_ms,
        file_identity: entry.file_identity.clone(),
        source_revision: entry.source_revision.clone(),
    };
    Some(PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: leased.change.id,
            lease_generation: leased.lease_generation,
            issue: None,
        },
        mutations: vec![CatalogDeltaMutation {
            change_id: leased.change.id,
            outcome: IncrementalReconciliationOutcome::Modified,
            evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
            remove_location_ids: removals,
            upsert_location: Some(current),
            retained_preview_expectation: None,
        }],
        terminal_media_evidence: Vec::new(),
        revalidation: vec![RevalidationTarget::Present {
            relative_path: entry.relative_path.clone(),
            expected,
        }],
    })
}

struct BuiltLocation {
    location: AssetLocationView,
    issue: Option<LibraryChangeFailure>,
}

enum BuildLocationFailure {
    Retry(LibraryChangeFailure),
    Terminal(LibraryChangeFailure),
}

fn build_location(
    discovery: &PublicationGuardedFileDiscovery,
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
    file: &DiscoveredFile,
    decision: &IncrementalReconciliationDecision,
    prior: Option<&AssetLocationView>,
) -> Result<BuiltLocation, BuildLocationFailure> {
    let retains_compatible = decision.evidence_disposition
        == DerivedEvidenceDisposition::RetainCompatible
        && prior.is_some_and(|prior| inspection_metadata_is_compatible(prior, inspector));
    let (width, height, metadata_engine_id, metadata_engine_version, capture_time, issue) =
        if retains_compatible {
            let prior = prior.expect("compatible evidence requires a prior location");
            (
                prior.width,
                prior.height,
                prior.metadata_engine_id.clone(),
                prior.metadata_engine_version.clone(),
                prior.capture_time.clone(),
                file.issues.first().map(issue_failure),
            )
        } else {
            let inspection = discovery
                .inspect_media(inspector, file)
                .map_err(|failure| match failure.kind {
                    MediaInspectionFailureKind::Retryable => {
                        BuildLocationFailure::Retry(issue_failure(&failure.issue))
                    }
                    MediaInspectionFailureKind::Terminal => {
                        BuildLocationFailure::Terminal(issue_failure(&failure.issue))
                    }
                })?;
            let issue = file
                .issues
                .first()
                .map(issue_failure)
                .or_else(|| inspection.metadata.issues.first().map(issue_failure));
            (
                inspection.width,
                inspection.height,
                inspection.metadata.engine_id,
                inspection.metadata.engine_version,
                inspection.metadata.capture_time,
                issue,
            )
        };
    let asset_id = match decision.outcome {
        IncrementalReconciliationOutcome::Unchanged
        | IncrementalReconciliationOutcome::Modified
        | IncrementalReconciliationOutcome::RenamedOrMoved => {
            prior.map(|prior| prior.asset_id.clone()).ok_or_else(|| {
                BuildLocationFailure::Retry(failure(
                    "incremental_prior_asset_missing",
                    "Identity-preserving reconciliation requires a prior asset",
                ))
            })?
        }
        IncrementalReconciliationOutcome::Added | IncrementalReconciliationOutcome::Replaced => {
            incremental_asset_id(leased, file)
        }
        _ => {
            return Err(BuildLocationFailure::Retry(failure(
                "incremental_upsert_outcome_invalid",
                "The reconciliation outcome cannot create a catalog location",
            )));
        }
    };
    let (preview_path, preview_status, preview_issue_code, preview_issue_message) =
        if retains_compatible {
            let prior = prior.expect("compatible evidence requires a prior location");
            (
                prior.preview_path.clone(),
                prior.preview_status.clone(),
                prior.preview_issue_code.clone(),
                prior.preview_issue_message.clone(),
            )
        } else {
            (String::new(), PreviewStatus::Pending, None, None)
        };
    Ok(BuiltLocation {
        location: AssetLocationView {
            asset_id,
            location_id: stable_location_id(&leased.change.intent.root_id, &file.relative_path),
            root_id: leased.change.intent.root_id.clone(),
            scan_id: prior.map_or_else(String::new, |prior| prior.scan_id.clone()),
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path.clone(),
            preview_path,
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: file.file_identity.clone(),
            source_revision: file.source_revision.clone(),
            source_generation: decision
                .current
                .as_ref()
                .map_or(0, |current| current.source_generation),
            width,
            height,
            preview_status,
            preview_issue_code,
            preview_issue_message,
            metadata_engine_id,
            metadata_engine_version,
            capture_time,
        },
        issue,
    })
}

fn inspection_metadata_is_compatible(
    prior: &AssetLocationView,
    inspector: &LocalMediaInspector,
) -> bool {
    prior.metadata_engine_id == inspector.metadata_engine_id()
        && prior.metadata_engine_version == inspector.metadata_engine_version()
}

fn inspect_path(discovery: &PublicationGuardedFileDiscovery, relative_path: &str) -> InspectedPath {
    match discovery.visit_relative_path(relative_path).outcome {
        FileVisitOutcome::File(file) => InspectedPath::File(file),
        FileVisitOutcome::TerminalMedia {
            file,
            issue,
            report_issue,
        } => InspectedPath::TerminalMedia {
            file,
            issue: issue_failure(&issue),
            report_issue,
        },
        FileVisitOutcome::Directory | FileVisitOutcome::Ignored => InspectedPath::CatalogAbsent,
        FileVisitOutcome::Issue(issue) if issue.code == "file_missing" => {
            InspectedPath::CatalogAbsent
        }
        FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped" => {
            InspectedPath::PreservedIssue(issue_failure(&issue))
        }
        FileVisitOutcome::Issue(issue) => InspectedPath::Retry(issue_failure(&issue)),
    }
}

fn select_prior<'a>(
    path_prior: Option<&'a AssetLocationView>,
    identity_prior: Option<&'a AssetLocationView>,
    candidate_prior: Option<&'a AssetLocationView>,
    file: &DiscoveredFile,
) -> Option<&'a AssetLocationView> {
    let identity_matches = |prior: &&AssetLocationView| {
        file.file_identity.is_some() && prior.file_identity == file.file_identity
    };
    path_prior
        .filter(identity_matches)
        .or_else(|| identity_prior.filter(identity_matches))
        .or_else(|| candidate_prior.filter(identity_matches))
        .or(path_prior)
        .or(candidate_prior)
}

fn revalidate_change(
    discovery: &PublicationGuardedFileDiscovery,
    change: &PreparedChange,
) -> Result<(), LibraryChangeFailure> {
    for target in &change.revalidation {
        match target {
            RevalidationTarget::Present {
                relative_path,
                expected,
            } => {
                discovery
                    .revalidate_relative_file_state(relative_path, expected)
                    .map_err(|issue| issue_failure(&issue))?;
            }
            RevalidationTarget::CatalogAbsent(relative_path) => {
                match inspect_path(discovery, relative_path) {
                    InspectedPath::CatalogAbsent | InspectedPath::TerminalMedia { .. } => {}
                    InspectedPath::File(_) => {
                        return Err(failure(
                            "incremental_source_changed_before_publication",
                            "A catalog-absent path became a supported file before publication",
                        ));
                    }
                    InspectedPath::PreservedIssue(issue) | InspectedPath::Retry(issue) => {
                        return Err(issue);
                    }
                }
            }
        }
    }
    Ok(())
}

fn retry_changes<Repository>(
    repository: &mut Repository,
    changes: &[LeasedLibraryChange],
    issue: &LibraryChangeFailure,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    report: &mut IncrementalLibraryChangeReport,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    for change in changes {
        match repository.retry_library_change(
            change.change.id,
            change.lease_generation,
            issue,
            now_unix_ms,
            policy,
        )? {
            LibraryChangeLeaseUpdateOutcome::Applied => {
                report.retried_count = report
                    .retried_count
                    .checked_add(1)
                    .ok_or_else(|| count_overflow("retried change count"))?;
            }
            LibraryChangeLeaseUpdateOutcome::Superseded
            | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
            | LibraryChangeLeaseUpdateOutcome::Missing => {
                report.superseded_count = report
                    .superseded_count
                    .checked_add(1)
                    .ok_or_else(|| count_overflow("superseded change count"))?;
            }
        }
    }
    Ok(())
}

fn defer_changes<Repository>(
    repository: &mut Repository,
    changes: &[PreparedChange],
    now_unix_ms: i64,
    report: &mut IncrementalLibraryChangeReport,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    for change in changes {
        match repository.defer_library_change(
            change.completion.change_id,
            change.completion.lease_generation,
            now_unix_ms,
        )? {
            LibraryChangeLeaseUpdateOutcome::Applied => {
                report.deferred_count = report
                    .deferred_count
                    .checked_add(1)
                    .ok_or_else(|| count_overflow("deferred change count"))?;
            }
            LibraryChangeLeaseUpdateOutcome::Superseded
            | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
            | LibraryChangeLeaseUpdateOutcome::Missing => {
                report.superseded_count = report
                    .superseded_count
                    .checked_add(1)
                    .ok_or_else(|| count_overflow("superseded change count"))?;
            }
        }
    }
    Ok(())
}

fn validate_request(root_id: &str, policy: LibraryChangeQueuePolicy) -> Result<(), ScanError> {
    if root_id.trim().is_empty() || root_id.contains('\0') {
        return Err(ScanError::new(
            "incremental_root_id_invalid",
            "The incremental library root ID must be non-empty and contain no NUL bytes",
        ));
    }
    if !policy.is_valid() {
        return Err(ScanError::new(
            "change_queue_policy_invalid",
            "The durable change queue policy must stay within its absolute bounds",
        ));
    }
    Ok(())
}

fn publication_failure(status: CatalogDeltaPublicationStatus) -> LibraryChangeFailure {
    match status {
        CatalogDeltaPublicationStatus::StaleLease => failure(
            "incremental_lease_superseded",
            "Newer library change evidence superseded the prepared catalog delta",
        ),
        CatalogDeltaPublicationStatus::StaleCatalogRevision => failure(
            "incremental_catalog_revision_changed",
            "The catalog revision changed while the incremental delta was prepared",
        ),
        CatalogDeltaPublicationStatus::StalePreviewState => failure(
            "incremental_preview_state_changed",
            "Preview state changed while the incremental delta was prepared",
        ),
        CatalogDeltaPublicationStatus::RootGenerationChanged => failure(
            "incremental_root_generation_changed",
            "The root generation changed while the incremental delta was prepared",
        ),
        CatalogDeltaPublicationStatus::RootScanInProgress => failure(
            "incremental_scan_in_progress",
            "Incremental publication waits for the active complete scan boundary",
        ),
        CatalogDeltaPublicationStatus::NoPublishedCatalog => failure(
            "incremental_catalog_not_published",
            "The root has no trustworthy published catalog for an incremental delta",
        ),
        CatalogDeltaPublicationStatus::Applied => {
            unreachable!("applied publication is not a failure")
        }
    }
}

fn is_publication_namespace_failure(error: &ScanError) -> bool {
    error.code.starts_with("root_publication_namespace_")
        || matches!(
            error.code.as_str(),
            "root_identity_unavailable"
                | "root_identity_changed"
                | "metadata_inventory_root_identity_changed"
        )
}

fn incremental_asset_id(leased: &LeasedLibraryChange, file: &DiscoveredFile) -> String {
    let identity = file
        .file_identity
        .as_ref()
        .map(|identity| format!("{}\0{}", identity.scheme, identity.value))
        .unwrap_or_default();
    stable_id(
        "incremental-asset-v1",
        &format!(
            "{}\0{}\0{}\0{}",
            leased.change.intent.root_id,
            leased.change.id.value(),
            file.relative_path,
            identity
        ),
    )
}

fn retained_preview_expectation(location: &AssetLocationView) -> RetainedPreviewExpectation {
    RetainedPreviewExpectation {
        location_id: location.location_id.clone(),
        preview_path: location.preview_path.clone(),
        preview_status: location.preview_status.clone(),
        preview_issue_code: location.preview_issue_code.clone(),
        preview_issue_message: location.preview_issue_message.clone(),
    }
}

#[cfg(windows)]
fn windows_case_alias(left: &str, right: &str) -> bool {
    left.replace('\\', "/")
        .eq_ignore_ascii_case(&right.replace('\\', "/"))
        && left != right
}

#[cfg(not(windows))]
fn windows_case_alias(_left: &str, _right: &str) -> bool {
    false
}

fn location_evidence(location: &AssetLocationView) -> ReconciliationFileEvidence {
    ReconciliationFileEvidence {
        relative_path: location.relative_path.clone(),
        file_size: location.file_size,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
        source_revision: location.source_revision.clone(),
        source_generation: location.source_generation,
    }
}

fn file_evidence(file: &DiscoveredFile) -> ReconciliationFileEvidence {
    ReconciliationFileEvidence {
        relative_path: file.relative_path.clone(),
        file_size: file.file_size,
        modified_unix_ms: file.modified_unix_ms,
        file_identity: file.file_identity.clone(),
        source_revision: file.source_revision.clone(),
        source_generation: 0,
    }
}

fn expected_state(file: &DiscoveredFile) -> ExpectedFileState {
    ExpectedFileState {
        absolute_path: file.absolute_path.clone(),
        file_size: file.file_size,
        modified_unix_ms: file.modified_unix_ms,
        file_identity: file.file_identity.clone(),
        source_revision: file.source_revision.clone(),
    }
}

fn issue_failure(issue: &ScanIssue) -> LibraryChangeFailure {
    failure(issue.code.clone(), issue.message.clone())
}

fn scan_failure(error: ScanError) -> LibraryChangeFailure {
    failure(error.code, error.message)
}

fn failure(code: impl Into<String>, message: impl Into<String>) -> LibraryChangeFailure {
    LibraryChangeFailure {
        code: code.into(),
        message: message.into(),
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn leased_catch_up_lineage(
    leased: &LeasedLibraryChange,
) -> Result<&[LibraryChangeCatchUpEvidence], LibraryChangeFailure> {
    match (
        leased.change.catch_up_source.as_ref(),
        leased.change.catch_up_watermark.as_ref(),
    ) {
        (Some(source), Some(watermark))
            if leased
                .change
                .catch_up_lineage
                .iter()
                .any(|evidence| evidence.source == *source && evidence.watermark == *watermark) =>
        {
            Ok(&leased.change.catch_up_lineage)
        }
        (None, None) if leased.change.catch_up_lineage.is_empty() => Ok(&[]),
        _ => Err(failure(
            "incremental_catch_up_evidence_incomplete",
            "A leased change contains incomplete catch-up handoff lineage",
        )),
    }
}

fn bounded_count(value: usize, field: &str) -> Result<u32, ScanError> {
    u32::try_from(value).map_err(|_| count_overflow(field))
}

fn count_overflow(field: &str) -> ScanError {
    ScanError::new(
        "incremental_count_overflow",
        format!("The {field} exceeded the supported range"),
    )
}

#[cfg(test)]
mod tests;

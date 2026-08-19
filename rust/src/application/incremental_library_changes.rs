use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{FileDiscovery, FileVisitOutcome, LocalMediaInspector, user_visible_path};
use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, DiscoveredFile, ExpectedFileState, IncrementalLibraryChangeReport,
    IncrementalReconciliationDecision, IncrementalReconciliationOutcome, LeasedLibraryChange,
    LibraryChangeCatchUpEvidence, LibraryChangeCompletion, LibraryChangeFailure,
    LibraryChangeIntentKind, LibraryChangeLeaseUpdateOutcome, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, PreviewStatus, ReconciliationFileEvidence,
    ReconciliationObservedState, RetainedPreviewExpectation, ScanError, ScanIssue,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector};

use super::directory_synchronization::reconcile_path_evidence;
use super::scan_library::{stable_id, stable_location_id};

struct PreparedChange {
    completion: LibraryChangeCompletion,
    mutations: Vec<CatalogDeltaMutation>,
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
    validate_request(root_id, policy)?;
    let mut report = IncrementalLibraryChangeReport::default();
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
    if root.has_running_scan {
        return Ok(report);
    }
    let discovery = match FileDiscovery::new(&root.root_path) {
        Ok(discovery) => discovery,
        Err(_) => return Ok(report),
    };
    let leased =
        repository.lease_path_library_changes(root_id, root_generation, now_unix_ms, policy)?;
    report.leased_count = bounded_count(leased.len(), "leased change count")?;
    if leased.is_empty() {
        return Ok(report);
    }
    let inspector = LocalMediaInspector::new();
    let mut prepared = Vec::new();
    let mut retries = Vec::new();
    for change in &leased {
        match prepare_change(repository, &discovery, &inspector, change) {
            Ok(change) => prepared.push(change),
            Err(issue) => retries.push((change.clone(), issue)),
        }
    }

    let mut ready = Vec::new();
    for change in prepared {
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

    if !ready.is_empty() {
        let batch = CatalogDeltaBatch {
            root_id: root_id.to_owned(),
            root_generation,
            expected_catalog_revision: root.catalog_revision,
            mutations: ready
                .iter()
                .flat_map(|change| change.mutations.clone())
                .collect(),
            completions: ready
                .iter()
                .map(|change| change.completion.clone())
                .collect(),
        };
        let publication = repository.publish_catalog_delta(&batch, now_unix_ms)?;
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
            }
            CatalogDeltaPublicationStatus::RootScanInProgress
            | CatalogDeltaPublicationStatus::NoPublishedCatalog => {
                defer_changes(repository, &ready, now_unix_ms, &mut report)?;
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

pub(super) struct AuthoritativePathSetRequest<'a> {
    pub root_id: &'a str,
    pub root_generation: LibraryRootGeneration,
    pub expected_catalog_revision: u64,
    pub leased: &'a LeasedLibraryChange,
    pub relative_paths: &'a [String],
    pub now_unix_ms: i64,
    pub queue_policy: LibraryChangeQueuePolicy,
    pub cancellation: &'a AtomicBool,
}

pub(super) fn process_authoritative_path_set<Repository>(
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
    let discovery = match repository.load_incremental_catalog_root(request.root_id)? {
        Some(root)
            if root.root_generation == request.root_generation
                && root.active_scan_id.is_some()
                && !root.has_running_scan =>
        {
            FileDiscovery::new(&root.root_path)?
        }
        _ => {
            let mut report = IncrementalLibraryChangeReport {
                leased_count: 1,
                catalog_revision: request.expected_catalog_revision,
                ..IncrementalLibraryChangeReport::default()
            };
            defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
            return Ok(report);
        }
    };
    let inspector = LocalMediaInspector::new();
    let mut combined = PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: request.leased.change.id,
            lease_generation: request.leased.lease_generation,
            issue: None,
        },
        mutations: Vec::new(),
        revalidation: Vec::new(),
    };
    for relative_path in request.relative_paths {
        if request.cancellation.load(Ordering::Relaxed) {
            return defer_authoritative_path_set(repository, &request);
        }
        let prepared = match prepare_path_change(
            repository,
            &discovery,
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
    if let Err(issue) = revalidate_change(&discovery, &combined) {
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
    let batch = CatalogDeltaBatch {
        root_id: request.root_id.to_owned(),
        root_generation: request.root_generation,
        expected_catalog_revision: request.expected_catalog_revision,
        mutations: combined.mutations.clone(),
        completions: vec![combined.completion.clone()],
    };
    let publication = repository.publish_catalog_delta(&batch, request.now_unix_ms)?;
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
    match repository.defer_library_change(leased.change.id, leased.lease_generation, now_unix_ms)? {
        LibraryChangeLeaseUpdateOutcome::Applied => report.deferred_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => report.superseded_count = 1,
    }
    Ok(())
}

fn prepare_change<Repository>(
    repository: &Repository,
    discovery: &FileDiscovery,
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
        LibraryChangeIntentKind::Reconcile => prepare_path_change(
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
        ),
        LibraryChangeIntentKind::RenameCandidate => {
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
                _ => None,
            };
            let previous_is_absent = match &previous_observed {
                InspectedPath::CatalogAbsent => true,
                InspectedPath::File(_) => false,
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
    discovery: &FileDiscovery,
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
    let observed = observed.unwrap_or_else(|| inspect_path(discovery, relative_path));
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
            if selected_prior.as_ref().is_some_and(|prior| {
                prior.metadata_engine_id != inspector.metadata_engine_id()
                    || prior.metadata_engine_version != inspector.metadata_engine_version()
            }) {
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
                let built =
                    build_location(inspector, leased, file, &decision, selected_prior.as_ref())?;
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
            let built =
                build_location(inspector, leased, file, &decision, selected_prior.as_ref())?;
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
        revalidation,
    })
}

struct BuiltLocation {
    location: AssetLocationView,
    issue: Option<LibraryChangeFailure>,
}

fn build_location(
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
    file: &DiscoveredFile,
    decision: &IncrementalReconciliationDecision,
    prior: Option<&AssetLocationView>,
) -> Result<BuiltLocation, LibraryChangeFailure> {
    let retains_compatible = decision.evidence_disposition
        == DerivedEvidenceDisposition::RetainCompatible
        && prior.is_some_and(|prior| {
            prior.metadata_engine_id == inspector.metadata_engine_id()
                && prior.metadata_engine_version == inspector.metadata_engine_version()
        });
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
            let inspection = inspector
                .inspect(file)
                .map_err(|issue| issue_failure(&issue))?;
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
                failure(
                    "incremental_prior_asset_missing",
                    "Identity-preserving reconciliation requires a prior asset",
                )
            })?
        }
        IncrementalReconciliationOutcome::Added | IncrementalReconciliationOutcome::Replaced => {
            incremental_asset_id(leased, file)
        }
        _ => {
            return Err(failure(
                "incremental_upsert_outcome_invalid",
                "The reconciliation outcome cannot create a catalog location",
            ));
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
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path.clone(),
            preview_path,
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: file.file_identity.clone(),
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

fn inspect_path(discovery: &FileDiscovery, relative_path: &str) -> InspectedPath {
    match discovery.visit_relative_path(relative_path).outcome {
        FileVisitOutcome::File(file) => InspectedPath::File(file),
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
    discovery: &FileDiscovery,
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
                    InspectedPath::CatalogAbsent => {}
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
    }
}

fn file_evidence(file: &DiscoveredFile) -> ReconciliationFileEvidence {
    ReconciliationFileEvidence {
        relative_path: file.relative_path.clone(),
        file_size: file.file_size,
        modified_unix_ms: file.modified_unix_ms,
        file_identity: file.file_identity.clone(),
    }
}

fn expected_state(file: &DiscoveredFile) -> ExpectedFileState {
    ExpectedFileState {
        absolute_path: file.absolute_path.clone(),
        file_size: file.file_size,
        modified_unix_ms: file.modified_unix_ms,
        file_identity: file.file_identity.clone(),
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

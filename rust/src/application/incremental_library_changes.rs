use crate::adapters::{
    FileDiscovery, FileVisitOutcome, LocalMediaInspector, revalidate_file_state, user_visible_path,
};
use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, DiscoveredFile, ExpectedFileState, IncrementalLibraryChangeReport,
    IncrementalReconciliationDecision, IncrementalReconciliationOutcome, LeasedLibraryChange,
    LibraryChangeCompletion, LibraryChangeFailure, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration, PreviewStatus, ReconciliationFileEvidence, ReconciliationObservedState,
    ScanError, ScanIssue,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector};

use super::directory_synchronization::reconcile_path_evidence;
use super::scan_library::{stable_id, stable_location_id};

struct PreparedChange {
    completion: LibraryChangeCompletion,
    mutation: Option<CatalogDeltaMutation>,
    revalidation: Vec<RevalidationTarget>,
}

struct PathChangeContext<'a> {
    relative_path: &'a str,
    candidate_prior: Option<AssetLocationView>,
    may_remove_candidate_prior: bool,
    removals: Vec<String>,
}

enum RevalidationTarget {
    Present(ExpectedFileState),
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
    let leased = repository.lease_library_changes(root_id, root_generation, now_unix_ms, policy)?;
    report.leased_count = bounded_count(leased.len(), "leased change count")?;
    if leased.is_empty() {
        return Ok(report);
    }
    let discovery = match FileDiscovery::new(&root.root_path) {
        Ok(discovery) => discovery,
        Err(error) => {
            retry_changes(
                repository,
                &leased,
                &failure(error.code, error.message),
                now_unix_ms,
                policy,
                &mut report,
            )?;
            return Ok(report);
        }
    };
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
                .filter_map(|change| change.mutation.clone())
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
            let previous_is_absent = match previous_observed {
                InspectedPath::CatalogAbsent => true,
                InspectedPath::File(_) => false,
                InspectedPath::PreservedIssue(issue) | InspectedPath::Retry(issue) => {
                    return Err(issue);
                }
            };
            let mut removals = Vec::new();
            let mut revalidation = Vec::new();
            if previous_is_absent {
                if let Some(prior) = &previous_prior {
                    removals.push(prior.location_id.clone());
                }
                revalidation.push(RevalidationTarget::CatalogAbsent(previous_path.to_owned()));
            }
            prepare_path_change(
                repository,
                discovery,
                inspector,
                leased,
                PathChangeContext {
                    relative_path: &intent.relative_path,
                    candidate_prior: previous_prior,
                    may_remove_candidate_prior: previous_is_absent,
                    removals,
                },
            )
            .map(|mut prepared| {
                prepared.revalidation.extend(revalidation);
                prepared
            })
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
        candidate_prior,
        may_remove_candidate_prior,
        mut removals,
    } = context;
    let intent = &leased.change.intent;
    let path_prior = repository
        .load_incremental_location_by_relative_path(&intent.root_id, relative_path)
        .map_err(scan_failure)?;
    let observed = inspect_path(discovery, relative_path);
    let mut completion_issue = None;
    let mut revalidation = Vec::new();
    let (decision, current_file, selected_prior) = match observed {
        InspectedPath::File(file) => {
            let identity_prior = file
                .file_identity
                .as_ref()
                .map(|identity| repository.load_incremental_location_by_file_identity(identity))
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
            if decision.outcome == IncrementalReconciliationOutcome::Unchanged
                && selected_prior.as_ref().is_some_and(|prior| {
                    prior.metadata_engine_id != inspector.metadata_engine_id()
                        || prior.metadata_engine_version != inspector.metadata_engine_version()
                })
            {
                decision.outcome = IncrementalReconciliationOutcome::Modified;
                decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
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
        InspectedPath::PreservedIssue(issue) => {
            completion_issue = Some(issue.clone());
            (
                IncrementalReconciliationDecision {
                    outcome: IncrementalReconciliationOutcome::Skipped,
                    evidence_disposition: DerivedEvidenceDisposition::PreserveLastTrustworthy,
                    current: None,
                    issue_code: Some(issue.code),
                },
                None,
                path_prior.clone().or(candidate_prior.clone()),
            )
        }
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
            (!removals.is_empty()).then_some(CatalogDeltaMutation {
                outcome: IncrementalReconciliationOutcome::Removed,
                evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
                remove_location_ids: removals,
                upsert_location: None,
            })
        }
        IncrementalReconciliationOutcome::Skipped
        | IncrementalReconciliationOutcome::RetryableFailure
        | IncrementalReconciliationOutcome::TerminalIssue => None,
        IncrementalReconciliationOutcome::Removed => {
            if let Some(prior) = selected_prior.as_ref().or(path_prior.as_ref()) {
                push_unique(&mut removals, prior.location_id.clone());
            }
            Some(CatalogDeltaMutation {
                outcome: decision.outcome,
                evidence_disposition: decision.evidence_disposition,
                remove_location_ids: removals,
                upsert_location: None,
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
            revalidation.push(RevalidationTarget::Present(expected_state(file)));
            Some(CatalogDeltaMutation {
                outcome: decision.outcome,
                evidence_disposition: decision.evidence_disposition,
                remove_location_ids: removals,
                upsert_location: Some(built.location),
            })
        }
    };
    Ok(PreparedChange {
        completion: LibraryChangeCompletion {
            change_id: leased.change.id,
            lease_generation: leased.lease_generation,
            issue: completion_issue,
        },
        mutation,
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
        IncrementalReconciliationOutcome::Modified
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
    let (preview_path, preview_status) = if retains_compatible {
        let prior = prior.expect("compatible evidence requires a prior location");
        (prior.preview_path.clone(), prior.preview_status.clone())
    } else {
        (String::new(), PreviewStatus::Pending)
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
            preview_issue_code: None,
            preview_issue_message: None,
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
            RevalidationTarget::Present(expected) => {
                revalidate_file_state(expected).map_err(|issue| issue_failure(&issue))?;
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

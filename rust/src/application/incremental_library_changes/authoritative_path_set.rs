use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery};
use crate::domain::{
    CatalogDeltaBatch, CatalogDeltaPublicationStatus, FileIdentityEvidence,
    IncrementalLibraryChangeReport, LeasedLibraryChange, LibraryChangeCompletion,
    LibraryChangeIntentKind, LibraryChangeLane, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanError,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

use super::preparation_catalog::PreparationCatalog;
use super::preparation_rebase::{RebaseContext, preparation_is_current};
use super::{
    MAX_CATALOG_REVISION_REBASE_ATTEMPTS, PathChangeContext, PreparedChange, defer_changes,
    defer_leased_change, failure, is_publication_namespace_failure, prepare_path_change,
    publication_failure, retry_changes, revalidate_change, validate_request,
};

pub(in crate::application) struct AuthoritativePathSetContext<'a> {
    pub(in crate::application) root_id: &'a str,
    pub(in crate::application) root_generation: LibraryRootGeneration,
    pub(in crate::application) expected_catalog_revision: u64,
    pub(in crate::application) expected_root_identity: &'a FileIdentityEvidence,
    pub(in crate::application) leased: &'a LeasedLibraryChange,
    pub(in crate::application) relative_paths: &'a [String],
    pub(in crate::application) now_unix_ms: i64,
    pub(in crate::application) queue_policy: LibraryChangeQueuePolicy,
    pub(in crate::application) cancellation: &'a AtomicBool,
}

pub(in crate::application) fn process_authoritative_path_set<Repository>(
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
    let preparation_root = repository
        .load_incremental_catalog_root(request.root_id)?
        .filter(|root| {
            root.root_generation == request.root_generation
                && root.active_scan_id.is_some()
                && (!root.has_running_scan || can_publish_during_scan)
                && root.publication_root_identity.as_ref() == Some(request.expected_root_identity)
        });
    let Some(preparation_root) = preparation_root else {
        let mut report = IncrementalLibraryChangeReport {
            leased_count: 1,
            catalog_revision: request.expected_catalog_revision,
            ..IncrementalLibraryChangeReport::default()
        };
        defer_leased_change(repository, request.leased, request.now_unix_ms, &mut report)?;
        return Ok(report);
    };
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
    let mut preparation = if is_single_source_reconciliation(&request) {
        PreparationCatalog::new(repository)
    } else {
        PreparationCatalog::untracked(repository)
    };
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
            &mut preparation,
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
    let reads = preparation.finish();
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
    let mut expected_catalog_revision = request.expected_catalog_revision;
    let mut rebase_attempts = 0;
    loop {
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
            expected_catalog_revision,
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
                return Ok(report);
            }
            CatalogDeltaPublicationStatus::RootScanInProgress
            | CatalogDeltaPublicationStatus::NoPublishedCatalog => {
                defer_changes(repository, &[combined], request.now_unix_ms, &mut report)?;
                return Ok(report);
            }
            CatalogDeltaPublicationStatus::StaleCatalogRevision
                if rebase_attempts < MAX_CATALOG_REVISION_REBASE_ATTEMPTS =>
            {
                if let Some(current_root) =
                    repository.load_incremental_catalog_root(request.root_id)?
                {
                    let context = RebaseContext {
                        previous_root: &preparation_root,
                        current_root: &current_root,
                        leased: std::slice::from_ref(request.leased),
                        cancelled: Some(request.cancellation),
                    };
                    if preparation_is_current(
                        repository,
                        request.discovery,
                        std::slice::from_ref(&combined),
                        &reads,
                        &context,
                    ) {
                        expected_catalog_revision = current_root.catalog_revision;
                        rebase_attempts += 1;
                        continue;
                    }
                }
                if request.cancellation.load(Ordering::Relaxed) {
                    defer_leased_change(
                        repository,
                        request.leased,
                        request.now_unix_ms,
                        &mut report,
                    )?;
                    return Ok(report);
                }
            }
            _ => {}
        }
        retry_changes(
            repository,
            std::slice::from_ref(request.leased),
            &publication_failure(publication.status),
            request.now_unix_ms,
            request.queue_policy,
            &mut report,
        )?;
        return Ok(report);
    }
}

fn is_single_source_reconciliation(request: &AuthoritativePathSetRequest<'_>) -> bool {
    let intent = &request.leased.change.intent;
    intent.origin == LibraryChangeOrigin::ConsistencyAudit
        && intent.scope == LibraryChangeScope::Path
        && intent.kind == LibraryChangeIntentKind::Reconcile
        && intent.previous_relative_path.is_none()
        && matches!(request.relative_paths, [path] if *path == intent.relative_path)
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

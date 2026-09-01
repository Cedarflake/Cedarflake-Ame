use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{FileVisitOutcome, PublicationGuardedFileDiscovery};
use crate::domain::{
    IncrementalCatalogRoot, IncrementalLibraryChangeReport, LeasedLibraryChange,
    LibraryChangeCapacityDeferral, LibraryChangeFailure, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanError, ScanIssue,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

use super::incremental_library_changes::{
    AuthoritativePathSetContext, process_authoritative_path_set,
};

const MAX_AUTHORITATIVE_ENTRIES: u32 = 4_096;
const MAX_AUTHORITATIVE_PATHS: u32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoritativeRecoveryPolicy {
    pub max_scope_entries: u32,
    pub max_scope_paths: u32,
}

impl AuthoritativeRecoveryPolicy {
    pub(crate) const fn is_valid(self) -> bool {
        self.max_scope_entries > 0
            && self.max_scope_entries <= MAX_AUTHORITATIVE_ENTRIES
            && self.max_scope_paths > 0
            && self.max_scope_paths <= MAX_AUTHORITATIVE_PATHS
    }
}

impl Default for AuthoritativeRecoveryPolicy {
    fn default() -> Self {
        Self {
            max_scope_entries: MAX_AUTHORITATIVE_ENTRIES,
            max_scope_paths: MAX_AUTHORITATIVE_PATHS,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AuthoritativeLibraryChangeReport {
    pub incremental: IncrementalLibraryChangeReport,
}

pub(crate) fn process_ready_authoritative_library_change<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
) -> Result<AuthoritativeLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    let cancellation = AtomicBool::new(false);
    process_ready_authoritative_library_change_cancellable(
        repository,
        root_id,
        root_generation,
        now_unix_ms,
        queue_policy,
        recovery_policy,
        &cancellation,
    )
}

pub(crate) fn process_ready_authoritative_library_change_cancellable<Repository>(
    repository: &mut Repository,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    cancellation: &AtomicBool,
) -> Result<AuthoritativeLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    validate_policy(recovery_policy)?;
    if cancellation.load(Ordering::Relaxed) {
        return Ok(AuthoritativeLibraryChangeReport::default());
    }
    let Some(root) = repository.load_incremental_catalog_root(root_id)? else {
        return Ok(AuthoritativeLibraryChangeReport::default());
    };
    if root.root_generation != root_generation
        || root.active_scan_id.is_none()
        || root.has_running_scan
    {
        return Ok(AuthoritativeLibraryChangeReport::default());
    }
    let Some(leased) = repository.lease_authoritative_library_change(
        root_id,
        root_generation,
        now_unix_ms,
        queue_policy,
    )?
    else {
        return Ok(AuthoritativeLibraryChangeReport {
            incremental: IncrementalLibraryChangeReport {
                catalog_revision: root.catalog_revision,
                ..IncrementalLibraryChangeReport::default()
            },
        });
    };
    process_leased_authoritative_library_change_cancellable(
        repository,
        &root,
        &leased,
        now_unix_ms,
        queue_policy,
        recovery_policy,
        cancellation,
    )
}

pub(crate) fn process_leased_authoritative_library_change_cancellable<Repository>(
    repository: &mut Repository,
    root: &IncrementalCatalogRoot,
    leased: &LeasedLibraryChange,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    cancellation: &AtomicBool,
) -> Result<AuthoritativeLibraryChangeReport, ScanError>
where
    Repository: IncrementalCatalogRepository + LibraryChangeQueue,
{
    validate_policy(recovery_policy)?;
    if leased.change.intent.root_id != root.root_id
        || leased.change.intent.root_generation != root.root_generation
    {
        return Err(ScanError::new(
            "authoritative_lease_root_mismatch",
            "The authoritative lease does not belong to the selected catalog root",
        ));
    }
    if cancellation.load(Ordering::Relaxed) {
        return defer_authoritative_change(repository, leased, root.catalog_revision, now_unix_ms);
    }
    if leased.change.intent.origin == LibraryChangeOrigin::LiveNotification
        && leased.change.intent.kind == LibraryChangeIntentKind::FreshnessUnknown
        && leased.change.intent.scope == LibraryChangeScope::Root
        && leased.change.intent.relative_path.is_empty()
        && leased.change.intent.previous_relative_path.is_none()
    {
        return retry_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            metadata_inventory_required(),
            now_unix_ms,
            queue_policy,
        );
    }
    let Some(expected_root_identity) = root.publication_root_identity.as_ref() else {
        return retry_authoritative_change(
            repository,
            leased,
            root.catalog_revision,
            LibraryChangeFailure {
                code: "root_publication_namespace_unproven".to_owned(),
                message: "The configured root has no trustworthy persistent namespace identity"
                    .to_owned(),
            },
            now_unix_ms,
            queue_policy,
        );
    };
    let discovery = match PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        expected_root_identity,
    ) {
        Ok(discovery) => discovery,
        Err(error) => {
            return retry_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                LibraryChangeFailure {
                    code: error.code,
                    message: error.message,
                },
                now_unix_ms,
                queue_policy,
            );
        }
    };
    let scopes = recovery_scopes(leased)?;
    let observed_paths = match enumerate_scopes(&discovery, &scopes, recovery_policy, cancellation)
    {
        Ok(paths) => paths,
        Err(EnumerationFailure::Capacity) => {
            return retry_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                metadata_inventory_required(),
                now_unix_ms,
                queue_policy,
            );
        }
        Err(EnumerationFailure::Issue(issue)) => {
            return retry_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                issue,
                now_unix_ms,
                queue_policy,
            );
        }
        Err(EnumerationFailure::Cancelled) => {
            return defer_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                now_unix_ms,
            );
        }
    };
    let mut paths = observed_paths;
    for scope in &scopes {
        if cancellation.load(Ordering::Relaxed) {
            return defer_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                now_unix_ms,
            );
        }
        let locations = repository.load_incremental_locations_in_subtree(
            &root.root_id,
            scope,
            recovery_policy.max_scope_paths.saturating_add(1),
        )?;
        if locations.len() > recovery_policy.max_scope_paths as usize {
            return retry_authoritative_change(
                repository,
                leased,
                root.catalog_revision,
                metadata_inventory_required(),
                now_unix_ms,
                queue_policy,
            );
        }
        for location in locations {
            paths.insert(location.relative_path);
            if paths.len() > recovery_policy.max_scope_paths as usize {
                return retry_authoritative_change(
                    repository,
                    leased,
                    root.catalog_revision,
                    metadata_inventory_required(),
                    now_unix_ms,
                    queue_policy,
                );
            }
        }
    }
    let relative_paths = paths.into_iter().collect::<Vec<_>>();
    let incremental = process_authoritative_path_set(
        repository,
        AuthoritativePathSetContext {
            root_id: &root.root_id,
            root_generation: root.root_generation,
            expected_catalog_revision: root.catalog_revision,
            expected_root_identity,
            leased,
            relative_paths: &relative_paths,
            now_unix_ms,
            queue_policy,
            cancellation,
        },
        &discovery,
    )?;
    Ok(AuthoritativeLibraryChangeReport { incremental })
}

fn recovery_scopes(leased: &LeasedLibraryChange) -> Result<Vec<String>, ScanError> {
    let intent = &leased.change.intent;
    if intent.kind == LibraryChangeIntentKind::FreshnessUnknown
        || intent.scope == LibraryChangeScope::Root
    {
        return Ok(vec![String::new()]);
    }
    if intent.scope != LibraryChangeScope::Subtree {
        return Err(ScanError::new(
            "authoritative_scope_invalid",
            "The authoritative worker received path-scoped work",
        ));
    }
    let mut scopes = BTreeSet::from([intent.relative_path.clone()]);
    if intent.kind == LibraryChangeIntentKind::RenameCandidate {
        let previous = intent.previous_relative_path.clone().ok_or_else(|| {
            ScanError::new(
                "authoritative_rename_previous_path_missing",
                "A subtree rename requires its previous relative path",
            )
        })?;
        scopes.insert(previous);
    }
    Ok(scopes.into_iter().collect())
}

fn enumerate_scopes(
    discovery: &PublicationGuardedFileDiscovery,
    scopes: &[String],
    policy: AuthoritativeRecoveryPolicy,
    cancellation: &AtomicBool,
) -> Result<BTreeSet<String>, EnumerationFailure> {
    let mut paths = BTreeSet::new();
    let mut directories = VecDeque::new();
    let mut scheduled_directories = BTreeSet::new();
    let mut visited_entries = 0_u32;
    for scope in scopes {
        if cancellation.load(Ordering::Relaxed) {
            return Err(EnumerationFailure::Cancelled);
        }
        if scope.is_empty() {
            schedule_directory(&mut directories, &mut scheduled_directories, String::new());
            continue;
        }
        match discovery.visit_relative_path(scope).outcome {
            FileVisitOutcome::Directory => {
                schedule_directory(&mut directories, &mut scheduled_directories, scope.clone())
            }
            FileVisitOutcome::File(file) => {
                paths.insert(file.relative_path);
            }
            FileVisitOutcome::TerminalMedia { file, .. } => {
                paths.insert(file.relative_path);
            }
            FileVisitOutcome::Ignored => {}
            FileVisitOutcome::Issue(issue) if issue.code == "file_missing" => {}
            FileVisitOutcome::Issue(issue) => {
                return Err(EnumerationFailure::Issue(scan_issue_failure(issue)));
            }
        }
    }
    while let Some(directory) = directories.pop_front() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(EnumerationFailure::Cancelled);
        }
        let entries = discovery
            .file_visits_in_directory(&directory)
            .map_err(|issue| EnumerationFailure::Issue(scan_issue_failure(issue)))?;
        for visit in entries {
            if cancellation.load(Ordering::Relaxed) {
                return Err(EnumerationFailure::Cancelled);
            }
            let visit =
                visit.map_err(|issue| EnumerationFailure::Issue(scan_issue_failure(issue)))?;
            visited_entries = visited_entries
                .checked_add(1)
                .ok_or(EnumerationFailure::Capacity)?;
            if visited_entries > policy.max_scope_entries {
                return Err(EnumerationFailure::Capacity);
            }
            let relative_path = visit.relative_path;
            match visit.outcome {
                FileVisitOutcome::Directory => {
                    schedule_directory(&mut directories, &mut scheduled_directories, relative_path)
                }
                FileVisitOutcome::File(file) => {
                    paths.insert(file.relative_path);
                    if paths.len() > policy.max_scope_paths as usize {
                        return Err(EnumerationFailure::Capacity);
                    }
                }
                FileVisitOutcome::TerminalMedia { file, .. } => {
                    paths.insert(file.relative_path);
                    if paths.len() > policy.max_scope_paths as usize {
                        return Err(EnumerationFailure::Capacity);
                    }
                }
                FileVisitOutcome::Ignored => {}
                FileVisitOutcome::Issue(issue) if issue.code == "file_missing" => {}
                FileVisitOutcome::Issue(issue) => {
                    return Err(EnumerationFailure::Issue(scan_issue_failure(issue)));
                }
            }
        }
    }
    Ok(paths)
}

fn schedule_directory(
    directories: &mut VecDeque<String>,
    scheduled: &mut BTreeSet<String>,
    relative_path: String,
) {
    if scheduled.insert(relative_path.clone()) {
        directories.push_back(relative_path);
    }
}

pub(crate) fn defer_authoritative_change<Repository>(
    repository: &mut Repository,
    leased: &LeasedLibraryChange,
    catalog_revision: u64,
    now_unix_ms: i64,
) -> Result<AuthoritativeLibraryChangeReport, ScanError>
where
    Repository: LibraryChangeQueue,
{
    let mut incremental = IncrementalLibraryChangeReport {
        leased_count: 1,
        catalog_revision,
        ..IncrementalLibraryChangeReport::default()
    };
    match repository.defer_library_change(leased.change.id, leased.lease_generation, now_unix_ms)? {
        LibraryChangeLeaseUpdateOutcome::Applied => incremental.deferred_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => incremental.superseded_count = 1,
    }
    Ok(AuthoritativeLibraryChangeReport { incremental })
}

pub(crate) fn retry_authoritative_change<Repository>(
    repository: &mut Repository,
    leased: &LeasedLibraryChange,
    catalog_revision: u64,
    failure: LibraryChangeFailure,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<AuthoritativeLibraryChangeReport, ScanError>
where
    Repository: LibraryChangeQueue,
{
    let mut incremental = IncrementalLibraryChangeReport {
        leased_count: 1,
        catalog_revision,
        ..IncrementalLibraryChangeReport::default()
    };
    let outcome = if failure.code == "metadata_inventory_required"
        && leased.change.intent.origin == crate::domain::LibraryChangeOrigin::LiveNotification
    {
        match repository.promote_live_watcher_gap_to_metadata_inventory(
            leased.change.id,
            leased.lease_generation,
            &failure,
            now_unix_ms,
            policy,
        ) {
            Ok(outcome) => outcome,
            Err(error) if error.code == "change_queue_backpressure" => repository
                .defer_library_change_for_capacity(
                    leased.change.id,
                    leased.lease_generation,
                    LibraryChangeCapacityDeferral::MetadataInventoryLane,
                    now_unix_ms,
                    policy,
                )?,
            Err(error) => return Err(error),
        }
    } else {
        repository.retry_library_change(
            leased.change.id,
            leased.lease_generation,
            &failure,
            now_unix_ms,
            policy,
        )?
    };
    match outcome {
        LibraryChangeLeaseUpdateOutcome::Applied => incremental.retried_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => incremental.superseded_count = 1,
    }
    Ok(AuthoritativeLibraryChangeReport { incremental })
}

fn metadata_inventory_required() -> LibraryChangeFailure {
    LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The authoritative scope exceeds one bounded metadata page".to_owned(),
    }
}

fn validate_policy(policy: AuthoritativeRecoveryPolicy) -> Result<(), ScanError> {
    if policy.is_valid() {
        return Ok(());
    }
    Err(ScanError::new(
        "authoritative_recovery_policy_invalid",
        "Authoritative recovery limits must stay within their absolute bounds",
    ))
}

fn scan_issue_failure(issue: ScanIssue) -> LibraryChangeFailure {
    LibraryChangeFailure {
        code: issue.code,
        message: issue.message,
    }
}

enum EnumerationFailure {
    Capacity,
    Issue(LibraryChangeFailure),
    Cancelled,
}

#[cfg(test)]
mod tests;

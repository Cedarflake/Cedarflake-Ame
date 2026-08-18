use std::collections::{BTreeSet, VecDeque};

use crate::adapters::{FileDiscovery, FileVisitOutcome};
use crate::domain::{
    IncrementalLibraryChangeReport, LeasedLibraryChange, LibraryChangeFailure, LibraryChangeId,
    LibraryChangeIntentKind, LibraryChangeLeaseUpdateOutcome, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanError, ScanIssue,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

use super::incremental_library_changes::{
    AuthoritativePathSetRequest, process_authoritative_path_set,
};

const MAX_AUTHORITATIVE_ENTRIES: u32 = 4_096;
const MAX_AUTHORITATIVE_PATHS: u32 = 128;
const MAX_AUDIT_INTERVAL_MILLIS: u64 = 365 * 24 * 60 * 60 * 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoritativeRecoveryPolicy {
    pub max_scope_entries: u32,
    pub max_scope_paths: u32,
    pub audit_interval_millis: u64,
}

impl AuthoritativeRecoveryPolicy {
    pub(crate) const fn is_valid(self) -> bool {
        self.max_scope_entries > 0
            && self.max_scope_entries <= MAX_AUTHORITATIVE_ENTRIES
            && self.max_scope_paths > 0
            && self.max_scope_paths <= MAX_AUTHORITATIVE_PATHS
            && self.audit_interval_millis > 0
            && self.audit_interval_millis <= MAX_AUDIT_INTERVAL_MILLIS
    }
}

impl Default for AuthoritativeRecoveryPolicy {
    fn default() -> Self {
        Self {
            max_scope_entries: MAX_AUTHORITATIVE_ENTRIES,
            max_scope_paths: MAX_AUTHORITATIVE_PATHS,
            audit_interval_millis: 7 * 24 * 60 * 60 * 1_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FullScanRecoveryRequest {
    pub root_id: String,
    pub root_path: String,
    pub root_generation: LibraryRootGeneration,
    pub queue_high_watermark: LibraryChangeId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AuthoritativeLibraryChangeReport {
    pub incremental: IncrementalLibraryChangeReport,
    pub full_scan: Option<FullScanRecoveryRequest>,
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
    validate_policy(recovery_policy)?;
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
            full_scan: None,
        });
    };
    let discovery = match FileDiscovery::new(&root.root_path) {
        Ok(discovery) => discovery,
        Err(error) => {
            return retry(
                repository,
                &leased,
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
    let scopes = recovery_scopes(&leased)?;
    let observed_paths = match enumerate_scopes(&discovery, &scopes, recovery_policy) {
        Ok(paths) => paths,
        Err(EnumerationFailure::Capacity) => {
            return escalate_to_full_scan(
                repository,
                &root.root_path,
                &leased,
                root.catalog_revision,
                now_unix_ms,
            );
        }
        Err(EnumerationFailure::Issue(issue)) => {
            return retry(
                repository,
                &leased,
                root.catalog_revision,
                issue,
                now_unix_ms,
                queue_policy,
            );
        }
    };
    let mut paths = observed_paths;
    for scope in &scopes {
        let locations = repository.load_incremental_locations_in_subtree(
            root_id,
            scope,
            recovery_policy.max_scope_paths.saturating_add(1),
        )?;
        if locations.len() > recovery_policy.max_scope_paths as usize {
            return escalate_to_full_scan(
                repository,
                &root.root_path,
                &leased,
                root.catalog_revision,
                now_unix_ms,
            );
        }
        for location in locations {
            paths.insert(location.relative_path);
            if paths.len() > recovery_policy.max_scope_paths as usize {
                return escalate_to_full_scan(
                    repository,
                    &root.root_path,
                    &leased,
                    root.catalog_revision,
                    now_unix_ms,
                );
            }
        }
    }
    let relative_paths = paths.into_iter().collect::<Vec<_>>();
    let incremental = process_authoritative_path_set(
        repository,
        AuthoritativePathSetRequest {
            root_id,
            root_generation,
            expected_catalog_revision: root.catalog_revision,
            leased: &leased,
            relative_paths: &relative_paths,
            now_unix_ms,
            queue_policy,
        },
    )?;
    Ok(AuthoritativeLibraryChangeReport {
        incremental,
        full_scan: None,
    })
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
    discovery: &FileDiscovery,
    scopes: &[String],
    policy: AuthoritativeRecoveryPolicy,
) -> Result<BTreeSet<String>, EnumerationFailure> {
    let mut paths = BTreeSet::new();
    let mut directories = VecDeque::new();
    let mut scheduled_directories = BTreeSet::new();
    let mut visited_entries = 0_u32;
    for scope in scopes {
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
            FileVisitOutcome::Ignored => {}
            FileVisitOutcome::Issue(issue) if issue.code == "file_missing" => {}
            FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped" => {}
            FileVisitOutcome::Issue(issue) => {
                return Err(EnumerationFailure::Issue(scan_issue_failure(issue)));
            }
        }
    }
    while let Some(directory) = directories.pop_front() {
        let entries = discovery
            .checked_entry_paths_in_directory(&directory)
            .map_err(|issue| EnumerationFailure::Issue(scan_issue_failure(issue)))?;
        for relative_path in entries {
            let relative_path = relative_path
                .map_err(|issue| EnumerationFailure::Issue(scan_issue_failure(issue)))?;
            visited_entries = visited_entries
                .checked_add(1)
                .ok_or(EnumerationFailure::Capacity)?;
            if visited_entries > policy.max_scope_entries {
                return Err(EnumerationFailure::Capacity);
            }
            match discovery.visit_relative_path(&relative_path).outcome {
                FileVisitOutcome::Directory => {
                    schedule_directory(&mut directories, &mut scheduled_directories, relative_path)
                }
                FileVisitOutcome::File(file) => {
                    paths.insert(file.relative_path);
                    if paths.len() > policy.max_scope_paths as usize {
                        return Err(EnumerationFailure::Capacity);
                    }
                }
                FileVisitOutcome::Ignored => {}
                FileVisitOutcome::Issue(issue) if issue.code == "file_missing" => {}
                FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped" => {}
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

fn escalate_to_full_scan<Repository>(
    repository: &mut Repository,
    root_path: &str,
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
    Ok(AuthoritativeLibraryChangeReport {
        incremental,
        full_scan: Some(FullScanRecoveryRequest {
            root_id: leased.change.intent.root_id.clone(),
            root_path: root_path.to_owned(),
            root_generation: leased.change.intent.root_generation,
            queue_high_watermark: leased.change.id,
        }),
    })
}

fn retry<Repository>(
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
    match repository.retry_library_change(
        leased.change.id,
        leased.lease_generation,
        &failure,
        now_unix_ms,
        policy,
    )? {
        LibraryChangeLeaseUpdateOutcome::Applied => incremental.retried_count = 1,
        LibraryChangeLeaseUpdateOutcome::Superseded
        | LibraryChangeLeaseUpdateOutcome::LeaseMismatch
        | LibraryChangeLeaseUpdateOutcome::Missing => incremental.superseded_count = 1,
    }
    Ok(AuthoritativeLibraryChangeReport {
        incremental,
        full_scan: None,
    })
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
}

#[cfg(test)]
mod tests;

use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::{
    AssetLocationView, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    PreviewRequest, ScanError, ScanIssue,
};
use crate::ports::{IncrementalCatalogRepository, SourceReconciliationRepository};

use super::super::incremental_library_changes::{
    AuthoritativePathSetContext, process_authoritative_path_set,
};
use super::super::{StoragePaths, catalog_session};

pub(super) fn recover_source_mismatch(
    storage: &StoragePaths,
    request: &PreviewRequest,
    location: &AssetLocationView,
    root_generation: LibraryRootGeneration,
    issue: ScanIssue,
) -> ScanError {
    if !matches!(
        issue.code.as_str(),
        "source_changed_during_scan"
            | "source_replaced_during_scan"
            | "source_revision_changed_during_scan"
    ) {
        return ScanError::new(issue.code, issue.message);
    }
    match reconcile_source(storage, request, location, root_generation) {
        Ok(true) => ScanError::new(
            "preview_request_superseded",
            format!(
                "{}: source context was superseded during reconciliation",
                issue.code
            ),
        ),
        Ok(false) => ScanError::new(issue.code, issue.message),
        Err(error) => ScanError::new(
            error.code,
            format!("{}; source reconciliation: {}", issue.code, error.message),
        ),
    }
}

fn reconcile_source(
    storage: &StoragePaths,
    request: &PreviewRequest,
    location: &AssetLocationView,
    root_generation: LibraryRootGeneration,
) -> Result<bool, ScanError> {
    let mut catalog =
        catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let Some(root) = catalog.load_incremental_catalog_root(&location.root_id)? else {
        return Ok(true);
    };
    if root.root_generation != root_generation
        || root.active_scan_id.as_deref() != Some(&request.expected_scan_id)
    {
        return Ok(true);
    }
    let Some(identity) = root.publication_root_identity.as_ref() else {
        return Ok(false);
    };
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        identity,
    )?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .ok_or_else(|| {
            ScanError::new(
                "source_reconciliation_clock_invalid",
                "The system clock is outside the supported timestamp range",
            )
        })?;
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 1,
        ..LibraryChangeQueuePolicy::default()
    };
    let intent = LibraryChangeIntent {
        root_id: location.root_id.clone(),
        root_generation,
        kind: LibraryChangeIntentKind::Reconcile,
        scope: LibraryChangeScope::Path,
        relative_path: location.relative_path.clone(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::ConsistencyAudit,
        first_observed_unix_ms: now,
        most_recent_observed_unix_ms: now,
        first_sequence: 0,
        most_recent_sequence: 0,
        coalesced_observation_count: 1,
    };
    let Some(leased) = catalog.admit_source_reconciliation(request, &intent, now, policy)? else {
        return Ok(false);
    };
    let report = process_authoritative_path_set(
        &mut catalog,
        AuthoritativePathSetContext {
            root_id: &location.root_id,
            root_generation,
            expected_catalog_revision: root.catalog_revision,
            expected_root_identity: identity,
            leased: &leased,
            relative_paths: std::slice::from_ref(&location.relative_path),
            now_unix_ms: now,
            queue_policy: policy,
            cancellation: &AtomicBool::new(false),
        },
        &discovery,
    )?;
    Ok(report.completed_count == 1)
}

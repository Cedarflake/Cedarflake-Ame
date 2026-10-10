use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::{
    AssetLocationView, IncrementalLibraryChangeReport, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeLane, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, PreviewRequest, ScanError, ScanIssue,
};
use crate::ports::{
    IncrementalCatalogRepository, SourceReconciliationAdmission, SourceReconciliationRepository,
};

use super::super::incremental_library_changes::{
    AuthoritativePathSetContext, process_authoritative_path_set,
};
use super::super::{StoragePaths, catalog_session};

enum SourceReconciliationOutcome {
    RequestRetired,
    PathWorkRetained,
    Unresolved,
}

impl SourceReconciliationOutcome {
    fn from_report(report: IncrementalLibraryChangeReport) -> Self {
        if report.completed_count == 1 {
            return Self::RequestRetired;
        }
        if report.superseded_count == 1 {
            return Self::RequestRetired;
        }
        // Acknowledged retry/deferral transfers completion to the durable path owner.
        // A storage error propagates before a report exists; it cannot retire the request.
        if report.retried_count == 1 {
            return Self::PathWorkRetained;
        }
        if report.deferred_count == 1 {
            return Self::PathWorkRetained;
        }
        Self::Unresolved
    }
}

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
            | "preview_source_missing"
    ) {
        return ScanError::new(issue.code, issue.message);
    }
    match reconcile_source(storage, request, location, root_generation) {
        Ok(SourceReconciliationOutcome::RequestRetired) => ScanError::new(
            "preview_request_superseded",
            format!(
                "{}: source context was superseded during reconciliation",
                issue.code
            ),
        ),
        Ok(SourceReconciliationOutcome::PathWorkRetained) => ScanError::new(
            "preview_request_superseded",
            "The changed source is retained by durable path reconciliation",
        ),
        Ok(SourceReconciliationOutcome::Unresolved) if issue.code == "preview_source_missing" => {
            ScanError::new(
                "preview_request_superseded",
                "The preview source is missing; catalog reconciliation retains its existing authority",
            )
        }
        Ok(SourceReconciliationOutcome::Unresolved) => ScanError::new(issue.code, issue.message),
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
) -> Result<SourceReconciliationOutcome, ScanError> {
    let mut catalog =
        catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let Some(root) = catalog.load_incremental_catalog_root(&location.root_id)? else {
        return Ok(SourceReconciliationOutcome::RequestRetired);
    };
    if root.root_generation != root_generation
        || root.active_scan_id.as_deref() != Some(&request.expected_scan_id)
    {
        return Ok(SourceReconciliationOutcome::RequestRetired);
    }
    let Some(identity) = root.publication_root_identity.as_ref() else {
        return Ok(SourceReconciliationOutcome::Unresolved);
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
    let leased = match catalog.admit_source_reconciliation(request, &intent, now, policy)? {
        SourceReconciliationAdmission::Leased(leased) => leased,
        SourceReconciliationAdmission::RequestSuperseded
        | SourceReconciliationAdmission::ExistingPathWork => {
            return Ok(SourceReconciliationOutcome::RequestRetired);
        }
        SourceReconciliationAdmission::LeaseUnavailable => {
            return Ok(SourceReconciliationOutcome::Unresolved);
        }
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
    Ok(SourceReconciliationOutcome::from_report(report))
}

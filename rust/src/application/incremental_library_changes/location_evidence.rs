use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery, user_visible_path};
use crate::domain::{
    AssetLocationView, DerivedEvidenceDisposition, DiscoveredFile,
    IncrementalReconciliationDecision, IncrementalReconciliationOutcome, LeasedLibraryChange,
    LibraryChangeFailure, PreviewStatus,
};
use crate::ports::{MediaInspectionFailureKind, MediaInspector};

use super::{failure, incremental_asset_id, issue_failure, stable_location_id};

pub(super) struct BuiltLocation {
    pub(super) location: AssetLocationView,
    pub(super) issue: Option<LibraryChangeFailure>,
}

pub(super) enum BuildLocationFailure {
    Retry(LibraryChangeFailure),
    Terminal(LibraryChangeFailure),
}

pub(super) fn build_location(
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
    let (width, height, metadata_engine_id, metadata_engine_version, camera, capture_time, issue) =
        if retains_compatible {
            let prior = prior.expect("compatible evidence requires a prior location");
            (
                prior.width,
                prior.height,
                prior.metadata_engine_id.clone(),
                prior.metadata_engine_version.clone(),
                prior.camera.clone(),
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
                inspection.metadata.camera,
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
            camera,
            capture_time,
        },
        issue,
    })
}

pub(super) fn inspection_metadata_is_compatible(
    prior: &AssetLocationView,
    inspector: &LocalMediaInspector,
) -> bool {
    prior.metadata_engine_id == inspector.metadata_engine_id()
        && prior.metadata_engine_version == inspector.metadata_engine_version()
}

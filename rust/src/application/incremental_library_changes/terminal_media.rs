use crate::adapters::{LocalMediaInspector, user_visible_path};
use crate::domain::{
    AssetLocationView, CatalogDeltaMutation, DerivedEvidenceDisposition, DiscoveredFile,
    IncrementalReconciliationOutcome, LeasedLibraryChange, LibraryChangeCompletion,
    LibraryChangeFailure, PreviewStatus, TerminalMediaEvidence, TerminalMediaEvidenceUpdate,
};
use crate::ports::MediaInspector;

use super::{
    PreparedChange, RevalidationTarget, expected_state, incremental_asset_id, stable_location_id,
};

pub(super) fn terminal_media_change(
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
    let evidence = TerminalMediaEvidenceUpdate {
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
            issue: issue.clone(),
        },
    };
    // Signature probing admits images without recognized suffixes, not arbitrary documents.
    // Known media (including another File ID alias) still retains terminal gallery evidence.
    if !report_issue && prior.is_none() && removals.is_empty() {
        return PreparedChange {
            completion: LibraryChangeCompletion {
                change_id: leased.change.id,
                lease_generation: leased.lease_generation,
                issue: None,
            },
            mutations: Vec::new(),
            terminal_media_evidence: vec![evidence],
            revalidation: vec![RevalidationTarget::Present {
                relative_path,
                expected,
            }],
        };
    }
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
        terminal_media_evidence: vec![evidence],
        revalidation: vec![RevalidationTarget::Present {
            relative_path,
            expected,
        }],
    }
}

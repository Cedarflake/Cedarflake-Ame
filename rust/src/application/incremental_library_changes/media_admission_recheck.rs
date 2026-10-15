use crate::domain::{
    DerivedEvidenceDisposition, IncrementalReconciliationOutcome, LeasedLibraryChange,
    LibraryChangeFailure,
};
use crate::ports::IncrementalCatalogRepository;

use super::{PreparationCatalog, PreparedChange, push_unique, scan_failure};

pub(super) fn apply(
    repository: &mut PreparationCatalog<'_, impl IncrementalCatalogRepository>,
    leased: &LeasedLibraryChange,
    prepared: &mut PreparedChange,
) -> Result<(), LibraryChangeFailure> {
    let mut recheck = repository
        .load_media_admission_recheck(
            &leased.change.intent.root_id,
            &leased.change.intent.relative_path,
        )
        .map_err(scan_failure)?;
    if recheck.is_none()
        && let Some(previous_path) = &leased.change.intent.previous_relative_path
    {
        recheck = repository
            .load_media_admission_recheck(&leased.change.intent.root_id, previous_path)
            .map_err(scan_failure)?;
    }
    let Some(recheck) = recheck else {
        return Ok(());
    };
    for mutation in &mut prepared.mutations {
        let Some(location) = &mut mutation.upsert_location else {
            continue;
        };
        if location.width > 0 && location.height > 0 {
            if location.file_identity.is_some() && location.file_identity == recheck.file_identity {
                location.asset_id.clone_from(&recheck.asset_id);
            }
            continue;
        }
        let owns_failed_location = location.location_id == recheck.location_id
            || (mutation.remove_location_ids.contains(&recheck.location_id)
                && location.file_identity.is_some()
                && location.file_identity == recheck.file_identity);
        let confirmed_non_image = recheck.requires_inspection
            && owns_failed_location
            && mutation.outcome == IncrementalReconciliationOutcome::TerminalIssue
            && prepared.terminal_media_evidence.iter().any(|update| {
                update.change_id == mutation.change_id
                    && update.evidence.relative_path == location.relative_path
                    && matches!(
                        update.evidence.issue.code.as_str(),
                        "image_format_unsupported" | "media_type_unsupported"
                    )
            });
        if confirmed_non_image {
            push_unique(
                &mut mutation.remove_location_ids,
                location.location_id.clone(),
            );
            mutation.upsert_location = None;
            mutation.outcome = IncrementalReconciliationOutcome::Removed;
            mutation.evidence_disposition = DerivedEvidenceDisposition::RemoveFromCurrentProjection;
            mutation.retained_preview_expectation = None;
            prepared.completion.issue = None;
        }
    }
    Ok(())
}

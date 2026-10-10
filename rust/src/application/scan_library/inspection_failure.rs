use crate::adapters::SqliteCatalog;
use crate::domain::{AssetLocationView, DiscoveredFile, ScanCheckpoint, ScanError, ScanIssue};
use crate::ports::{CatalogRepository, MediaInspectionFailure, MediaInspectionFailureKind};

use super::finalization::FinalizationPlan;

pub(super) struct FailedFileContext<'a> {
    pub scan_id: &'a str,
    pub root_id: &'a str,
    pub file: &'a DiscoveredFile,
    pub preservation_prior: Option<&'a AssetLocationView>,
    pub had_published_root: bool,
    pub accepted_items: u64,
}

pub(super) fn record_failed_file(
    catalog: &mut SqliteCatalog,
    finalization: &mut FinalizationPlan,
    context: FailedFileContext<'_>,
    failure: MediaInspectionFailure,
    checkpoint: &mut ScanCheckpoint,
    issue_count: &mut u64,
) -> Result<(u64, ScanIssue), ScanError> {
    let is_retryable = failure.kind == MediaInspectionFailureKind::Retryable;
    if !is_retryable {
        finalization.record_rejected_input(catalog, context.file)?;
    }
    let issue = failure.issue;
    *issue_count = issue_count.checked_add(1).ok_or_else(|| {
        ScanError::new(
            "issue_count_overflow",
            "The issue count exceeded the supported range",
        )
    })?;
    catalog.record_issue(context.scan_id, &issue)?;
    let mut accepted_items = context.accepted_items;
    if context.had_published_root
        && is_retryable
        && let Some(prior) = context.preservation_prior
    {
        catalog.stage_location(context.scan_id, context.root_id, prior)?;
        accepted_items = accepted_items.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "accepted_item_count_overflow",
                "The accepted item count exceeded the supported range",
            )
        })?;
    }
    if is_retryable && !finalization.record_retryable_path(&context.file.relative_path) {
        checkpoint.requires_previous_snapshot = true;
    }
    checkpoint.accepted_items = accepted_items;
    checkpoint.issue_count = *issue_count;
    catalog.checkpoint_scan(context.scan_id, checkpoint)?;
    Ok((accepted_items, issue))
}

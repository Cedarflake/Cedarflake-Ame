use super::*;

use crate::adapters::RejectedInputValidationRoster;
use crate::domain::DiscoveredFile;

#[derive(Default)]
pub(super) struct RejectedInputs {
    roster: Option<RejectedInputValidationRoster>,
}

impl RejectedInputs {
    pub(super) fn record(
        &mut self,
        catalog: &SqliteCatalog,
        file: &DiscoveredFile,
    ) -> Result<(), ScanError> {
        if self.roster.is_none() {
            self.roster = Some(RejectedInputValidationRoster::create(catalog)?);
        }
        if let Some(roster) = &self.roster {
            roster.record(catalog, file)?;
        }
        Ok(())
    }

    fn load_window(
        &self,
        catalog: &SqliteCatalog,
        after: Option<&str>,
    ) -> Result<Vec<(String, crate::domain::ExpectedFileState)>, ScanError> {
        self.roster.as_ref().map_or_else(
            || Ok(Vec::new()),
            |roster| roster.load_window(catalog, after, FINALIZATION_WINDOW),
        )
    }
}

pub(super) fn validate_rejected_inputs(
    catalog: &mut SqliteCatalog,
    plan: &mut FinalizationPlan,
    context: &FinalizationContext<'_>,
    guard: &PublicationGuardedFileDiscovery,
    total_items: u64,
    issue_count: &mut u64,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<bool, ScanError> {
    let mut after = None;
    let mut checked = 0_u64;
    loop {
        let window = plan
            .rejected_inputs
            .load_window(catalog, after.as_deref())?;
        if window.is_empty() {
            return Ok(true);
        }
        for (relative_path, expected) in window {
            if super::super::finish_if_controlled(
                context.control.load(Ordering::Relaxed),
                catalog,
                context.request,
                context.checkpoint,
                *issue_count,
                publish,
                context.had_published_root,
            )? {
                return Ok(false);
            }
            // A terminal decoder result can exclude only the exact source version inspected.
            let validation = if expected.source_revision.is_none() {
                Err(ScanIssue {
                    path: Some(expected.absolute_path.clone()),
                    code: "source_revision_unavailable".to_owned(),
                    message: "The rejected media input has no verifiable source revision"
                        .to_owned(),
                })
            } else {
                guard.revalidate_relative_file_state(&relative_path, &expected)
            };
            if let Err(issue) = validation {
                if !plan.record_retryable_path(&relative_path) {
                    return Err(ScanError::new(
                        "scan_rejected_input_handoff_overflow",
                        "Changed rejected inputs exceeded the bounded recovery handoff",
                    ));
                }
                *issue_count = issue_count.checked_add(1).ok_or_else(|| {
                    ScanError::new(
                        "scan_issue_count_overflow",
                        "The scan issue count exceeded the supported range",
                    )
                })?;
                catalog.record_issue(&context.request.scan_id, &issue)?;
                if !publish(ScanEvent::Issue {
                    scan_id: context.request.scan_id.clone(),
                    issue: super::super::user_visible_issue(issue),
                }) {
                    return retain_detached(catalog, context, *issue_count);
                }
            }
            after = Some(relative_path);
            checked += 1;
            if checked.is_multiple_of(super::super::CHECKPOINT_INTERVAL)
                && !publish(finalizing_event(context, 0, total_items, *issue_count))
            {
                return retain_detached(catalog, context, *issue_count);
            }
        }
    }
}

fn retain_detached(
    catalog: &mut SqliteCatalog,
    context: &FinalizationContext<'_>,
    issue_count: u64,
) -> Result<bool, ScanError> {
    super::super::retain_detached_scan(
        catalog,
        context.control.load(Ordering::Relaxed),
        context.request,
        context.checkpoint,
        issue_count,
        context.had_published_root,
    )?;
    Ok(false)
}

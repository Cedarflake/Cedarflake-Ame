use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use crate::adapters::{SqliteCatalog, ValidatedStagingProof};
use crate::domain::{
    LibraryChangeLane, LibraryChangeQueuePolicy, LibraryRootGeneration, ScanCheckpoint, ScanError,
    ScanEvent, ScanRequest,
};

use super::finalization::FinalizationPlan;

const PUBLICATION_RETRY_WAIT: Duration = Duration::from_millis(250);
const PUBLICATION_PROGRESS_INTERVAL: Duration = Duration::from_secs(1);

pub(super) struct ForegroundPublicationContext<'a> {
    pub(super) control: &'a AtomicU8,
    pub(super) request: &'a ScanRequest,
    pub(super) checkpoint: &'a ScanCheckpoint,
    pub(super) root_id: &'a str,
    pub(super) root_generation: LibraryRootGeneration,
    pub(super) accepted_items: u64,
    pub(super) validated_items: u64,
    pub(super) total_items: u64,
    pub(super) visited_entries: u64,
    pub(super) issue_count: u64,
    pub(super) had_published_root: bool,
    pub(super) validation_proof: &'a ValidatedStagingProof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ForegroundPublicationOutcome {
    Published,
    Interrupted,
}

pub(super) fn publish_foreground_scan(
    catalog: &mut SqliteCatalog,
    finalization: &FinalizationPlan,
    context: ForegroundPublicationContext<'_>,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<ForegroundPublicationOutcome, ScanError> {
    finalization.handoff_foreground_revalidation(
        catalog,
        context.root_id,
        context.root_generation,
    )?;
    let mut last_progress = Instant::now();
    loop {
        let observed_write_epoch = catalog.completed_write_epoch();
        match context.validation_proof.publish(
            catalog,
            context.root_id,
            context.accepted_items,
            context.issue_count,
        ) {
            Ok(()) => return Ok(ForegroundPublicationOutcome::Published),
            Err(error) => {
                let Some(delay) = PublicationDelay::from_error(&error) else {
                    return Err(error);
                };
                if super::finish_if_controlled(
                    context.control.load(Ordering::Relaxed),
                    catalog,
                    context.request,
                    context.checkpoint,
                    publish,
                    context.had_published_root,
                )? {
                    return Ok(ForegroundPublicationOutcome::Interrupted);
                }
                let drained_live_work = delay == PublicationDelay::LiveChangesPending
                    && context.had_published_root
                    && drain_one_live_batch(catalog, &context)?;
                if last_progress.elapsed() >= PUBLICATION_PROGRESS_INTERVAL {
                    if !publish(ScanEvent::Finalizing {
                        scan_id: context.request.scan_id.clone(),
                        validated_items: context.validated_items,
                        total_items: context.total_items,
                        visited_entries: context.visited_entries,
                        accepted_items: context.accepted_items,
                        issue_count: context.issue_count,
                    }) {
                        super::retain_detached_scan(
                            catalog,
                            context.control.load(Ordering::Relaxed),
                            context.request,
                            context.checkpoint,
                            context.issue_count,
                            context.had_published_root,
                        )?;
                        return Ok(ForegroundPublicationOutcome::Interrupted);
                    }
                    last_progress = Instant::now();
                }
                if drained_live_work {
                    continue;
                }
                catalog
                    .wait_for_completed_write_after(observed_write_epoch, PUBLICATION_RETRY_WAIT);
            }
        }
    }
}

fn drain_one_live_batch(
    catalog: &mut SqliteCatalog,
    context: &ForegroundPublicationContext<'_>,
) -> Result<bool, ScanError> {
    let report = super::super::incremental_library_changes::process_ready_library_changes_in_lane(
        catalog,
        context.root_id,
        context.root_generation,
        LibraryChangeLane::Live,
        super::current_unix_ms()?,
        LibraryChangeQueuePolicy::default(),
    )?;
    Ok(report.leased_count > 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublicationDelay {
    LiveChangesPending,
    Preempted,
}

impl PublicationDelay {
    fn from_error(error: &ScanError) -> Option<Self> {
        match error.code.as_str() {
            "catalog_scan_live_changes_pending" => Some(Self::LiveChangesPending),
            "catalog_scan_publication_preempted" => Some(Self::Preempted),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_delay_accepts_only_change_first_retry_contracts() {
        assert_eq!(
            PublicationDelay::from_error(&ScanError::new(
                "catalog_scan_live_changes_pending",
                "pending",
            )),
            Some(PublicationDelay::LiveChangesPending),
        );
        assert_eq!(
            PublicationDelay::from_error(&ScanError::new(
                "catalog_scan_publication_preempted",
                "preempted",
            )),
            Some(PublicationDelay::Preempted),
        );
        assert_eq!(
            PublicationDelay::from_error(&ScanError::new("catalog_database_busy", "busy")),
            None,
        );
    }
}

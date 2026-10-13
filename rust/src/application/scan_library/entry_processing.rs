use crate::adapters::{FileVisitOutcome, SqliteCatalog};
use crate::domain::{ScanCheckpoint, ScanError, ScanEvent};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, MediaInspector};

use super::file_preparation::PreparedScanFile;
use super::finalization::FinalizationPlan;
use super::inspection_failure::{FailedFileContext, record_failed_file};
use super::user_visible_issue;

#[cfg(test)]
mod tests;

pub(super) struct ScanEntryContext<'a> {
    pub scan_id: &'a str,
    pub root_id: &'a str,
    pub relative_path: &'a str,
    pub had_published_root: bool,
    pub has_active_locations: bool,
    pub accepted_items: u64,
    pub outcome: FileVisitOutcome,
}

pub(super) enum ScanEntryOutcome {
    Applied {
        accepted_items: u64,
        event: Option<ScanEvent>,
    },
    Detached,
}

pub(super) fn apply_scan_entry(
    catalog: &mut SqliteCatalog,
    finalization: &mut FinalizationPlan,
    inspector: &impl MediaInspector,
    context: ScanEntryContext<'_>,
    checkpoint: &mut ScanCheckpoint,
    issue_count: &mut u64,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<ScanEntryOutcome, ScanError> {
    let ScanEntryContext {
        scan_id,
        root_id,
        relative_path,
        had_published_root,
        has_active_locations,
        mut accepted_items,
        outcome,
    } = context;
    let mut discovered_event = None;
    match outcome {
        FileVisitOutcome::Directory => {
            measure_scan_operation!(
                DirectoryPersistence,
                catalog.enqueue_directory(scan_id, relative_path)
            )?;
        }
        FileVisitOutcome::Ignored => {}
        FileVisitOutcome::TerminalMedia {
            file,
            issue,
            report_issue,
        } => {
            let known_media = finalization.has_retained_rejection(&file.relative_path)
                || has_active_locations
                    && catalog
                        .load_incremental_location_by_relative_path(root_id, &file.relative_path)?
                        .is_some();
            if known_media {
                finalization.record_rejected_input(catalog, &file)?;
            }
            if report_issue || known_media {
                *issue_count += 1;
                catalog.record_issue(scan_id, &issue)?;
                discovered_event = Some(ScanEvent::Issue {
                    scan_id: scan_id.to_owned(),
                    issue: user_visible_issue(issue),
                });
            }
        }
        FileVisitOutcome::Issue(issue) => {
            *issue_count += 1;
            catalog.record_issue(scan_id, &issue)?;
            if had_published_root {
                if let Some(prior) =
                    catalog.load_incremental_location_by_relative_path(root_id, relative_path)?
                {
                    catalog.stage_location(scan_id, root_id, &prior)?;
                    accepted_items = accepted_items.checked_add(1).ok_or_else(|| {
                        ScanError::new(
                            "accepted_item_count_overflow",
                            "The accepted item count exceeded the supported range",
                        )
                    })?;
                }
                checkpoint.accepted_items = accepted_items;
                checkpoint.issue_count = *issue_count;
                checkpoint.requires_previous_snapshot = true;
                catalog.checkpoint_scan(scan_id, checkpoint)?;
            }
            discovered_event = Some(ScanEvent::Issue {
                scan_id: scan_id.to_owned(),
                issue: user_visible_issue(issue),
            });
        }
        FileVisitOutcome::RetryableFile { file, issue } => {
            let prior = if had_published_root {
                catalog.load_incremental_location_by_relative_path(root_id, &file.relative_path)?
            } else {
                None
            };
            let (accepted, issue) = record_failed_file(
                catalog,
                finalization,
                FailedFileContext {
                    scan_id,
                    root_id,
                    file: &file,
                    preservation_prior: prior.as_ref(),
                    had_published_root,
                    accepted_items,
                },
                crate::ports::MediaInspectionFailure {
                    kind: crate::ports::MediaInspectionFailureKind::Retryable,
                    issue,
                },
                checkpoint,
                issue_count,
            )?;
            accepted_items = accepted;
            discovered_event = Some(ScanEvent::Issue {
                scan_id: scan_id.to_owned(),
                issue: user_visible_issue(issue),
            });
        }
        FileVisitOutcome::File(file) => {
            for issue in &file.issues {
                *issue_count += 1;
                catalog.record_issue(scan_id, issue)?;
                if !publish(ScanEvent::Issue {
                    scan_id: scan_id.to_owned(),
                    issue: user_visible_issue(issue.clone()),
                }) {
                    return Ok(ScanEntryOutcome::Detached);
                }
            }
            let prepared = measure_scan_operation!(
                PriorSelection,
                PreparedScanFile::load(
                    catalog,
                    inspector,
                    scan_id,
                    root_id,
                    file,
                    has_active_locations,
                )
            )?;
            let inspection = measure_scan_operation!(MediaInspection, prepared.inspect(inspector));
            match inspection {
                Ok(mut inspection) => {
                    for issue in std::mem::take(&mut inspection.metadata.issues) {
                        *issue_count += 1;
                        catalog.record_issue(scan_id, &issue)?;
                        if !publish(ScanEvent::Issue {
                            scan_id: scan_id.to_owned(),
                            issue: user_visible_issue(issue),
                        }) {
                            return Ok(ScanEntryOutcome::Detached);
                        }
                    }
                    let asset = prepared.into_asset(inspection);
                    measure_scan_operation!(
                        LocationStaging,
                        catalog.stage_location(scan_id, root_id, &asset)
                    )?;
                    accepted_items += 1;
                    discovered_event = Some(ScanEvent::AssetDiscovered {
                        scan_id: scan_id.to_owned(),
                        asset: Box::new(asset),
                    });
                }
                Err(failure) => {
                    let (accepted, issue) = record_failed_file(
                        catalog,
                        finalization,
                        FailedFileContext {
                            scan_id,
                            root_id,
                            file: prepared.file(),
                            preservation_prior: prepared.preservation_prior(),
                            had_published_root,
                            accepted_items,
                        },
                        failure,
                        checkpoint,
                        issue_count,
                    )?;
                    accepted_items = accepted;
                    discovered_event = Some(ScanEvent::Issue {
                        scan_id: scan_id.to_owned(),
                        issue: user_visible_issue(issue),
                    });
                }
            }
        }
    }

    Ok(ScanEntryOutcome::Applied {
        accepted_items,
        event: discovered_event,
    })
}

use std::sync::atomic::{AtomicU8, Ordering};

use crate::adapters::{FileDiscovery, SqliteCatalog};
use crate::domain::{ScanCheckpoint, ScanError, ScanEvent, ScanIssue, ScanRequest};
use crate::ports::{CatalogRepository, MediaInspector};

use super::entry_processing::{ScanEntryContext, ScanEntryOutcome, apply_scan_entry};
use super::finalization::FinalizationPlan;
use super::{CHECKPOINT_INTERVAL, finish_if_controlled, retain_detached_scan, user_visible_issue};

const DIRECTORY_ENTRY_BATCH: usize = 256;
const DIRECTORY_ENTRY_WINDOW: u32 = 256;

pub(super) struct ScanTraversalContext<'a, I> {
    pub request: &'a ScanRequest,
    pub control: &'a AtomicU8,
    pub discovery: &'a FileDiscovery,
    pub inspector: &'a I,
    pub root_id: &'a str,
    pub had_published_root: bool,
    pub has_active_locations: bool,
}

pub(super) enum ScanTraversalOutcome {
    Stopped,
    Traversed {
        visited_entries: u64,
        accepted_items: u64,
        was_limited: bool,
    },
}

pub(super) fn traverse_scan(
    catalog: &mut SqliteCatalog,
    finalization: &mut FinalizationPlan,
    context: ScanTraversalContext<'_, impl MediaInspector>,
    checkpoint: &mut ScanCheckpoint,
    issue_count: &mut u64,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<ScanTraversalOutcome, ScanError> {
    let ScanTraversalContext {
        request,
        control,
        discovery,
        inspector,
        root_id,
        had_published_root,
        has_active_locations,
    } = context;
    let mut visited_entries = checkpoint.visited_entries;
    let mut accepted_items = checkpoint.accepted_items;
    let mut was_limited = false;
    'traversal: loop {
        if finish_if_controlled(
            control.load(Ordering::Relaxed),
            catalog,
            request,
            checkpoint,
            *issue_count,
            publish,
            had_published_root,
        )? {
            return Ok(ScanTraversalOutcome::Stopped);
        }
        let Some(relative_directory) = measure_scan_operation!(
            DirectoryPersistence,
            catalog.claim_next_directory(&request.scan_id)
        )?
        else {
            break;
        };
        if !measure_scan_operation!(
            DirectoryPersistence,
            catalog.is_current_directory_enumerated(&request.scan_id, &relative_directory)
        )? {
            let entries = match measure_scan_operation!(
                SourceDiscovery,
                discovery.entry_paths_in_directory(&relative_directory)
            ) {
                Ok(entries) => entries,
                Err(issue) => {
                    *issue_count += 1;
                    catalog.record_issue(&request.scan_id, &issue)?;
                    checkpoint.last_visited_relative_path = None;
                    checkpoint.issue_count = *issue_count;
                    if !publish(ScanEvent::Issue {
                        scan_id: request.scan_id.clone(),
                        issue: user_visible_issue(issue),
                    }) {
                        retain_detached_scan(
                            catalog,
                            control.load(Ordering::Relaxed),
                            request,
                            checkpoint,
                            *issue_count,
                            had_published_root,
                        )?;
                        return Ok(ScanTraversalOutcome::Stopped);
                    }
                    if had_published_root {
                        catalog.abandon_scan(&request.scan_id, "stale", *issue_count)?;
                        publish(ScanEvent::Stale {
                            scan_id: request.scan_id.clone(),
                            accepted_items,
                            issue_count: *issue_count,
                        });
                        return Ok(ScanTraversalOutcome::Stopped);
                    }
                    measure_scan_operation!(
                        DirectoryPersistence,
                        catalog.complete_directory(&request.scan_id, checkpoint)
                    )?;
                    continue;
                }
            };
            #[cfg(test)]
            let entries = super::operation_cost::DiscoveryCostIterator::new(entries);
            let mut batch = Vec::with_capacity(DIRECTORY_ENTRY_BATCH);
            for relative_path in entries {
                batch.push(relative_path);
                if batch.len() == DIRECTORY_ENTRY_BATCH {
                    measure_scan_operation!(
                        DirectoryPersistence,
                        catalog.stage_directory_entries(
                            &request.scan_id,
                            &relative_directory,
                            &batch,
                        )
                    )?;
                    batch.clear();
                    if finish_if_controlled(
                        control.load(Ordering::Relaxed),
                        catalog,
                        request,
                        checkpoint,
                        *issue_count,
                        publish,
                        had_published_root,
                    )? {
                        return Ok(ScanTraversalOutcome::Stopped);
                    }
                }
            }
            measure_scan_operation!(
                DirectoryPersistence,
                catalog.stage_directory_entries(&request.scan_id, &relative_directory, &batch)
            )?;
            measure_scan_operation!(
                DirectoryPersistence,
                catalog.complete_directory_enumeration(&request.scan_id, &relative_directory)
            )?;
        }

        if let Some(saved_path) = checkpoint.last_visited_relative_path.as_deref()
            && !measure_scan_operation!(
                DirectoryPersistence,
                catalog.has_directory_entry(&request.scan_id, &relative_directory, saved_path)
            )?
        {
            let issue = ScanIssue {
                path: checkpoint.last_visited_relative_path.clone(),
                code: "scan_checkpoint_unavailable".to_owned(),
                message: "The saved position no longer exists in the current directory".to_owned(),
            };
            *issue_count += 1;
            catalog.record_issue(&request.scan_id, &issue)?;
            publish(ScanEvent::Issue {
                scan_id: request.scan_id.clone(),
                issue: user_visible_issue(issue),
            });
            catalog.abandon_scan(&request.scan_id, "stale", *issue_count)?;
            publish(ScanEvent::Stale {
                scan_id: request.scan_id.clone(),
                accepted_items,
                issue_count: *issue_count,
            });
            return Ok(ScanTraversalOutcome::Stopped);
        }

        if request
            .max_entries
            .is_some_and(|limit| visited_entries >= u64::from(limit))
            || request
                .max_items
                .is_some_and(|limit| accepted_items >= u64::from(limit))
        {
            was_limited = true;
            break 'traversal;
        }

        loop {
            let relative_paths = measure_scan_operation!(
                DirectoryPersistence,
                catalog.load_directory_entry_window(
                    &request.scan_id,
                    &relative_directory,
                    checkpoint.last_visited_relative_path.as_deref(),
                    DIRECTORY_ENTRY_WINDOW,
                )
            )?;
            if relative_paths.is_empty() {
                break;
            }

            for relative_path in relative_paths {
                if finish_if_controlled(
                    control.load(Ordering::Relaxed),
                    catalog,
                    request,
                    checkpoint,
                    *issue_count,
                    publish,
                    had_published_root,
                )? {
                    return Ok(ScanTraversalOutcome::Stopped);
                }

                let visit = measure_scan_operation!(
                    SourceDiscovery,
                    discovery.visit_relative_path(&relative_path)
                );

                visited_entries = visited_entries.checked_add(1).ok_or_else(|| {
                    ScanError::new(
                        "entry_count_overflow",
                        "The directory entry count exceeded the supported range",
                    )
                })?;
                if request
                    .max_entries
                    .is_some_and(|limit| visited_entries > u64::from(limit))
                {
                    was_limited = true;
                    break 'traversal;
                }

                let discovered_event = match apply_scan_entry(
                    catalog,
                    finalization,
                    inspector,
                    ScanEntryContext {
                        scan_id: &request.scan_id,
                        root_id,
                        relative_path: &visit.relative_path,
                        had_published_root,
                        has_active_locations,
                        accepted_items,
                        outcome: visit.outcome,
                    },
                    checkpoint,
                    issue_count,
                    publish,
                )? {
                    ScanEntryOutcome::Applied {
                        accepted_items: accepted,
                        event,
                    } => {
                        accepted_items = accepted;
                        event
                    }
                    ScanEntryOutcome::Detached => {
                        retain_detached_scan(
                            catalog,
                            control.load(Ordering::Relaxed),
                            request,
                            checkpoint,
                            *issue_count,
                            had_published_root,
                        )?;
                        return Ok(ScanTraversalOutcome::Stopped);
                    }
                };

                checkpoint.last_visited_relative_path = Some(visit.relative_path);
                checkpoint.visited_entries = visited_entries;
                checkpoint.accepted_items = accepted_items;
                checkpoint.issue_count = *issue_count;
                if visited_entries.is_multiple_of(CHECKPOINT_INTERVAL) {
                    measure_scan_operation!(
                        CheckpointPersistence,
                        catalog.checkpoint_scan(&request.scan_id, checkpoint)
                    )?;
                }

                let did_accept_asset =
                    matches!(&discovered_event, Some(ScanEvent::AssetDiscovered { .. }));
                if discovered_event.is_some_and(|event| !publish(event)) {
                    retain_detached_scan(
                        catalog,
                        control.load(Ordering::Relaxed),
                        request,
                        checkpoint,
                        *issue_count,
                        had_published_root,
                    )?;
                    return Ok(ScanTraversalOutcome::Stopped);
                }
                let should_publish_progress = visited_entries == 1
                    || visited_entries.is_multiple_of(CHECKPOINT_INTERVAL)
                    || did_accept_asset && accepted_items > 0 && accepted_items.is_multiple_of(25);
                if should_publish_progress
                    && !publish(ScanEvent::Progress {
                        scan_id: request.scan_id.clone(),
                        visited_entries,
                        accepted_items,
                        issue_count: *issue_count,
                    })
                {
                    retain_detached_scan(
                        catalog,
                        control.load(Ordering::Relaxed),
                        request,
                        checkpoint,
                        *issue_count,
                        had_published_root,
                    )?;
                    return Ok(ScanTraversalOutcome::Stopped);
                }
                if request
                    .max_items
                    .is_some_and(|limit| accepted_items >= u64::from(limit))
                {
                    was_limited = true;
                    break 'traversal;
                }
            }
        }

        checkpoint.last_visited_relative_path = None;
        measure_scan_operation!(
            DirectoryPersistence,
            catalog.complete_directory(&request.scan_id, checkpoint)
        )?;
    }

    measure_scan_operation!(
        CheckpointPersistence,
        catalog.checkpoint_scan(&request.scan_id, checkpoint)
    )?;

    Ok(ScanTraversalOutcome::Traversed {
        visited_entries,
        accepted_items,
        was_limited,
    })
}

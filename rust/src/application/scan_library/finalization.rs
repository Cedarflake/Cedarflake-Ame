use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::adapters::{
    PublicationGuardedFileDiscovery, SqliteCatalog, StagedValidationOutcome,
    StagedValidationRoster, ValidatedStagingProof, revalidate_file_state,
};
use crate::domain::{
    FileIdentityEvidence, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration, ScanCheckpoint, ScanError,
    ScanEvent, ScanIssue, ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue};

const FINALIZATION_WINDOW: u32 = 256;
const RETRY_PATH_LIMIT: usize = LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES as usize;

#[cfg(all(test, windows))]
mod control_tests;
mod rejected_inputs;
mod retained_issue_recovery;
use rejected_inputs::{RejectedInputs, validate_rejected_inputs};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FinalizationMode {
    Foreground,
    AuthoritativeRecovery,
}

pub(super) struct FinalizationPlan {
    mode: FinalizationMode,
    authoritative_retry_paths: BTreeSet<String>,
    foreground_revalidation: ForegroundRevalidationPlan,
    rejected_inputs: RejectedInputs,
}

impl FinalizationPlan {
    pub(super) fn new(mode: FinalizationMode) -> Self {
        Self {
            mode,
            authoritative_retry_paths: BTreeSet::new(),
            foreground_revalidation: ForegroundRevalidationPlan::default(),
            rejected_inputs: RejectedInputs::default(),
        }
    }

    pub(super) fn restore_retained_scan_issues(
        &mut self,
        catalog: &mut SqliteCatalog,
        scan_id: &str,
        canonical_root: &Path,
        requested_root: &Path,
    ) -> Result<bool, ScanError> {
        retained_issue_recovery::restore(self, catalog, scan_id, canonical_root, requested_root)
    }

    pub(super) fn record_retryable_path(&mut self, relative_path: &str) -> bool {
        match self.mode {
            FinalizationMode::AuthoritativeRecovery => {
                retain_bounded_path(&mut self.authoritative_retry_paths, relative_path)
            }
            FinalizationMode::Foreground => {
                self.foreground_revalidation.record(relative_path);
                true
            }
        }
    }

    pub(super) fn record_rejected_input(
        &mut self,
        catalog: &SqliteCatalog,
        file: &crate::domain::DiscoveredFile,
    ) -> Result<(), ScanError> {
        self.rejected_inputs.record(catalog, file)?;
        self.authoritative_retry_paths.remove(&file.relative_path);
        self.foreground_revalidation
            .paths
            .remove(&file.relative_path);
        Ok(())
    }

    pub(super) fn has_retained_rejection(&self, relative_path: &str) -> bool {
        self.authoritative_retry_paths.contains(relative_path)
            || self.foreground_revalidation.paths.contains(relative_path)
    }

    fn preserves_authoritative_retry_evidence(&self, relative_path: &str) -> bool {
        self.mode == FinalizationMode::AuthoritativeRecovery
            && self.authoritative_retry_paths.contains(relative_path)
    }

    pub(super) fn authoritative_retry_paths(&self) -> Vec<String> {
        debug_assert_eq!(self.mode, FinalizationMode::AuthoritativeRecovery);
        self.authoritative_retry_paths.iter().cloned().collect()
    }

    pub(super) fn handoff_foreground_revalidation(
        &self,
        catalog: &mut SqliteCatalog,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<(), ScanError> {
        debug_assert_eq!(self.mode, FinalizationMode::Foreground);
        self.foreground_revalidation
            .enqueue(catalog, root_id, root_generation)
    }
}

#[derive(Default)]
struct ForegroundRevalidationPlan {
    paths: BTreeSet<String>,
    overflowed: bool,
}

impl ForegroundRevalidationPlan {
    fn record(&mut self, relative_path: &str) {
        if self.overflowed || self.paths.contains(relative_path) {
            return;
        }
        if self.paths.len() >= RETRY_PATH_LIMIT {
            self.paths.clear();
            self.overflowed = true;
            return;
        }
        self.paths.insert(relative_path.to_owned());
    }

    fn enqueue(
        &self,
        catalog: &mut SqliteCatalog,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<(), ScanError> {
        if self.paths.is_empty() && !self.overflowed {
            return Ok(());
        }
        let observed_unix_ms = super::current_unix_ms()?;
        let intents = self.intents(root_id, root_generation, observed_unix_ms);
        let report = catalog.enqueue_library_change_intents(
            &intents,
            observed_unix_ms,
            LibraryChangeQueuePolicy::default(),
        )?;
        if report.stale_generation_count != 0
            || (report.inserted_count == 0
                && report.coalesced_count == 0
                && !report.freshness_unknown_enqueued)
        {
            return Err(ScanError::new(
                "foreground_scan_change_handoff_failed",
                "Changes observed during scan validation did not enter the durable live queue",
            ));
        }
        Ok(())
    }

    fn intents(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        observed_unix_ms: i64,
    ) -> Vec<LibraryChangeIntent> {
        if self.overflowed {
            return vec![LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: observed_unix_ms,
                most_recent_observed_unix_ms: observed_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }];
        }
        self.paths
            .iter()
            .zip(1_u64..)
            .map(|(relative_path, sequence)| LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: relative_path.clone(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: observed_unix_ms,
                most_recent_observed_unix_ms: observed_unix_ms,
                first_sequence: sequence,
                most_recent_sequence: sequence,
                coalesced_observation_count: 1,
            })
            .collect()
    }
}

pub(super) struct FinalizationContext<'a> {
    pub(super) control: &'a AtomicU8,
    pub(super) request: &'a ScanRequest,
    pub(super) checkpoint: &'a ScanCheckpoint,
    pub(super) root_id: &'a str,
    pub(super) root_path: &'a str,
    pub(super) publication_root_identity: &'a FileIdentityEvidence,
    pub(super) had_published_root: bool,
    pub(super) visited_entries: u64,
    pub(super) accepted_items: u64,
}

pub(super) struct ValidatedScan {
    pub(super) publication_guard: PublicationGuardedFileDiscovery,
    pub(super) validated_items: u64,
    pub(super) total_items: u64,
    pub(super) foreground_proof: Option<ValidatedStagingProof>,
}

pub(super) fn validate_staged_locations(
    catalog: &mut SqliteCatalog,
    plan: &mut FinalizationPlan,
    context: FinalizationContext<'_>,
    issue_count: &mut u64,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<Option<ValidatedScan>, ScanError> {
    let roster = if plan.mode == FinalizationMode::Foreground {
        Some(StagedValidationRoster::capture(
            catalog,
            &context.request.scan_id,
        )?)
    } else {
        None
    };
    let total_items = match &roster {
        Some(roster) => roster.total_items(),
        None => catalog.count_staged_file_states(&context.request.scan_id)?,
    };
    let publication_guard = match PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        context.root_path,
        context.publication_root_identity,
    ) {
        Ok(guard) => guard,
        Err(failure) => {
            if let Err(cleanup) = catalog.fail_scan_publication_namespace(
                &context.request.scan_id,
                context.root_id,
                &failure,
                *issue_count,
            ) {
                return Err(super::scan_cleanup_failure(&failure, &cleanup));
            }
            return Err(failure);
        }
    };
    if !publish(finalizing_event(&context, 0, total_items, *issue_count)) {
        super::retain_detached_scan(
            catalog,
            context.control.load(Ordering::Relaxed),
            context.request,
            context.checkpoint,
            *issue_count,
            context.had_published_root,
        )?;
        return Ok(None);
    }

    let mut validated_items = 0_u64;
    if !validate_rejected_inputs(
        catalog,
        plan,
        &context,
        &publication_guard,
        total_items,
        issue_count,
        publish,
    )? {
        return Ok(None);
    }
    let mut after_location_id = None;
    loop {
        let window = match &roster {
            Some(roster) => {
                roster.load_window(catalog, after_location_id.as_deref(), FINALIZATION_WINDOW)?
            }
            None => catalog.load_staged_file_state_window(
                &context.request.scan_id,
                after_location_id.as_deref(),
                FINALIZATION_WINDOW,
            )?,
        };
        if window.is_empty() {
            break;
        }
        for (location_id, relative_path, expected) in window {
            if super::finish_if_controlled(
                context.control.load(Ordering::Relaxed),
                catalog,
                context.request,
                context.checkpoint,
                *issue_count,
                publish,
                context.had_published_root,
            )? {
                return Ok(None);
            }
            let mut outcome = StagedValidationOutcome::FilesystemVerified;
            let issue = if plan.preserves_authoritative_retry_evidence(&relative_path) {
                None
            } else if let Err(issue) = revalidate_file_state(&expected) {
                if let Some(roster) = &roster
                    && roster.superseded_by_live(catalog, context.root_id, &location_id)?
                {
                    outcome = StagedValidationOutcome::SupersededByLive;
                    None
                } else {
                    outcome = StagedValidationOutcome::AwaitingLive;
                    Some(issue)
                }
            } else {
                None
            };
            if let Some(issue) = issue {
                *issue_count = (*issue_count).checked_add(1).ok_or_else(|| {
                    ScanError::new(
                        "scan_issue_count_overflow",
                        "The scan issue count exceeded the supported range",
                    )
                })?;
                catalog.record_issue(&context.request.scan_id, &issue)?;
                if !publish(ScanEvent::Issue {
                    scan_id: context.request.scan_id.clone(),
                    issue: super::user_visible_issue(issue),
                }) {
                    super::retain_detached_scan(
                        catalog,
                        context.control.load(Ordering::Relaxed),
                        context.request,
                        context.checkpoint,
                        *issue_count,
                        context.had_published_root,
                    )?;
                    return Ok(None);
                }
                if !plan.record_retryable_path(&relative_path) {
                    catalog.abandon_scan(&context.request.scan_id, "stale", *issue_count)?;
                    publish(ScanEvent::Stale {
                        scan_id: context.request.scan_id.clone(),
                        accepted_items: context.accepted_items,
                        issue_count: *issue_count,
                    });
                    return Ok(None);
                }
            }
            if let Some(roster) = &roster {
                roster.record_outcome(catalog, &location_id, outcome)?;
            }
            validated_items = validated_items.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "finalization_count_overflow",
                    "The final validation progress exceeded the supported range",
                )
            })?;
            after_location_id = Some(location_id);
            if (validated_items == total_items
                || validated_items.is_multiple_of(super::CHECKPOINT_INTERVAL))
                && !publish(finalizing_event(
                    &context,
                    validated_items,
                    total_items,
                    *issue_count,
                ))
            {
                super::retain_detached_scan(
                    catalog,
                    context.control.load(Ordering::Relaxed),
                    context.request,
                    context.checkpoint,
                    *issue_count,
                    context.had_published_root,
                )?;
                return Ok(None);
            }
        }
    }
    if validated_items != total_items {
        return Err(ScanError::new(
            "finalization_count_changed",
            "The staged catalog changed during final validation",
        ));
    }
    if super::finish_if_controlled(
        context.control.load(Ordering::Relaxed),
        catalog,
        context.request,
        context.checkpoint,
        *issue_count,
        publish,
        context.had_published_root,
    )? {
        return Ok(None);
    }
    Ok(Some(ValidatedScan {
        publication_guard,
        validated_items,
        total_items,
        foreground_proof: roster
            .map(|roster| roster.finish(validated_items))
            .transpose()?,
    }))
}

fn finalizing_event(
    context: &FinalizationContext<'_>,
    validated_items: u64,
    total_items: u64,
    issue_count: u64,
) -> ScanEvent {
    ScanEvent::Finalizing {
        scan_id: context.request.scan_id.clone(),
        validated_items,
        total_items,
        visited_entries: context.visited_entries,
        accepted_items: context.accepted_items,
        issue_count,
    }
}

fn retain_bounded_path(paths: &mut BTreeSet<String>, relative_path: &str) -> bool {
    if paths.contains(relative_path) {
        return true;
    }
    if paths.len() >= RETRY_PATH_LIMIT {
        return false;
    }
    paths.insert(relative_path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_revalidation_deduplicates_exact_paths() {
        let mut plan = FinalizationPlan::new(FinalizationMode::Foreground);
        assert!(plan.record_retryable_path("album/image.png"));
        assert!(plan.record_retryable_path("album/image.png"));

        let intents =
            plan.foreground_revalidation
                .intents("root", LibraryRootGeneration::initial(), 42);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].scope, LibraryChangeScope::Path);
        assert_eq!(intents[0].relative_path, "album/image.png");
    }

    #[test]
    fn foreground_revalidation_overflow_becomes_one_root_gap() {
        let mut plan = FinalizationPlan::new(FinalizationMode::Foreground);
        for index in 0..=RETRY_PATH_LIMIT {
            assert!(plan.record_retryable_path(&format!("image-{index}.png")));
        }

        let intents =
            plan.foreground_revalidation
                .intents("root", LibraryRootGeneration::initial(), 42);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].scope, LibraryChangeScope::Root);
        assert_eq!(intents[0].kind, LibraryChangeIntentKind::FreshnessUnknown);
    }
}

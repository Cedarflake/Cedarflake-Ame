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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FinalizationMode {
    Foreground,
    AuthoritativeRecovery,
}

pub(super) struct FinalizationPlan {
    mode: FinalizationMode,
    authoritative_retry_paths: BTreeSet<String>,
    foreground_revalidation: ForegroundRevalidationPlan,
}

impl FinalizationPlan {
    pub(super) fn new(mode: FinalizationMode) -> Self {
        Self {
            mode,
            authoritative_retry_paths: BTreeSet::new(),
            foreground_revalidation: ForegroundRevalidationPlan::default(),
        }
    }

    pub(super) fn restore_authoritative_scan_issues(
        &mut self,
        catalog: &mut SqliteCatalog,
        scan_id: &str,
        canonical_root: &Path,
        requested_root: &Path,
    ) -> Result<bool, ScanError> {
        let issue_limit = LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES.saturating_add(1);
        let issues = catalog.load_scan_issues(scan_id, issue_limit)?;
        let evidence_is_complete = issues.len() <= RETRY_PATH_LIMIT;
        Ok(self.restore_authoritative_issues(
            &issues,
            canonical_root,
            requested_root,
            evidence_is_complete,
        ))
    }

    fn restore_authoritative_issues(
        &mut self,
        issues: &[ScanIssue],
        canonical_root: &Path,
        requested_root: &Path,
        evidence_is_complete: bool,
    ) -> bool {
        debug_assert_eq!(self.mode, FinalizationMode::AuthoritativeRecovery);
        let mut has_convertible_authoritative_issue = false;
        let mut all_evidence_is_compatible = evidence_is_complete;
        for issue in issues {
            if is_retryable_authoritative_path_issue(&issue.code) {
                has_convertible_authoritative_issue = true;
                let Some(relative_path) = issue.path.as_deref().and_then(|path| {
                    authoritative_retry_relative_path(path, canonical_root, requested_root)
                }) else {
                    all_evidence_is_compatible = false;
                    continue;
                };
                if !retain_bounded_path(&mut self.authoritative_retry_paths, &relative_path) {
                    all_evidence_is_compatible = false;
                }
            } else if is_terminal_media_issue(&issue.code) {
                has_convertible_authoritative_issue = true;
            } else if !is_nonblocking_scan_issue(&issue.code) {
                all_evidence_is_compatible = false;
            }
        }
        has_convertible_authoritative_issue && all_evidence_is_compatible
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

fn authoritative_retry_relative_path(
    issue_path: &str,
    canonical_root: &Path,
    requested_root: &Path,
) -> Option<String> {
    let issue_path = Path::new(issue_path);
    let relative_path = issue_path
        .strip_prefix(requested_root)
        .or_else(|_| issue_path.strip_prefix(canonical_root))
        .ok()?;
    if relative_path.as_os_str().is_empty() {
        return None;
    }
    Some(relative_path.to_string_lossy().replace('\\', "/"))
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

fn is_retryable_media_issue(code: &str) -> bool {
    matches!(code, "image_open_failed" | "image_header_read_failed")
}

fn is_retryable_authoritative_path_issue(code: &str) -> bool {
    is_retryable_media_issue(code)
        || matches!(
            code,
            "source_changed_during_scan"
                | "source_replaced_during_scan"
                | "source_revalidation_failed"
                | "source_became_unavailable"
                | "source_identity_unavailable"
        )
}

fn is_terminal_media_issue(code: &str) -> bool {
    matches!(
        code,
        "image_dimensions_failed"
            | "image_format_unsupported"
            | "image_decode_invalid"
            | "image_limits_exceeded"
            | "image_decoder_rejected"
            | "image_dimensions_exceeded"
            | "media_type_unsupported"
    )
}

fn is_nonblocking_scan_issue(code: &str) -> bool {
    matches!(
        code,
        "file_identity_unavailable"
            | "orientation_read_failed"
            | "metadata_read_failed"
            | "metadata_size_exceeded"
            | "metadata_parse_failed"
            | "capture_time_invalid"
    )
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

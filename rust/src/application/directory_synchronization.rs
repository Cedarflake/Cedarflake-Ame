use std::collections::BTreeMap;

use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, DerivedEvidenceDisposition,
    IncrementalReconciliationDecision, IncrementalReconciliationOutcome, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningContext, LibraryChangePlanningError,
    LibraryChangePlanningIssue, LibraryChangePlanningLimits, LibraryChangePlanningResult,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability,
    ReconciliationFileEvidence, ReconciliationObservedState,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IntentKey {
    kind: u8,
    scope: u8,
    previous_relative_path: String,
    relative_path: String,
}

pub fn plan_library_changes(
    context: &LibraryChangePlanningContext,
    observations: impl IntoIterator<Item = LibraryChangeObservation>,
    limits: LibraryChangePlanningLimits,
) -> Result<LibraryChangePlanningResult, LibraryChangePlanningError> {
    validate_planning_request(context, limits)?;
    let mut received_observation_count = 0_u64;
    let mut bounded_observations = Vec::new();
    let mut exceeded_observation_limit = false;
    for observation in observations {
        received_observation_count = received_observation_count.saturating_add(1);
        if bounded_observations.len() == limits.max_observations {
            exceeded_observation_limit = true;
            break;
        }
        bounded_observations.push(observation);
    }
    bounded_observations.sort_by_key(|observation| observation.sequence);

    let mut issues = Vec::new();
    let mut superseded_observation_count = 0_u64;
    let mut intents = BTreeMap::<IntentKey, LibraryChangeIntent>::new();
    let mut must_reconcile_root = exceeded_observation_limit;
    let mut root_gap_origin = LibraryChangeOrigin::ConsistencyAudit;
    let mut root_gap_observed_unix_ms = 0_i64;
    let mut root_gap_sequence = 0_u64;

    if exceeded_observation_limit {
        issues.push(LibraryChangePlanningIssue::ObservationLimitExceeded);
    }
    if !matches!(context.source_health, LibraryChangeSourceHealth::Healthy) {
        must_reconcile_root = true;
        issues.push(LibraryChangePlanningIssue::ChangeSourceUnhealthy);
    }

    for observation in bounded_observations {
        if observation.root_id != context.root_id
            || observation.root_generation != context.root_generation
        {
            superseded_observation_count = superseded_observation_count.saturating_add(1);
            continue;
        }
        root_gap_origin = observation.origin;
        root_gap_observed_unix_ms = observation.observed_unix_ms;
        root_gap_sequence = observation.sequence;
        if matches!(observation.kind, LibraryChangeObservationKind::EvidenceGap) {
            must_reconcile_root = true;
            push_unique_issue(&mut issues, LibraryChangePlanningIssue::ChangeEvidenceGap);
            continue;
        }
        let relative_path = match normalize_relative_path(&observation.relative_path) {
            Some(path) => path,
            None => {
                must_reconcile_root = true;
                push_unique_issue(&mut issues, LibraryChangePlanningIssue::InvalidRelativePath);
                continue;
            }
        };
        if relative_path.is_empty()
            && !matches!(
                observation.kind,
                LibraryChangeObservationKind::DirectoryChanged
            )
        {
            must_reconcile_root = true;
            push_unique_issue(&mut issues, LibraryChangePlanningIssue::InvalidRelativePath);
            continue;
        }
        let should_use_previous_path = matches!(
            observation.kind,
            LibraryChangeObservationKind::Renamed { .. }
        ) && observation.scope != LibraryChangeScope::Root
            && !relative_path.is_empty();
        let previous_relative_path = if should_use_previous_path {
            match observation.previous_relative_path.as_deref() {
                Some(path) => match normalize_nonempty_relative_path(path) {
                    Some(path) => Some(path),
                    None => {
                        must_reconcile_root = true;
                        push_unique_issue(
                            &mut issues,
                            LibraryChangePlanningIssue::InvalidRelativePath,
                        );
                        continue;
                    }
                },
                None => None,
            }
        } else {
            None
        };

        let new_intents =
            normalized_intents(context, observation, relative_path, previous_relative_path);
        for intent in new_intents {
            merge_intent(&mut intents, intent);
            if intents.len() > limits.max_intents {
                must_reconcile_root = true;
                intents.clear();
                push_unique_issue(&mut issues, LibraryChangePlanningIssue::IntentLimitExceeded);
                break;
            }
        }
        if must_reconcile_root && issues.contains(&LibraryChangePlanningIssue::IntentLimitExceeded)
        {
            break;
        }
    }

    let mut planned_intents = if must_reconcile_root {
        vec![root_freshness_intent(
            context,
            root_gap_origin,
            root_gap_observed_unix_ms,
            root_gap_sequence,
            received_observation_count,
        )]
    } else {
        remove_subtree_redundancy(intents.into_values().collect())
    };
    planned_intents.sort_by(intent_sort_order);

    let (freshness, freshness_cause) = project_freshness(
        context,
        &planned_intents,
        &issues,
        exceeded_observation_limit,
    );
    Ok(LibraryChangePlanningResult {
        root_id: context.root_id.clone(),
        root_generation: context.root_generation,
        freshness,
        freshness_cause,
        intents: planned_intents,
        issues,
        received_observation_count,
        superseded_observation_count,
    })
}

pub fn reconcile_path_evidence(
    prior: Option<&ReconciliationFileEvidence>,
    observed: ReconciliationObservedState,
) -> IncrementalReconciliationDecision {
    match observed {
        ReconciliationObservedState::Present(current) => reconcile_present_path(prior, current),
        ReconciliationObservedState::Missing {
            is_authoritative: true,
        } if prior.is_some() => IncrementalReconciliationDecision {
            outcome: IncrementalReconciliationOutcome::Removed,
            evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
            current: None,
            issue_code: None,
        },
        ReconciliationObservedState::Missing {
            is_authoritative: true,
        } => IncrementalReconciliationDecision {
            outcome: IncrementalReconciliationOutcome::Unchanged,
            evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
            current: None,
            issue_code: None,
        },
        ReconciliationObservedState::Missing {
            is_authoritative: false,
        } => preserved_decision(
            IncrementalReconciliationOutcome::RetryableFailure,
            "path_absence_not_authoritative".to_owned(),
        ),
        ReconciliationObservedState::RetryableFailure { code } => {
            preserved_decision(IncrementalReconciliationOutcome::RetryableFailure, code)
        }
        ReconciliationObservedState::TerminalIssue { code } => {
            preserved_decision(IncrementalReconciliationOutcome::TerminalIssue, code)
        }
        ReconciliationObservedState::Skipped { code } => {
            preserved_decision(IncrementalReconciliationOutcome::Skipped, code)
        }
    }
}

fn validate_planning_request(
    context: &LibraryChangePlanningContext,
    limits: LibraryChangePlanningLimits,
) -> Result<(), LibraryChangePlanningError> {
    if context.root_id.trim().is_empty() {
        return Err(LibraryChangePlanningError::new(
            "change_root_id_invalid",
            "A library root identifier is required for change planning",
        ));
    }
    if limits.max_observations == 0 || limits.max_intents == 0 {
        return Err(LibraryChangePlanningError::new(
            "change_planning_limit_invalid",
            "Change planning limits must be greater than zero",
        ));
    }
    Ok(())
}

fn normalized_intents(
    context: &LibraryChangePlanningContext,
    observation: LibraryChangeObservation,
    relative_path: String,
    previous_relative_path: Option<String>,
) -> Vec<LibraryChangeIntent> {
    let scope = if relative_path.is_empty() || observation.scope == LibraryChangeScope::Root {
        LibraryChangeScope::Root
    } else {
        match observation.kind {
            LibraryChangeObservationKind::DirectoryChanged => LibraryChangeScope::Subtree,
            _ => observation.scope,
        }
    };
    let relative_path = if scope == LibraryChangeScope::Root {
        String::new()
    } else {
        relative_path
    };
    if scope == LibraryChangeScope::Root {
        return vec![new_intent(
            context,
            &observation,
            LibraryChangeIntentKind::Reconcile,
            LibraryChangeScope::Root,
            relative_path,
            None,
        )];
    }
    if let LibraryChangeObservationKind::Renamed { is_reliably_paired } = observation.kind {
        if is_reliably_paired && previous_relative_path.is_some() {
            return vec![new_intent(
                context,
                &observation,
                LibraryChangeIntentKind::RenameCandidate,
                scope,
                relative_path,
                previous_relative_path,
            )];
        }
        let mut intents = Vec::with_capacity(2);
        if let Some(previous_relative_path) = previous_relative_path {
            intents.push(new_intent(
                context,
                &observation,
                LibraryChangeIntentKind::Reconcile,
                scope,
                previous_relative_path,
                None,
            ));
        }
        intents.push(new_intent(
            context,
            &observation,
            LibraryChangeIntentKind::Reconcile,
            scope,
            relative_path,
            None,
        ));
        return intents;
    }
    vec![new_intent(
        context,
        &observation,
        LibraryChangeIntentKind::Reconcile,
        scope,
        relative_path,
        None,
    )]
}

fn new_intent(
    context: &LibraryChangePlanningContext,
    observation: &LibraryChangeObservation,
    kind: LibraryChangeIntentKind,
    scope: LibraryChangeScope,
    relative_path: String,
    previous_relative_path: Option<String>,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: context.root_id.clone(),
        root_generation: context.root_generation,
        kind,
        scope,
        relative_path,
        previous_relative_path,
        origin: observation.origin,
        first_observed_unix_ms: observation.observed_unix_ms,
        most_recent_observed_unix_ms: observation.observed_unix_ms,
        first_sequence: observation.sequence,
        most_recent_sequence: observation.sequence,
        coalesced_observation_count: 1,
    }
}

fn merge_intent(
    intents: &mut BTreeMap<IntentKey, LibraryChangeIntent>,
    intent: LibraryChangeIntent,
) {
    let key = intent_key(&intent);
    if let Some(existing) = intents.get_mut(&key) {
        existing.first_observed_unix_ms = existing
            .first_observed_unix_ms
            .min(intent.first_observed_unix_ms);
        existing.most_recent_observed_unix_ms = existing
            .most_recent_observed_unix_ms
            .max(intent.most_recent_observed_unix_ms);
        existing.first_sequence = existing.first_sequence.min(intent.first_sequence);
        if intent.most_recent_sequence > existing.most_recent_sequence
            || (intent.most_recent_sequence == existing.most_recent_sequence
                && origin_rank(intent.origin) > origin_rank(existing.origin))
        {
            existing.most_recent_sequence = intent.most_recent_sequence;
            existing.origin = intent.origin;
        }
        existing.coalesced_observation_count = existing
            .coalesced_observation_count
            .saturating_add(intent.coalesced_observation_count);
    } else {
        intents.insert(key, intent);
    }
}

fn origin_rank(origin: LibraryChangeOrigin) -> u8 {
    match origin {
        LibraryChangeOrigin::LiveNotification => 0,
        LibraryChangeOrigin::StartupCatchUp => 1,
        LibraryChangeOrigin::UserRefresh => 2,
        LibraryChangeOrigin::ConsistencyAudit => 3,
    }
}

fn intent_key(intent: &LibraryChangeIntent) -> IntentKey {
    IntentKey {
        kind: match intent.kind {
            LibraryChangeIntentKind::Reconcile => 0,
            LibraryChangeIntentKind::RenameCandidate => 1,
            LibraryChangeIntentKind::FreshnessUnknown => 2,
        },
        scope: match intent.scope {
            LibraryChangeScope::Path => 0,
            LibraryChangeScope::Subtree => 1,
            LibraryChangeScope::Root => 2,
        },
        previous_relative_path: intent.previous_relative_path.clone().unwrap_or_default(),
        relative_path: intent.relative_path.clone(),
    }
}

fn root_freshness_intent(
    context: &LibraryChangePlanningContext,
    origin: LibraryChangeOrigin,
    observed_unix_ms: i64,
    sequence: u64,
    observation_count: u64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: context.root_id.clone(),
        root_generation: context.root_generation,
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin,
        first_observed_unix_ms: observed_unix_ms,
        most_recent_observed_unix_ms: observed_unix_ms,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: u32::try_from(observation_count).unwrap_or(u32::MAX),
    }
}

fn remove_subtree_redundancy(intents: Vec<LibraryChangeIntent>) -> Vec<LibraryChangeIntent> {
    if intents.iter().any(|intent| {
        intent.kind == LibraryChangeIntentKind::Reconcile
            && intent.scope == LibraryChangeScope::Root
    }) {
        return intents
            .into_iter()
            .filter(|intent| {
                intent.kind == LibraryChangeIntentKind::Reconcile
                    && intent.scope == LibraryChangeScope::Root
            })
            .collect();
    }
    let subtree_paths = intents
        .iter()
        .filter(|intent| {
            intent.kind == LibraryChangeIntentKind::Reconcile
                && intent.scope == LibraryChangeScope::Subtree
        })
        .map(|intent| intent.relative_path.clone())
        .collect::<Vec<_>>();
    intents
        .into_iter()
        .filter(|intent| {
            if intent.scope != LibraryChangeScope::Path {
                return true;
            }
            !subtree_paths.iter().any(|subtree| {
                is_within_subtree(&intent.relative_path, subtree)
                    && intent
                        .previous_relative_path
                        .as_deref()
                        .is_none_or(|previous| is_within_subtree(previous, subtree))
            })
        })
        .collect()
}

fn is_within_subtree(path: &str, subtree: &str) -> bool {
    subtree.is_empty()
        || path == subtree
        || path
            .strip_prefix(subtree)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalize_relative_path(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.contains(':') {
        return None;
    }
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            _ => segments.push(segment),
        }
    }
    Some(segments.join("/"))
}

fn intent_sort_order(
    left: &LibraryChangeIntent,
    right: &LibraryChangeIntent,
) -> std::cmp::Ordering {
    left.first_sequence
        .cmp(&right.first_sequence)
        .then_with(|| intent_key(left).cmp(&intent_key(right)))
}

fn project_freshness(
    context: &LibraryChangePlanningContext,
    intents: &[LibraryChangeIntent],
    issues: &[LibraryChangePlanningIssue],
    exceeded_observation_limit: bool,
) -> (CatalogFreshnessState, CatalogFreshnessCause) {
    if !matches!(context.availability, LibraryRootAvailability::Available) {
        return (
            CatalogFreshnessState::Unavailable,
            CatalogFreshnessCause::RootUnavailable,
        );
    }
    if exceeded_observation_limit
        || issues.contains(&LibraryChangePlanningIssue::IntentLimitExceeded)
    {
        return (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::BoundedCapacityExceeded,
        );
    }
    if issues.contains(&LibraryChangePlanningIssue::ChangeEvidenceGap)
        || issues.contains(&LibraryChangePlanningIssue::InvalidRelativePath)
    {
        return (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::EvidenceGap,
        );
    }
    if !matches!(context.source_health, LibraryChangeSourceHealth::Healthy) {
        return (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::ChangeSourceUnhealthy,
        );
    }
    if intents.is_empty() {
        (
            CatalogFreshnessState::Synchronized,
            CatalogFreshnessCause::NoPendingChanges,
        )
    } else {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
        )
    }
}

fn push_unique_issue(
    issues: &mut Vec<LibraryChangePlanningIssue>,
    issue: LibraryChangePlanningIssue,
) {
    if !issues.contains(&issue) {
        issues.push(issue);
    }
}

fn reconcile_present_path(
    prior: Option<&ReconciliationFileEvidence>,
    mut current: ReconciliationFileEvidence,
) -> IncrementalReconciliationDecision {
    let Some(current_relative_path) = normalize_nonempty_relative_path(&current.relative_path)
    else {
        return preserved_decision(
            IncrementalReconciliationOutcome::TerminalIssue,
            "current_relative_path_invalid".to_owned(),
        );
    };
    current.relative_path = current_relative_path;
    let Some(prior) = prior else {
        return IncrementalReconciliationDecision {
            outcome: IncrementalReconciliationOutcome::Added,
            evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
            current: Some(current),
            issue_code: None,
        };
    };
    let Some(prior_relative_path) = normalize_nonempty_relative_path(&prior.relative_path) else {
        return preserved_decision(
            IncrementalReconciliationOutcome::TerminalIssue,
            "prior_relative_path_invalid".to_owned(),
        );
    };
    let is_same_state =
        prior.file_size == current.file_size && prior.modified_unix_ms == current.modified_unix_ms;
    let has_matching_identity =
        prior.file_identity.is_some() && prior.file_identity == current.file_identity;
    let has_conflicting_identity = prior.file_identity.is_some()
        && current.file_identity.is_some()
        && prior.file_identity != current.file_identity;
    let is_same_path = prior_relative_path == current.relative_path;

    let (outcome, evidence_disposition) = if has_matching_identity && !is_same_path {
        (
            IncrementalReconciliationOutcome::RenamedOrMoved,
            if is_same_state {
                DerivedEvidenceDisposition::RetainCompatible
            } else {
                DerivedEvidenceDisposition::InvalidateDerived
            },
        )
    } else if has_matching_identity && is_same_state {
        (
            IncrementalReconciliationOutcome::Unchanged,
            DerivedEvidenceDisposition::RetainCompatible,
        )
    } else if has_matching_identity {
        (
            IncrementalReconciliationOutcome::Modified,
            DerivedEvidenceDisposition::InvalidateDerived,
        )
    } else if is_same_path && has_conflicting_identity {
        (
            IncrementalReconciliationOutcome::Replaced,
            DerivedEvidenceDisposition::NoReusableEvidence,
        )
    } else if is_same_path && is_same_state {
        (
            IncrementalReconciliationOutcome::Unchanged,
            DerivedEvidenceDisposition::RetainCompatible,
        )
    } else if is_same_path {
        (
            IncrementalReconciliationOutcome::Replaced,
            DerivedEvidenceDisposition::NoReusableEvidence,
        )
    } else {
        (
            IncrementalReconciliationOutcome::Added,
            DerivedEvidenceDisposition::NoReusableEvidence,
        )
    };
    IncrementalReconciliationDecision {
        outcome,
        evidence_disposition,
        current: Some(current),
        issue_code: None,
    }
}

fn normalize_nonempty_relative_path(path: &str) -> Option<String> {
    normalize_relative_path(path).filter(|path| !path.is_empty())
}

fn preserved_decision(
    outcome: IncrementalReconciliationOutcome,
    issue_code: String,
) -> IncrementalReconciliationDecision {
    IncrementalReconciliationDecision {
        outcome,
        evidence_disposition: DerivedEvidenceDisposition::PreserveLastTrustworthy,
        current: None,
        issue_code: Some(issue_code),
    }
}

#[cfg(test)]
mod tests;

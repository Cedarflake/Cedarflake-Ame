use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Debug)]
struct AccumulatedIntent {
    intent: LibraryChangeIntent,
    evidence_ids: BTreeSet<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationEvidence {
    sequence: u64,
    observed_unix_ms: i64,
    origin: LibraryChangeOrigin,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ObservationRange {
    first_observed_unix_ms: i64,
    most_recent_observed_unix_ms: i64,
    first_sequence: u64,
    most_recent_sequence: u64,
    observation_count: u64,
}

pub fn plan_library_changes<Observations>(
    context: &LibraryChangePlanningContext,
    observations: Observations,
    limits: LibraryChangePlanningLimits,
) -> Result<LibraryChangePlanningResult, LibraryChangePlanningError>
where
    Observations: IntoIterator<Item = LibraryChangeObservation>,
    Observations::IntoIter: ExactSizeIterator,
{
    validate_planning_request(context, limits)?;
    let observations = observations.into_iter();
    let received_observation_count = u64::try_from(observations.len()).unwrap_or(u64::MAX);
    let exceeded_observation_limit = observations.len() > limits.max_observations;
    let mut bounded_observations = if exceeded_observation_limit {
        Vec::new()
    } else {
        observations.collect()
    };
    bounded_observations.sort_by(observation_sort_order);

    let mut issues = Vec::new();
    let mut superseded_observation_count = 0_u64;
    let mut intents = BTreeMap::<IntentKey, AccumulatedIntent>::new();
    let mut observation_evidence = Vec::<ObservationEvidence>::new();
    let mut must_reconcile_root = exceeded_observation_limit;
    let mut root_gap_origin = None;
    let mut observation_range = None;

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
        update_observation_range(&mut observation_range, &observation);
        update_latest_origin(&mut root_gap_origin, &observation);
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
        if matches!(
            observation.kind,
            LibraryChangeObservationKind::Renamed { .. }
        ) && observation.scope != LibraryChangeScope::Root
            && previous_relative_path.is_none()
        {
            must_reconcile_root = true;
            push_unique_issue(&mut issues, LibraryChangePlanningIssue::ChangeEvidenceGap);
            continue;
        }

        let evidence_id = observation_evidence.len();
        observation_evidence.push(ObservationEvidence {
            sequence: observation.sequence,
            observed_unix_ms: observation.observed_unix_ms,
            origin: observation.origin,
        });
        let new_intents =
            normalized_intents(context, observation, relative_path, previous_relative_path);
        for intent in new_intents {
            merge_intent(&mut intents, intent, evidence_id);
        }
    }

    let mut compacted_intents = compact_intents(intents.into_values().collect());
    for accumulated in &mut compacted_intents {
        apply_evidence_summary(
            &mut accumulated.intent,
            &accumulated.evidence_ids,
            &observation_evidence,
        );
    }
    if compacted_intents.len() > limits.max_intents {
        must_reconcile_root = true;
        push_unique_issue(&mut issues, LibraryChangePlanningIssue::IntentLimitExceeded);
    }
    let mut planned_intents = if must_reconcile_root {
        vec![root_freshness_intent(
            context,
            root_gap_origin
                .map(|(_, _, origin)| origin)
                .unwrap_or(LibraryChangeOrigin::LiveNotification),
            if exceeded_observation_limit {
                None
            } else {
                observation_range
            },
            exceeded_observation_limit.then_some(received_observation_count),
        )]
    } else {
        compacted_intents
            .into_iter()
            .map(|accumulated| accumulated.intent)
            .collect()
    };
    issues.sort();
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
            relative_path,
            is_authoritative,
        } => reconcile_missing_path(prior, relative_path, is_authoritative),
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
    if context.root_id.trim().is_empty() || context.root_id.contains('\0') {
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
    if limits.max_observations > LibraryChangePlanningLimits::MAX_OBSERVATIONS
        || limits.max_intents > LibraryChangePlanningLimits::MAX_INTENTS
    {
        return Err(LibraryChangePlanningError::new(
            "change_planning_limit_exceeded",
            "Change planning limits exceed the absolute capacity bound",
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
        if previous_relative_path.as_deref() == Some(relative_path.as_str()) {
            return vec![new_intent(
                context,
                &observation,
                LibraryChangeIntentKind::Reconcile,
                scope,
                relative_path,
                None,
            )];
        }
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
    intents: &mut BTreeMap<IntentKey, AccumulatedIntent>,
    intent: LibraryChangeIntent,
    evidence_id: usize,
) {
    let key = intent_key(&intent);
    if let Some(existing) = intents.get_mut(&key) {
        let should_replace_origin = (
            intent.most_recent_sequence,
            intent.most_recent_observed_unix_ms,
            origin_rank(intent.origin),
        ) > (
            existing.intent.most_recent_sequence,
            existing.intent.most_recent_observed_unix_ms,
            origin_rank(existing.intent.origin),
        );
        existing.intent.first_observed_unix_ms = existing
            .intent
            .first_observed_unix_ms
            .min(intent.first_observed_unix_ms);
        existing.intent.most_recent_observed_unix_ms = existing
            .intent
            .most_recent_observed_unix_ms
            .max(intent.most_recent_observed_unix_ms);
        existing.intent.first_sequence = existing.intent.first_sequence.min(intent.first_sequence);
        if should_replace_origin {
            existing.intent.most_recent_sequence = intent.most_recent_sequence;
            existing.intent.origin = intent.origin;
        }
        existing.intent.coalesced_observation_count = existing
            .intent
            .coalesced_observation_count
            .saturating_add(intent.coalesced_observation_count);
        existing.evidence_ids.insert(evidence_id);
    } else {
        intents.insert(
            key,
            AccumulatedIntent {
                intent,
                evidence_ids: BTreeSet::from([evidence_id]),
            },
        );
    }
}

fn compact_intents(mut intents: Vec<AccumulatedIntent>) -> Vec<AccumulatedIntent> {
    if let Some(root_index) = intents.iter().position(|accumulated| {
        accumulated.intent.kind == LibraryChangeIntentKind::Reconcile
            && accumulated.intent.scope == LibraryChangeScope::Root
    }) {
        let mut root = intents.swap_remove(root_index);
        for intent in intents {
            root.evidence_ids.extend(intent.evidence_ids);
        }
        return vec![root];
    }

    let subtree_indexes = intents
        .iter()
        .enumerate()
        .filter(|(_, accumulated)| {
            accumulated.intent.kind == LibraryChangeIntentKind::Reconcile
                && accumulated.intent.scope == LibraryChangeScope::Subtree
        })
        .map(|(index, accumulated)| (accumulated.intent.relative_path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let absorbed_by = intents
        .iter()
        .enumerate()
        .map(|(index, accumulated)| {
            covering_subtree_index(index, &accumulated.intent, &subtree_indexes)
        })
        .collect::<Vec<_>>();
    let mut inherited_evidence = vec![BTreeSet::new(); intents.len()];
    let mut slots = intents.drain(..).map(Some).collect::<Vec<_>>();
    for (index, target) in absorbed_by.into_iter().enumerate() {
        if let Some(target) = target
            && let Some(absorbed) = slots[index].take()
        {
            inherited_evidence[target].extend(absorbed.evidence_ids);
        }
    }
    slots
        .into_iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            slot.map(|mut accumulated| {
                accumulated
                    .evidence_ids
                    .append(&mut inherited_evidence[index]);
                accumulated
            })
        })
        .collect()
}

fn covering_subtree_index(
    intent_index: usize,
    intent: &LibraryChangeIntent,
    subtree_indexes: &BTreeMap<String, usize>,
) -> Option<usize> {
    if intent.scope == LibraryChangeScope::Root {
        return None;
    }
    let (subtree, covering_index) = ancestor_paths(&intent.relative_path)
        .filter_map(|path| {
            subtree_indexes
                .get(path)
                .copied()
                .map(|index| (path, index))
        })
        .find(|(_, index)| *index != intent_index)?;
    intent
        .previous_relative_path
        .as_deref()
        .is_none_or(|previous| is_within_subtree(previous, subtree))
        .then_some(covering_index)
}

fn ancestor_paths(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/')
        .map(|(index, _)| &path[..index])
        .chain(std::iter::once(path))
}

fn apply_evidence_summary(
    intent: &mut LibraryChangeIntent,
    evidence_ids: &BTreeSet<usize>,
    observation_evidence: &[ObservationEvidence],
) {
    let mut evidence = evidence_ids
        .iter()
        .filter_map(|evidence_id| observation_evidence.get(*evidence_id));
    let Some(first) = evidence.next() else {
        return;
    };
    let mut first_sequence = first.sequence;
    let mut first_observed_unix_ms = first.observed_unix_ms;
    let mut most_recent_sequence = first.sequence;
    let mut most_recent_observed_unix_ms = first.observed_unix_ms;
    let mut latest = *first;
    for item in evidence {
        first_sequence = first_sequence.min(item.sequence);
        first_observed_unix_ms = first_observed_unix_ms.min(item.observed_unix_ms);
        most_recent_sequence = most_recent_sequence.max(item.sequence);
        most_recent_observed_unix_ms = most_recent_observed_unix_ms.max(item.observed_unix_ms);
        if (
            item.sequence,
            item.observed_unix_ms,
            origin_rank(item.origin),
        ) > (
            latest.sequence,
            latest.observed_unix_ms,
            origin_rank(latest.origin),
        ) {
            latest = *item;
        }
    }
    intent.first_sequence = first_sequence;
    intent.first_observed_unix_ms = first_observed_unix_ms;
    intent.most_recent_sequence = most_recent_sequence;
    intent.most_recent_observed_unix_ms = most_recent_observed_unix_ms;
    intent.coalesced_observation_count = u32::try_from(evidence_ids.len()).unwrap_or(u32::MAX);
    intent.origin = latest.origin;
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
    range: Option<ObservationRange>,
    overflow_observation_count: Option<u64>,
) -> LibraryChangeIntent {
    let range = range.unwrap_or(ObservationRange {
        observation_count: 1,
        ..ObservationRange::default()
    });
    LibraryChangeIntent {
        root_id: context.root_id.clone(),
        root_generation: context.root_generation,
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin,
        first_observed_unix_ms: range.first_observed_unix_ms,
        most_recent_observed_unix_ms: range.most_recent_observed_unix_ms,
        first_sequence: range.first_sequence,
        most_recent_sequence: range.most_recent_sequence,
        coalesced_observation_count: u32::try_from(
            overflow_observation_count.unwrap_or(range.observation_count),
        )
        .unwrap_or(u32::MAX),
    }
}

fn is_within_subtree(path: &str, subtree: &str) -> bool {
    subtree.is_empty()
        || path == subtree
        || path
            .strip_prefix(subtree)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalize_relative_path(path: &str) -> Option<String> {
    if path.contains('\0') {
        return None;
    }
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

fn observation_sort_order(
    left: &LibraryChangeObservation,
    right: &LibraryChangeObservation,
) -> std::cmp::Ordering {
    left.sequence
        .cmp(&right.sequence)
        .then_with(|| left.observed_unix_ms.cmp(&right.observed_unix_ms))
        .then_with(|| left.origin.cmp(&right.origin))
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left.scope.cmp(&right.scope))
        .then_with(|| left.relative_path.cmp(&right.relative_path))
        .then_with(|| {
            left.previous_relative_path
                .cmp(&right.previous_relative_path)
        })
}

fn update_observation_range(
    range: &mut Option<ObservationRange>,
    observation: &LibraryChangeObservation,
) {
    let range = range.get_or_insert(ObservationRange {
        first_observed_unix_ms: observation.observed_unix_ms,
        most_recent_observed_unix_ms: observation.observed_unix_ms,
        first_sequence: observation.sequence,
        most_recent_sequence: observation.sequence,
        observation_count: 0,
    });
    range.first_observed_unix_ms = range
        .first_observed_unix_ms
        .min(observation.observed_unix_ms);
    range.most_recent_observed_unix_ms = range
        .most_recent_observed_unix_ms
        .max(observation.observed_unix_ms);
    range.first_sequence = range.first_sequence.min(observation.sequence);
    range.most_recent_sequence = range.most_recent_sequence.max(observation.sequence);
    range.observation_count = range.observation_count.saturating_add(1);
}

fn update_latest_origin(
    latest: &mut Option<(u64, i64, LibraryChangeOrigin)>,
    observation: &LibraryChangeObservation,
) {
    let candidate = (
        observation.sequence,
        observation.observed_unix_ms,
        observation.origin,
    );
    if latest.is_none_or(|current| {
        (candidate.0, candidate.1, origin_rank(candidate.2))
            > (current.0, current.1, origin_rank(current.2))
    }) {
        *latest = Some(candidate);
    }
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
    if current
        .file_identity
        .as_ref()
        .is_some_and(invalid_file_identity)
    {
        return preserved_decision(
            IncrementalReconciliationOutcome::TerminalIssue,
            "current_file_identity_invalid".to_owned(),
        );
    }
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
    if prior
        .file_identity
        .as_ref()
        .is_some_and(invalid_file_identity)
    {
        return preserved_decision(
            IncrementalReconciliationOutcome::TerminalIssue,
            "prior_file_identity_invalid".to_owned(),
        );
    }
    let is_same_state =
        prior.file_size == current.file_size && prior.modified_unix_ms == current.modified_unix_ms;
    let has_matching_identity =
        prior.file_identity.is_some() && prior.file_identity == current.file_identity;
    let has_conflicting_identity = prior.file_identity.is_some()
        && current.file_identity.is_some()
        && prior.file_identity != current.file_identity;
    let is_same_path = prior_relative_path == current.relative_path;

    if is_same_path && prior.file_identity.is_some() && current.file_identity.is_none() {
        return preserved_decision(
            IncrementalReconciliationOutcome::RetryableFailure,
            "current_file_identity_unavailable".to_owned(),
        );
    }

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

fn reconcile_missing_path(
    prior: Option<&ReconciliationFileEvidence>,
    relative_path: String,
    is_authoritative: bool,
) -> IncrementalReconciliationDecision {
    let Some(relative_path) = normalize_nonempty_relative_path(&relative_path) else {
        return preserved_decision(
            IncrementalReconciliationOutcome::TerminalIssue,
            "missing_relative_path_invalid".to_owned(),
        );
    };
    if let Some(prior) = prior {
        let Some(prior_relative_path) = normalize_nonempty_relative_path(&prior.relative_path)
        else {
            return preserved_decision(
                IncrementalReconciliationOutcome::TerminalIssue,
                "prior_relative_path_invalid".to_owned(),
            );
        };
        if prior_relative_path != relative_path {
            return preserved_decision(
                IncrementalReconciliationOutcome::TerminalIssue,
                "missing_relative_path_mismatch".to_owned(),
            );
        }
    }
    if !is_authoritative {
        return preserved_decision(
            IncrementalReconciliationOutcome::RetryableFailure,
            "path_absence_not_authoritative".to_owned(),
        );
    }
    if prior.is_some() {
        IncrementalReconciliationDecision {
            outcome: IncrementalReconciliationOutcome::Removed,
            evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
            current: None,
            issue_code: None,
        }
    } else {
        IncrementalReconciliationDecision {
            outcome: IncrementalReconciliationOutcome::Unchanged,
            evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
            current: None,
            issue_code: None,
        }
    }
}

fn invalid_file_identity(identity: &crate::domain::FileIdentityEvidence) -> bool {
    identity.scheme.trim().is_empty()
        || identity.value.trim().is_empty()
        || identity.scheme.contains('\0')
        || identity.value.contains('\0')
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
mod blue_team_tests;
#[cfg(test)]
mod tests;

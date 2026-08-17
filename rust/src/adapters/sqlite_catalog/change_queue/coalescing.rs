use rusqlite::Transaction;

use crate::domain::{
    LibraryChangeEnqueueReport, LibraryChangeFailure, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeQueueStatus, LibraryChangeScope,
    ScanError,
};

use super::persistence::{
    ActiveChange, insert_change, load_active_changes, mark_superseded, update_change,
};

const MAX_FAILURE_CODE_BYTES: usize = 128;
const MAX_FAILURE_MESSAGE_BYTES: usize = 2_048;
const MAX_ROOT_ID_BYTES: usize = 1_024;
const MAX_RELATIVE_PATH_BYTES: usize = 131_072;

#[derive(Clone, Copy)]
struct DegradationContext {
    enqueued_unix_ms: i64,
    catalog_revision: u64,
    policy: LibraryChangeQueuePolicy,
    capacity_degraded: bool,
}

pub(super) fn enqueue_one(
    transaction: &Transaction<'_>,
    incoming: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    catalog_revision: u64,
    policy: LibraryChangeQueuePolicy,
    report: &mut LibraryChangeEnqueueReport,
) -> Result<(), ScanError> {
    let active = load_active_changes(
        transaction,
        &incoming.root_id,
        incoming.root_generation,
        policy.max_unresolved_changes,
    )?;
    if active.len() > usize::try_from(policy.max_unresolved_changes).unwrap_or(usize::MAX) {
        return degrade_to_root(
            transaction,
            active,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: true,
            },
        );
    }
    if has_conflicting_rename(&active, incoming) {
        return degrade_to_root(
            transaction,
            active,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: false,
            },
        );
    }
    if has_ambiguous_leased_overlap(&active, incoming) {
        return degrade_to_root(
            transaction,
            active,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: false,
            },
        );
    }
    let covering = active
        .iter()
        .position(|change| is_unleased(change.status) && intent_covers(&change.intent, incoming));
    if let Some(target_index) = covering {
        let target = &active[target_index];
        let mut merged = target.intent.clone();
        merge_evidence(&mut merged, incoming);
        if incoming.kind == LibraryChangeIntentKind::FreshnessUnknown {
            merged.kind = LibraryChangeIntentKind::FreshnessUnknown;
        }
        let absorbed = active
            .iter()
            .enumerate()
            .filter(|(index, change)| {
                *index != target_index
                    && ((is_unleased(change.status) && intent_covers(&merged, &change.intent))
                        || (change.status == LibraryChangeQueueStatus::Leased
                            && stale_overlap(&change.intent, incoming)))
            })
            .map(|(_, change)| change)
            .collect::<Vec<_>>();
        for change in &absorbed {
            merge_evidence(&mut merged, &change.intent);
        }
        update_change(transaction, target.id, &merged, enqueued_unix_ms, policy)?;
        let superseded = mark_superseded(
            transaction,
            absorbed.iter().map(|change| change.id),
            Some(target.id),
            enqueued_unix_ms,
        )?;
        report.coalesced_count = report.coalesced_count.saturating_add(1);
        report.superseded_count = report.superseded_count.saturating_add(superseded);
        report.freshness_unknown_enqueued |=
            merged.kind == LibraryChangeIntentKind::FreshnessUnknown;
        return Ok(());
    }

    let absorbed = active
        .iter()
        .filter(|change| {
            intent_covers(incoming, &change.intent)
                || (change.status == LibraryChangeQueueStatus::Leased
                    && stale_overlap(&change.intent, incoming))
        })
        .collect::<Vec<_>>();
    let remaining = active.len().saturating_sub(absorbed.len());
    if remaining.saturating_add(1)
        > usize::try_from(policy.max_unresolved_changes).unwrap_or(usize::MAX)
    {
        return degrade_to_root(
            transaction,
            active,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: true,
            },
        );
    }
    let stronger = absorbed
        .iter()
        .filter(|change| intent_covers(&change.intent, incoming))
        .max_by_key(|change| intent_strength(&change.intent));
    let mut merged = incoming.clone();
    if let Some(stronger) = stronger {
        merged = stronger.intent.clone();
        merge_evidence(&mut merged, incoming);
    }
    for change in &absorbed {
        if stronger.is_some_and(|stronger| stronger.id == change.id) {
            continue;
        }
        merge_evidence(&mut merged, &change.intent);
    }
    let change_id = insert_change(
        transaction,
        &merged,
        enqueued_unix_ms,
        catalog_revision,
        policy,
    )?;
    let superseded = mark_superseded(
        transaction,
        absorbed.iter().map(|change| change.id),
        Some(change_id),
        enqueued_unix_ms,
    )?;
    report.inserted_count = report.inserted_count.saturating_add(1);
    report.superseded_count = report.superseded_count.saturating_add(superseded);
    report.freshness_unknown_enqueued |= merged.kind == LibraryChangeIntentKind::FreshnessUnknown;
    Ok(())
}

fn degrade_to_root(
    transaction: &Transaction<'_>,
    active: Vec<ActiveChange>,
    incoming: &LibraryChangeIntent,
    report: &mut LibraryChangeEnqueueReport,
    context: DegradationContext,
) -> Result<(), ScanError> {
    let mut root = incoming.clone();
    root.kind = LibraryChangeIntentKind::FreshnessUnknown;
    root.scope = LibraryChangeScope::Root;
    root.relative_path.clear();
    root.previous_relative_path = None;
    for change in &active {
        merge_evidence(&mut root, &change.intent);
    }
    let existing_root = active.iter().find(|change| {
        is_unleased(change.status)
            && change.intent.scope == LibraryChangeScope::Root
            && change.intent.kind == LibraryChangeIntentKind::FreshnessUnknown
    });
    let target_id = if let Some(existing_root) = existing_root {
        update_change(
            transaction,
            existing_root.id,
            &root,
            context.enqueued_unix_ms,
            context.policy,
        )?;
        report.coalesced_count = report.coalesced_count.saturating_add(1);
        existing_root.id
    } else {
        let id = insert_change(
            transaction,
            &root,
            context.enqueued_unix_ms,
            context.catalog_revision,
            context.policy,
        )?;
        report.inserted_count = report.inserted_count.saturating_add(1);
        id
    };
    let superseded = mark_superseded(
        transaction,
        active
            .iter()
            .filter(|change| change.id != target_id)
            .map(|change| change.id),
        Some(target_id),
        context.enqueued_unix_ms,
    )?;
    report.superseded_count = report.superseded_count.saturating_add(superseded);
    report.capacity_degraded |= context.capacity_degraded;
    report.freshness_unknown_enqueued = true;
    Ok(())
}

pub(super) fn validate_policy(policy: LibraryChangeQueuePolicy) -> Result<(), ScanError> {
    if policy.is_valid() {
        Ok(())
    } else {
        Err(ScanError::new(
            "change_queue_policy_invalid",
            "The durable change queue policy must stay within its absolute bounds",
        ))
    }
}

pub(super) fn validate_root_id(root_id: &str) -> Result<(), ScanError> {
    if root_id.trim().is_empty() || root_id.len() > MAX_ROOT_ID_BYTES || root_id.contains('\0') {
        Err(ScanError::new(
            "change_queue_root_id_invalid",
            "A durable change requires a valid library root identifier",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_intent_batch(
    intents: &[LibraryChangeIntent],
    first: &LibraryChangeIntent,
) -> Result<(), ScanError> {
    validate_root_id(&first.root_id)?;
    if intents.len()
        > usize::try_from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES).unwrap_or(usize::MAX)
    {
        return Err(ScanError::new(
            "change_queue_batch_limit_exceeded",
            "One durable enqueue batch exceeds the absolute unresolved-work bound",
        ));
    }
    for intent in intents {
        if intent.root_id != first.root_id || intent.root_generation != first.root_generation {
            return Err(ScanError::new(
                "change_queue_batch_mismatch",
                "One durable enqueue batch must belong to one root generation",
            ));
        }
        if intent.coalesced_observation_count == 0
            || intent.first_observed_unix_ms > intent.most_recent_observed_unix_ms
            || intent.first_sequence > intent.most_recent_sequence
        {
            return Err(ScanError::new(
                "change_queue_evidence_invalid",
                "Durable change evidence must contain a non-empty ordered observation range",
            ));
        }
        validate_intent_shape(intent)?;
    }
    Ok(())
}

fn validate_intent_shape(intent: &LibraryChangeIntent) -> Result<(), ScanError> {
    let has_path = valid_normalized_path(&intent.relative_path);
    let has_previous_path = intent
        .previous_relative_path
        .as_deref()
        .is_some_and(valid_normalized_path);
    let shape_is_valid = match intent.kind {
        LibraryChangeIntentKind::FreshnessUnknown => {
            intent.scope == LibraryChangeScope::Root
                && intent.relative_path.is_empty()
                && intent.previous_relative_path.is_none()
        }
        LibraryChangeIntentKind::RenameCandidate => {
            intent.scope != LibraryChangeScope::Root
                && has_path
                && has_previous_path
                && intent.previous_relative_path.as_deref() != Some(intent.relative_path.as_str())
        }
        LibraryChangeIntentKind::Reconcile => match intent.scope {
            LibraryChangeScope::Root => {
                intent.relative_path.is_empty() && intent.previous_relative_path.is_none()
            }
            LibraryChangeScope::Path | LibraryChangeScope::Subtree => {
                has_path && intent.previous_relative_path.is_none()
            }
        },
    };
    if shape_is_valid {
        Ok(())
    } else {
        Err(ScanError::new(
            "change_queue_intent_invalid",
            "The durable change intent does not match its normalized kind and scope",
        ))
    }
}

fn valid_normalized_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_RELATIVE_PATH_BYTES
        && !path.contains(['\0', '\\', ':'])
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

pub(super) fn validate_failure(failure: &LibraryChangeFailure) -> Result<(), ScanError> {
    if failure.code.trim().is_empty()
        || failure.code.len() > MAX_FAILURE_CODE_BYTES
        || failure.code.contains('\0')
        || failure.message.len() > MAX_FAILURE_MESSAGE_BYTES
        || failure.message.contains('\0')
    {
        return Err(ScanError::new(
            "change_queue_failure_invalid",
            "Structured retry failure evidence exceeds its supported bounds",
        ));
    }
    Ok(())
}

fn is_unleased(status: LibraryChangeQueueStatus) -> bool {
    matches!(
        status,
        LibraryChangeQueueStatus::Pending | LibraryChangeQueueStatus::RetryWait
    )
}

fn intent_covers(covering: &LibraryChangeIntent, candidate: &LibraryChangeIntent) -> bool {
    if covering.scope == LibraryChangeScope::Root {
        return true;
    }
    if covering.scope == LibraryChangeScope::Subtree {
        return affected_paths(candidate)
            .all(|path| is_within_subtree(path, &covering.relative_path));
    }
    same_work_key(covering, candidate)
        || covering.kind == LibraryChangeIntentKind::RenameCandidate
            && candidate.kind == LibraryChangeIntentKind::Reconcile
            && candidate.scope == LibraryChangeScope::Path
            && covering.relative_path == candidate.relative_path
}

fn stale_overlap(leased: &LibraryChangeIntent, incoming: &LibraryChangeIntent) -> bool {
    intent_covers(leased, incoming)
        || intent_covers(incoming, leased)
        || affected_paths_overlap(leased, incoming)
}

fn same_work_key(left: &LibraryChangeIntent, right: &LibraryChangeIntent) -> bool {
    left.kind == right.kind
        && left.scope == right.scope
        && left.relative_path == right.relative_path
        && left.previous_relative_path == right.previous_relative_path
}

fn affected_paths(intent: &LibraryChangeIntent) -> impl Iterator<Item = &str> {
    std::iter::once(intent.relative_path.as_str()).chain(intent.previous_relative_path.as_deref())
}

fn is_within_subtree(path: &str, subtree: &str) -> bool {
    path == subtree
        || path
            .strip_prefix(subtree)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn has_conflicting_rename(active: &[ActiveChange], incoming: &LibraryChangeIntent) -> bool {
    if incoming.kind != LibraryChangeIntentKind::RenameCandidate {
        return false;
    }
    active.iter().any(|change| {
        change.intent.kind == LibraryChangeIntentKind::RenameCandidate
            && !same_work_key(&change.intent, incoming)
            && affected_paths_overlap(&change.intent, incoming)
    })
}

fn has_ambiguous_leased_overlap(active: &[ActiveChange], incoming: &LibraryChangeIntent) -> bool {
    active.iter().any(|change| {
        change.status == LibraryChangeQueueStatus::Leased
            && affected_paths_overlap(&change.intent, incoming)
            && !intent_covers(&change.intent, incoming)
            && !intent_covers(incoming, &change.intent)
    })
}

fn affected_paths_overlap(left: &LibraryChangeIntent, right: &LibraryChangeIntent) -> bool {
    affected_paths(left).any(|left_path| {
        affected_paths(right).any(|right_path| {
            left_path == right_path
                || left.scope == LibraryChangeScope::Subtree
                    && is_within_subtree(right_path, &left.relative_path)
                || right.scope == LibraryChangeScope::Subtree
                    && is_within_subtree(left_path, &right.relative_path)
        })
    })
}

fn intent_strength(intent: &LibraryChangeIntent) -> u8 {
    match (intent.kind, intent.scope) {
        (LibraryChangeIntentKind::FreshnessUnknown, _) => 5,
        (_, LibraryChangeScope::Root) => 4,
        (_, LibraryChangeScope::Subtree) => 3,
        (LibraryChangeIntentKind::RenameCandidate, _) => 2,
        _ => 1,
    }
}

fn merge_evidence(target: &mut LibraryChangeIntent, evidence: &LibraryChangeIntent) {
    let target_recency = evidence_recency(target);
    let incoming_recency = evidence_recency(evidence);
    target.first_observed_unix_ms = target
        .first_observed_unix_ms
        .min(evidence.first_observed_unix_ms);
    target.most_recent_observed_unix_ms = target
        .most_recent_observed_unix_ms
        .max(evidence.most_recent_observed_unix_ms);
    target.first_sequence = target.first_sequence.min(evidence.first_sequence);
    if incoming_recency > target_recency {
        target.most_recent_sequence = evidence.most_recent_sequence;
        target.origin = evidence.origin;
    } else {
        target.most_recent_sequence = target
            .most_recent_sequence
            .max(evidence.most_recent_sequence);
    }
    target.coalesced_observation_count = target
        .coalesced_observation_count
        .saturating_add(evidence.coalesced_observation_count);
}

fn evidence_recency(intent: &LibraryChangeIntent) -> (u64, i64, u8) {
    (
        intent.most_recent_sequence,
        intent.most_recent_observed_unix_ms,
        origin_rank(intent.origin),
    )
}

fn origin_rank(origin: LibraryChangeOrigin) -> u8 {
    match origin {
        LibraryChangeOrigin::LiveNotification => 0,
        LibraryChangeOrigin::StartupCatchUp => 1,
        LibraryChangeOrigin::UserRefresh => 2,
        LibraryChangeOrigin::ConsistencyAudit => 3,
    }
}

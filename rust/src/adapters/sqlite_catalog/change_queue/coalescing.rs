use super::gap_promotion::is_typed_capacity_deferred_live_gap;
use rusqlite::Transaction;

use crate::domain::{
    LibraryChangeCapacityDeferral, LibraryChangeCatchUpEvidence, LibraryChangeEnqueueReport,
    LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLane, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeQueueStatus,
    LibraryChangeScope, ScanError, persistent_journal_canonical_intent_entry,
};

use super::persistence::{
    ActiveChange, insert_change, load_active_changes, mark_superseded, transfer_catch_up_lineage,
    update_change,
};

const MAX_FAILURE_CODE_BYTES: usize = 128;
const MAX_FAILURE_MESSAGE_BYTES: usize = 2_048;
const MAX_ROOT_ID_BYTES: usize = 1_024;
const MAX_RELATIVE_PATH_BYTES: usize = 131_072;

#[derive(Clone, Copy)]
struct DegradationContext<'a> {
    enqueued_unix_ms: i64,
    catalog_revision: u64,
    policy: LibraryChangeQueuePolicy,
    capacity_degraded: bool,
    evidence: Option<&'a LibraryChangeCatchUpEvidence>,
}

pub(super) struct EnqueueContext<'a> {
    pub(super) enqueued_unix_ms: i64,
    pub(super) catalog_revision: u64,
    pub(super) policy: LibraryChangeQueuePolicy,
    pub(super) evidence: Option<&'a LibraryChangeCatchUpEvidence>,
    pub(super) protected_change_ids: &'a [LibraryChangeId],
    pub(super) allow_scope_degradation: bool,
}

pub(super) fn enqueue_one(
    transaction: &Transaction<'_>,
    incoming: &LibraryChangeIntent,
    context: EnqueueContext<'_>,
    report: &mut LibraryChangeEnqueueReport,
) -> Result<LibraryChangeId, ScanError> {
    let EnqueueContext {
        enqueued_unix_ms,
        catalog_revision,
        policy,
        evidence,
        protected_change_ids,
        allow_scope_degradation,
    } = context;
    let all_active = load_active_changes(
        transaction,
        &incoming.root_id,
        incoming.root_generation,
        policy.max_unresolved_changes,
    )?;
    let preserve_inventory_control = preserves_inventory_control(incoming);
    let mut capacity_deferred_gap_ids = Vec::new();
    for change in &all_active {
        if change.last_failure_code.as_deref()
            == Some(LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code())
            && is_typed_capacity_deferred_live_gap(transaction, change.id)?
        {
            capacity_deferred_gap_ids.push(change.id);
        }
    }
    let persistent_watermark = evidence
        .filter(|value| value.source == super::PERSISTENT_JOURNAL_CATCH_UP_SOURCE)
        .map(|value| value.watermark.as_str());
    let incoming_lane = incoming.origin.lane();
    if incoming_lane != LibraryChangeLane::Live && lane_capacity(policy, incoming_lane) == 0 {
        return Err(queue_backpressure());
    }
    if incoming_lane == LibraryChangeLane::Recovery
        && let Some(higher_priority) = all_active
            .iter()
            .filter(|change| {
                change.intent.origin.lane() != LibraryChangeLane::Recovery
                    && exact_path_work_identity(&change.intent, incoming)
            })
            .min_by_key(|change| (lane_priority(change.intent.origin.lane()), change.id))
    {
        report.coalesced_count = report.coalesced_count.saturating_add(1);
        return Ok(higher_priority.id);
    }
    let same_lane = all_active
        .iter()
        .filter(|change| {
            if change.intent.origin.lane() != incoming_lane {
                return false;
            }
            let change_persistent_watermark = change
                .catch_up_evidence
                .as_ref()
                .filter(|value| value.source == super::PERSISTENT_JOURNAL_CATCH_UP_SOURCE)
                .map(|value| value.watermark.as_str());
            match persistent_watermark {
                Some(watermark) => change_persistent_watermark == Some(watermark),
                None => change_persistent_watermark.is_none(),
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    let quota_active = all_active
        .iter()
        .filter(|change| {
            change.intent.origin.lane() != incoming_lane
                || !protected_change_ids.contains(&change.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let lane_counts = active_lane_counts(&quota_active);
    let active = same_lane
        .iter()
        .filter(|change| !preserve_inventory_control || !protected_change_ids.contains(&change.id))
        .cloned()
        .collect::<Vec<_>>();
    let protected_count = all_active
        .len()
        .saturating_sub(active.len())
        .saturating_add(
            capacity_deferred_gap_ids
                .iter()
                .filter(|change_id| !protected_change_ids.contains(change_id))
                .count(),
        );
    if has_conflicting_rename(&active, incoming) {
        if !allow_scope_degradation {
            return Err(metadata_inventory_backpressure());
        }
        return degrade_to_root(
            transaction,
            same_lane,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: false,
                evidence,
            },
        );
    }
    if has_ambiguous_leased_overlap(&active, incoming, &capacity_deferred_gap_ids) {
        if !allow_scope_degradation {
            return Err(metadata_inventory_backpressure());
        }
        let root_id = degrade_to_root(
            transaction,
            same_lane,
            incoming,
            report,
            DegradationContext {
                enqueued_unix_ms,
                catalog_revision,
                policy,
                capacity_degraded: false,
                evidence,
            },
        )?;
        if is_precise_dirty_reconcile(incoming) {
            return enqueue_one(
                transaction,
                incoming,
                EnqueueContext {
                    enqueued_unix_ms,
                    catalog_revision,
                    policy,
                    evidence,
                    protected_change_ids,
                    allow_scope_degradation,
                },
                report,
            );
        }
        return Ok(root_id);
    }
    let covering = active.iter().position(|change| {
        is_unleased(change.status)
            && intent_covers(&change.intent, incoming)
            && !capacity_deferred_gap_preserves_precise_work(
                change,
                incoming,
                &capacity_deferred_gap_ids,
            )
    });
    if let Some(target_index) = covering {
        let target_id = active[target_index].id;
        let mut merged = active[target_index].intent.clone();
        merge_newer_evidence(&mut merged, incoming);
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
                    && !capacity_deferred_gap_preserves_precise_work(
                        change,
                        incoming,
                        &capacity_deferred_gap_ids,
                    )
            })
            .map(|(_, change)| change)
            .collect::<Vec<_>>();
        for change in &absorbed {
            merge_older_evidence(&mut merged, &change.intent);
        }
        let retained_evidence = evidence.or_else(|| {
            absorbed
                .iter()
                .rev()
                .find_map(|change| change.catch_up_evidence.as_ref())
        });
        update_change(
            transaction,
            target_id,
            &merged,
            retained_evidence,
            enqueued_unix_ms,
            policy,
        )?;
        transfer_catch_up_lineage(
            transaction,
            absorbed.iter().map(|change| change.id),
            target_id,
        )?;
        let superseded = mark_superseded(
            transaction,
            absorbed.iter().map(|change| change.id),
            Some(target_id),
            enqueued_unix_ms,
        )?;
        report.coalesced_count = report.coalesced_count.saturating_add(1);
        report.superseded_count = report.superseded_count.saturating_add(superseded);
        report.freshness_unknown_enqueued |=
            merged.kind == LibraryChangeIntentKind::FreshnessUnknown;
        supersede_recovery_candidates_covered_by_higher_lane(
            transaction,
            &all_active,
            incoming_lane,
            target_id,
            &merged,
            enqueued_unix_ms,
            report,
        )?;
        return Ok(target_id);
    }

    let absorbed = active
        .iter()
        .filter(|change| {
            (intent_covers(incoming, &change.intent)
                || (change.status == LibraryChangeQueueStatus::Leased
                    && stale_overlap(&change.intent, incoming)))
                && !capacity_deferred_gap_preserves_precise_work(
                    change,
                    incoming,
                    &capacity_deferred_gap_ids,
                )
        })
        .collect::<Vec<_>>();
    let admitted_counts = lane_counts.replacing(incoming_lane, absorbed.len());
    let absolute_count = all_active
        .len()
        .saturating_sub(absorbed.len())
        .saturating_add(1);
    if !lane_admission_allows(policy, incoming_lane, admitted_counts)
        || absolute_count
            > usize::try_from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES)
                .unwrap_or(usize::MAX)
    {
        if protected_count > 0 {
            return Err(if allow_scope_degradation {
                queue_backpressure()
            } else {
                metadata_inventory_backpressure()
            });
        }
        if !allow_scope_degradation {
            return Err(metadata_inventory_backpressure());
        }
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
                evidence,
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
        merge_newer_evidence(&mut merged, incoming);
    }
    for change in &absorbed {
        if stronger.is_some_and(|stronger| stronger.id == change.id) {
            continue;
        }
        merge_older_evidence(&mut merged, &change.intent);
    }
    let retained_evidence = evidence.or_else(|| {
        absorbed
            .iter()
            .rev()
            .find_map(|change| change.catch_up_evidence.as_ref())
    });
    let change_id = insert_change(
        transaction,
        &merged,
        enqueued_unix_ms,
        catalog_revision,
        retained_evidence,
        policy,
    )?;
    transfer_catch_up_lineage(
        transaction,
        absorbed.iter().map(|change| change.id),
        change_id,
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
    supersede_recovery_candidates_covered_by_higher_lane(
        transaction,
        &all_active,
        incoming_lane,
        change_id,
        &merged,
        enqueued_unix_ms,
        report,
    )?;
    Ok(change_id)
}

fn capacity_deferred_gap_preserves_precise_work(
    change: &ActiveChange,
    incoming: &LibraryChangeIntent,
    capacity_deferred_gap_ids: &[LibraryChangeId],
) -> bool {
    capacity_deferred_gap_ids.contains(&change.id)
        && incoming.origin == LibraryChangeOrigin::LiveNotification
        && incoming.scope == LibraryChangeScope::Path
        && incoming.kind != LibraryChangeIntentKind::FreshnessUnknown
}

fn supersede_recovery_candidates_covered_by_higher_lane(
    transaction: &Transaction<'_>,
    all_active: &[ActiveChange],
    incoming_lane: LibraryChangeLane,
    target_id: LibraryChangeId,
    target: &LibraryChangeIntent,
    enqueued_unix_ms: i64,
    report: &mut LibraryChangeEnqueueReport,
) -> Result<(), ScanError> {
    if incoming_lane == LibraryChangeLane::Recovery || target.scope != LibraryChangeScope::Path {
        return Ok(());
    }
    let superseded = mark_superseded(
        transaction,
        all_active
            .iter()
            .filter(|change| {
                change.intent.origin.lane() == LibraryChangeLane::Recovery
                    && exact_path_work_identity(&change.intent, target)
            })
            .map(|change| change.id),
        Some(target_id),
        enqueued_unix_ms,
    )?;
    report.superseded_count = report.superseded_count.saturating_add(superseded);
    Ok(())
}

fn exact_path_work_identity(left: &LibraryChangeIntent, right: &LibraryChangeIntent) -> bool {
    left.scope == LibraryChangeScope::Path
        && right.scope == LibraryChangeScope::Path
        && left.relative_path == right.relative_path
        && left.previous_relative_path == right.previous_relative_path
}

const fn lane_priority(lane: LibraryChangeLane) -> u8 {
    match lane {
        LibraryChangeLane::Live => 0,
        LibraryChangeLane::Journal => 1,
        LibraryChangeLane::Recovery => 2,
    }
}

fn metadata_inventory_backpressure() -> ScanError {
    ScanError::new(
        "metadata_inventory_backpressure",
        "The durable path queue must drain before inventory comparison continues",
    )
}

pub(super) fn lane_capacity(policy: LibraryChangeQueuePolicy, lane: LibraryChangeLane) -> usize {
    usize::try_from(policy.lane_capacity(lane)).unwrap_or(usize::MAX)
}

#[derive(Clone, Copy)]
pub(super) struct ActiveLaneCounts {
    live: usize,
    journal: usize,
    recovery: usize,
}

impl ActiveLaneCounts {
    fn replacing(self, lane: LibraryChangeLane, absorbed_count: usize) -> Self {
        let mut result = self;
        let lane_count = match lane {
            LibraryChangeLane::Live => &mut result.live,
            LibraryChangeLane::Journal => &mut result.journal,
            LibraryChangeLane::Recovery => &mut result.recovery,
        };
        *lane_count = lane_count.saturating_sub(absorbed_count).saturating_add(1);
        result
    }

    pub(super) fn adding(self, lane: LibraryChangeLane, count: usize) -> Self {
        let mut result = self;
        let lane_count = match lane {
            LibraryChangeLane::Live => &mut result.live,
            LibraryChangeLane::Journal => &mut result.journal,
            LibraryChangeLane::Recovery => &mut result.recovery,
        };
        *lane_count = lane_count.saturating_add(count);
        result
    }
}

pub(super) fn active_lane_counts(active: &[ActiveChange]) -> ActiveLaneCounts {
    let mut counts = ActiveLaneCounts {
        live: 0,
        journal: 0,
        recovery: 0,
    };
    for change in active {
        counts = counts.adding(change.intent.origin.lane(), 1);
    }
    counts
}

pub(super) fn lane_admission_allows(
    policy: LibraryChangeQueuePolicy,
    incoming_lane: LibraryChangeLane,
    counts: ActiveLaneCounts,
) -> bool {
    let total = counts
        .live
        .saturating_add(counts.journal)
        .saturating_add(counts.recovery);
    if total > usize::try_from(policy.max_unresolved_changes).unwrap_or(usize::MAX) {
        return false;
    }
    match incoming_lane {
        LibraryChangeLane::Live => true,
        LibraryChangeLane::Journal => {
            counts.journal.saturating_add(counts.recovery)
                <= lane_capacity(policy, LibraryChangeLane::Journal)
        }
        LibraryChangeLane::Recovery => {
            counts.journal.saturating_add(counts.recovery)
                <= lane_capacity(policy, LibraryChangeLane::Journal)
                && counts.recovery <= lane_capacity(policy, LibraryChangeLane::Recovery)
        }
    }
}

pub(in crate::adapters::sqlite_catalog) fn normalize_persistent_journal_intents(
    intents: &[LibraryChangeIntent],
) -> Result<Vec<LibraryChangeIntent>, ScanError> {
    let Some(first) = intents.first() else {
        return Ok(Vec::new());
    };
    validate_intent_batch(intents, first)?;
    let mut ordered = intents.to_vec();
    ordered.sort_by_cached_key(|intent| {
        (
            intent.most_recent_sequence,
            intent.first_sequence,
            persistent_journal_canonical_intent_entry(intent),
        )
    });
    let mut normalized = Vec::new();
    for incoming in ordered {
        normalize_persistent_journal_intent(&mut normalized, incoming);
    }
    normalized.sort_by_cached_key(persistent_journal_canonical_intent_entry);
    Ok(normalized)
}

fn normalize_persistent_journal_intent(
    active: &mut Vec<LibraryChangeIntent>,
    incoming: LibraryChangeIntent,
) {
    if rename_lineage_previous_path(&incoming).is_some()
        && active.iter().any(|change| {
            rename_lineage_previous_path(change).is_some()
                && !same_work_key(change, &incoming)
                && affected_paths_overlap(change, &incoming)
        })
    {
        let mut root = incoming;
        root.kind = LibraryChangeIntentKind::FreshnessUnknown;
        root.scope = LibraryChangeScope::Root;
        root.relative_path.clear();
        root.previous_relative_path = None;
        for change in active.iter() {
            merge_older_evidence(&mut root, change);
        }
        active.clear();
        active.push(root);
        return;
    }

    if let Some(target_index) = active
        .iter()
        .position(|change| intent_covers(change, &incoming))
    {
        let mut merged = active[target_index].clone();
        merge_newer_evidence(&mut merged, &incoming);
        if incoming.kind == LibraryChangeIntentKind::FreshnessUnknown {
            merged.kind = LibraryChangeIntentKind::FreshnessUnknown;
        }
        for (index, change) in active.iter().enumerate() {
            if index != target_index && intent_covers(&merged, change) {
                merge_older_evidence(&mut merged, change);
            }
        }
        let mut retained = Vec::with_capacity(active.len());
        for (index, change) in active.drain(..).enumerate() {
            if index == target_index {
                retained.push(merged.clone());
            } else if !intent_covers(&merged, &change) {
                retained.push(change);
            }
        }
        *active = retained;
        return;
    }

    let absorbed = active
        .iter()
        .enumerate()
        .filter(|(_, change)| intent_covers(&incoming, change))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let stronger = absorbed
        .iter()
        .copied()
        .max_by_key(|index| intent_strength(&active[*index]));
    let mut merged = stronger
        .map(|index| active[index].clone())
        .unwrap_or_else(|| incoming.clone());
    if stronger.is_some() {
        merge_newer_evidence(&mut merged, &incoming);
    }
    for index in &absorbed {
        if Some(*index) != stronger {
            merge_older_evidence(&mut merged, &active[*index]);
        }
    }
    for index in absorbed.into_iter().rev() {
        active.remove(index);
    }
    active.push(merged);
}

fn preserves_inventory_control(incoming: &LibraryChangeIntent) -> bool {
    incoming.kind != LibraryChangeIntentKind::FreshnessUnknown
        && !matches!(
            incoming.origin,
            LibraryChangeOrigin::StartupCatchUp | LibraryChangeOrigin::ConsistencyAudit
        )
}

pub(super) fn queue_backpressure() -> ScanError {
    ScanError::new(
        "change_queue_backpressure",
        "The durable path queue must drain before more live observations are retained",
    )
}

fn degrade_to_root(
    transaction: &Transaction<'_>,
    active: Vec<ActiveChange>,
    incoming: &LibraryChangeIntent,
    report: &mut LibraryChangeEnqueueReport,
    context: DegradationContext<'_>,
) -> Result<LibraryChangeId, ScanError> {
    let mut root = incoming.clone();
    root.kind = LibraryChangeIntentKind::FreshnessUnknown;
    root.scope = LibraryChangeScope::Root;
    root.relative_path.clear();
    root.previous_relative_path = None;
    for change in &active {
        merge_older_evidence(&mut root, &change.intent);
    }
    let retained_evidence = context.evidence.or_else(|| {
        active
            .iter()
            .rev()
            .find_map(|change| change.catch_up_evidence.as_ref())
    });
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
            retained_evidence,
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
            retained_evidence,
            context.policy,
        )?;
        report.inserted_count = report.inserted_count.saturating_add(1);
        id
    };
    transfer_catch_up_lineage(
        transaction,
        active.iter().map(|change| change.id),
        target_id,
    )?;
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
    Ok(target_id)
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
            LibraryChangeScope::Path => {
                has_path
                    && match intent.previous_relative_path.as_deref() {
                        None => true,
                        Some(previous) => {
                            has_previous_path && previous != intent.relative_path.as_str()
                        }
                    }
            }
            LibraryChangeScope::Subtree => has_path && intent.previous_relative_path.is_none(),
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
    if LibraryChangeCapacityDeferral::is_reserved_failure_code(&failure.code) {
        return Err(ScanError::new(
            "change_queue_failure_code_reserved",
            "Capacity-deferral failure codes are reserved for the typed queue operation",
        ));
    }
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
    if covering.scope != LibraryChangeScope::Path && is_precise_dirty_reconcile(candidate) {
        return false;
    }
    if covering.scope == LibraryChangeScope::Root {
        return true;
    }
    if covering.scope == LibraryChangeScope::Subtree {
        return affected_paths(candidate).all(|candidate_path| {
            affected_paths(covering)
                .any(|subtree_path| is_within_subtree(candidate_path, subtree_path))
        });
    }
    same_work_key(covering, candidate)
        || path_rename_lineage_previous_path(covering).is_some()
            && candidate.kind == LibraryChangeIntentKind::Reconcile
            && candidate.scope == LibraryChangeScope::Path
            && covering.relative_path == candidate.relative_path
}

fn is_precise_dirty_reconcile(intent: &LibraryChangeIntent) -> bool {
    intent.kind == LibraryChangeIntentKind::Reconcile
        && intent.scope == LibraryChangeScope::Path
        && matches!(
            intent.origin,
            LibraryChangeOrigin::LiveNotification
                | LibraryChangeOrigin::StartupCatchUp
                | LibraryChangeOrigin::MetadataInventory
        )
}

fn stale_overlap(leased: &LibraryChangeIntent, incoming: &LibraryChangeIntent) -> bool {
    intent_covers(leased, incoming)
        || intent_covers(incoming, leased)
        || affected_paths_overlap(leased, incoming)
}

fn same_work_key(left: &LibraryChangeIntent, right: &LibraryChangeIntent) -> bool {
    (left.kind == right.kind
        && left.scope == right.scope
        && left.relative_path == right.relative_path
        && left.previous_relative_path == right.previous_relative_path)
        || same_rename_lineage(left, right)
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
    if rename_lineage_previous_path(incoming).is_none() {
        return false;
    }
    active.iter().any(|change| {
        rename_lineage_previous_path(&change.intent).is_some()
            && !same_work_key(&change.intent, incoming)
            && affected_paths_overlap(&change.intent, incoming)
    })
}

fn has_ambiguous_leased_overlap(
    active: &[ActiveChange],
    incoming: &LibraryChangeIntent,
    capacity_deferred_gap_ids: &[LibraryChangeId],
) -> bool {
    active.iter().any(|change| {
        change.status == LibraryChangeQueueStatus::Leased
            && !capacity_deferred_gap_preserves_precise_work(
                change,
                incoming,
                capacity_deferred_gap_ids,
            )
            && affected_paths_overlap(&change.intent, incoming)
            && !intent_covers(&change.intent, incoming)
            && !intent_covers(incoming, &change.intent)
    })
}

pub(super) fn affected_paths_overlap(
    left: &LibraryChangeIntent,
    right: &LibraryChangeIntent,
) -> bool {
    affected_paths(left).any(|left_path| {
        affected_paths(right).any(|right_path| {
            left_path == right_path
                || left.scope == LibraryChangeScope::Subtree
                    && is_within_subtree(right_path, left_path)
                || right.scope == LibraryChangeScope::Subtree
                    && is_within_subtree(left_path, right_path)
        })
    })
}

fn intent_strength(intent: &LibraryChangeIntent) -> u8 {
    if intent.kind == LibraryChangeIntentKind::Reconcile
        && path_rename_lineage_previous_path(intent).is_some()
    {
        return 2;
    }
    match (intent.kind, intent.scope) {
        (LibraryChangeIntentKind::FreshnessUnknown, _) => 5,
        (_, LibraryChangeScope::Root) => 4,
        (_, LibraryChangeScope::Subtree) => 3,
        (LibraryChangeIntentKind::RenameCandidate, _) => 2,
        _ => 1,
    }
}

pub(super) fn merge_newer_evidence(
    target: &mut LibraryChangeIntent,
    evidence: &LibraryChangeIntent,
) {
    merge_evidence_range(target, evidence);
    target.most_recent_observed_unix_ms = evidence.most_recent_observed_unix_ms;
    target.most_recent_sequence = evidence.most_recent_sequence;
    target.origin = evidence.origin;
}

pub(super) fn merge_older_evidence(
    target: &mut LibraryChangeIntent,
    evidence: &LibraryChangeIntent,
) {
    merge_evidence_range(target, evidence);
}

fn merge_evidence_range(target: &mut LibraryChangeIntent, evidence: &LibraryChangeIntent) {
    merge_dirty_rename_semantics(target, evidence);
    target.first_observed_unix_ms = target
        .first_observed_unix_ms
        .min(evidence.first_observed_unix_ms);
    target.first_sequence = target.first_sequence.min(evidence.first_sequence);
    target.coalesced_observation_count = target
        .coalesced_observation_count
        .saturating_add(evidence.coalesced_observation_count);
}

fn rename_lineage_previous_path(intent: &LibraryChangeIntent) -> Option<&str> {
    (intent.kind == LibraryChangeIntentKind::RenameCandidate
        || intent.kind == LibraryChangeIntentKind::Reconcile
            && intent.scope == LibraryChangeScope::Path)
        .then_some(intent.previous_relative_path.as_deref())
        .flatten()
}

fn path_rename_lineage_previous_path(intent: &LibraryChangeIntent) -> Option<&str> {
    (intent.scope == LibraryChangeScope::Path)
        .then_some(rename_lineage_previous_path(intent))
        .flatten()
}

fn same_rename_lineage(left: &LibraryChangeIntent, right: &LibraryChangeIntent) -> bool {
    left.scope == LibraryChangeScope::Path
        && right.scope == LibraryChangeScope::Path
        && left.relative_path == right.relative_path
        && path_rename_lineage_previous_path(left).is_some()
        && path_rename_lineage_previous_path(left) == path_rename_lineage_previous_path(right)
}

fn merge_dirty_rename_semantics(target: &mut LibraryChangeIntent, evidence: &LibraryChangeIntent) {
    if target.scope != LibraryChangeScope::Path
        || evidence.scope != LibraryChangeScope::Path
        || target.relative_path != evidence.relative_path
    {
        return;
    }
    let target_previous = path_rename_lineage_previous_path(target).map(str::to_owned);
    let evidence_previous = path_rename_lineage_previous_path(evidence).map(str::to_owned);
    if target_previous.is_some()
        && evidence_previous.is_some()
        && target_previous != evidence_previous
    {
        return;
    }
    let previous = if target.kind == LibraryChangeIntentKind::Reconcile {
        evidence_previous
    } else if evidence.kind == LibraryChangeIntentKind::Reconcile {
        target_previous
    } else {
        None
    };
    let Some(previous) = previous else {
        return;
    };
    target.kind = LibraryChangeIntentKind::Reconcile;
    target.previous_relative_path = Some(previous);
}

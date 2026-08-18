use crate::domain::{
    LibraryChangeCatchUpEvidence, LibraryChangeCatchUpQueueBatch, LibraryChangeEnqueueReport,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangePlanningResult,
    LibraryChangeQueuePolicy, LibraryChangeScope, ScanError,
};
use crate::ports::LibraryChangeQueue;

pub fn enqueue_library_change_plan<Queue>(
    queue: &mut Queue,
    plan: &LibraryChangePlanningResult,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeEnqueueReport, ScanError>
where
    Queue: LibraryChangeQueue,
{
    if !policy.is_valid() {
        return Err(ScanError::new(
            "change_queue_policy_invalid",
            "The durable change queue policy must stay within its absolute bounds",
        ));
    }
    for intent in &plan.intents {
        validate_intent(plan, intent)?;
    }
    queue.enqueue_library_change_intents(&plan.intents, enqueued_unix_ms, policy)
}

pub(crate) fn prepare_library_change_catch_up_plan(
    plan: &LibraryChangePlanningResult,
    evidence: Option<&LibraryChangeCatchUpEvidence>,
) -> Result<LibraryChangeCatchUpQueueBatch, ScanError> {
    if let Some(evidence) = evidence
        && (evidence.source.trim().is_empty() || evidence.watermark.trim().is_empty())
    {
        return Err(ScanError::new(
            "library_change_catch_up_evidence_invalid",
            "Journal-derived work requires a non-empty source and watermark",
        ));
    }
    for intent in &plan.intents {
        validate_intent(plan, intent)?;
    }
    Ok(LibraryChangeCatchUpQueueBatch {
        intents: plan.intents.clone(),
        evidence: evidence.cloned(),
    })
}

fn validate_intent(
    plan: &LibraryChangePlanningResult,
    intent: &LibraryChangeIntent,
) -> Result<(), ScanError> {
    if intent.root_id != plan.root_id || intent.root_generation != plan.root_generation {
        return Err(ScanError::new(
            "change_queue_plan_mismatch",
            "Every durable change intent must belong to the planned root generation",
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
    let has_path = !intent.relative_path.is_empty();
    let has_previous_path = intent
        .previous_relative_path
        .as_ref()
        .is_some_and(|path| !path.is_empty());
    let shape_is_valid = match intent.kind {
        LibraryChangeIntentKind::FreshnessUnknown => {
            intent.scope == LibraryChangeScope::Root && !has_path && !has_previous_path
        }
        LibraryChangeIntentKind::RenameCandidate => {
            intent.scope != LibraryChangeScope::Root && has_path && has_previous_path
        }
        LibraryChangeIntentKind::Reconcile => match intent.scope {
            LibraryChangeScope::Root => !has_path && !has_previous_path,
            LibraryChangeScope::Path | LibraryChangeScope::Subtree => {
                has_path && !has_previous_path
            }
        },
    };
    if !shape_is_valid {
        return Err(ScanError::new(
            "change_queue_intent_invalid",
            "The durable change intent does not match its normalized kind and scope",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

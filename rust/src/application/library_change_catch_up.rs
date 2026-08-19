use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    IncrementalCatalogRoot, LibraryChangeCatchUpCompletedRoot, LibraryChangeCatchUpEvidence,
    LibraryChangeCatchUpLimits, LibraryChangeCatchUpQueueBatch, LibraryChangeCatchUpReport,
    LibraryChangeObservation, LibraryChangeObservationKind, LibraryChangeOrigin,
    LibraryChangePlanningContext, LibraryChangePlanningLimits, LibraryChangePlanningResult,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryChangeSourceHealth,
    LibraryRootAvailability, ScanError,
};
use crate::ports::{
    LibraryChangeCatchUpRepository, LibraryChangeCatchUpSource, LibraryChangeQueue,
};

use super::{plan_library_changes, prepare_library_change_catch_up_plan};

const CHECKPOINT_RETENTION_MILLIS: i64 = 7 * 24 * 60 * 60 * 1_000;
const CHECKPOINT_CLEANUP_LIMIT: u32 = 128;

#[derive(Clone, Copy)]
pub(crate) struct LibraryChangeCatchUpExecution {
    pub(crate) now_unix_ms: i64,
    pub(crate) planning_limits: LibraryChangePlanningLimits,
    pub(crate) catch_up_limits: LibraryChangeCatchUpLimits,
    pub(crate) queue_policy: LibraryChangeQueuePolicy,
}

impl LibraryChangeCatchUpExecution {
    pub(crate) fn at(now_unix_ms: i64, queue_policy: LibraryChangeQueuePolicy) -> Self {
        Self {
            now_unix_ms,
            planning_limits: LibraryChangePlanningLimits::default(),
            catch_up_limits: LibraryChangeCatchUpLimits::default(),
            queue_policy,
        }
    }
}

pub(crate) fn process_library_change_catch_up<Source, Repository>(
    source: &Source,
    repository: &mut Repository,
    roots: &[IncrementalCatalogRoot],
    execution: LibraryChangeCatchUpExecution,
    cancelled: &AtomicBool,
) -> Result<LibraryChangeCatchUpReport, ScanError>
where
    Source: LibraryChangeCatchUpSource,
    Repository: LibraryChangeCatchUpRepository + LibraryChangeQueue,
{
    if !execution.catch_up_limits.is_valid() {
        return Err(ScanError::new(
            "library_change_catch_up_limits_invalid",
            "Downtime catch-up limits must stay within their absolute bounds",
        ));
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(catch_up_cancelled());
    }
    let checkpoints = repository.load_library_change_catch_up_checkpoints()?;
    let batch = source.read_changes(
        roots,
        &checkpoints,
        execution.now_unix_ms,
        execution.catch_up_limits,
        cancelled,
    )?;
    validate_batch(roots, &batch.roots, execution.catch_up_limits)?;

    let mut report = LibraryChangeCatchUpReport::default();
    let mut queue_batches = Vec::new();
    for root in batch.roots {
        if cancelled.load(Ordering::Acquire) {
            return Err(catch_up_cancelled());
        }
        let fallback_code = root.fallback_code.clone();
        let observations = if fallback_code.is_some() {
            vec![fallback_observation(
                &root.root_id,
                root.root_generation,
                execution.now_unix_ms,
            )]
        } else {
            root.observations
        };
        let observation_count = u64::try_from(observations.len()).unwrap_or(u64::MAX);
        let context = LibraryChangePlanningContext {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            availability: LibraryRootAvailability::Available,
            source_health: LibraryChangeSourceHealth::Healthy,
        };
        let plan = plan_library_changes(&context, observations, execution.planning_limits)
            .map_err(|error| ScanError::new(error.code, error.message))?;
        if let Some(queue_batch) = prepare_catch_up_queue_batch(&plan, root.evidence.as_ref())? {
            queue_batches.push(queue_batch);
        }
        report.observation_count = report
            .observation_count
            .checked_add(observation_count)
            .ok_or_else(|| {
                ScanError::new(
                    "library_change_catch_up_count_overflow",
                    "The downtime catch-up observation count exceeded the supported range",
                )
            })?;
        if fallback_code.is_some() {
            report.fallback_count = report.fallback_count.saturating_add(1);
        }
        report
            .completed_roots
            .push(LibraryChangeCatchUpCompletedRoot {
                root_id: root.root_id,
                root_generation: root.root_generation,
                fallback_code,
            });
    }

    if cancelled.load(Ordering::Acquire) {
        return Err(catch_up_cancelled());
    }
    repository.enqueue_library_change_catch_up_batches(
        &queue_batches,
        execution.now_unix_ms,
        execution.queue_policy,
    )?;
    if cancelled.load(Ordering::Acquire) {
        return Err(catch_up_cancelled());
    }
    for checkpoint in &batch.checkpoints {
        repository.save_library_change_catch_up_checkpoint(checkpoint)?;
        report.checkpoint_count = report.checkpoint_count.saturating_add(1);
    }
    let retained_volume_ids = batch
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.volume_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let updated_before_unix_ms = execution
        .now_unix_ms
        .saturating_sub(CHECKPOINT_RETENTION_MILLIS);
    report.checkpoint_cleanup_count = repository
        .cleanup_obsolete_library_change_catch_up_checkpoints(
            &retained_volume_ids,
            updated_before_unix_ms,
            CHECKPOINT_CLEANUP_LIMIT,
        )?;
    Ok(report)
}

fn prepare_catch_up_queue_batch(
    plan: &LibraryChangePlanningResult,
    evidence: Option<&LibraryChangeCatchUpEvidence>,
) -> Result<Option<LibraryChangeCatchUpQueueBatch>, ScanError> {
    if plan.intents.is_empty() {
        return Ok(None);
    }
    if let Some(evidence) = evidence {
        prepare_library_change_catch_up_plan(plan, Some(evidence)).map(Some)
    } else if plan.intents.iter().all(|intent| {
        intent.kind == crate::domain::LibraryChangeIntentKind::FreshnessUnknown
            && intent.scope == LibraryChangeScope::Root
    }) {
        prepare_library_change_catch_up_plan(plan, None).map(Some)
    } else {
        Err(ScanError::new(
            "library_change_catch_up_evidence_missing",
            "Journal-derived path work must retain its source and exclusive watermark",
        ))
    }
}

fn validate_batch(
    requested_roots: &[IncrementalCatalogRoot],
    results: &[crate::domain::LibraryChangeCatchUpRootResult],
    limits: LibraryChangeCatchUpLimits,
) -> Result<(), ScanError> {
    let requested = requested_roots
        .iter()
        .map(|root| (root.root_id.as_str(), root.root_generation))
        .collect::<BTreeMap<_, _>>();
    let mut returned = BTreeSet::new();
    for result in results {
        if requested.get(result.root_id.as_str()) != Some(&result.root_generation)
            || !returned.insert(result.root_id.as_str())
        {
            return Err(ScanError::new(
                "library_change_catch_up_root_mismatch",
                "Downtime catch-up must return each requested root generation exactly once",
            ));
        }
        if result.fallback_code.is_some() && !result.observations.is_empty() {
            return Err(ScanError::new(
                "library_change_catch_up_fallback_invalid",
                "A fallback root cannot also claim journal observations",
            ));
        }
        if result.observations.len() > limits.max_observations_per_root {
            return Err(ScanError::new(
                "library_change_catch_up_observation_limit_exceeded",
                "A downtime catch-up root exceeded its absolute observation bound",
            ));
        }
        for observation in &result.observations {
            if observation.root_id != result.root_id
                || observation.root_generation != result.root_generation
                || observation.origin != LibraryChangeOrigin::StartupCatchUp
            {
                return Err(ScanError::new(
                    "library_change_catch_up_observation_mismatch",
                    "Every journal observation must belong to its requested root generation",
                ));
            }
        }
    }
    if returned.len() != requested.len() {
        return Err(ScanError::new(
            "library_change_catch_up_root_missing",
            "Downtime catch-up omitted a requested root generation",
        ));
    }
    Ok(())
}

fn fallback_observation(
    root_id: &str,
    root_generation: crate::domain::LibraryRootGeneration,
    observed_unix_ms: i64,
) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: root_id.to_owned(),
        root_generation,
        sequence: 1,
        observed_unix_ms,
        kind: LibraryChangeObservationKind::EvidenceGap,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::StartupCatchUp,
    }
}

fn catch_up_cancelled() -> ScanError {
    ScanError::new(
        "library_change_catch_up_cancelled",
        "Downtime catch-up was cancelled before its checkpoint could advance",
    )
}

#[cfg(test)]
mod tests;

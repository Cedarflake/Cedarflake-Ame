use crate::domain::{
    AssetLocationView, DerivedEvidenceDisposition, IncrementalReconciliationDecision,
    IncrementalReconciliationOutcome, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeScope,
};

use crate::application::media_metadata_is_invalidated;

pub(super) fn requires_metadata_recheck(
    prior: &AssetLocationView,
    intent: &LibraryChangeIntent,
) -> bool {
    media_metadata_is_invalidated(prior)
        && intent.origin == LibraryChangeOrigin::ConsistencyAudit
        && intent.kind == LibraryChangeIntentKind::Reconcile
        && intent.scope == LibraryChangeScope::Path
        && intent.previous_relative_path.is_none()
}

pub(super) fn apply_evidence_policy(
    decision: &mut IncrementalReconciliationDecision,
    prior: Option<&AssetLocationView>,
    intent: &LibraryChangeIntent,
    metadata_compatible: bool,
) {
    let dirty_reconcile = decision.outcome == IncrementalReconciliationOutcome::Unchanged
        || intent.previous_relative_path.is_some()
            && decision.outcome == IncrementalReconciliationOutcome::RenamedOrMoved;
    if prior.is_some()
        && matches!(
            intent.origin,
            LibraryChangeOrigin::LiveNotification
                | LibraryChangeOrigin::StartupCatchUp
                | LibraryChangeOrigin::MetadataInventory
        )
        && intent.kind == LibraryChangeIntentKind::Reconcile
        && intent.scope == LibraryChangeScope::Path
        && dirty_reconcile
    {
        decision.outcome = IncrementalReconciliationOutcome::Modified;
        invalidate_generation(decision);
    }
    if intent.origin == LibraryChangeOrigin::MetadataInventory
        && intent.kind == LibraryChangeIntentKind::RenameCandidate
        && decision.outcome == IncrementalReconciliationOutcome::RenamedOrMoved
    {
        invalidate_generation(decision);
    }
    let Some(prior) = prior else { return };
    if metadata_compatible {
        return;
    }
    match decision.outcome {
        IncrementalReconciliationOutcome::Unchanged if requires_metadata_recheck(prior, intent) => {
            // The dirty event already advanced source generation. Rebuilding metadata after
            // an unchanged physical-state proof must not invalidate a newly computed hash.
        }
        IncrementalReconciliationOutcome::Unchanged => {
            decision.outcome = IncrementalReconciliationOutcome::Modified;
            decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
        }
        IncrementalReconciliationOutcome::RenamedOrMoved => {
            decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
        }
        _ => {}
    }
}

fn invalidate_generation(decision: &mut IncrementalReconciliationDecision) {
    decision.evidence_disposition = DerivedEvidenceDisposition::InvalidateDerived;
    if let Some(current) = decision.current.as_mut() {
        current.source_generation = 0;
    }
}

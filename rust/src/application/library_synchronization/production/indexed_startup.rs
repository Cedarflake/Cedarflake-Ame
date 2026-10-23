use std::collections::BTreeMap;

use crate::domain::{
    CatalogFreshnessState, LeasedLibraryChange, LibraryChangeScope, LibraryChangeSourceHealth,
    LibraryRootAvailability, LibrarySynchronizationPhase, LibrarySynchronizationSnapshot,
    ScanError,
};
use crate::ports::MetadataInventoryRepository;

#[derive(Default)]
pub(super) struct IndexedStartupOwner {
    attempts: BTreeMap<String, IndexedAttempt>,
}

struct IndexedAttempt {
    generation: u64,
    change_id: crate::domain::LibraryChangeId,
    finished: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecoveryPreparation {
    IndexedStartup,
    Verification,
}

impl IndexedStartupOwner {
    pub(super) fn preparation(
        &self,
        catalog: &impl MetadataInventoryRepository,
        lease: &LeasedLibraryChange,
    ) -> Result<RecoveryPreparation, ScanError> {
        let intent = &lease.change.intent;
        if intent.scope == LibraryChangeScope::Root
            && !self.attempts.get(&intent.root_id).is_some_and(|attempt| {
                attempt.generation == intent.root_generation.value()
                    && attempt.change_id == lease.change.id
            })
            && catalog
                .load_metadata_inventory_recovery_authority(lease.change.id)?
                .is_some_and(|authority| {
                    authority.retired_unix_ms.is_none()
                        && authority.root_id == intent.root_id
                        && authority.root_generation == intent.root_generation
                        && crate::application::indexed_startup::permits_indexed_startup(
                            authority.reason,
                        )
                })
        {
            return Ok(RecoveryPreparation::IndexedStartup);
        }
        Ok(RecoveryPreparation::Verification)
    }

    pub(super) fn started(
        &mut self,
        lease: &LeasedLibraryChange,
        preparation: RecoveryPreparation,
    ) {
        if matches!(preparation, RecoveryPreparation::IndexedStartup) {
            self.attempts.insert(
                lease.change.intent.root_id.clone(),
                IndexedAttempt {
                    generation: lease.change.intent.root_generation.value(),
                    change_id: lease.change.id,
                    finished: false,
                },
            );
        }
    }

    pub(super) fn finished(&mut self, root_id: &str) {
        if let Some(attempt) = self.attempts.get_mut(root_id) {
            attempt.finished = true;
        }
    }

    pub(super) fn project(&self, snapshot: &mut LibrarySynchronizationSnapshot) {
        for status in &mut snapshot.roots {
            if self.attempts.get(&status.root_id).is_some_and(|attempt| {
                attempt.generation == status.root_generation && attempt.finished
            }) && status.availability == LibraryRootAvailability::Available
                && status.source_health == LibraryChangeSourceHealth::Healthy
                && !status.recovery_blocked
                && status.freshness != CatalogFreshnessState::Synchronized
                && matches!(
                    status.phase,
                    LibrarySynchronizationPhase::InventoryEnumeration
                        | LibrarySynchronizationPhase::InventoryComparison
                        | LibrarySynchronizationPhase::QueuePublication
                        | LibrarySynchronizationPhase::Reconciliation
                        | LibrarySynchronizationPhase::RetryWait
                )
            {
                status.phase = LibrarySynchronizationPhase::BackgroundVerification;
                status.freshness = CatalogFreshnessState::Updating;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        CatalogFreshnessCause, LibraryChangeQueueHealth, LibraryRootSynchronizationStatus,
        PersistentJournalContinuityState,
    };

    fn snapshot() -> LibrarySynchronizationSnapshot {
        LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision: 1,
            applied_mutation_count: 0,
            roots: vec![LibraryRootSynchronizationStatus {
                root_id: "root-a".into(),
                root_generation: 1,
                availability: LibraryRootAvailability::Available,
                freshness: CatalogFreshnessState::Updating,
                freshness_cause: CatalogFreshnessCause::EvidenceGap,
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                phase: LibrarySynchronizationPhase::InventoryEnumeration,
                source_health: LibraryChangeSourceHealth::Healthy,
                queue_health: LibraryChangeQueueHealth::Healthy,
                pending_change_count: 80,
                retry_wait_count: 0,
                freshness_unknown_count: 1,
                recovery_blocked: false,
                last_issue_code: Some("journal_trim".into()),
            }],
        }
    }

    #[test]
    fn background_projection_preserves_incomplete_evidence_and_error_precedence() {
        let owner = IndexedStartupOwner {
            attempts: BTreeMap::from([(
                "root-a".into(),
                IndexedAttempt {
                    generation: 1,
                    change_id: crate::domain::LibraryChangeId::new(1).unwrap(),
                    finished: true,
                },
            )]),
        };
        let mut healthy = snapshot();
        owner.project(&mut healthy);
        assert_eq!(
            healthy.roots[0].phase,
            LibrarySynchronizationPhase::BackgroundVerification
        );
        assert_eq!(healthy.roots[0].freshness, CatalogFreshnessState::Updating);
        assert_eq!(healthy.roots[0].pending_change_count, 80);
        assert_eq!(
            healthy.roots[0].continuity,
            PersistentJournalContinuityState::RecoveryRequired
        );
        let mut blocked = snapshot();
        blocked.roots[0].recovery_blocked = true;
        let mut unavailable = snapshot();
        unavailable.roots[0].availability = LibraryRootAvailability::Offline;
        let mut failed = snapshot();
        failed.roots[0].source_health = LibraryChangeSourceHealth::Failed;
        for mut state in [blocked, unavailable, failed] {
            let original = state.clone();
            owner.project(&mut state);
            assert_eq!(state, original);
        }
    }

    #[test]
    fn stale_or_unfinished_startup_cannot_quiet_a_new_root_binding() {
        for attempt in [(1, false), (2, true)] {
            let owner = IndexedStartupOwner {
                attempts: BTreeMap::from([(
                    "root-a".into(),
                    IndexedAttempt {
                        generation: attempt.0,
                        change_id: crate::domain::LibraryChangeId::new(1).unwrap(),
                        finished: attempt.1,
                    },
                )]),
            };
            let mut state = snapshot();
            let original = state.clone();
            owner.project(&mut state);
            assert_eq!(state, original);
        }
    }
}

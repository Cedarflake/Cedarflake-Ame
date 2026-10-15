use std::collections::BTreeMap;

use crate::adapters::SqliteCatalog;
use crate::application::JournalCaptureWindowReport;
use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, LibraryChangeSourceHealth,
    LibraryRootAvailability, LibraryRootGeneration, LibraryRootSynchronizationStatus,
    LibrarySynchronizationPhase, LibrarySynchronizationSnapshot, MetadataInventoryRunStatus,
    PersistentJournalBaselinePhase, PersistentJournalContinuityState, ScanError,
};
use crate::journal_broker::PersistentChangeJournalConnection;
use crate::ports::{MetadataInventoryRepository, PersistentJournalRepository};

#[derive(Default)]
pub(super) struct JournalStatusOwner {
    checked_roots: BTreeMap<String, LibraryRootGeneration>,
}

impl JournalStatusOwner {
    pub(super) fn record_window(&mut self, window: &JournalCaptureWindowReport) {
        self.checked_roots.extend(
            window
                .checked_roots()
                .map(|(root_id, generation)| (root_id.to_owned(), generation)),
        );
    }

    pub(super) fn project(
        &mut self,
        connection: &PersistentChangeJournalConnection,
        catalog: &SqliteCatalog,
        snapshot: &mut LibrarySynchronizationSnapshot,
    ) -> Result<(), ScanError> {
        self.checked_roots.retain(|root_id, generation| {
            snapshot.roots.iter().any(|status| {
                status.root_id == *root_id && status.root_generation == generation.value()
            })
        });
        let capabilities = catalog.load_persistent_journal_capabilities()?;
        let baselines = catalog.load_persistent_journal_baselines()?;
        for status in &mut snapshot.roots {
            if status.availability != LibraryRootAvailability::Available {
                status.continuity = PersistentJournalContinuityState::Unavailable;
                continue;
            }
            match connection {
                PersistentChangeJournalConnection::LiveOnly(_) => {
                    status.continuity = PersistentJournalContinuityState::LiveOnly;
                    continue;
                }
                PersistentChangeJournalConnection::Connected(_) => {
                    status.continuity = capabilities
                        .iter()
                        .find(|capability| {
                            capability.root_id == status.root_id
                                && capability.root_generation.value() == status.root_generation
                        })
                        .map_or(
                            PersistentJournalContinuityState::BaselineRequired,
                            |capability| capability.continuity,
                        );
                }
            }
            self.apply_session_progress(status);
            let Some(baseline) = baselines.iter().find(|baseline| {
                baseline.root_id == status.root_id
                    && baseline.root_generation.value() == status.root_generation
                    && baseline.phase != PersistentJournalBaselinePhase::Completed
            }) else {
                continue;
            };
            let Some(run) = catalog.load_metadata_inventory_run(&baseline.run_id)? else {
                continue;
            };
            if run.request.root_id != baseline.root_id
                || run.request.root_generation != baseline.root_generation
                || run.last_issue_code.is_some()
            {
                continue;
            }
            if let Some(phase) = inventory_phase(run.status) {
                apply_progress(status, &baseline.root_id, baseline.root_generation, phase);
            }
        }
        Ok(())
    }

    fn apply_session_progress(&self, status: &mut LibraryRootSynchronizationStatus) {
        if status.continuity == PersistentJournalContinuityState::Current
            && self
                .checked_roots
                .get(&status.root_id)
                .is_none_or(|generation| generation.value() != status.root_generation)
        {
            status.continuity = PersistentJournalContinuityState::CatchingUp;
        }
        if status.continuity == PersistentJournalContinuityState::CatchingUp
            && status.phase == LibrarySynchronizationPhase::Synchronized
            && let Some(generation) = LibraryRootGeneration::new(status.root_generation)
        {
            let root_id = status.root_id.clone();
            apply_progress(
                status,
                &root_id,
                generation,
                LibrarySynchronizationPhase::Reconciliation,
            );
        }
    }
}

fn inventory_phase(status: MetadataInventoryRunStatus) -> Option<LibrarySynchronizationPhase> {
    match status {
        MetadataInventoryRunStatus::Running => {
            Some(LibrarySynchronizationPhase::InventoryEnumeration)
        }
        MetadataInventoryRunStatus::Comparing => {
            Some(LibrarySynchronizationPhase::InventoryComparison)
        }
        MetadataInventoryRunStatus::Completed => {
            Some(LibrarySynchronizationPhase::QueuePublication)
        }
        MetadataInventoryRunStatus::Failed
        | MetadataInventoryRunStatus::Cancelled
        | MetadataInventoryRunStatus::Superseded => None,
    }
}

fn apply_progress(
    status: &mut LibraryRootSynchronizationStatus,
    root_id: &str,
    generation: LibraryRootGeneration,
    phase: LibrarySynchronizationPhase,
) -> bool {
    if status.root_id != root_id
        || status.root_generation != generation.value()
        || status.availability != LibraryRootAvailability::Available
        || status.source_health != LibraryChangeSourceHealth::Healthy
        || status.recovery_blocked
        || status.last_issue_code.is_some()
        || status.phase == LibrarySynchronizationPhase::FullScan
        || !matches!(
            status.continuity,
            PersistentJournalContinuityState::BaselineRequired
                | PersistentJournalContinuityState::CatchingUp
        )
    {
        return false;
    }
    status.freshness = CatalogFreshnessState::Updating;
    status.freshness_cause = CatalogFreshnessCause::PendingChanges;
    status.phase = phase;
    true
}

#[cfg(test)]
mod tests;

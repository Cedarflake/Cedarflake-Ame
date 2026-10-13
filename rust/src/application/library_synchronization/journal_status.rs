use crate::adapters::SqliteCatalog;
use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, LibraryChangeSourceHealth,
    LibraryRootAvailability, LibraryRootGeneration, LibraryRootSynchronizationStatus,
    LibrarySynchronizationPhase, LibrarySynchronizationSnapshot, MetadataInventoryRunStatus,
    PersistentJournalBaselinePhase, PersistentJournalContinuityState, ScanError,
};
use crate::journal_broker::PersistentChangeJournalConnection;
use crate::ports::{MetadataInventoryRepository, PersistentJournalRepository};

pub(super) fn project(
    connection: &PersistentChangeJournalConnection,
    catalog: &SqliteCatalog,
    snapshot: &mut LibrarySynchronizationSnapshot,
) -> Result<(), ScanError> {
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

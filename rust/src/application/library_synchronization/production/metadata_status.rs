use crate::adapters::SqliteCatalog;
use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, LibraryChangeSourceHealth,
    LibraryRootAvailability, LibraryRootSynchronizationStatus, LibrarySynchronizationPhase,
    LibrarySynchronizationSnapshot, ScanError,
};

pub(super) fn project_pending_metadata(
    catalog: &SqliteCatalog,
    snapshot: &mut LibrarySynchronizationSnapshot,
) -> Result<(), ScanError> {
    for root_id in catalog.roots_with_pending_metadata()? {
        if let Some(status) = snapshot
            .roots
            .iter_mut()
            .find(|status| status.root_id == root_id)
        {
            retain_metadata_progress(status);
        }
    }
    Ok(())
}

fn retain_metadata_progress(status: &mut LibraryRootSynchronizationStatus) {
    if status.freshness == CatalogFreshnessState::Synchronized
        && status.phase == LibrarySynchronizationPhase::Synchronized
        && status.availability == LibraryRootAvailability::Available
        && status.source_health == LibraryChangeSourceHealth::Healthy
        && !status.recovery_blocked
        && status.last_issue_code.is_none()
    {
        // Queue admission can yield to a writer. Pending metadata is still durable work
        // and must retain progress between the directory pass and the next admitted page.
        status.freshness = CatalogFreshnessState::Updating;
        status.freshness_cause = CatalogFreshnessCause::PendingChanges;
        status.phase = LibrarySynchronizationPhase::Reconciliation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LibraryChangeQueueHealth, PersistentJournalContinuityState};

    fn synchronized() -> LibraryRootSynchronizationStatus {
        LibraryRootSynchronizationStatus {
            root_id: "root-a".into(),
            root_generation: 1,
            availability: LibraryRootAvailability::Available,
            freshness: CatalogFreshnessState::Synchronized,
            freshness_cause: CatalogFreshnessCause::NoPendingChanges,
            continuity: PersistentJournalContinuityState::Current,
            phase: LibrarySynchronizationPhase::Synchronized,
            source_health: LibraryChangeSourceHealth::Healthy,
            queue_health: LibraryChangeQueueHealth::Idle,
            pending_change_count: 0,
            retry_wait_count: 0,
            freshness_unknown_count: 0,
            recovery_blocked: false,
            last_issue_code: None,
        }
    }

    #[test]
    fn pending_metadata_keeps_progress_before_admission_and_after_worker_retirement() {
        let mut status = synchronized();
        retain_metadata_progress(&mut status);
        assert_eq!(status.freshness, CatalogFreshnessState::Updating);
        assert_eq!(status.phase, LibrarySynchronizationPhase::Reconciliation);
    }

    #[test]
    fn metadata_progress_preserves_unavailability_and_failures() {
        let mut states = vec![synchronized(); 4];
        states[0].availability = LibraryRootAvailability::Offline;
        states[1].recovery_blocked = true;
        states[2].source_health = LibraryChangeSourceHealth::Failed;
        states[3].last_issue_code = Some("source_read_failed".into());
        for mut status in states {
            retain_metadata_progress(&mut status);
            assert_eq!(status.phase, LibrarySynchronizationPhase::Synchronized);
        }
    }
}

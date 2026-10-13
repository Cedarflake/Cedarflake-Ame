use super::*;
use crate::domain::LibraryChangeQueueHealth;

fn paused_between_pages() -> LibraryRootSynchronizationStatus {
    LibraryRootSynchronizationStatus {
        root_id: "root-a".to_owned(),
        root_generation: 1,
        availability: LibraryRootAvailability::Available,
        freshness: CatalogFreshnessState::NeedsReconciliation,
        freshness_cause: CatalogFreshnessCause::EvidenceGap,
        continuity: PersistentJournalContinuityState::BaselineRequired,
        phase: LibrarySynchronizationPhase::Blocked,
        source_health: LibraryChangeSourceHealth::Healthy,
        queue_health: LibraryChangeQueueHealth::Degraded,
        pending_change_count: 1,
        retry_wait_count: 0,
        freshness_unknown_count: 1,
        recovery_blocked: false,
        last_issue_code: None,
    }
}

#[test]
fn admitted_baseline_remains_updating_between_bounded_worker_pages() {
    let mut status = paused_between_pages();
    assert!(apply_progress(
        &mut status,
        "root-a",
        LibraryRootGeneration::initial(),
        LibrarySynchronizationPhase::InventoryEnumeration,
    ));
    assert_eq!(status.freshness, CatalogFreshnessState::Updating);
    assert_eq!(
        status.continuity,
        PersistentJournalContinuityState::BaselineRequired
    );
    assert_eq!(status.freshness_unknown_count, 1);
    assert!(!status.recovery_blocked);
}

#[test]
fn stale_or_failed_recovery_cannot_be_projected_as_active_progress() {
    let mut variants = vec![paused_between_pages(); 6];
    variants[0].root_generation = 2;
    variants[1].root_id = "root-b".to_owned();
    variants[2].source_health = LibraryChangeSourceHealth::Failed;
    variants[3].recovery_blocked = true;
    variants[4].last_issue_code = Some("metadata_inventory_enumeration_failed".to_owned());
    variants[5].availability = LibraryRootAvailability::Missing;
    for mut status in variants {
        let before = status.clone();
        assert!(!apply_progress(
            &mut status,
            "root-a",
            LibraryRootGeneration::initial(),
            LibrarySynchronizationPhase::InventoryEnumeration,
        ));
        assert_eq!(status, before);
    }
    for terminal in [
        MetadataInventoryRunStatus::Failed,
        MetadataInventoryRunStatus::Cancelled,
        MetadataInventoryRunStatus::Superseded,
    ] {
        assert_eq!(inventory_phase(terminal), None);
    }
}

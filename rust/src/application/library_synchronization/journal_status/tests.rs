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

fn retained_current(root_id: &str, generation: u64) -> LibraryRootSynchronizationStatus {
    LibraryRootSynchronizationStatus {
        root_id: root_id.to_owned(),
        root_generation: generation,
        availability: LibraryRootAvailability::Available,
        freshness: CatalogFreshnessState::Synchronized,
        freshness_cause: CatalogFreshnessCause::NoPendingChanges,
        continuity: PersistentJournalContinuityState::Current,
        phase: LibrarySynchronizationPhase::Synchronized,
        source_health: LibraryChangeSourceHealth::Healthy,
        queue_health: LibraryChangeQueueHealth::Healthy,
        pending_change_count: 0,
        retry_wait_count: 0,
        freshness_unknown_count: 0,
        recovery_blocked: false,
        last_issue_code: None,
    }
}

#[test]
fn retained_current_waits_for_this_runtime_to_check_each_root_generation() {
    let mut owner = JournalStatusOwner::default();
    let mut starting = retained_current("root-a", 1);
    owner.apply_session_progress(&mut starting);
    assert_eq!(starting.freshness, CatalogFreshnessState::Updating);
    assert_eq!(
        starting.continuity,
        PersistentJournalContinuityState::CatchingUp
    );
    assert_eq!(starting.phase, LibrarySynchronizationPhase::Reconciliation);
    assert_eq!(starting.pending_change_count, 0);
    assert_eq!(starting.freshness_unknown_count, 0);
    assert!(starting.last_issue_code.is_none());

    owner
        .checked_roots
        .insert("root-a".to_owned(), LibraryRootGeneration::initial());
    let mut checked = retained_current("root-a", 1);
    owner.apply_session_progress(&mut checked);
    assert_eq!(checked, retained_current("root-a", 1));
    for (root_id, generation) in [("root-b", 1), ("root-a", 2)] {
        let mut unverified = retained_current(root_id, generation);
        owner.apply_session_progress(&mut unverified);
        assert_eq!(unverified.freshness, CatalogFreshnessState::Updating);
        assert_eq!(
            unverified.continuity,
            PersistentJournalContinuityState::CatchingUp
        );
    }

    let restarted = JournalStatusOwner::default();
    restarted.apply_session_progress(&mut checked);
    assert_eq!(checked.freshness, CatalogFreshnessState::Updating);
}

#[test]
fn startup_check_preserves_unavailable_failed_and_active_work_feedback() {
    let owner = JournalStatusOwner::default();
    let mut variants = vec![retained_current("root-a", 1); 6];
    variants[0].availability = LibraryRootAvailability::Offline;
    variants[0].freshness = CatalogFreshnessState::Unavailable;
    variants[0].phase = LibrarySynchronizationPhase::Unavailable;
    variants[1].source_health = LibraryChangeSourceHealth::Failed;
    variants[1].freshness = CatalogFreshnessState::NeedsReconciliation;
    variants[1].phase = LibrarySynchronizationPhase::Blocked;
    variants[2].last_issue_code = Some("journal_read_failed".to_owned());
    variants[2].freshness = CatalogFreshnessState::NeedsReconciliation;
    variants[2].phase = LibrarySynchronizationPhase::Blocked;
    variants[3].recovery_blocked = true;
    variants[3].phase = LibrarySynchronizationPhase::Blocked;
    variants[4].freshness = CatalogFreshnessState::Updating;
    variants[4].phase = LibrarySynchronizationPhase::RetryWait;
    variants[4].retry_wait_count = 1;
    variants[5].freshness = CatalogFreshnessState::Updating;
    variants[5].phase = LibrarySynchronizationPhase::FullScan;
    for mut status in variants {
        let before = status.clone();
        owner.apply_session_progress(&mut status);
        assert_eq!(status.freshness, before.freshness);
        assert_eq!(status.phase, before.phase);
        assert_eq!(status.last_issue_code, before.last_issue_code);
        assert_eq!(status.recovery_blocked, before.recovery_blocked);
    }
    for continuity in [
        PersistentJournalContinuityState::LiveOnly,
        PersistentJournalContinuityState::RecoveryRequired,
        PersistentJournalContinuityState::Unavailable,
    ] {
        let mut status = retained_current("root-a", 1);
        status.continuity = continuity;
        let before = status.clone();
        owner.apply_session_progress(&mut status);
        assert_eq!(status, before);
    }
}

use super::*;
use crate::application::library_synchronization::production::journal_recovery_admission;
use crate::domain::{
    LibraryChangeQueuePolicy, PersistentJournalRootFailure, PersistentJournalRootFailureKind,
};

fn interrupt(fixture: &mut Fixture, root_id: &str, code: &str) {
    assert!(
        fixture
            .catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: root_id.to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: PersistentJournalRootFailureKind::NonRecoverable,
                    failure: PersistentJournalFailure {
                        code: code.to_owned(),
                        message: "Retained native lookup failure".to_owned(),
                    },
                    opening_boundary: None,
                },
                2,
                LibraryChangeQueuePolicy::default(),
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn retained_alias_failure_selects_one_fresh_probe_and_preserves_the_checkpoint() {
    assert_retained_failure_admission("identity_journal_hardlinks_not_supported");
}

#[test]
fn retained_registration_failure_selects_one_probe_without_clearing_evidence() {
    assert_retained_failure_admission("persistent_journal_request_invalid");
}

fn assert_retained_failure_admission(code: &str) {
    let mut fixture = Fixture::new();
    for index in 0..10 {
        let root = fixture.add_root(index, ("volume-a", 7, 44));
        interrupt(&mut fixture, &root, code);
    }
    let before = fixture
        .catalog
        .load_persistent_journal_checkpoint("root-000", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    let work = journal_recovery_admission::select(&fixture.catalog, &fixture.snapshot)
        .unwrap()
        .unwrap();
    assert_eq!(work.roots.len(), 1);
    assert_eq!(work.roots[0].checkpoint, before);
    assert_eq!(
        work.roots[0].root_path,
        PathBuf::from("C:/fixture/root-000")
    );
    assert_eq!(
        fixture
            .catalog
            .load_persistent_journal_checkpoint("root-000", LibraryRootGeneration::initial(),)
            .unwrap()
            .unwrap(),
        before
    );
    // The first probe's durable recovery work excludes it from further admission.
    fixture.snapshot.roots[0].pending_change_count = 1;
    let next = journal_recovery_admission::select(&fixture.catalog, &fixture.snapshot)
        .unwrap()
        .unwrap();
    assert_eq!(next.roots[0].checkpoint.root_id, "root-001");
}

#[test]
fn unavailable_stale_busy_and_other_failure_roots_do_not_enter_automatic_recovery() {
    let mut fixture = Fixture::new();
    for index in 0..7 {
        let root = fixture.add_root(index, ("volume-a", 7, 44));
        let code = if index == 0 {
            "protocol_binding_invalid"
        } else {
            "identity_journal_hardlinks_not_supported"
        };
        interrupt(&mut fixture, &root, code);
    }
    fixture.snapshot.roots[1].availability = LibraryRootAvailability::Offline;
    fixture.snapshot.roots[2].source_health = LibraryChangeSourceHealth::Failed;
    fixture.snapshot.roots[3].root_generation += 1;
    fixture.snapshot.roots[4].phase = LibrarySynchronizationPhase::FullScan;
    fixture.snapshot.roots[5].recovery_blocked = true;
    fixture.snapshot.roots.remove(6);
    assert!(
        journal_recovery_admission::select(&fixture.catalog, &fixture.snapshot)
            .unwrap()
            .is_none()
    );
    fixture.snapshot.roots[1].availability = LibraryRootAvailability::Available;
    let work = journal_recovery_admission::select(&fixture.catalog, &fixture.snapshot)
        .unwrap()
        .unwrap();
    assert_eq!(work.roots[0].checkpoint.root_id, "root-001");
}

#[test]
fn retained_failure_with_pending_or_delayed_work_waits_for_its_existing_owner() {
    for state in 0..3 {
        let mut fixture = Fixture::new();
        let root = fixture.add_root(0, ("volume-a", 7, 44));
        interrupt(
            &mut fixture,
            &root,
            "identity_journal_hardlinks_not_supported",
        );
        match state {
            0 => fixture.snapshot.roots[0].pending_change_count = 1,
            1 => fixture.snapshot.roots[0].retry_wait_count = 1,
            _ => fixture.snapshot.roots[0].freshness_unknown_count = 1,
        }
        assert!(
            journal_recovery_admission::select(&fixture.catalog, &fixture.snapshot)
                .unwrap()
                .is_none()
        );
    }
}

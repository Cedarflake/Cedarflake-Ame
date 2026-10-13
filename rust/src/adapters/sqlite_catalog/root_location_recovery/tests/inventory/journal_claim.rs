use super::*;
use crate::domain::{
    LibraryChangeFailure, LibraryChangeLeaseUpdateOutcome, PersistentJournalEnrollmentBatch,
    PersistentJournalRangeState, PersistentJournalSourceRange, PersistentJournalVolumeBatch,
    PersistentJournalVolumePage, persistent_journal_batch_id,
};

#[test]
fn root_location_recovery_retires_a_consumed_journal_gap_and_reopens() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    completed::seed_completed_baseline(&mut catalog, &fixture.root);
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    let now = unix_time_ms();
    catalog
        .enqueue_library_change_intents(
            &[recovery_gap(
                &fixture.root,
                fixture.root.root_generation,
                now,
            )],
            now,
            policy,
        )
        .unwrap();
    let lease = catalog
        .lease_live_authoritative_library_change(
            &fixture.root.root_id,
            fixture.root.root_generation,
            now,
            policy,
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                lease.change.id,
                lease.lease_generation,
                &LibraryChangeFailure {
                    code: "metadata_inventory_required".into(),
                    message: "Controlled gap".into()
                },
                now + 1,
                policy,
            )
            .unwrap(),
        LibraryChangeLeaseUpdateOutcome::Applied
    );
    let mut checkpoint = catalog
        .load_persistent_journal_checkpoint(&fixture.root.root_id, fixture.root.root_generation)
        .unwrap()
        .unwrap();
    let mut enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: String::new(),
            root_id: fixture.root.root_id.clone(),
            root_generation: fixture.root.root_generation,
            volume: checkpoint.volume.clone(),
            journal_id: checkpoint.journal_id,
            requested_start_usn: checkpoint.next_unread_usn,
            requested_end_usn: JournalUsn::new(21).unwrap(),
            covered_until_usn: JournalUsn::new(21).unwrap(),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: now + 2,
            checkpointed_unix_ms: None,
        },
        intents: Vec::new(),
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    enrollment.range.batch_id = persistent_journal_batch_id(&enrollment);
    checkpoint.next_unread_usn = enrollment.range.covered_until_usn;
    checkpoint.captured_exclusive_end = enrollment.range.requested_end_usn;
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    checkpoint.updated_unix_ms = now + 2;
    catalog
        .publish_persistent_journal_volume_batch(
            &PersistentJournalVolumeBatch {
                pages: vec![PersistentJournalVolumePage {
                    enrollment,
                    checkpoint,
                }],
            },
            now + 2,
            policy,
        )
        .unwrap();
    let claims: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM library_live_gap_recovery_claims WHERE consumer_kind = 'journal_source_range'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(claims, 1);
    drop(catalog);
    let mut catalog = fixture.catalog();
    catalog
        .recover_located_root(&fixture.root, &fixture.located(), &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let catalog = fixture.catalog();
    let (remaining_claims, retired_ranges): (i64, i64) = catalog.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_live_gap_recovery_claims WHERE root_id = ?1),
                (SELECT COUNT(*) FROM library_persistent_journal_source_ranges WHERE root_id = ?1 AND status = 'superseded')",
        [&fixture.root.root_id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(remaining_claims, 0);
    assert_eq!(retired_ranges, 1);
}

use super::*;
use crate::domain::LibrarySynchronizationPhase;
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRecoveryAuthorityReason,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaselineStartRequest,
    PersistentJournalCapability, PersistentJournalCapabilityState,
    PersistentJournalContinuityState, PersistentJournalVolumeIdentity, ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue, PersistentJournalRepository};

#[test]
fn recovery_retry_is_bounded_exponential_and_isolated_by_root() {
    let mut lane = RecoveryLane::default();

    lane.record_failure("root-a", 1_000);
    assert!(!lane.is_due("root-a", 1_999));
    assert!(lane.is_due("root-a", 2_000));
    assert!(lane.is_due("root-b", 1_000));
    for _ in 0..40 {
        lane.record_failure("root-a", 2_000);
    }
    let retry = lane.retries.get("root-a").expect("root retry state");
    assert_eq!(
        retry.next_attempt_unix_ms,
        2_000 + RECOVERY_RETRY_MAXIMUM_MILLIS
    );
}

#[test]
fn authoritative_root_cursor_prevents_a_busy_first_root_from_starving_peers() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let generation = crate::domain::LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut baseline_change_ids = BTreeMap::new();
    for (index, (root_id, root_path)) in [("root-a", "C:\\RootA"), ("root-b", "C:\\RootB")]
        .into_iter()
        .enumerate()
    {
        let scan_id = format!("scan-{root_id}");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: scan_id.clone(),
                    root_path: root_path.to_owned(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                root_id,
                root_path,
            )
            .expect("begin root scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove recovery cursor fixture first-import handoff");
        catalog
            .publish_scan(&scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: 1_000,
            })
            .expect("persist supported baseline capability");
        let identity_byte = u8::try_from(index + 1).expect("fixture identity byte");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &PersistentJournalBaselineStartRequest {
                    run_id: format!("round-robin-{root_id}"),
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    authority_reason: LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                    volume: PersistentJournalVolumeIdentity {
                        volume_guid: format!("round-robin-volume-{index}"),
                        volume_serial: u64::try_from(index + 1).expect("fixture volume serial"),
                    },
                    root_file_reference: JournalFileReference::V2([identity_byte; 8]),
                    journal_id: JournalIdentifier::new(44).expect("fixture journal ID"),
                    opening_next_usn: JournalUsn::new(20).expect("fixture opening USN"),
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    authorized_unix_ms: 1_000,
                },
                policy,
            )
            .expect("begin ready baseline recovery fixture");
        baseline_change_ids.insert(root_id, baseline.change_id);
    }
    catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: "root-a".to_owned(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                scope: crate::domain::LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 1_500,
                most_recent_observed_unix_ms: 1_500,
                first_sequence: 2,
                most_recent_sequence: 2,
                coalesced_observation_count: 1,
            }],
            1_500,
            policy,
        )
        .expect("enqueue competing root-a P0 gap");
    let competing = catalog
        .lease_live_authoritative_library_change("root-a", generation, 1_500, policy)
        .expect("lease competing root-a P0 gap")
        .expect("competing root-a P0 gap");
    catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            competing.change.id,
            competing.lease_generation,
            &crate::domain::LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "Fixture gap requires a distinct P2 owner".to_owned(),
            },
            1_500,
            policy,
        )
        .expect("promote competing root-a P2 owner");
    let root_a_baseline = baseline_change_ids["root-a"];
    rusqlite::Connection::open(catalog.catalog_path())
        .expect("open root-a affinity evidence")
        .execute(
            "UPDATE library_change_queue SET ready_unix_ms = 3000 WHERE id = ?1",
            [i64::try_from(root_a_baseline.value()).expect("root-a baseline ID")],
        )
        .expect("delay exact root-a baseline owner");
    let snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots: ["root-a", "root-b"]
            .into_iter()
            .map(|root_id| crate::domain::LibraryRootSynchronizationStatus {
                root_id: root_id.to_owned(),
                root_generation: generation.value(),
                availability: crate::domain::LibraryRootAvailability::Available,
                freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
                freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
                continuity: PersistentJournalContinuityState::RecoveryRequired,
                phase: LibrarySynchronizationPhase::Blocked,
                source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
                queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
                pending_change_count: 1,
                retry_wait_count: 0,
                freshness_unknown_count: 1,
                recovery_blocked: false,
                last_issue_code: None,
            })
            .collect(),
    };
    let mut lane = RecoveryLane::default();

    let first = lane
        .select(
            &catalog,
            &snapshot,
            2_000,
            LibraryChangeQueuePolicy::default(),
            &[],
        )
        .expect("skip unavailable exact root-a owner")
        .expect("root-b remains ready");
    assert_eq!(first.root_id(), "root-b");
    rusqlite::Connection::open(catalog.catalog_path())
        .expect("open root-a affinity release")
        .execute(
            "UPDATE library_change_queue SET ready_unix_ms = 2000 WHERE id = ?1",
            [i64::try_from(root_a_baseline.value()).expect("root-a baseline ID")],
        )
        .expect("release exact root-a baseline owner");
    let second = lane
        .select(
            &catalog,
            &snapshot,
            2_000,
            LibraryChangeQueuePolicy::default(),
            &[],
        )
        .expect("select released exact root-a owner")
        .expect("root-a becomes ready");
    assert_eq!(second.root_id(), "root-a");
    let leased_root_a = catalog
        .lease_metadata_inventory_recovery("root-a", generation, 2_000, policy)
        .expect("lease exact root-a baseline owner")
        .expect("exact root-a baseline owner");
    assert_eq!(leased_root_a.change.id, root_a_baseline);
    let wrapped = lane
        .select(
            &catalog,
            &snapshot,
            2_000,
            LibraryChangeQueuePolicy::default(),
            &[],
        )
        .expect("wrapped ready root")
        .expect("wrapped root");

    assert_eq!(wrapped.root_id(), "root-b");
}

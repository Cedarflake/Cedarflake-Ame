use std::collections::BTreeSet;

use super::*;
use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, JournalFileReference, JournalIdentifier,
    JournalUsn, LibraryChangeQueueHealth, LibraryRootSynchronizationStatus,
    LibrarySynchronizationPhase, PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalCapability,
    PersistentJournalFailure, PersistentJournalVolumeIdentity, ScanRequest,
};
use crate::ports::CatalogRepository;

mod recovery_admission;

struct Fixture {
    catalog: SqliteCatalog,
    snapshot: LibrarySynchronizationSnapshot,
    checkpoints: BTreeMap<String, PersistentJournalCheckpoint>,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("fixture directory");
        let catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("fixture catalog");
        Self {
            catalog,
            snapshot: LibrarySynchronizationSnapshot {
                is_running: true,
                catalog_revision: 0,
                applied_mutation_count: 0,
                roots: Vec::new(),
            },
            checkpoints: BTreeMap::new(),
            _directory: directory,
        }
    }

    fn add_root(&mut self, index: usize, scope: (&str, u64, u64)) -> String {
        let root_id = format!("root-{index:03}");
        let generation = LibraryRootGeneration::initial();
        let request = ScanRequest {
            scan_id: format!("scan-{root_id}"),
            root_path: format!("C:/fixture/{root_id}"),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        self.catalog
            .begin_scan(&request, &root_id, &request.root_path)
            .expect("register source");
        self.catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("first-import handoff");
        self.catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish source");
        self.save_capability(
            &root_id,
            PersistentJournalCapabilityState::Supported,
            PersistentJournalContinuityState::Current,
        );
        let marker = u64::try_from(index + 1).expect("root identity");
        let next_usn = 100 + i64::try_from(index).expect("checkpoint position");
        let checkpoint = PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: scope.0.to_owned(),
                volume_serial: scope.1,
            },
            root_file_reference: JournalFileReference::V2(marker.to_le_bytes()),
            journal_id: JournalIdentifier::new(scope.2).expect("journal ID"),
            next_unread_usn: JournalUsn::new(next_usn).expect("next USN"),
            captured_exclusive_end: JournalUsn::new(next_usn + 37).expect("captured end"),
            covered_catalog_revision: 1,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::CatchingUp,
            failure: None,
            updated_unix_ms: 1,
        };
        self.save_checkpoint(checkpoint);
        self.snapshot.roots.push(LibraryRootSynchronizationStatus {
            root_id: root_id.clone(),
            root_generation: generation.value(),
            availability: LibraryRootAvailability::Available,
            freshness: CatalogFreshnessState::Updating,
            freshness_cause: CatalogFreshnessCause::PendingChanges,
            continuity: PersistentJournalContinuityState::CatchingUp,
            phase: LibrarySynchronizationPhase::Reconciliation,
            source_health: LibraryChangeSourceHealth::Healthy,
            queue_health: LibraryChangeQueueHealth::Healthy,
            pending_change_count: 0,
            retry_wait_count: 0,
            freshness_unknown_count: 0,
            recovery_blocked: false,
            last_issue_code: None,
        });
        root_id
    }

    fn save_capability(
        &mut self,
        root_id: &str,
        state: PersistentJournalCapabilityState,
        continuity: PersistentJournalContinuityState,
    ) {
        self.catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state,
                continuity,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("source capability");
    }

    fn save_checkpoint(&mut self, checkpoint: PersistentJournalCheckpoint) {
        self.catalog
            .seed_persistent_journal_checkpoint_for_test(&checkpoint)
            .expect("retained source progress");
        self.checkpoints
            .insert(checkpoint.root_id.clone(), checkpoint);
    }

    fn round(&self, cursor: Option<&str>) -> Vec<JournalVolumeWork> {
        let mut cursor = cursor.map(str::to_owned);
        let mut keys = BTreeSet::new();
        let mut batches = Vec::<JournalVolumeWork>::new();
        for _ in 0..=self.snapshot.roots.len() {
            let Some(work) = select(&self.catalog, &self.snapshot, cursor.as_deref())
                .expect("select journal batch")
            else {
                assert!(batches.is_empty());
                return batches;
            };
            if !keys.insert(work.key.clone()) {
                assert_eq!(
                    work.key, batches[0].key,
                    "rotation returns to its first batch"
                );
                return batches;
            }
            cursor = Some(work.key.clone());
            batches.push(work);
        }
        panic!("journal rotation did not close within the source-count bound");
    }

    fn assert_membership(&self, batches: &[JournalVolumeWork], expected_ids: &[String]) {
        let mut seen = BTreeSet::new();
        for batch in batches {
            assert!(
                (1..=8).contains(&batch.roots.len()),
                "{} exceeds reader capacity",
                batch.key
            );
            let first = &batch.roots[0].checkpoint;
            for root in &batch.roots {
                let checkpoint = &root.checkpoint;
                assert!(
                    seen.insert(checkpoint.root_id.clone()),
                    "source selected twice"
                );
                assert_eq!(checkpoint.volume, first.volume);
                assert_eq!(checkpoint.journal_id, first.journal_id);
                assert_eq!(checkpoint, &self.checkpoints[&checkpoint.root_id]);
                assert_eq!(
                    root.root_path,
                    PathBuf::from(format!("C:/fixture/{}", checkpoint.root_id))
                );
            }
        }
        assert_eq!(seen, expected_ids.iter().cloned().collect::<BTreeSet<_>>());
        for checkpoint in self.checkpoints.values() {
            let retained = self
                .catalog
                .load_persistent_journal_checkpoint(&checkpoint.root_id, checkpoint.root_generation)
                .expect("reload retained progress")
                .expect("retained checkpoint");
            assert_eq!(&retained, checkpoint, "selection must not rewrite progress");
        }
    }
}

#[test]
fn batching_preserves_capacity_and_membership_across_decimal_boundaries() {
    for (root_count, mut expected_sizes) in [
        (0, vec![]),
        (8, vec![8]),
        (9, vec![8, 1]),
        (80, vec![8; 10]),
        (88, vec![8; 11]),
        (89, vec![8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 1]),
        (100, vec![8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 4]),
    ] {
        let mut fixture = Fixture::new();
        let expected_ids = (0..root_count)
            .map(|index| fixture.add_root(index, ("volume-a", 7, 44)))
            .collect::<Vec<_>>();
        let batches = fixture.round(None);
        fixture.assert_membership(&batches, &expected_ids);
        let mut sizes = batches
            .iter()
            .map(|batch| batch.roots.len())
            .collect::<Vec<_>>();
        sizes.sort_unstable();
        expected_sizes.sort_unstable();
        assert_eq!(sizes, expected_sizes, "{root_count} sources");
        if root_count == 88 {
            let keys = batches
                .iter()
                .take(4)
                .map(|batch| batch.key.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                keys,
                [
                    "volume-a|7|44|0",
                    "volume-a|7|44|1",
                    "volume-a|7|44|10",
                    "volume-a|7|44|2"
                ]
            );
        }
    }
}

#[test]
fn interleaved_volumes_and_journals_have_independent_batch_capacity() {
    let mut fixture = Fixture::new();
    let scopes = [
        ("volume-a", 7, 44),
        ("volume-b", 7, 44),
        ("volume-a", 8, 44),
        ("volume-a", 7, 45),
    ];
    let expected_ids = (0..36)
        .map(|index| fixture.add_root(index, scopes[index % scopes.len()]))
        .collect::<Vec<_>>();
    let batches = fixture.round(None);
    fixture.assert_membership(&batches, &expected_ids);
    assert_eq!(batches.len(), 8);
    for (guid, serial, journal) in scopes {
        for (chunk, expected_size) in [(0, 8), (1, 1)] {
            let key = format!("{guid}|{serial}|{journal}|{chunk}");
            let batch = batches
                .iter()
                .find(|batch| batch.key == key)
                .expect("volume batch");
            assert_eq!(batch.roots.len(), expected_size);
        }
    }
}

#[test]
fn filtered_sources_do_not_consume_batch_capacity() {
    let mut fixture = Fixture::new();
    let ids = (0..16)
        .map(|index| fixture.add_root(index, ("volume-a", 7, 44)))
        .collect::<Vec<_>>();
    fixture.snapshot.roots[1].availability = LibraryRootAvailability::Offline;
    fixture.snapshot.roots[2].source_health = LibraryChangeSourceHealth::Failed;
    fixture
        .snapshot
        .roots
        .retain(|status| status.root_id != ids[0]);
    fixture.save_capability(
        &ids[3],
        PersistentJournalCapabilityState::LiveOnly,
        PersistentJournalContinuityState::LiveOnly,
    );
    fixture.save_capability(
        &ids[4],
        PersistentJournalCapabilityState::LiveOnly,
        PersistentJournalContinuityState::LiveOnly,
    );
    fixture.save_capability(
        &ids[4],
        PersistentJournalCapabilityState::Supported,
        PersistentJournalContinuityState::RecoveryRequired,
    );
    let mut recovery = fixture.checkpoints[&ids[5]].clone();
    recovery.continuity = PersistentJournalContinuityState::RecoveryRequired;
    fixture.save_checkpoint(recovery);
    let mut failed = fixture.checkpoints[&ids[6]].clone();
    failed.failure = Some(PersistentJournalFailure {
        code: "fixture_read_failure".to_owned(),
        message: "Retained read failure".to_owned(),
    });
    fixture.save_checkpoint(failed);

    let batches = fixture.round(None);
    fixture.assert_membership(&batches, &ids[7..]);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].roots.len(), 8);
    assert_eq!(batches[1].roots.len(), 1);
}

#[test]
fn source_addition_removal_and_progress_advance_preserve_surviving_checkpoints() {
    let mut fixture = Fixture::new();
    let mut expected_ids = (0..88)
        .map(|index| fixture.add_root(index, ("volume-a", 7, 44)))
        .collect::<Vec<_>>();
    let initial = fixture.round(None);
    fixture.assert_membership(&initial, &expected_ids);

    expected_ids.push(fixture.add_root(88, ("volume-a", 7, 44)));
    let expanded = fixture.round(Some("volume-a|7|44|10"));
    fixture.assert_membership(&expanded, &expected_ids);
    let tail = expanded
        .iter()
        .find(|batch| batch.key == "volume-a|7|44|11")
        .expect("new partial batch");
    assert_eq!(tail.roots.len(), 1);
    assert_eq!(tail.roots[0].checkpoint.root_id, "root-088");

    let mut advanced = fixture.checkpoints["root-088"].clone();
    advanced.next_unread_usn = advanced.captured_exclusive_end;
    advanced.continuity = PersistentJournalContinuityState::Current;
    advanced.updated_unix_ms = 2;
    fixture.save_checkpoint(advanced);
    fixture.save_capability(
        "root-088",
        PersistentJournalCapabilityState::Supported,
        PersistentJournalContinuityState::Current,
    );
    assert!(
        fixture
            .catalog
            .unregister_root("root-010")
            .expect("remove source")
    );
    fixture
        .snapshot
        .roots
        .retain(|status| status.root_id != "root-010");
    fixture.checkpoints.remove("root-010");
    expected_ids.retain(|root_id| root_id != "root-010");

    let reduced = fixture.round(Some("volume-a|7|44|11"));
    fixture.assert_membership(&reduced, &expected_ids);
    assert_eq!(reduced.len(), 11);
    assert_eq!(
        reduced[0].key, "volume-a|7|44|0",
        "removed cursor restarts rotation"
    );
    assert!(reduced.iter().all(|batch| batch.roots.len() == 8));

    expected_ids.push(fixture.add_root(89, ("volume-a", 7, 44)));
    let reexpanded = fixture.round(Some("volume-a|7|44|10"));
    fixture.assert_membership(&reexpanded, &expected_ids);
    assert_eq!(reexpanded.len(), 12);
    let continued = reexpanded
        .iter()
        .flat_map(|batch| &batch.roots)
        .find(|root| root.checkpoint.root_id == "root-088")
        .expect("surviving source with advanced progress");
    assert_eq!(continued.checkpoint.next_unread_usn.value(), 225);
}

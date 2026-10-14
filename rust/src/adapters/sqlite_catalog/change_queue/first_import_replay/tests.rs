use tempfile::{TempDir, tempdir};

use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeCatchUpEvidence,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthorityReason,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselineStartRequest,
    PersistentJournalVolumeIdentity, ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue, PersistentJournalRepository};

use super::*;

const ROOT: &str = "replay-root";
const NOW: i64 = 5_000;

struct Fixture {
    catalog: SqliteCatalog,
    _directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("isolated catalog");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
        let request = ScanRequest {
            scan_id: "first-import-replay".to_owned(),
            root_path: "C:/first-import-replay".to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        catalog
            .begin_scan(&request, ROOT, &request.root_path)
            .expect("start import");
        let volume = PersistentJournalVolumeIdentity {
            volume_guid: "replay-volume".to_owned(),
            volume_serial: 7,
        };
        let reference = JournalFileReference::V2([1; 8]);
        let journal = JournalIdentifier::new(9).expect("journal ID");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &PersistentJournalBaselineStartRequest {
                    run_id: request.scan_id.clone(),
                    root_id: ROOT.to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    authority_reason: LibraryRecoveryAuthorityReason::FirstImportBoundary,
                    volume: volume.clone(),
                    root_file_reference: reference.clone(),
                    journal_id: journal,
                    opening_next_usn: JournalUsn::new(10).expect("opening USN"),
                    protocol_version: 1,
                    contract_version: 1,
                    authorized_unix_ms: 1_000,
                },
                LibraryChangeQueuePolicy::default(),
            )
            .expect("opening boundary");
        catalog
            .publish_scan(&request.scan_id, ROOT, 0, 0)
            .expect("publish inventory");
        catalog
            .capture_persistent_journal_baseline_closing_boundary(
                &PersistentJournalBaselineClosingBoundary {
                    change_id: baseline.change_id,
                    volume,
                    root_file_reference: reference,
                    journal_id: journal,
                    closing_next_usn: JournalUsn::new(10).expect("closing USN"),
                    protocol_version: 1,
                    captured_unix_ms: 2_000,
                },
            )
            .expect("closing boundary");
        Self {
            catalog,
            _directory: directory,
        }
    }

    fn enqueue(&mut self, scope: LibraryChangeScope, journal_lineage: bool) {
        let intent = LibraryChangeIntent {
            root_id: ROOT.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: LibraryChangeIntentKind::Reconcile,
            scope,
            relative_path: if scope == LibraryChangeScope::Path {
                "moved/photo.png".to_owned()
            } else {
                String::new()
            },
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
            first_observed_unix_ms: 2_500,
            most_recent_observed_unix_ms: 2_500,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        };
        let policy = LibraryChangeQueuePolicy::default();
        if journal_lineage {
            self.catalog
                .enqueue_library_change_intents_with_catch_up(
                    &[intent],
                    &LibraryChangeCatchUpEvidence {
                        source: "persistent_journal_v1".to_owned(),
                        watermark: "replay-range".to_owned(),
                    },
                    2_500,
                    policy,
                )
                .expect("journal-owned live work");
        } else {
            self.catalog
                .enqueue_library_change_intents(&[intent], 2_500, policy)
                .expect("unlineaged live work");
        }
    }

    fn cover_closing(&mut self) {
        let checkpoint = self
            .catalog
            .load_persistent_journal_checkpoint(ROOT, LibraryRootGeneration::initial())
            .expect("checkpoint")
            .expect("opening checkpoint");
        assert!(
            self.catalog
                .record_persistent_journal_empty_replay(&checkpoint, 3_000)
                .expect("read through closing boundary")
        );
    }
}

#[test]
fn live_removals_wait_for_replay_without_blocking_capture_or_baseline_completion() {
    let mut fixture = Fixture::new();
    fixture.enqueue(LibraryChangeScope::Path, false);
    let policy = LibraryChangeQueuePolicy::default();
    let generation = LibraryRootGeneration::initial();
    assert!(
        !fixture
            .catalog
            .has_ready_live_path_library_change(ROOT, generation, NOW, policy)
            .expect("deferred readiness")
    );
    assert!(
        !fixture
            .catalog
            .journal_capture_should_yield(NOW, policy)
            .expect("replay can run")
    );
    assert!(
        fixture
            .catalog
            .lease_path_library_changes(ROOT, generation, NOW, policy)
            .expect("path leases respect replay")
            .is_empty()
    );
    let all = fixture
        .catalog
        .lease_library_changes(ROOT, generation, NOW, policy)
        .expect("general leases respect replay");
    assert!(
        all.iter()
            .all(|lease| lease.change.intent.origin != LibraryChangeOrigin::LiveNotification)
    );
    fixture.cover_closing();
    assert!(
        fixture
            .catalog
            .has_ready_live_path_library_change(ROOT, generation, NOW, policy)
            .expect("ready before baseline completion")
    );
    assert!(
        fixture
            .catalog
            .journal_capture_should_yield(NOW, policy)
            .expect("live priority resumes")
    );
    let leases = fixture
        .catalog
        .lease_path_library_changes_in_lane(ROOT, generation, LibraryChangeLane::Live, NOW, policy)
        .expect("live cleanup can finish the baseline");
    assert_eq!(leases.len(), 1);
}

#[test]
fn journal_owned_live_work_remains_runnable_during_first_import_replay() {
    let mut fixture = Fixture::new();
    fixture.enqueue(LibraryChangeScope::Path, true);
    let policy = LibraryChangeQueuePolicy::default();
    let generation = LibraryRootGeneration::initial();
    assert!(
        fixture
            .catalog
            .has_ready_live_path_library_change(ROOT, generation, NOW, policy)
            .expect("journal handoff protects identity")
    );
    assert!(
        fixture
            .catalog
            .journal_capture_should_yield(NOW, policy)
            .expect("enrolled work has priority")
    );
    assert_eq!(
        fixture
            .catalog
            .lease_path_library_changes_in_lane(
                ROOT,
                generation,
                LibraryChangeLane::Live,
                NOW,
                policy,
            )
            .expect("lease enrolled live work")
            .len(),
        1
    );
}

#[test]
fn root_reconciliation_and_other_generations_keep_the_same_replay_boundary() {
    let mut fixture = Fixture::new();
    fixture.enqueue(LibraryChangeScope::Root, false);
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy::default();
    assert!(
        !fixture
            .catalog
            .has_ready_live_authoritative_library_change(ROOT, generation, NOW, policy)
            .expect("root work waits for replay")
    );
    assert!(
        fixture
            .catalog
            .lease_live_authoritative_library_change(ROOT, generation, NOW, policy)
            .expect("root lease waits")
            .is_none()
    );
    assert!(
        !blocks_unlineaged_live_changes(&fixture.catalog.connection, "other-root", generation)
            .expect("other root unaffected")
    );
    assert!(
        !blocks_unlineaged_live_changes(
            &fixture.catalog.connection,
            ROOT,
            generation.next().unwrap()
        )
        .expect("other generation unaffected")
    );
    fixture.cover_closing();
    assert!(
        fixture
            .catalog
            .has_ready_live_authoritative_library_change(ROOT, generation, NOW, policy)
            .expect("root work resumes")
    );
}

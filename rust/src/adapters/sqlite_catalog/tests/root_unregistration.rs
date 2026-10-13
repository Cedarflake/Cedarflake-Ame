use std::fs;

use rusqlite::params;
use tempfile::{TempDir, tempdir};

use crate::adapters::sqlite_catalog::{SqliteCatalog, change_queue};
use crate::domain::{
    LeasedLibraryChange, LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeLeaseUpdateOutcome, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRecoveryAuthority, LibraryRecoveryAuthorityReason,
    LibraryRootGeneration, MetadataInventoryFrontierEntry, MetadataInventoryPage,
    MetadataInventoryRunStatus, MetadataInventoryScope, MetadataInventoryStartRequest, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue,
    MetadataInventoryRepository,
};

mod compatibility;

#[derive(Clone, Copy, Debug)]
enum InventoryPhase {
    Authorized,
    Running,
    Comparing,
}

#[test]
fn root_unregistration_retires_authority_in_each_inventory_phase() {
    for phase in [
        InventoryPhase::Authorized,
        InventoryPhase::Running,
        InventoryPhase::Comparing,
    ] {
        let mut fixture = Fixture::new(&["removed"]);
        let leased = fixture.authorize("removed", "removed-inventory", 2_000);
        fixture.advance("removed-inventory", "removed", phase);
        let before = fixture.authority(leased.change.id);
        fixture.reopen();

        assert!(
            fixture
                .catalog
                .unregister_root("removed")
                .expect("remove registered root")
        );
        let retired = fixture.authority(leased.change.id);
        let retired_at = retired.retired_unix_ms.expect("removed authority retired");
        assert!(retired_at >= before.authorized_unix_ms);
        assert_eq!(
            retired,
            LibraryRecoveryAuthority {
                retired_unix_ms: Some(retired_at),
                ..before
            }
        );
        assert_eq!(fixture.queue_status(leased.change.id), "superseded");
        assert!(
            fixture
                .catalog
                .load_metadata_inventory_run("removed-inventory")
                .expect("load removed inventory")
                .is_none()
        );
        fixture.reopen();
        assert!(
            fixture
                .catalog
                .load_incremental_catalog_root("removed")
                .expect("load removed root")
                .is_none()
        );
        assert_eq!(fixture.authority(leased.change.id), retired);
        assert!(
            !fixture
                .catalog
                .unregister_root("removed")
                .expect("idempotent removal")
        );
    }
}

#[test]
fn root_unregistration_preserves_other_roots_and_completed_authority_history() {
    let mut fixture = Fixture::new(&["removed", "retained"]);
    let history = fixture.authorize("removed", "completed-history", 1_000);
    let revision = fixture
        .catalog
        .load_incremental_catalog_root("removed")
        .expect("load root")
        .expect("registered root")
        .catalog_revision;
    assert_eq!(
        fixture
            .catalog
            .complete_library_change(history.change.id, history.lease_generation, revision, 1_100,)
            .expect("complete historical queue lease"),
        LibraryChangeLeaseUpdateOutcome::Applied
    );
    let historical_authority = fixture
        .catalog
        .retire_metadata_inventory_recovery_authority(history.change.id, 1_100)
        .expect("retire historical authority");
    let active = fixture.authorize("removed", "active-removed", 2_000);
    fixture.advance("active-removed", "removed", InventoryPhase::Comparing);
    let peer = fixture.authorize("retained", "active-retained", 2_000);
    fixture.advance("active-retained", "retained", InventoryPhase::Running);
    let peer_authority = fixture.authority(peer.change.id);
    let peer_run = fixture
        .catalog
        .load_metadata_inventory_run("active-retained")
        .expect("load retained run");
    fixture.reopen();

    assert!(
        fixture
            .catalog
            .unregister_root("removed")
            .expect("remove only selected root")
    );

    assert!(
        fixture
            .authority(active.change.id)
            .retired_unix_ms
            .is_some()
    );
    assert_eq!(fixture.authority(history.change.id), historical_authority);
    assert_eq!(fixture.queue_status(history.change.id), "completed");
    assert_eq!(fixture.authority(peer.change.id), peer_authority);
    assert_eq!(fixture.queue_status(peer.change.id), "leased");
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run("active-retained")
            .expect("retained inventory after peer removal"),
        peer_run
    );
    fixture.reopen();
}

#[test]
fn root_unregistration_retirement_cannot_precede_future_authorization() {
    let mut fixture = Fixture::new(&["removed"]);
    let authorized_at = 4_000_000_000_000;
    let leased = fixture.authorize("removed", "future-authority", authorized_at);
    fixture.reopen();

    assert!(
        fixture
            .catalog
            .unregister_root("removed")
            .expect("remove after clock rollback")
    );

    assert_eq!(
        fixture.authority(leased.change.id).retired_unix_ms,
        Some(authorized_at)
    );
    fixture.reopen();
}

#[test]
fn root_unregistration_authority_write_failure_rolls_back_the_entire_removal() {
    let mut fixture = Fixture::new(&["removed"]);
    let leased = fixture.authorize("removed", "rollback-inventory", 2_000);
    fixture.advance("rollback-inventory", "removed", InventoryPhase::Comparing);
    let before = fixture.authority(leased.change.id);
    let root = fixture
        .catalog
        .load_incremental_catalog_root("removed")
        .expect("root snapshot");
    let run = fixture
        .catalog
        .load_metadata_inventory_run("rollback-inventory")
        .expect("run snapshot");
    fixture.reopen();
    fixture
        .catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER abort_removed_authority
         BEFORE UPDATE OF retired_unix_ms ON library_recovery_authorities
         WHEN NEW.root_id = 'removed' AND NEW.retired_unix_ms IS NOT NULL
           AND NOT EXISTS(SELECT 1 FROM library_roots WHERE id = NEW.root_id)
         BEGIN SELECT RAISE(ABORT, 'injected removal authority failure'); END;",
        )
        .expect("inject failure after root and run deletion");

    let error = fixture
        .catalog
        .unregister_root("removed")
        .expect_err("retirement must fail");

    assert!(error.message.contains("injected removal authority failure"));
    assert!(fixture.catalog.connection.is_autocommit());
    assert_eq!(fixture.authority(leased.change.id), before);
    assert_eq!(fixture.queue_status(leased.change.id), "leased");
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_root("removed")
            .expect("restored root"),
        root
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run("rollback-inventory")
            .expect("restored run"),
        run
    );
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER abort_removed_authority;")
        .expect("remove injected trigger");
    fixture.reopen();
}

#[test]
fn registered_generation_retirement_does_not_revoke_inventory_authority() {
    let mut fixture = Fixture::new(&["retained"]);
    let leased = fixture.authorize("retained", "registered-inventory", 2_000);
    fixture.advance(
        "registered-inventory",
        "retained",
        InventoryPhase::Comparing,
    );
    let before = fixture.authority(leased.change.id);
    fixture.reopen();
    let transaction = fixture
        .catalog
        .connection
        .transaction()
        .expect("generation transaction");

    change_queue::retire_root_change_queue(&transaction, "retained", 3_000)
        .expect("shared generation retirement");
    change_queue::root_retirement::retire_removed_root_recovery_authorities(
        &transaction,
        "retained",
    )
    .expect("registered root is not removal proof");

    let retired: Option<i64> = transaction
        .query_row(
            "SELECT retired_unix_ms FROM library_recovery_authorities WHERE run_id = ?1",
            ["registered-inventory"],
            |row| row.get(0),
        )
        .expect("authority during generation retirement");
    assert_eq!(retired, None);
    transaction.rollback().expect("restore generation fixture");
    assert_eq!(fixture.authority(leased.change.id), before);
    fixture.reopen();
}

struct Fixture {
    source: TempDir,
    _storage: TempDir,
    catalog: SqliteCatalog,
}

impl Fixture {
    fn new(roots: &[&str]) -> Self {
        let source = tempdir().expect("empty generated roots");
        let storage = tempdir().expect("isolated catalog");
        let catalog =
            SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("fresh catalog");
        let mut fixture = Self {
            source,
            _storage: storage,
            catalog,
        };
        for root_id in roots {
            let path = fixture.source.path().join(root_id);
            fs::create_dir(&path).expect("empty root directory");
            let request = ScanRequest {
                scan_id: format!("baseline-{root_id}"),
                root_path: path.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            };
            fixture
                .catalog
                .begin_scan(&request, root_id, &request.root_path)
                .expect("begin baseline");
            fixture
                .catalog
                .prove_live_only_first_import_handoff_for_test(&request.scan_id)
                .expect("establish live-only first-import handoff");
            fixture
                .catalog
                .publish_scan(&request.scan_id, root_id, 0, 0)
                .expect("publish empty baseline");
        }
        fixture
    }

    fn authorize(&mut self, root_id: &str, run_id: &str, now: i64) -> LeasedLibraryChange {
        let policy = LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..LibraryChangeQueuePolicy::default()
        };
        let generation = LibraryRootGeneration::initial();
        self.catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    kind: LibraryChangeIntentKind::FreshnessUnknown,
                    scope: LibraryChangeScope::Root,
                    relative_path: String::new(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: now,
                    most_recent_observed_unix_ms: now,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                now,
                policy,
            )
            .expect("enqueue actual P2 recovery");
        let leased = self
            .catalog
            .lease_authoritative_library_change(root_id, generation, now, policy)
            .expect("lease P2 recovery")
            .expect("P2 recovery available");
        self.catalog
            .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
                change_id: leased.change.id,
                run_id: run_id.to_owned(),
                root_id: root_id.to_owned(),
                root_generation: generation,
                reason: LibraryRecoveryAuthorityReason::ContainmentFailure,
                opening_boundary: None,
                authorized_unix_ms: now,
                retired_unix_ms: None,
            })
            .expect("persist legitimate P2 authority");
        leased
    }

    fn advance(&mut self, run_id: &str, root_id: &str, phase: InventoryPhase) {
        if matches!(phase, InventoryPhase::Authorized) {
            assert!(
                self.catalog
                    .load_metadata_inventory_run(run_id)
                    .expect("no inventory yet")
                    .is_none()
            );
            return;
        }
        let run = self
            .catalog
            .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
                run_id: run_id.to_owned(),
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                scope: MetadataInventoryScope::Root,
                started_unix_ms: 2_000,
            })
            .expect("begin owned inventory");
        assert_eq!(run.status, MetadataInventoryRunStatus::Running);
        if matches!(phase, InventoryPhase::Comparing) {
            let run = self
                .catalog
                .stage_metadata_inventory_page(
                    run_id,
                    &MetadataInventoryPage {
                        page_index: 1,
                        entries: Vec::new(),
                        cursor: None,
                        is_complete: true,
                        frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
                    },
                    2_100,
                )
                .expect("finish empty root enumeration");
            assert_eq!(run.status, MetadataInventoryRunStatus::Comparing);
        }
    }

    fn authority(&self, change_id: LibraryChangeId) -> LibraryRecoveryAuthority {
        self.catalog
            .load_metadata_inventory_recovery_authority(change_id)
            .expect("load recovery authority")
            .expect("retained authority history")
    }

    fn queue_status(&self, change_id: LibraryChangeId) -> String {
        self.catalog
            .connection
            .query_row(
                "SELECT status FROM library_change_queue WHERE id = ?1",
                [i64::try_from(change_id.value()).expect("SQLite change id")],
                |row| row.get(0),
            )
            .expect("queue state")
    }

    fn reopen(&mut self) {
        self.catalog = SqliteCatalog::open(self.catalog.catalog_path().to_path_buf())
            .expect("full catalog reopen, including all authority row validators");
    }

    fn emulate_legacy_unregistration(&mut self) -> LibraryChangeId {
        let leased = self.authorize("removed", "legacy-removed-inventory", 2_000);
        self.advance(
            "legacy-removed-inventory",
            "removed",
            InventoryPhase::Comparing,
        );
        self.reopen();
        assert!(
            self.catalog
                .unregister_root("removed")
                .expect("real root removal")
        );
        assert!(self.authority(leased.change.id).retired_unix_ms.is_some());
        // The old implementation performed the same removal but omitted this single update.
        self.catalog.connection.execute(
            "UPDATE library_recovery_authorities SET retired_unix_ms = NULL WHERE change_id = ?1",
            params![i64::try_from(leased.change.id.value()).expect("SQLite change id")],
        ).expect("reconstruct exact prerelease output without replacing the removal workflow");
        assert_eq!(self.authority(leased.change.id).retired_unix_ms, None);
        leased.change.id
    }
}

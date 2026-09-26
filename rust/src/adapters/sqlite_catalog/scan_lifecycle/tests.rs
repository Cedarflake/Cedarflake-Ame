use rusqlite::types::Value;

use crate::domain::{
    AssetLocationView, FileIdentityEvidence, LibraryChangeCatchUpEvidence, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration, PreviewStatus, ScanCheckpoint, ScanRequest, SourceRevisionEvidence,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue};

use super::super::SqliteCatalog;

#[test]
fn cancelling_unpublished_foreground_claim_restores_consumer_before_root_retirement() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let root_id = "unpublished-explicit-claim";
    let root_path = "C:\\SyntheticUnpublishedExplicitClaim";
    super::super::tests::migrate_v29_ambiguous_gap_fixture(&path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("migrated derived fixture");
    let request = ScanRequest {
        scan_id: "first-import-claimed-gap".to_owned(),
        root_path: root_path.to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("claim pending explicit recovery");
    let consumer: String = catalog
        .connection
        .query_row(
            "SELECT consumer_kind FROM library_live_gap_recovery_claims WHERE root_id = ?1",
            [root_id],
            |row| row.get(0),
        )
        .expect("consumer");
    assert_eq!(consumer, "foreground_scan");
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("restore then retire in one transaction");
    let evidence: (String, i64, i64) = catalog.connection.query_row(
        "SELECT scan.status, state.is_active, (SELECT COUNT(*) FROM library_live_gap_recovery_claims WHERE root_id = ?1)
         FROM scan_runs AS scan JOIN library_change_root_state AS state ON state.root_id = scan.root_id
         WHERE scan.id = ?2", rusqlite::params![root_id, request.scan_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).expect("retirement evidence");
    assert_eq!(evidence, ("cancelled".to_owned(), 0, 0));
    drop(catalog);
    SqliteCatalog::open(path).expect("full schema authority remains verifiable");
}

#[test]
fn late_abandon_preserves_active_published_projection_and_handoff_authority() {
    let mut fixture = PublicationFixture::with_pending_handoff();
    assert_eq!(fixture.count("asset_locations", "scan_id = 'new'"), 1);
    assert_eq!(
        fixture.count("library_change_scan_handoff_items", "batch_id = 'new'"),
        1
    );
    assert_eq!(
        fixture.count("library_change_scan_handoff_lineage", "batch_id = 'new'"),
        1
    );
    assert_eq!(
        fixture.count("scan_run_catch_up_lineage", "scan_id = 'new'"),
        0
    );
    let before = durable_state(&fixture.catalog);
    for status in ["failed", "cancelled", "stale", "superseded"] {
        fixture
            .catalog
            .abandon_scan("new", status, 99)
            .expect("late abandon is harmless");
        assert_eq!(durable_state(&fixture.catalog), before, "late {status}");
    }
    fixture.reopen_and_assert(before);
}

#[test]
fn late_abandon_of_replaced_published_scan_preserves_current_and_retained_previews() {
    let mut fixture = PublicationFixture::with_pending_handoff();
    assert_eq!(fixture.count("asset_locations", "scan_id = 'old'"), 0);
    assert_eq!(
        fixture.count("scan_runs", "id = 'old' AND status = 'completed'"),
        1
    );
    assert_eq!(
        fixture.count(
            "library_change_scan_handoff_items",
            "preview_path = 'C:/Cache/old.jpg'"
        ),
        1
    );
    assert_eq!(
        fixture.count("preview_artifact_locations", "artifact_key = 'new'"),
        1
    );
    let before = durable_state(&fixture.catalog);
    fixture
        .catalog
        .abandon_scan("old", "cancelled", 99)
        .expect("old terminal scan");
    assert_eq!(durable_state(&fixture.catalog), before);
    fixture.reopen_and_assert(before);
}

#[test]
fn duplicate_abandon_releases_only_live_staging_and_keeps_first_terminal_result() {
    for paused in [false, true] {
        let mut fixture = PublicationFixture::new();
        fixture.publish("active", "root");
        fixture.begin_and_stage("cancelled", "root");
        if paused {
            fixture
                .catalog
                .pause_scan("cancelled", &ScanCheckpoint::default())
                .expect("pause staging");
        }
        assert_eq!(fixture.count("asset_locations", "scan_id = 'cancelled'"), 1);
        fixture
            .catalog
            .abandon_scan("cancelled", "cancelled", 3)
            .expect("first cancellation");
        assert_eq!(fixture.count("asset_locations", "scan_id = 'cancelled'"), 0);
        assert_eq!(fixture.count("asset_locations", "scan_id = 'active'"), 1);
        assert_eq!(fixture.count("assets", "id = 'asset-cancelled'"), 0);
        assert_eq!(
            fixture.count(
                "scan_runs",
                "id = 'cancelled' AND status = 'cancelled' AND issue_count = 3"
            ),
            1
        );
        let before = durable_state(&fixture.catalog);
        for status in ["cancelled", "failed"] {
            fixture
                .catalog
                .abandon_scan("cancelled", status, 99)
                .expect("duplicate cleanup");
            assert_eq!(durable_state(&fixture.catalog), before);
        }
        fixture.reopen_and_assert(before);
    }
}

#[test]
fn missing_scan_abandon_does_not_touch_published_or_retained_owners() {
    let mut fixture = PublicationFixture::with_pending_handoff();
    let before = durable_state(&fixture.catalog);
    fixture
        .catalog
        .abandon_scan("missing", "failed", 99)
        .expect("missing scan is terminal");
    assert_eq!(durable_state(&fixture.catalog), before);
    fixture.reopen_and_assert(before);
}

struct PublicationFixture {
    catalog: SqliteCatalog,
    directory: tempfile::TempDir,
    identity_sequence: u64,
}

impl PublicationFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("isolated lifecycle catalog");
        let catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        Self {
            catalog,
            directory,
            identity_sequence: 0,
        }
    }

    fn with_pending_handoff() -> Self {
        let mut fixture = Self::new();
        fixture.publish("old", "root");
        fixture.publish("peer", "peer-root");
        let intents = ["root", "peer-root"].map(|root_id| LibraryChangeIntent {
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Path,
            relative_path: "one.png".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::StartupCatchUp,
            first_observed_unix_ms: 1,
            most_recent_observed_unix_ms: 1,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        });
        for intent in intents {
            let report = fixture
                .catalog
                .enqueue_library_change_intents_with_catch_up(
                    &[intent],
                    &LibraryChangeCatchUpEvidence {
                        source: "lifecycle-fixture".to_owned(),
                        watermark: "pending-peer".to_owned(),
                    },
                    1,
                    LibraryChangeQueuePolicy::default(),
                )
                .expect("durable per-root peer consumer");
            assert_eq!(report.inserted_count, 1);
        }
        fixture.publish("new", "root");
        assert_eq!(
            fixture.count(
                "library_change_queue",
                "root_id = 'peer-root' AND status = 'pending'"
            ),
            1
        );
        let path = fixture.directory.path().join("catalog.sqlite3");
        drop(fixture.catalog);
        fixture.catalog = SqliteCatalog::open(path).expect("valid published handoff fixture");
        fixture
    }

    fn begin_and_stage(&mut self, scan_id: &str, root_id: &str) {
        let root_path = format!("C:\\SyntheticScanLifecycle\\{root_id}");
        let request = ScanRequest {
            scan_id: scan_id.to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        self.catalog
            .begin_scan(&request, root_id, &root_path)
            .expect("begin live scan");
        let is_first_import: bool = self
            .catalog
            .connection
            .query_row(
                "SELECT active_scan_id IS NULL FROM library_roots WHERE id = ?1",
                [root_id],
                |row| row.get(0),
            )
            .expect("publication baseline");
        if is_first_import {
            self.catalog
                .prove_live_only_first_import_handoff_for_test(scan_id)
                .expect("fixture capture proof");
        }
        self.identity_sequence += 1;
        let identity = FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".to_owned(),
            value: format!("0000000000000001:{:032x}", self.identity_sequence),
        };
        let preview_path = format!("C:/Cache/{scan_id}.jpg");
        let location = AssetLocationView {
            asset_id: format!("asset-{scan_id}"),
            location_id: format!("location-{scan_id}"),
            root_id: root_id.to_owned(),
            scan_id: scan_id.to_owned(),
            absolute_path: format!("{root_path}\\one.png"),
            display_path: format!("{root_path}\\one.png"),
            relative_path: "one.png".to_owned(),
            preview_path,
            file_size: 20,
            created_unix_ms: Some(25),
            modified_unix_ms: 30,
            file_identity: Some(identity),
            source_revision: Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 0,
            width: 40,
            height: 50,
            preview_status: PreviewStatus::Ready,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture-metadata".to_owned(),
            metadata_engine_version: "1".to_owned(),
            capture_time: None,
        };
        self.catalog
            .stage_location(scan_id, root_id, &location)
            .expect("stage owned location");
        assert_eq!(
            self.catalog
                .count_staged_file_states(scan_id)
                .expect("flush staging"),
            1
        );
        let inserted = self
            .catalog
            .connection
            .execute(
                "INSERT INTO preview_artifacts(
               artifact_key, source_file_size, source_modified_unix_ms,
               source_identity_scheme, source_identity_value, algorithm_id, algorithm_version,
               orientation_contract, size_bucket, encoded_width, encoded_height,
               artifact_path, byte_size, lifecycle_state, created_unix_ms, last_used_unix_ms,
               source_revision_token, source_generation
             ) SELECT ?1, file_size, modified_unix_ms, file_identity_scheme,
                      file_identity_value, 'fixture-preview', 1, 'fixture-orientation', 128,
                      width, height, preview_path, 1, 'ready', 1, 1,
                      source_revision_token, source_generation
               FROM asset_locations WHERE scan_id = ?1",
                [scan_id],
            )
            .expect("preview bound to allocated staging evidence");
        assert_eq!(inserted, 1);
        let exact_generation: bool = self
            .catalog
            .connection
            .query_row(
                "SELECT location.source_generation > 0
                 AND catalog.next_source_generation > location.source_generation
                 AND artifact.source_generation = location.source_generation
                 AND artifact.source_revision_token = location.source_revision_token
             FROM asset_locations AS location
             JOIN preview_artifacts AS artifact ON artifact.artifact_key = location.scan_id
             CROSS JOIN catalog_state AS catalog WHERE location.scan_id = ?1",
                [scan_id],
                |row| row.get(0),
            )
            .expect("canonical preview generation evidence");
        assert!(exact_generation);
    }

    fn publish(&mut self, scan_id: &str, root_id: &str) {
        self.begin_and_stage(scan_id, root_id);
        self.catalog
            .publish_scan(scan_id, root_id, 1, 0)
            .expect("publish complete scan");
    }

    fn count(&self, table: &str, predicate: &str) -> i64 {
        self.catalog
            .connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"),
                [],
                |row| row.get(0),
            )
            .expect("fixed lifecycle evidence")
    }

    fn reopen_and_assert(self, expected: Vec<Vec<Vec<Value>>>) {
        let path = self.directory.path().join("catalog.sqlite3");
        drop(self.catalog);
        let reopened =
            SqliteCatalog::open(path).expect("full published and handoff authority remains valid");
        assert_eq!(durable_state(&reopened), expected);
    }
}

fn durable_state(catalog: &SqliteCatalog) -> Vec<Vec<Vec<Value>>> {
    [
        "scan_runs",
        "library_roots",
        "catalog_state",
        "assets",
        "asset_locations",
        "scan_run_catch_up_lineage",
        "scan_directory_frontier",
        "scan_directory_entries",
        "library_scan_publication_namespace_bindings",
        "library_change_root_state",
        "preview_artifacts",
        "preview_artifact_locations",
        "library_change_queue",
        "library_change_queue_catch_up_lineage",
        "library_change_catch_up_handoffs",
        "library_change_scan_handoff_batches",
        "library_change_scan_handoff_lineage",
        "library_change_scan_handoff_items",
    ]
    .into_iter()
    .map(|table| {
        let query = format!("SELECT * FROM {table}");
        let columns = catalog
            .connection
            .prepare(&query)
            .expect("fixed evidence table")
            .column_count();
        let ordering = (1..=columns)
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut statement = catalog
            .connection
            .prepare(&format!("{query} ORDER BY {ordering}"))
            .expect("complete deterministic evidence order");
        statement
            .query_map([], |row| {
                (0..columns)
                    .map(|index| row.get::<_, Value>(index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("evidence rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("durable evidence values")
    })
    .collect()
}

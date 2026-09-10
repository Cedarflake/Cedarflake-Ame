use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRecoveryAuthority, LibraryRecoveryAuthorityReason,
    LibraryRootGeneration, MetadataInventoryCleanupReport, MetadataInventoryEntry,
    MetadataInventoryEntryKind, MetadataInventoryFrontierEntry, MetadataInventoryPage,
    MetadataInventoryPlaceholderState, MetadataInventoryRun, MetadataInventoryRunStatus,
    MetadataInventoryScope, MetadataInventoryStartRequest, ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue, MetadataInventoryRepository};

use super::{SqliteCatalog, WriteOperation, before_write_scope};

#[test]
fn begin_next_rereads_a_peer_created_run_after_its_read_snapshot_ends() {
    let (_source, _storage, mut catalog) = catalog_with_baseline();
    let mut peer = SqliteCatalog::open(catalog.catalog_path().to_path_buf()).expect("peer catalog");
    let request = request("inventory-peer-created");
    let peer_request = request.clone();
    let expected = Rc::new(RefCell::new(None));
    let peer_result = Rc::clone(&expected);
    let admission = Arc::clone(&catalog.write_admission);
    let _hook = before_write_scope(move |operation| {
        assert_eq!(operation, WriteOperation::BeginNext);
        assert!(
            !admission.state.lock().expect("admission state").is_active,
            "the peer must run before the original writer permit is acquired"
        );
        authorize_inventory(&mut peer, &peer_request);
        peer.begin_next_metadata_inventory(&peer_request)
            .expect("peer creates same run");
        let staged = stage_page(&mut peer, &peer_request.run_id, "peer.txt");
        *peer_result.borrow_mut() = Some(staged);
    });

    let actual = catalog
        .begin_next_metadata_inventory(&request)
        .expect("writer reselects the peer-created run");

    let expected = expected
        .borrow_mut()
        .take()
        .expect("before-write peer actually executed");
    assert_eq!(actual, expected);
    assert_eq!(actual.request.epoch, 1);
    assert_eq!(actual.next_page_index, 2);
    assert_eq!(actual.staged_entry_count, 1);
    assert!(!actual.frontier.is_empty());
    assert_eq!(
        staged_rows(&catalog),
        [(request.run_id, "peer.txt".to_owned())]
    );
    let runs: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs",
            [],
            |row| row.get(0),
        )
        .expect("inventory count");
    assert_eq!(runs, 1, "the original call cannot allocate a second epoch");
    assert!(catalog.connection.is_autocommit());
    SqliteCatalog::open(catalog.catalog_path().to_path_buf())
        .expect("peer-created active inventory retains valid recovery authority");
}

#[test]
fn cleanup_rereads_peer_exhausted_candidates_and_keeps_active_rows() {
    let (_source, _storage, mut catalog) = catalog_with_baseline();
    let terminal = request("inventory-terminal");
    catalog
        .begin_next_metadata_inventory(&terminal)
        .expect("begin terminal candidate");
    stage_page(&mut catalog, &terminal.run_id, "terminal.txt");
    catalog
        .terminate_metadata_inventory(
            &terminal.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_200,
        )
        .expect("terminate candidate");
    let active = request("inventory-active");
    authorize_inventory(&mut catalog, &active);
    catalog
        .begin_next_metadata_inventory(&active)
        .expect("begin active inventory");
    let retained = stage_page(&mut catalog, &active.run_id, "active.txt");
    let mut peer = SqliteCatalog::open(catalog.catalog_path().to_path_buf()).expect("peer catalog");
    let ran = Rc::new(Cell::new(false));
    let peer_ran = Rc::clone(&ran);
    let admission = Arc::clone(&catalog.write_admission);
    let _hook = before_write_scope(move |operation| {
        assert_eq!(operation, WriteOperation::CleanupTerminal);
        assert!(
            !admission.state.lock().expect("admission state").is_active,
            "the hint does not own a writer permit"
        );
        let removed = peer
            .cleanup_terminal_metadata_inventories(3_000, 16, 16, Default::default())
            .expect("peer exhausts terminal candidates");
        assert_eq!(removed.removed_entry_count, 1);
        assert_eq!(removed.removed_run_count, 1);
        assert!(!removed.has_more);
        peer_ran.set(true);
    });

    let report = catalog
        .cleanup_terminal_metadata_inventories(3_000, 16, 16, Default::default())
        .expect("reselect after positive hint becomes empty");

    assert!(ran.get(), "the positive hint must invoke the peer boundary");
    assert_eq!(report, MetadataInventoryCleanupReport::default());
    assert_eq!(
        catalog
            .load_metadata_inventory_run(&active.run_id)
            .expect("load active run")
            .expect("active run retained"),
        retained
    );
    assert!(
        catalog
            .load_metadata_inventory_run(&terminal.run_id)
            .expect("load terminal run")
            .is_none()
    );
    assert_eq!(
        staged_rows(&catalog),
        [(active.run_id, "active.txt".to_owned())]
    );
    assert!(catalog.connection.is_autocommit());
    SqliteCatalog::open(catalog.catalog_path().to_path_buf())
        .expect("cleanup preserves a valid active inventory across full reopen");
}

fn authorize_inventory(catalog: &mut SqliteCatalog, request: &MetadataInventoryStartRequest) {
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: request.root_id.clone(),
                root_generation: request.root_generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: request.started_unix_ms,
                most_recent_observed_unix_ms: request.started_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            request.started_unix_ms,
            policy,
        )
        .expect("enqueue root recovery intent");
    let leased = catalog
        .lease_authoritative_library_change(
            &request.root_id,
            request.root_generation,
            request.started_unix_ms,
            policy,
        )
        .expect("lease root recovery intent")
        .expect("root recovery intent available");
    catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: leased.change.id,
            run_id: request.run_id.clone(),
            root_id: request.root_id.clone(),
            root_generation: request.root_generation,
            reason: LibraryRecoveryAuthorityReason::ContainmentFailure,
            opening_boundary: None,
            authorized_unix_ms: request.started_unix_ms,
            retired_unix_ms: None,
        })
        .expect("persist active inventory recovery authority");
}

pub(super) fn catalog_with_baseline() -> (tempfile::TempDir, tempfile::TempDir, SqliteCatalog) {
    let source = tempfile::tempdir().expect("generated empty source");
    let storage = tempfile::tempdir().expect("generated catalog storage");
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    let scan = ScanRequest {
        scan_id: "inventory-race-baseline".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&scan, "inventory-race-root", &scan.root_path)
        .expect("begin baseline");
    catalog
        .prove_live_only_first_import_handoff_for_test(&scan.scan_id)
        .expect("establish first-import handoff");
    catalog
        .publish_scan(&scan.scan_id, "inventory-race-root", 0, 0)
        .expect("publish real empty baseline");
    (source, storage, catalog)
}

pub(super) fn request(run_id: &str) -> MetadataInventoryStartRequest {
    MetadataInventoryStartRequest {
        run_id: run_id.to_owned(),
        root_id: "inventory-race-root".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 2_000,
    }
}

fn stage_page(
    catalog: &mut SqliteCatalog,
    run_id: &str,
    relative_path: &str,
) -> MetadataInventoryRun {
    catalog
        .stage_metadata_inventory_page(
            run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![MetadataInventoryEntry {
                    relative_path: relative_path.to_owned(),
                    kind: MetadataInventoryEntryKind::File,
                    file_size: Some(1),
                    modified_unix_ms: 1,
                    file_identity: None,
                    source_revision: None,
                    placeholder_state: MetadataInventoryPlaceholderState::Available,
                    is_reparse_point: false,
                }],
                cursor: Some(relative_path.to_owned()),
                is_complete: false,
                frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                    0,
                    "",
                    None,
                    Some(relative_path.to_owned()),
                    1,
                )],
            },
            2_100,
        )
        .expect("stage real catalog page and frontier")
}

fn staged_rows(catalog: &SqliteCatalog) -> Vec<(String, String)> {
    let mut statement = catalog.connection.prepare(
        "SELECT run_id, relative_path FROM library_metadata_inventory_entries ORDER BY run_id, relative_path",
    ).expect("staged evidence query");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("staged evidence rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("staged evidence values")
}

use std::path::Path;

use super::*;
use crate::domain::LibraryChangeLeaseUpdateOutcome;

fn completed_fallback(directory: &Path) -> (SqliteCatalog, LibraryChangeId, String) {
    let source_root = directory.join("source");
    std::fs::create_dir(&source_root).expect("source directory");
    let source_path = source_root.to_string_lossy().into_owned();
    let mut catalog = SqliteCatalog::open(directory.join("catalog.sqlite3")).expect("catalog");
    let mut request = first_import_scan_request("first-import", ROOT);
    request.root_path = source_path.clone();
    catalog
        .begin_scan(&request, ROOT, &source_path)
        .expect("begin import");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, ROOT, 10),
            policy(),
        )
        .expect("opening boundary");
    catalog
        .publish_scan(&request.scan_id, ROOT, 0, 0)
        .expect("publish empty import");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect("closing boundary");
    let change_id = admit_fallback(&mut catalog);
    let authority = catalog
        .load_metadata_inventory_recovery_authority(change_id)
        .expect("recovery authority")
        .expect("admitted recovery");
    let run_id = authority.run_id;
    catalog
        .begin_metadata_inventory(&MetadataInventoryRunRequest {
            run_id: run_id.clone(),
            root_id: ROOT.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 3_001,
        })
        .expect("begin independent inventory");
    let root_identity = crate::adapters::FileDiscovery::new(&source_path)
        .expect("pin source directory")
        .metadata_inventory_root_identity()
        .expect("read root identity")
        .expect("stable root identity");
    catalog
        .connection
        .execute(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value,
               scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms
             ) VALUES (?1, ?2, ?3, 1, ?4, ?5, 'root', '', 'ready', 3001, 3001)",
            params![
                run_id,
                sqlite_integer(change_id.value(), "change ID").expect("change ID"),
                ROOT,
                root_identity.scheme,
                root_identity.value,
            ],
        )
        .expect("persist pinned root proof");
    catalog
        .stage_metadata_inventory_page(
            &run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: Vec::new(),
                cursor: None,
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
            },
            3_002,
        )
        .expect("complete enumeration");
    catalog
        .authorize_metadata_inventory_absence(&run_id, 3_003)
        .expect("verify absence");
    let recovery = catalog
        .lease_metadata_inventory_recovery(ROOT, LibraryRootGeneration::initial(), 3_004, policy())
        .expect("lease independent recovery")
        .expect("recovery lease");
    assert_eq!(recovery.change.id, change_id);
    let revision = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("catalog revision");
    assert_eq!(
        catalog
            .finish_metadata_inventory_recovery(
                change_id,
                recovery.lease_generation,
                u64::try_from(revision).expect("nonnegative catalog revision"),
                3_005,
            )
            .expect("publish recovery completion"),
        Some(LibraryChangeLeaseUpdateOutcome::Applied),
    );
    (catalog, change_id, run_id)
}

#[test]
fn completed_fallback_reopens_after_root_removal() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("catalog.sqlite3");
    let (catalog, change_id, run_id) = completed_fallback(directory.path());
    let receipt = import_control(&catalog, change_id);
    assert_eq!(receipt, ("completed".to_owned(), Some(3_005)));
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("reopen completed inventory");
    assert!(
        catalog
            .unregister_root(ROOT)
            .expect("remove registered root")
    );
    assert!(
        catalog
            .load_metadata_inventory_run(&run_id)
            .expect("read retired run")
            .is_none()
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("reopen removed-root completion history");
    assert_eq!(import_control(&catalog, change_id), receipt);
    assert!(directory.path().join("source").is_dir());
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT COUNT(*) FROM library_roots", [], |row| row
                .get::<_, i64>(0))
            .expect("remaining roots"),
        0,
    );
}

#[test]
fn completed_fallback_reopens_after_inventory_cleanup() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut catalog, change_id, run_id) = completed_fallback(directory.path());
    let receipt = import_control(&catalog, change_id);
    let cleanup = catalog
        .cleanup_terminal_metadata_inventories(4_000, 128, 128, Default::default())
        .expect("clean expired derived inventory");
    assert_eq!(cleanup.removed_run_count, 1);
    assert!(!cleanup.has_more);
    assert!(
        catalog
            .load_metadata_inventory_run(&run_id)
            .expect("read cleaned run")
            .is_none()
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("reopen retained-root completion history");
    assert_eq!(import_control(&catalog, change_id), receipt);
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT COUNT(*) FROM library_roots", [], |row| row
                .get::<_, i64>(0))
            .expect("retained root"),
        1,
    );
}

#[test]
fn completed_fallback_rejects_unretired_authority_or_incomplete_retained_run() {
    for (corruption, expected_code) in [
        (
            "UPDATE library_recovery_authorities SET retired_unix_ms = NULL
             WHERE change_id = ?1",
            "catalog_recovery_execution_contract_unverifiable",
        ),
        (
            "UPDATE library_metadata_inventory_runs
             SET status = 'comparing', completed_unix_ms = NULL
             WHERE id = (SELECT run_id FROM library_recovery_authorities WHERE change_id = ?1)",
            "catalog_persistent_journal_contract_unverifiable",
        ),
    ] {
        let directory = tempdir().expect("directory");
        let path = directory.path().join("catalog.sqlite3");
        let (catalog, change_id, _) = completed_fallback(directory.path());
        catalog
            .connection
            .execute(
                corruption,
                [sqlite_integer(change_id.value(), "change ID").expect("change ID")],
            )
            .expect("inject invalid retained completion proof");
        drop(catalog);
        let error = SqliteCatalog::open(path)
            .err()
            .expect("incomplete completion proof must block reopening");
        assert_eq!(error.code, expected_code, "rejection for {corruption}");
    }
}

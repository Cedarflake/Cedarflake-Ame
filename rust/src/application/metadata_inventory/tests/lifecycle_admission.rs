use super::*;

#[test]
fn existing_inventory_resume_preserves_frontier_without_write_admission() {
    let mut fixture = InventoryFixture::new(&[]);
    let (mut request, staged) = stage_partial_inventory(&mut fixture);
    request.started_unix_ms = 9_000;
    let write_epoch = fixture.catalog.completed_write_epoch();

    let resumed = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("resume existing inventory");

    assert_eq!(resumed, staged);
    assert_eq!(staged_paths(&fixture), ["a.txt", "b.txt"]);
    assert_eq!(fixture.catalog.completed_write_epoch(), write_epoch);
}

#[test]
fn empty_terminal_inventory_cleanup_preserves_active_rows_without_write_admission() {
    let mut fixture = InventoryFixture::new(&[]);
    let (request, staged) = stage_partial_inventory(&mut fixture);
    let write_epoch = fixture.catalog.completed_write_epoch();

    let report = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(10_000, 1, 1, Default::default())
        .expect("cleanup with only active inventory rows");

    assert_eq!(
        report,
        crate::domain::MetadataInventoryCleanupReport::default()
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load active inventory")
            .expect("active inventory retained"),
        staged,
    );
    assert_eq!(staged_paths(&fixture), ["a.txt", "b.txt"]);
    assert_eq!(fixture.catalog.completed_write_epoch(), write_epoch);
}

#[test]
fn existing_inventory_resume_rechecks_the_active_root_publication_boundary() {
    let mut fixture = InventoryFixture::new(&[]);
    let (request, staged) = stage_partial_inventory(&mut fixture);
    let replacement = ScanRequest {
        scan_id: "inventory-admission-replacement".to_owned(),
        root_path: fixture.root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    fixture
        .catalog
        .begin_scan(&replacement, &fixture.root_id, &fixture.root_path)
        .expect("begin real replacement scan");

    let error = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect_err("a matching run cannot bypass an active replacement scan");

    assert_eq!(error.code, "metadata_inventory_root_stale");
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load rejected inventory")
            .expect("rejected inventory retained"),
        staged,
    );
    assert_eq!(staged_paths(&fixture), ["a.txt", "b.txt"]);
    fixture
        .catalog
        .abandon_scan(&replacement.scan_id, "cancelled", 0)
        .expect("retire replacement scan");
    assert_eq!(
        fixture
            .catalog
            .begin_next_metadata_inventory(&request)
            .expect("resume after replacement retirement"),
        staged,
    );
}

#[test]
fn terminal_inventory_cleanup_keeps_the_strict_expiry_boundary_and_input_validation() {
    let mut fixture = InventoryFixture::new(&[]);
    let (request, _) = stage_partial_inventory(&mut fixture);
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_200,
        )
        .expect("terminate inventory");

    let staged_cleanup = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 2, 1, Default::default())
        .expect("terminal staging cleanup is independent of summary age");
    assert_eq!(staged_cleanup.removed_entry_count, 2);
    assert_eq!(staged_cleanup.removed_run_count, 0);
    assert!(!staged_cleanup.has_more);
    assert!(staged_paths(&fixture).is_empty());

    let boundary = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(2_200, 2, 1, Default::default())
        .expect("summary at the cutoff is not expired");
    assert_eq!(
        boundary,
        crate::domain::MetadataInventoryCleanupReport::default()
    );
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load retained summary")
            .is_some()
    );
    let expired = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(2_201, 2, 1, Default::default())
        .expect("remove strictly expired summary");
    assert_eq!(expired.removed_entry_count, 0);
    assert_eq!(expired.removed_run_count, 1);
    assert!(!expired.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load removed summary")
            .is_none()
    );
    let error = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(2_201, 0, 1, Default::default())
        .expect_err("empty cleanup still validates its bounds");
    assert_eq!(error.code, "metadata_inventory_cleanup_limit_invalid");
}

fn stage_partial_inventory(
    fixture: &mut InventoryFixture,
) -> (MetadataInventoryStartRequest, MetadataInventoryRun) {
    let request = MetadataInventoryStartRequest {
        run_id: "inventory-admission".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 2_000,
    };
    fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin inventory");
    let staged = fixture
        .catalog
        .stage_metadata_inventory_page(
            &request.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("a.txt"), metadata_entry("b.txt")],
                cursor: Some("b.txt".to_owned()),
                is_complete: false,
                frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                    0,
                    "",
                    None,
                    Some("b.txt".to_owned()),
                    2,
                )],
            },
            2_100,
        )
        .expect("stage a real durable page and nonempty frontier");
    assert_eq!(staged.staged_entry_count, 2);
    assert_eq!(staged.next_page_index, 2);
    assert!(!staged.frontier.is_empty());
    (request, staged)
}

fn staged_paths(fixture: &InventoryFixture) -> Vec<String> {
    let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open generated catalog evidence");
    let mut statement = connection
        .prepare(
            "SELECT relative_path FROM library_metadata_inventory_entries ORDER BY relative_path",
        )
        .expect("prepare staged path evidence");
    statement
        .query_map([], |row| row.get(0))
        .expect("read staged path evidence")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode staged path evidence")
}

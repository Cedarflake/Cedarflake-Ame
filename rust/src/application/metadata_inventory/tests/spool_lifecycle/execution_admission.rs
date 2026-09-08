use super::source_execution::release_and_reacquire;
use super::*;

#[test]
fn every_raw_directory_operation_requires_the_current_execution() {
    for active in [false, true] {
        let mut fixture = InventoryFixture::new(&["a.png", "b.png"]);
        let (run, lease) =
            begin_named_spool(&mut fixture, MetadataInventoryScope::Root, "raw-admission");
        drop(open_inventory_source(
            &fixture,
            &MetadataInventoryScope::Root,
            &run,
            &lease,
        ));
        let identity = fixture
            .catalog
            .load_metadata_inventory_root_identity(&run.request.run_id)
            .expect("load source identity")
            .expect("source identity");
        let execution = fixture
            .catalog
            .initialize_metadata_inventory_spool(&run, &lease, &identity, None, Some(""), 4_000)
            .expect("capture raw execution");
        if active {
            fixture
                .catalog
                .begin_metadata_inventory_spool_directory(&execution, "", &identity, 4_001)
                .expect("begin owned directory");
        }
        let before = spool_rows(&fixture.catalog, &run.request.run_id);
        release_and_reacquire(&mut fixture, &lease);
        let results = [
            fixture
                .catalog
                .validate_metadata_inventory_spool_execution(&execution),
            fixture
                .catalog
                .metadata_inventory_spool_is_ready(&execution)
                .map(|_| ()),
            fixture
                .catalog
                .next_metadata_inventory_spool_directory(&execution)
                .map(|_| ()),
            fixture
                .catalog
                .begin_metadata_inventory_spool_directory(&execution, "", &identity, 5_100),
            fixture.catalog.append_metadata_inventory_spool_entries(
                &execution,
                "",
                &[metadata_entry("a.png")],
                5_101,
            ),
            fixture.catalog.complete_metadata_inventory_spool_directory(
                &execution,
                "",
                &identity,
                &[metadata_entry("b.png")],
                5_102,
            ),
        ];
        for result in results {
            assert_eq!(
                result.expect_err("retired execution rejected").code,
                "metadata_inventory_spool_authority_mismatch"
            );
        }
        assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
        SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
            .expect("all rejected raw operations leave a valid catalog");
    }
}

#[test]
fn source_execution_is_revoked_by_terminal_run_or_root_removal() {
    for removed in [false, true] {
        let mut fixture = InventoryFixture::new(&["a.png", "b.png"]);
        let bytes =
            fs::read(fixture.source.path().join("a.png")).expect("original generated source");
        let (run, lease) =
            begin_named_spool(&mut fixture, MetadataInventoryScope::Root, "raw-revocation");
        reset_source_enumeration_instrumentation(&fixture.root_path);
        let mut source =
            open_inventory_source(&fixture, &MetadataInventoryScope::Root, &run, &lease);
        let cancellation = AtomicBool::new(false);
        source
            .prepare_next_page(1, &cancellation)
            .expect("retain active iterator");
        let reads = source_entry_read_count(&fixture.root_path);
        if removed {
            fixture
                .catalog
                .unregister_root(&fixture.root_id)
                .expect("remove root");
        } else {
            fixture
                .catalog
                .terminate_metadata_inventory(
                    &run.request.run_id,
                    MetadataInventoryRunStatus::Cancelled,
                    None,
                    5_000,
                )
                .expect("cancel inventory");
        }
        assert!(source.rebind_recovery_lease(&lease).is_err());
        assert!(source.prepare_next_page(1, &cancellation).is_err());
        assert!(source.next_page(4, &cancellation).is_err());
        assert_eq!(source_entry_read_count(&fixture.root_path), reads);
        assert_eq!(
            spool_rows(&fixture.catalog, &run.request.run_id),
            SpoolRows::default()
        );
        assert_eq!(
            fs::read(fixture.source.path().join("a.png")).expect("preserved source"),
            bytes
        );
        SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
            .expect("valid revoked state");
    }
}

#[test]
fn raw_page_authority_frontier_and_rows_share_one_read_snapshot() {
    let mut fixture = InventoryFixture::new(&["a.png", "b.png"]);
    let (run, lease) =
        begin_named_spool(&mut fixture, MetadataInventoryScope::Root, "raw-snapshot");
    let mut source = open_inventory_source(&fixture, &MetadataInventoryScope::Root, &run, &lease);
    let cancellation = AtomicBool::new(false);
    assert_eq!(
        source
            .prepare_next_page(4, &cancellation)
            .expect("prepare raw entries"),
        MetadataInventorySourcePreparation::Yielded
    );
    assert_eq!(
        source
            .prepare_next_page(4, &cancellation)
            .expect("confirm ready"),
        MetadataInventorySourcePreparation::Ready
    );
    let mut writer = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("independent writer");
    let run_id = run.request.run_id.clone();
    let page = SqliteCatalog::with_metadata_inventory_source_read_hook(
        move || {
            writer
                .terminate_metadata_inventory(
                    &run_id,
                    MetadataInventoryRunStatus::Cancelled,
                    None,
                    5_000,
                )
                .expect("commit retirement after read authority check");
            assert_eq!(spool_rows(&writer, &run_id), SpoolRows::default());
        },
        || source.next_page(4, &cancellation),
    )
    .expect("admitted read completes from its original snapshot");
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["a.png", "b.png"]
    );
    assert!(page.frontier[0].directory_identity.is_some());
    assert!(source.next_page(4, &cancellation).is_err());
    assert!(
        fixture
            .catalog
            .stage_metadata_inventory_page(&run.request.run_id, &page, 5_001)
            .is_err(),
        "a consistent old read is not authority to stage into a terminal run"
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("valid snapshot retirement");
}

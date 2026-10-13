use super::*;

#[test]
fn retired_source_execution_cannot_append_after_the_same_run_is_released() {
    let mut fixture = InventoryFixture::new(&["a.png", "b.png", "c.png"]);
    let (run, lease) =
        begin_named_spool(&mut fixture, MetadataInventoryScope::Root, "stale-writer");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let mut source = open_inventory_source(&fixture, &MetadataInventoryScope::Root, &run, &lease);
    let cancellation = AtomicBool::new(false);
    assert!(matches!(
        source
            .prepare_next_page(1, &cancellation)
            .expect("first raw entry"),
        MetadataInventorySourcePreparation::Yielded,
    ));
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    assert_eq!(before.entries, 1);
    let reads_before = source_entry_read_count(&fixture.root_path);
    let successor = release_and_reacquire(&mut fixture, &lease);
    assert!(successor.lease_generation > lease.lease_generation);
    let result = source.prepare_next_page(1, &cancellation);
    let after = spool_rows(&fixture.catalog, &run.request.run_id);
    assert!(
        result.is_err(),
        "a source retained by the retired lease must not advance the successor's raw spool; before={before:?}; after={after:?}",
    );
    assert_eq!(after, before);
    assert_eq!(source_entry_read_count(&fixture.root_path), reads_before);
    assert!(source.rebind_recovery_lease(&lease).is_err());
    source
        .rebind_recovery_lease(&successor)
        .expect("the current execution explicitly accepts its successor lease");
    let mut ready = false;
    for _ in 0..5 {
        if source
            .prepare_next_page(1, &cancellation)
            .expect("continue raw iterator")
            == MetadataInventorySourcePreparation::Ready
        {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let page = source
        .next_page(4, &cancellation)
        .expect("current ordered page");
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["a.png", "b.png", "c.png"],
    );
    assert_eq!(source_entry_read_count(&fixture.root_path), 3);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("rejected stale writer leaves valid source evidence");
}

#[test]
fn retired_source_execution_cannot_read_a_ready_spool_after_lease_handoff() {
    let mut fixture = InventoryFixture::new(&["a.png"]);
    let (run, lease) =
        begin_named_spool(&mut fixture, MetadataInventoryScope::Root, "stale-reader");
    let mut source = open_inventory_source(&fixture, &MetadataInventoryScope::Root, &run, &lease);
    let cancellation = AtomicBool::new(false);
    let mut ready = false;
    for _ in 0..4 {
        if matches!(
            source
                .prepare_next_page(4, &cancellation)
                .expect("prepare ready spool"),
            MetadataInventorySourcePreparation::Ready
        ) {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let successor = release_and_reacquire(&mut fixture, &lease);
    assert!(
        source.next_page(4, &cancellation).is_err(),
        "ready raw data is not permission for a retired execution to produce logical input",
    );
    let current = fixture
        .catalog
        .load_metadata_inventory_run(&run.request.run_id)
        .expect("load current inventory")
        .expect("inventory retained");
    assert_eq!(current.staged_entry_count, 0);
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id).entries, 1);
    source
        .rebind_recovery_lease(&successor)
        .expect("ready spool continuation");
    assert!(source.rebind_recovery_lease(&lease).is_err());
    assert_eq!(
        source
            .next_page(4, &cancellation)
            .expect("retained current binding")
            .entries
            .len(),
        1
    );
}

pub(super) fn release_and_reacquire(
    fixture: &mut InventoryFixture,
    lease: &LeasedLibraryChange,
) -> LeasedLibraryChange {
    assert_eq!(
        fixture
            .catalog
            .defer_library_change(lease.change.id, lease.lease_generation, 5_000)
            .expect("return raw source lease"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Applied
    );
    fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            5_001,
            queue_policy(),
        )
        .expect("lease retained inventory")
        .expect("retained inventory is available")
}

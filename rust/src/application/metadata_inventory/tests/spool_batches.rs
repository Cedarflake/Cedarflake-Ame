use super::*;

#[test]
fn small_and_empty_directories_share_bounded_batches_and_keep_completed_progress() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..20 {
        let populated = fixture.source.path().join(format!("album-{index:02}"));
        fs::create_dir(&populated).expect("small source directory");
        fs::write(populated.join("entry.jpg"), b"metadata").expect("source entry");
        fs::create_dir(fixture.source.path().join(format!("empty-{index:02}")))
            .expect("empty source directory");
    }
    let leased = enqueue_authorized_recovery_control(&mut fixture, "small-batches", 4_000);
    let request = MetadataInventoryStartRequest {
        run_id: "small-batches".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 4_000,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin durable inventory");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    let mut ready = false;
    for batch in 0..8 {
        let prior_reads = source_entry_read_count(&fixture.root_path);
        let prior_opens = source_spool_open_count(&fixture.root_path);
        let outcome = source
            .prepare_next_page(4_095, &cancellation)
            .expect("bounded source batch");
        let reads = source_entry_read_count(&fixture.root_path);
        let opens = source_spool_open_count(&fixture.root_path);
        assert!(reads - prior_reads <= 128);
        assert!(opens - prior_opens <= 16);
        if batch == 0 {
            assert!(opens > 1, "small directories must share the first batch");
            cancellation.store(true, std::sync::atomic::Ordering::Release);
            assert_eq!(
                source
                    .prepare_next_page(4_095, &cancellation)
                    .expect_err("cancel before reading another batch")
                    .code,
                "metadata_inventory_cancelled"
            );
            assert_eq!(source_entry_read_count(&fixture.root_path), reads);
            assert_eq!(source_spool_open_count(&fixture.root_path), opens);
            cancellation.store(false, std::sync::atomic::Ordering::Release);
        }
        if outcome == MetadataInventorySourcePreparation::Ready {
            ready = true;
            break;
        }
        drop(source);
        source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    }
    assert!(
        ready,
        "forty small directories must finish within eight batches"
    );
    assert_eq!(source_entry_read_count(&fixture.root_path), 60);
    assert_eq!(source_spool_open_count(&fixture.root_path), 41);
    let page = source
        .next_page(4_095, &cancellation)
        .expect("complete durable source page");
    assert!(page.is_complete);
    assert_eq!(page.entries.len(), 60);
}

#[test]
fn raw_batch_respects_a_smaller_requested_source_read_limit() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..20 {
        fs::write(
            fixture.source.path().join(format!("entry-{index:02}.jpg")),
            b"metadata",
        )
        .expect("source entry");
    }
    let leased = enqueue_authorized_recovery_control(&mut fixture, "small-read-limit", 4_000);
    let request = MetadataInventoryStartRequest {
        run_id: "small-read-limit".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 4_000,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin durable inventory");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    assert_eq!(
        source
            .prepare_next_page(7, &AtomicBool::new(false))
            .expect("small source quantum"),
        MetadataInventorySourcePreparation::Yielded
    );
    assert_eq!(source_entry_read_count(&fixture.root_path), 7);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
}

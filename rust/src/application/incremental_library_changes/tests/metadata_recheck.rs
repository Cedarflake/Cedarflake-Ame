use super::*;

fn pending_metadata(count: usize) -> CatalogFixture {
    let source = tempdir().unwrap();
    let names = (0..count)
        .map(|index| format!("photo-{index:03}.png"))
        .collect::<Vec<_>>();
    for name in &names {
        write_png(&source.path().join(name), 3, 2, [11, 22, 33]);
    }
    let paths = names.iter().map(String::as_str).collect::<Vec<_>>();
    seed_catalog_with_metadata(
        source,
        &paths,
        Some((
            crate::application::INVALIDATED_MEDIA_METADATA_ENGINE_ID,
            crate::application::INVALIDATED_MEDIA_METADATA_ENGINE_VERSION,
        )),
    )
}

fn recover(
    fixture: &mut CatalogFixture,
    cancelled: bool,
) -> crate::domain::IncrementalLibraryChangeReport {
    crate::application::process_ready_unowned_recovery_paths_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        4_000,
        policy(),
        &AtomicBool::new(cancelled),
    )
    .unwrap()
}

#[test]
fn metadata_rechecks_are_bounded_durable_and_do_not_invalidate_current_content() {
    let mut fixture = pending_metadata(40);
    let before = fixture.location("photo-000.png").unwrap();
    let revision = fixture.revision();
    reset_source_enumeration_instrumentation(&fixture.root_path);
    reset_source_content_open_instrumentation(&fixture.root_path);
    assert_eq!(
        fixture
            .catalog
            .enqueue_metadata_rechecks(3_000, policy())
            .unwrap(),
        32
    );
    assert_eq!(
        fixture
            .catalog
            .enqueue_metadata_rechecks(3_000, policy())
            .unwrap(),
        8
    );
    assert_eq!(
        fixture
            .catalog
            .enqueue_metadata_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    let path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(path).unwrap();
    assert_eq!(recover(&mut fixture, false).completed_count, 40);
    let after = fixture.location("photo-000.png").unwrap();
    assert_eq!(after.source_generation, before.source_generation);
    assert_eq!(after.asset_id, before.asset_id);
    assert_eq!((after.width, after.height), (3, 2));
    assert!(!crate::application::media_metadata_is_invalidated(&after));
    assert!(fixture.revision() > revision);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    assert_eq!(
        fixture
            .catalog
            .enqueue_metadata_rechecks(5_000, policy())
            .unwrap(),
        0
    );
}

#[test]
fn metadata_recheck_revalidates_a_file_changed_after_admission() {
    let mut fixture = pending_metadata(1);
    let before = fixture.location("photo-000.png").unwrap();
    fixture
        .catalog
        .enqueue_metadata_rechecks(3_000, policy())
        .unwrap();
    write_png(
        &fixture.source.path().join("photo-000.png"),
        8,
        7,
        [33, 22, 11],
    );
    assert_eq!(recover(&mut fixture, false).completed_count, 1);
    let after = fixture.location("photo-000.png").unwrap();
    assert_ne!(after.source_generation, before.source_generation);
    assert_eq!((after.width, after.height), (8, 7));
}

#[test]
fn cancelled_metadata_recovery_leaves_retryable_work_without_source_reads() {
    let mut fixture = pending_metadata(1);
    let before = fixture.location("photo-000.png").unwrap();
    fixture
        .catalog
        .enqueue_metadata_rechecks(3_000, policy())
        .unwrap();
    reset_source_content_open_instrumentation(&fixture.root_path);
    assert_eq!(recover(&mut fixture, true).completed_count, 0);
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    assert_eq!(
        fixture.location("photo-000.png").unwrap().source_generation,
        before.source_generation
    );
    assert_eq!(recover(&mut fixture, false).completed_count, 1);
}

#[test]
fn precise_dirty_evidence_keeps_ownership_ahead_of_metadata_recovery() {
    let mut fixture = pending_metadata(1);
    let before = fixture.location("photo-000.png").unwrap();
    fixture.enqueue(&[intent(&fixture.root_id, "photo-000.png", None, 1)]);
    assert_eq!(
        fixture
            .catalog
            .enqueue_metadata_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    assert_eq!(fixture.process().completed_count, 1);
    let after = fixture.location("photo-000.png").unwrap();
    assert_ne!(after.source_generation, before.source_generation);
    assert!(!crate::application::media_metadata_is_invalidated(&after));
}

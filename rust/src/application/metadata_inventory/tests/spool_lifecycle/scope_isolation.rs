use super::*;

fn subtree_scope() -> MetadataInventoryScope {
    MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    }
}

#[test]
fn removing_one_root_preserves_another_roots_real_subtree_spool() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let removed_id = fixture.root_id.clone();
    let removed_file = fixture.source.path().join("album/retained.png");
    let removed_bytes = fs::read(&removed_file).expect("first source bytes");
    let removed_run = stage_real_spool(&mut fixture, subtree_scope());

    let second_source = tempdir().expect("second controlled source");
    let retained_file = second_source.path().join("album/retained.png");
    write_png(&retained_file, 2, 2, [20, 30, 40]);
    let retained_bytes = fs::read(&retained_file).expect("second source bytes");
    let root_path = FileDiscovery::new(&second_source.path().to_string_lossy())
        .expect("second discovery")
        .canonical_root()
        .expect("second canonical source")
        .to_string_lossy()
        .into_owned();
    let storage = fixture._storage.path();
    run_scan_with_storage(
        ScanRequest {
            scan_id: "second-source-scan".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        StoragePaths {
            catalog_path: storage.join("catalog.sqlite3"),
            preview_root: storage.join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.join("settings.sqlite3"),
        },
    )
    .expect("publish independent source");
    let _removed_source = std::mem::replace(&mut fixture.source, second_source);
    fixture.root_id = stable_id("library-root-v1", &root_path);
    fixture.root_path = root_path;
    let (retained_run, _) = stage_named_spool(&mut fixture, subtree_scope(), "preserved-spool");
    let retained_raw = spool_rows(&fixture.catalog, &retained_run.request.run_id);
    assert_eq!(
        (retained_raw.entries, retained_raw.entries_without_directory),
        (2, 1)
    );
    let retained_authority =
        recovery_retirement_evidence(&fixture.catalog, &retained_run.request.run_id);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    SqliteCatalog::open(catalog_path.clone()).expect("both real spools pass full validation");

    assert!(
        fixture
            .catalog
            .unregister_root(&removed_id)
            .expect("remove only first root")
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &removed_run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &retained_run.request.run_id),
        retained_raw
    );
    assert_eq!(
        recovery_retirement_evidence(&fixture.catalog, &retained_run.request.run_id),
        retained_authority
    );
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("full reopen retains second owner");
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&retained_run.request.run_id)
            .expect("retained run"),
        Some(retained_run.clone())
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &retained_run.request.run_id),
        retained_raw
    );
    assert_eq!(
        fs::read(removed_file).expect("first source unchanged"),
        removed_bytes
    );
    assert_eq!(
        fs::read(retained_file).expect("second source unchanged"),
        retained_bytes
    );
}

#[test]
fn newer_epoch_retires_the_previous_subtree_initial_observation() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes");
    let (old_run, leased) = stage_real_spool_with_lease(&mut fixture, subtree_scope());
    assert_eq!(
        spool_rows(&fixture.catalog, &old_run.request.run_id).entries_without_directory,
        1
    );
    authorize_leased_containment_recovery(&mut fixture, &leased, "newer-spool", 6_000);
    let next = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "newer-spool".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: subtree_scope(),
            started_unix_ms: 6_000,
        })
        .expect("begin next authority-owned epoch");
    assert_eq!(next.request.epoch, old_run.request.epoch + 1);
    let old = fixture
        .catalog
        .load_metadata_inventory_run(&old_run.request.run_id)
        .expect("load old epoch")
        .expect("retained old run history");
    assert_eq!(old.status, MetadataInventoryRunStatus::Superseded);
    assert!(!old.absence_authority);
    assert_eq!(
        spool_rows(&fixture.catalog, &old_run.request.run_id),
        SpoolRows::default()
    );
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("full reopen after epoch replacement");
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run("newer-spool")
            .expect("new epoch"),
        Some(next)
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &old_run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        fs::read(source_path).expect("source unchanged"),
        source_bytes
    );
}

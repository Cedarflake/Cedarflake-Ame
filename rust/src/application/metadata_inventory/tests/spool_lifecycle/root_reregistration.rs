use super::*;
use crate::application::scan_library::catalog_has_active_scan;
use crate::domain::ScanEvent;

#[test]
fn removed_root_reregistration_preserves_new_generation_while_old_spool_drains() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let storage = StoragePaths {
        catalog_path: fixture._storage.path().join("catalog.sqlite3"),
        preview_root: fixture._storage.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: fixture._storage.path().join("settings.sqlite3"),
    };
    let first_source = tempdir().expect("independent A source");
    let second_source = tempdir().expect("independent B source");
    let first_path = canonical_source(first_source.path());
    let second_path = canonical_source(second_source.path());
    let first_id = stable_id("library-root-v1", &first_path);
    let second_id = stable_id("library-root-v1", &second_path);
    for (source, root_path, scan_id, color) in [
        (
            first_source.path(),
            &first_path,
            "overlap-a-baseline",
            [40, 50, 60],
        ),
        (
            second_source.path(),
            &second_path,
            "overlap-b-baseline",
            [70, 80, 90],
        ),
    ] {
        write_png(&source.join("retained.png"), 2, 2, color);
        scan_source(&storage, root_path, scan_id, 1, |_| {});
        write_png(&source.join("added.png"), 3, 2, color);
    }
    let sources_before = [
        source_manifest(first_source.path()),
        source_manifest(second_source.path()),
        source_manifest(fixture.source.path()),
    ];
    assert_eq!(sources_before.each_ref().map(Vec::len), [2, 2, 1]);
    let scope = MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    };
    let (old_run, old_lease) = begin_named_spool(&mut fixture, scope.clone(), "removed-c-spool");
    let (old_source, old_execution) =
        prepare_opened_spool(&mut fixture, &scope, &old_run, &old_lease);
    drop(old_source);
    assert_eq!(old_run.status, MetadataInventoryRunStatus::Running);
    let old_raw = spool_rows(&fixture.catalog, &old_run.request.run_id);
    let old_generation = old_run.request.root_generation;
    let mut removal_count = 0;
    let mut first_published_while_second_running = false;

    // Started is outside the registration transaction. Nested real scans fix the
    // publication order without waiting while holding a database transaction.
    scan_source(&storage, &second_path, "overlap-b-update", 2, |event| {
        if matches!(event, ScanEvent::Started { .. }) {
            scan_source(
                &storage,
                &first_path,
                "overlap-a-update",
                2,
                |event| match event {
                    ScanEvent::Started { .. } => {
                        for root_id in [&first_id, &second_id] {
                            assert!(registered_root(&fixture.catalog, root_id).has_running_scan);
                        }
                        assert_current_queue_lease(&fixture.catalog, &old_lease);
                        fixture
                            .catalog
                            .validate_metadata_inventory_spool_execution(&old_execution)
                            .expect("C source execution remains current before removal");
                        assert!(
                            fixture
                                .catalog
                                .unregister_root(&fixture.root_id)
                                .expect("remove C while both updates own execution")
                        );
                        removal_count += 1;
                        assert_retired_spool(&fixture.catalog, &old_run.request.run_id, &old_raw);
                        assert_execution_rejected(&fixture.catalog, &old_execution);
                        let batch = fixture
                            .catalog
                            .cleanup_terminal_metadata_inventories(10_000, 1, 1, Default::default())
                            .expect("one admitted old-spool cleanup batch");
                        assert_eq!(batch.removed_entry_count, 1);
                        assert!(batch.removed_run_count <= 1);
                        assert!(batch.has_more);
                        let partial = spool_rows(&fixture.catalog, &old_run.request.run_id);
                        assert_eq!(partial.entries, old_raw.entries - 1);
                        assert!(partial.entries > 0);
                        assert_eq!(
                            spool_state(&fixture.catalog, &old_run.request.run_id).as_deref(),
                            Some("retired")
                        );
                        SqliteCatalog::open(storage.catalog_path.clone())
                            .expect("FULL open accepts partial cleanup with both scans active");
                    }
                    ScanEvent::Completed { .. } => {
                        assert_published_root(&fixture.catalog, &first_id, "overlap-a-update");
                        let second = registered_root(&fixture.catalog, &second_id);
                        assert!(second.has_running_scan);
                        assert_eq!(second.active_scan_id.as_deref(), Some("overlap-b-baseline"));
                        assert!(
                            fixture
                                .catalog
                                .load_incremental_location_by_relative_path(&first_id, "added.png",)
                                .expect("A publishes new content")
                                .is_some()
                        );
                        assert!(fixture.catalog.load_incremental_location_by_relative_path(
                            &second_id, "added.png",
                        ).expect("B retains its published baseline").is_none());
                        assert!(
                            fixture
                                .catalog
                                .load_incremental_catalog_root(&fixture.root_id)
                                .expect("C remains removed during A publication")
                                .is_none()
                        );
                        first_published_while_second_running = true;
                    }
                    _ => {}
                },
            );
        }
    });
    assert_eq!(removal_count, 1);
    assert!(first_published_while_second_running);
    assert_published_root(&fixture.catalog, &second_id, "overlap-b-update");
    assert!(
        !catalog_has_active_scan(&storage.catalog_path, None)
            .expect("both update execution registrations retire before primary import")
    );
    let partial_old_raw = spool_rows(&fixture.catalog, &old_run.request.run_id);
    assert!(partial_old_raw.entries > 0);

    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(storage.catalog_path.clone())
        .expect("FULL reopen before C registration retains old cleanup debt");
    let removed_root_id = fixture.root_id.clone();
    scan_source(&storage, &fixture.root_path, "reregister-c-scan", 1, |_| {});
    let current = registered_root(&fixture.catalog, &removed_root_id);
    assert_eq!(current.root_id, removed_root_id);
    assert_eq!(
        current.root_generation,
        old_generation.next().expect("next generation")
    );
    assert_published_root(&fixture.catalog, &removed_root_id, "reregister-c-scan");
    assert_retired_spool(&fixture.catalog, &old_run.request.run_id, &partial_old_raw);
    assert_execution_rejected(&fixture.catalog, &old_execution);

    let (new_run, new_lease) =
        begin_named_spool(&mut fixture, scope.clone(), "reregistered-c-spool");
    let (new_source, new_execution) =
        prepare_opened_spool(&mut fixture, &scope, &new_run, &new_lease);
    drop(new_source);
    assert_eq!(new_run.status, MetadataInventoryRunStatus::Running);
    assert_current_queue_lease(&fixture.catalog, &new_lease);
    assert_eq!(new_run.request.root_id, old_run.request.root_id);
    assert_eq!(new_run.request.root_generation, current.root_generation);
    assert_ne!(new_run.request.run_id, old_run.request.run_id);
    let new_raw = spool_rows(&fixture.catalog, &new_run.request.run_id);
    let new_authority = recovery_retirement_evidence(&fixture.catalog, &new_run.request.run_id);
    assert_eq!(new_authority.len(), 1);
    assert!(new_authority[0].0.is_none());
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(storage.catalog_path.clone())
        .expect("FULL reopen accepts old and new C generations together");
    let cleanup = cleanup_inventory_until_idle(&mut fixture, 10_000);
    assert_eq!(
        i64::from(cleanup.removed_entry_count),
        partial_old_raw.entries
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &old_run.request.run_id),
        SpoolRows::default()
    );
    assert_execution_rejected(&fixture.catalog, &old_execution);
    assert_eq!(
        spool_rows(&fixture.catalog, &new_run.request.run_id),
        new_raw
    );
    assert_eq!(
        recovery_retirement_evidence(&fixture.catalog, &new_run.request.run_id),
        new_authority
    );
    assert_current_queue_lease(&fixture.catalog, &new_lease);
    fixture
        .catalog
        .validate_metadata_inventory_spool_execution(&new_execution)
        .expect("old cleanup preserves current generation execution");

    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(storage.catalog_path)
        .expect("FULL reopen after old cleanup preserves all current roots");
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_roots()
            .expect("current roots")
            .len(),
        3
    );
    for (root_id, scan_id) in [
        (&first_id, "overlap-a-update"),
        (&second_id, "overlap-b-update"),
        (&removed_root_id, "reregister-c-scan"),
    ] {
        assert_published_root(&fixture.catalog, root_id, scan_id);
    }
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&new_run.request.run_id)
            .expect("current inventory survives old cleanup"),
        Some(new_run.clone())
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &old_run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        spool_rows(&fixture.catalog, "reregistered-c-spool"),
        new_raw
    );
    assert_eq!(
        recovery_retirement_evidence(&fixture.catalog, "reregistered-c-spool"),
        new_authority
    );
    assert_execution_rejected(&fixture.catalog, &old_execution);
    fixture
        .catalog
        .validate_metadata_inventory_spool_execution(&new_execution)
        .expect("current execution remains admitted after FULL reopen");
    assert!(
        fixture
            .catalog
            .metadata_inventory_spool_is_ready(&new_execution)
            .expect("current raw observations remain available")
    );
    assert_current_queue_lease(&fixture.catalog, &new_lease);
    let page = fixture
        .catalog
        .load_metadata_inventory_spool_page(&new_execution, 4_095)
        .expect("current execution reads its own generation after old cleanup");
    assert!(page.is_complete);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["album", "album/retained.png"]
    );
    let comparing = fixture
        .catalog
        .stage_metadata_inventory_page(&new_run.request.run_id, &page, 6_000)
        .expect("publish the complete current-generation page");
    assert_eq!(comparing.status, MetadataInventoryRunStatus::Comparing);
    assert!(comparing.enumeration_complete);
    assert_current_queue_lease(&fixture.catalog, &new_lease);
    assert_execution_rejected(&fixture.catalog, &new_execution);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path)
        .expect("FULL reopen retains the comparison owner after source execution retires");
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&new_run.request.run_id)
            .expect("retained comparison owner"),
        Some(comparing)
    );
    assert_current_queue_lease(&fixture.catalog, &new_lease);
    assert_execution_rejected(&fixture.catalog, &new_execution);
    assert_eq!(
        sources_before,
        [
            source_manifest(first_source.path()),
            source_manifest(second_source.path()),
            source_manifest(fixture.source.path()),
        ]
    );
}

fn assert_current_queue_lease(catalog: &SqliteCatalog, leased: &LeasedLibraryChange) {
    let connection = rusqlite::Connection::open_with_flags(
        catalog.catalog_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read exact queue lease evidence");
    let actual: (String, i64) = connection
        .query_row(
            "SELECT status, lease_generation FROM library_change_queue WHERE id = ?1",
            [i64::try_from(leased.change.id.value()).expect("queue ID")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("current queue lease evidence");
    assert_eq!(
        actual,
        (
            "leased".to_owned(),
            i64::try_from(leased.lease_generation).expect("lease generation")
        )
    );
}

fn canonical_source(source: &Path) -> String {
    FileDiscovery::new(&source.to_string_lossy())
        .expect("source discovery")
        .canonical_root()
        .expect("canonical source")
        .to_string_lossy()
        .into_owned()
}

fn scan_source(
    storage: &StoragePaths,
    root_path: &str,
    scan_id: &str,
    expected_assets: u64,
    mut observe: impl FnMut(&ScanEvent),
) {
    let mut started_count = 0;
    let mut completed_count = 0;
    run_scan_with_storage(
        ScanRequest {
            scan_id: scan_id.to_owned(),
            root_path: root_path.to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            observe(&event);
            match event {
                ScanEvent::Started { .. } => started_count += 1,
                ScanEvent::Completed {
                    asset_count,
                    issue_count,
                    was_limited,
                    ..
                } => {
                    completed_count += 1;
                    assert_eq!(asset_count, expected_assets);
                    assert_eq!(issue_count, 0);
                    assert!(!was_limited);
                }
                _ => {}
            }
            true
        },
        storage.clone(),
    )
    .expect("production scan completes");
    assert_eq!((started_count, completed_count), (1, 1));
}

fn registered_root(catalog: &SqliteCatalog, root_id: &str) -> IncrementalCatalogRoot {
    catalog
        .load_incremental_catalog_root(root_id)
        .expect("load registered root")
        .expect("registered root")
}

fn assert_published_root(catalog: &SqliteCatalog, root_id: &str, scan_id: &str) {
    let root = registered_root(catalog, root_id);
    assert_eq!(root.active_scan_id.as_deref(), Some(scan_id));
    assert!(!root.has_running_scan);
}

fn source_manifest(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>, std::time::SystemTime)> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("owned fixture directory") {
            let entry = entry.expect("owned fixture entry");
            let path = entry.path();
            let metadata = entry.metadata().expect("owned fixture metadata");
            if metadata.is_dir() {
                pending.push(path);
            } else {
                files.push((
                    path.strip_prefix(root)
                        .expect("fixture-relative path")
                        .to_path_buf(),
                    fs::read(&path).expect("fixture bytes"),
                    metadata.modified().expect("fixture mtime"),
                ));
            }
        }
    }
    files.sort();
    files
}

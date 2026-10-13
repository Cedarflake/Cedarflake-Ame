use super::*;

#[test]
fn root_location_recovers_automatically_and_continues_without_a_foreground_scan() {
    let fixture = ProductionGapFixture::new_with_media_baseline("automatic-directory-location");
    let before = std::fs::read(fixture.source_root.join("unchanged.png")).unwrap();
    let removed_before = std::fs::read(fixture.source_root.join("removed.png")).unwrap();
    let destination = fixture.source_root.with_file_name("renamed");
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let old_root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let old_asset = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "unchanged.png")
        .unwrap()
        .unwrap();
    drop(catalog);
    std::fs::rename(&fixture.source_root, &destination).unwrap();
    let mut runtime = fast_gap_runtime(QueuedSourceFactory::default(), test_live_only_connection());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let snapshot = poll_runtime_with_storage(&mut runtime, &fixture.storage).unwrap();
            let root = snapshot
                .roots
                .iter()
                .find(|root| root.root_id == fixture.root_id)
                .unwrap();
            if root.root_generation > old_root.root_generation.value()
                && root.freshness == crate::domain::CatalogFreshnessState::Synchronized
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "automatic recovery did not converge: {root:?}"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
        let root = catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            std::path::Path::new(&root.root_path)
                .canonicalize()
                .unwrap(),
            destination.canonicalize().unwrap()
        );
        assert_eq!(root.active_scan_id, old_root.active_scan_id);
        let current = catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "unchanged.png")
            .unwrap()
            .unwrap();
        assert_eq!(current.asset_id, old_asset.asset_id);
        assert_eq!(current.location_id, old_asset.location_id);
        assert_eq!(std::fs::read(&current.absolute_path).unwrap(), before);
        let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
        let scans = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap();
        assert_eq!(scans, fixture.scan_rows_before);
    }));
    runtime.stop().unwrap();
    assert_eq!(
        std::fs::read(destination.join("unchanged.png")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(destination.join("removed.png")).unwrap(),
        removed_before
    );
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

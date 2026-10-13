use super::*;

#[test]
fn terminal_retirement_preserves_raw_rows_until_bounded_reclamation() {
    for remove_root in [false, true] {
        let mut fixture = InventoryFixture::new(&["album/retained.png"]);
        let path = fixture.source.path().join("album/retained.png");
        let bytes = fs::read(&path).expect("source bytes");
        let run = stage_real_spool(
            &mut fixture,
            MetadataInventoryScope::Subtree {
                relative_path: "album".into(),
            },
        );
        let before = spool_rows(&fixture.catalog, &run.request.run_id);
        if remove_root {
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
                    6_000,
                )
                .expect("cancel inventory");
        }
        assert_eq!(
            spool_rows(&fixture.catalog, &run.request.run_id),
            before,
            "terminal transition must revoke execution without deleting the raw payload"
        );
        let catalog_path = fixture.catalog.catalog_path().to_path_buf();
        drop(fixture.catalog);
        fixture.catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen retired data");
        assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
        let mut finished = false;
        for _ in 0..16 {
            let report = fixture
                .catalog
                .cleanup_terminal_metadata_inventories(10_000, 1, 1, Default::default())
                .expect("bounded cleanup");
            assert!(report.removed_entry_count <= 1);
            assert!(report.removed_run_count <= 1);
            drop(fixture.catalog);
            fixture.catalog =
                SqliteCatalog::open(catalog_path.clone()).expect("reopen partial cleanup");
            if !report.has_more {
                finished = true;
                break;
            }
        }
        assert!(finished, "persistent cleanup debt must eventually drain");
        assert_eq!(
            spool_rows(&fixture.catalog, &run.request.run_id),
            SpoolRows::default()
        );
        assert_eq!(fs::read(path).expect("preserved source"), bytes);
    }
}

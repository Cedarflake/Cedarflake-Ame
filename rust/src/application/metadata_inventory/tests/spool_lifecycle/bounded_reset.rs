use super::*;

#[test]
fn interrupted_raw_enumerator_restarts_only_after_bounded_reset_across_reopen() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let path = fixture.source.path().join("album/retained.png");
    let bytes = fs::read(&path).expect("source bytes");
    let scope = MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    };
    let (run, lease) = begin_named_spool(&mut fixture, scope.clone(), "bounded-reset");
    drop(open_inventory_source(&fixture, &scope, &run, &lease));
    let identity = rusqlite::Connection::open(fixture.catalog.catalog_path()).expect("fixture evidence")
        .query_row("SELECT root_identity_scheme, root_identity_value FROM library_metadata_inventory_spools WHERE run_id = ?1",
            [&run.request.run_id], |row| Ok(FileIdentityEvidence { scheme: row.get(0)?, value: row.get(1)? }))
        .expect("captured root identity");
    let execution = fixture
        .catalog
        .initialize_metadata_inventory_spool(&run, &lease, &identity, None, Some("album"), 4_000)
        .expect("capture active execution");
    fixture
        .catalog
        .begin_metadata_inventory_spool_directory(&execution, "album", &identity, 5_000)
        .expect("begin an interrupted directory");
    let entries = (0..257)
        .map(|index| metadata_entry(&format!("album/old-{index:04}.png")))
        .collect::<Vec<_>>();
    for batch in entries.chunks(128) {
        fixture
            .catalog
            .append_metadata_inventory_spool_entries(&execution, "album", batch, 5_000)
            .expect("persist bounded observations before process exit");
    }
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id).entries,
        258
    );
    let mut source = open_inventory_source(&fixture, &scope, &run, &lease);
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id).entries,
        258,
        "opening a continuation must not synchronously delete the old directory prefix"
    );
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let cancellation = AtomicBool::new(false);
    for remaining in [158, 58, 1] {
        assert_eq!(
            source
                .prepare_next_page(100, &cancellation)
                .expect("one bounded reset"),
            MetadataInventorySourcePreparation::Yielded
        );
        assert_eq!(
            spool_rows(&fixture.catalog, &run.request.run_id).entries,
            remaining
        );
        assert_eq!(
            spool_rows(&fixture.catalog, &run.request.run_id).entries_without_directory,
            1
        );
        assert_eq!(
            source_entry_read_count(&fixture.root_path),
            0,
            "no new directory enumeration may mix with its incomplete predecessor"
        );
        drop(source);
        let catalog_path = fixture.catalog.catalog_path().to_path_buf();
        drop(fixture.catalog);
        fixture.catalog =
            SqliteCatalog::open(catalog_path).expect("full reopen after partial reset");
        source = open_inventory_source(&fixture, &scope, &run, &lease);
    }
    let mut ready = false;
    for _ in 0..4 {
        if source
            .prepare_next_page(100, &cancellation)
            .expect("fresh directory enumeration")
            == MetadataInventorySourcePreparation::Ready
        {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let page = source
        .next_page(100, &cancellation)
        .expect("ordered current source evidence");
    assert!(page.is_complete);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["album", "album/retained.png"]
    );
    assert_eq!(source_entry_read_count(&fixture.root_path), 1);
    assert_eq!(fs::read(path).expect("retained source"), bytes);
}

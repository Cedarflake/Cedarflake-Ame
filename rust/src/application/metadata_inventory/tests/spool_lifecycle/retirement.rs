use super::*;

fn subtree_scope() -> MetadataInventoryScope {
    MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    }
}

#[test]
fn cancelled_subtree_spool_retires_initial_observation() {
    assert_terminal_spool_cleanup(subtree_scope(), MetadataInventoryRunStatus::Cancelled);
}

#[test]
fn superseded_subtree_spool_retires_initial_observation() {
    assert_terminal_spool_cleanup(subtree_scope(), MetadataInventoryRunStatus::Superseded);
}

#[test]
fn current_terminal_trigger_retires_initial_observation_before_reopen() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes");
    let (run, _, execution) =
        stage_named_spool_with_execution(&mut fixture, subtree_scope(), "spool-lifecycle");
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    let connection =
        rusqlite::Connection::open(&catalog_path).expect("current trigger fixture connection");
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .expect("current schema version"),
        32
    );
    connection
        .execute(
            "UPDATE library_metadata_inventory_runs
             SET status = 'failed', absence_authority = 0 WHERE id = ?1",
            [&run.request.run_id],
        )
        .expect("current terminal trigger revokes storage authority");
    drop(connection);
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    assert_execution_rejected(&fixture.catalog, &execution);
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("full open preserves retired payload");
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone()).expect("idempotent full reopen");
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    cleanup_inventory_until_idle(&mut fixture, 10_000);
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("full reopen after bounded reclamation");
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        fs::read(source_path).expect("retained source"),
        source_bytes
    );
}

#[test]
fn root_removal_failure_restores_the_initial_observation_and_all_ownership() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes");
    let run = stage_real_spool(&mut fixture, subtree_scope());
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    assert_eq!(before.entries_without_directory, 1);
    let recovery_before = recovery_retirement_evidence(&fixture.catalog, &run.request.run_id);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    let connection = rusqlite::Connection::open(&catalog_path).expect("fault injection connection");
    connection
        .execute_batch(
            "CREATE TRIGGER fixture_reject_spool_retirement
             BEFORE UPDATE OF state ON library_metadata_inventory_spools
             WHEN OLD.state <> 'retired' AND NEW.state = 'retired'
             BEGIN
               SELECT CASE WHEN NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_spool_entries
                 WHERE run_id = OLD.run_id AND directory_relative_path IS NULL
               ) THEN RAISE(ABORT, 'fixture_initial_observation_missing') END;
               SELECT RAISE(ABORT, 'fixture_retirement_failure');
             END;",
        )
        .expect("inject retirement failure while the initial observation remains intact");
    drop(connection);

    let error = fixture
        .catalog
        .unregister_root(&fixture.root_id)
        .expect_err("root removal fails");
    assert!(
        error.message.contains("fixture_retirement_failure"),
        "{error:?}"
    );
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        spool_state(&fixture.catalog, &run.request.run_id).as_deref(),
        Some("ready")
    );
    assert_eq!(
        recovery_retirement_evidence(&fixture.catalog, &run.request.run_id),
        recovery_before
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&run.request.run_id)
            .expect("load run"),
        Some(run.clone())
    );
    let connection = rusqlite::Connection::open(&catalog_path).expect("retire injected failure");
    connection
        .execute_batch("DROP TRIGGER fixture_reject_spool_retirement")
        .expect("remove owned trigger");
    drop(connection);
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("full reopen after rollback");
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        spool_state(&fixture.catalog, &run.request.run_id).as_deref(),
        Some("ready")
    );
    assert_eq!(
        recovery_retirement_evidence(&fixture.catalog, &run.request.run_id),
        recovery_before
    );
    assert!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load root")
            .is_some()
    );
    assert_eq!(
        fs::read(source_path).expect("retained source"),
        source_bytes
    );
}

#[test]
fn completed_subtree_recovery_retires_initial_observation_before_queue_pruning() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes");
    let (run, mut leased, execution) =
        stage_named_spool_with_execution(&mut fixture, subtree_scope(), "spool-lifecycle");
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    let mut complete = false;
    for turn in 0..8 {
        let observed = 6_000 + turn;
        let root = fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load root")
            .expect("root");
        let report = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &leased,
            observed,
            4_095,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("complete real subtree recovery");
        if report.inventory.is_complete {
            complete = true;
            break;
        }
        process_ready_library_changes(
            &mut fixture.catalog,
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            observed,
            queue_policy(),
        )
        .expect("publish any candidates");
        leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                observed + 1,
                queue_policy(),
            )
            .expect("continue recovery lease")
            .expect("recovery continuation");
    }
    assert!(
        complete,
        "two-entry recovery must finish within eight work pages"
    );
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    assert_execution_rejected(&fixture.catalog, &execution);
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("completed recovery with retired payload remains restart-safe");
    let evidence = recovery_retirement_evidence(&fixture.catalog, &run.request.run_id);
    assert_eq!(evidence.len(), 1);
    assert!(evidence[0].0.is_some());
    let retained_candidates = candidate_owner_count(&fixture);
    assert!(
        retained_candidates > 0,
        "real recovery retains candidate ownership"
    );
    fixture
        .catalog
        .cleanup_terminal_library_changes(10_000, 16)
        .expect("prune terminal queue");
    assert_eq!(candidate_owner_count(&fixture), retained_candidates);
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    let cleanup = cleanup_inventory_until_idle(&mut fixture, 10_000);
    assert_eq!(cleanup.removed_run_count, 1);
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(candidate_owner_count(&fixture), 0);
    assert!(
        fixture
            .catalog
            .cleanup_terminal_library_changes(10_000, 16)
            .expect("prune released candidates")
            > 0
    );
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("full reopen after completion and pruning");
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        fs::read(source_path).expect("retained source"),
        source_bytes
    );
}

fn candidate_owner_count(fixture: &InventoryFixture) -> i64 {
    let connection = rusqlite::Connection::open_with_flags(
        fixture.catalog.catalog_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read candidate ownership");
    connection.query_row("SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = 'spool-lifecycle'", [], |row| row.get(0)).expect("count retained candidates")
}

#[test]
fn terminal_cleanup_cannot_cascade_a_still_owned_completed_spool() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes");
    let run = stage_real_spool(&mut fixture, subtree_scope());
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    assert_eq!(
        spool_state(&fixture.catalog, &run.request.run_id).as_deref(),
        Some("ready")
    );
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    let connection =
        rusqlite::Connection::open(&catalog_path).expect("retained completed-state fixture");
    connection
        .execute(
            "UPDATE library_metadata_inventory_runs
             SET status = 'completed', absence_authority = 1, completed_unix_ms = updated_unix_ms
             WHERE id = ?1",
            [&run.request.run_id],
        )
        .expect("retain completed run before authority retirement");
    drop(connection);
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone())
        .expect("full validator accepts retained completed owner");
    let run = fixture
        .catalog
        .load_metadata_inventory_run(&run.request.run_id)
        .expect("read retained run")
        .expect("retained run");
    let cleanup = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(10_000, 16, 16, Default::default())
        .expect("cleanup retains owned spool");
    assert_eq!(
        cleanup,
        crate::domain::MetadataInventoryCleanupReport::default()
    );
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        spool_state(&fixture.catalog, &run.request.run_id).as_deref(),
        Some("ready")
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&run.request.run_id)
            .expect("load retained run"),
        Some(run.clone())
    );
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("full reopen after retained cleanup");
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        fs::read(source_path).expect("retained source"),
        source_bytes
    );
}

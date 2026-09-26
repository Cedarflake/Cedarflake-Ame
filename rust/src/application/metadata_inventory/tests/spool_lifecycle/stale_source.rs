use super::*;

#[test]
fn stale_source_open_cannot_recreate_a_cancelled_inventory_spool() {
    assert_terminal_source_rejected(MetadataInventoryRunStatus::Cancelled);
}

#[test]
fn stale_source_open_cannot_recreate_failed_or_superseded_spools() {
    for status in [
        MetadataInventoryRunStatus::Failed,
        MetadataInventoryRunStatus::Superseded,
    ] {
        assert_terminal_source_rejected(status);
    }
}

fn assert_terminal_source_rejected(status: MetadataInventoryRunStatus) {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let scope = MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    };
    let (stale_run, lease) = begin_named_spool(&mut fixture, scope.clone(), "stale-open");
    drop(open_inventory_source(&fixture, &scope, &stale_run, &lease));
    assert_eq!(stale_run.status, MetadataInventoryRunStatus::Running);
    let identity = spool_root_identity(&fixture, &stale_run.request.run_id);
    let before = spool_rows(&fixture.catalog, &stale_run.request.run_id);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("original generated media");

    fixture
        .catalog
        .terminate_metadata_inventory(&stale_run.request.run_id, status, None, 6_000)
        .expect("commit termination before delayed source initialization");
    assert_retired_spool(&fixture.catalog, &stale_run.request.run_id, &before);
    let result = fixture.catalog.initialize_metadata_inventory_spool(
        &stale_run,
        &lease,
        &identity,
        None,
        Some("album"),
        6_001,
    );
    let retained = spool_rows(&fixture.catalog, &stale_run.request.run_id);
    let reopened = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf());
    assert_eq!(
        fs::read(&source_path).expect("retained media"),
        source_bytes
    );
    assert!(
        result.is_err(),
        "a stale Running snapshot must not recreate terminal spool storage; status={status:?}; rows={retained:?}; reopen={:?}",
        reopened.as_ref().err(),
    );
    assert_eq!(
        result.unwrap_err().code,
        "metadata_inventory_spool_authority_mismatch"
    );
    assert_eq!(retained, before);
    assert_retired_spool(&fixture.catalog, &stale_run.request.run_id, &before);
    reopened.expect("termination remains a valid restart boundary");
    cleanup_inventory_until_idle(&mut fixture, 6_001);
    assert_eq!(
        spool_rows(&fixture.catalog, &stale_run.request.run_id),
        SpoolRows::default()
    );
    assert!(
        fixture
            .catalog
            .initialize_metadata_inventory_spool(
                &stale_run,
                &lease,
                &identity,
                None,
                Some("album"),
                6_002,
            )
            .is_err(),
        "reclamation cannot permit recreation from an old Running snapshot"
    );
    assert_eq!(
        spool_rows(&fixture.catalog, &stale_run.request.run_id),
        SpoolRows::default()
    );
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("reclaimed storage and rejected stale initialization remain valid");
    assert_eq!(
        fs::read(source_path).expect("retained media after cleanup"),
        source_bytes
    );
}

#[test]
fn stale_source_lease_cannot_reset_a_released_or_reacquired_directory() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let scope = MetadataInventoryScope::Root;
    let (run, old_lease) = begin_named_spool(&mut fixture, scope.clone(), "lease-reset");
    drop(open_inventory_source(&fixture, &scope, &run, &old_lease));
    let identity = spool_root_identity(&fixture, &run.request.run_id);
    let execution = fixture
        .catalog
        .initialize_metadata_inventory_spool(&run, &old_lease, &identity, None, Some(""), 4_000)
        .expect("capture directory execution");
    fixture
        .catalog
        .begin_metadata_inventory_spool_directory(&execution, "", &identity, 4_100)
        .expect("begin root directory");
    let mut child = metadata_entry("album");
    child.kind = MetadataInventoryEntryKind::Directory;
    child.file_size = None;
    fixture
        .catalog
        .complete_metadata_inventory_spool_directory(&execution, "", &identity, &[child], 4_200)
        .expect("retain completed parent directory and discover child");
    fixture
        .catalog
        .begin_metadata_inventory_spool_directory(&execution, "album", &identity, 4_300)
        .expect("begin child directory");
    fixture
        .catalog
        .append_metadata_inventory_spool_entries(
            &execution,
            "album",
            &[metadata_entry("album/partial.txt")],
            4_400,
        )
        .expect("stage incomplete child prefix");
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    let before_directory = directory_state(&fixture, &run.request.run_id, "album");
    assert_eq!(
        fixture
            .catalog
            .defer_library_change(old_lease.change.id, old_lease.lease_generation, 4_500)
            .expect("return first source lease"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
    );
    assert_stale_reset_rejected(&mut fixture, &run, &old_lease, &identity);
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        directory_state(&fixture, &run.request.run_id, "album"),
        before_directory
    );

    let current = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            4_600,
            queue_policy(),
        )
        .expect("lease continuation")
        .expect("continuation available");
    assert_eq!(current.change.id, old_lease.change.id);
    assert!(current.lease_generation > old_lease.lease_generation);
    assert_stale_reset_rejected(&mut fixture, &run, &old_lease, &identity);
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        directory_state(&fixture, &run.request.run_id, "album"),
        before_directory
    );

    let successor_execution = fixture
        .catalog
        .initialize_metadata_inventory_spool(&run, &current, &identity, None, Some(""), 4_700)
        .expect("current continuation may reset its incomplete directory");
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    assert_eq!(
        directory_state(&fixture, &run.request.run_id, "album"),
        ("resetting".to_owned(), 1, Some(identity.scheme.clone())),
    );
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("interrupted directory reset is restart-safe");
    assert!(
        fixture
            .catalog
            .reset_metadata_inventory_spool_batch(&execution, 1, 4_800)
            .is_err(),
        "a revoked lease cannot reclaim its successor's raw observations"
    );
    assert!(
        fixture
            .catalog
            .reset_metadata_inventory_spool_batch(&successor_execution, 1, 4_800)
            .expect("one bounded reset batch")
    );
    assert_eq!(
        directory_state(&fixture, &run.request.run_id, "album"),
        ("pending".to_owned(), 0, None),
    );
    assert_eq!(
        directory_state(&fixture, &run.request.run_id, ""),
        ("completed".to_owned(), 1, Some(identity.scheme.clone())),
    );
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id).entries, 1);
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("current continuation preserves the complete spool contract");
}

#[test]
fn source_initialization_rejects_a_stale_run_epoch_without_resetting_rows() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let scope = MetadataInventoryScope::Root;
    let (run, lease) = begin_named_spool(&mut fixture, scope.clone(), "epoch-reset");
    drop(open_inventory_source(&fixture, &scope, &run, &lease));
    let identity = spool_root_identity(&fixture, &run.request.run_id);
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    let mut wrong_epoch = run.clone();
    wrong_epoch.request.epoch += 1;
    assert_stale_reset_rejected(&mut fixture, &wrong_epoch, &lease, &identity);
    assert_eq!(spool_rows(&fixture.catalog, &run.request.run_id), before);
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("run identity mismatch leaves the catalog valid");
}

fn assert_stale_reset_rejected(
    fixture: &mut InventoryFixture,
    run: &MetadataInventoryRun,
    lease: &LeasedLibraryChange,
    identity: &FileIdentityEvidence,
) {
    let error = fixture
        .catalog
        .initialize_metadata_inventory_spool(run, lease, identity, None, Some(""), 4_700)
        .expect_err("stale source cannot reset retained raw entries");
    assert_eq!(error.code, "metadata_inventory_spool_authority_mismatch");
}

fn directory_state(
    fixture: &InventoryFixture,
    run_id: &str,
    relative_directory: &str,
) -> (String, i64, Option<String>) {
    rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("read generated directory evidence")
        .query_row(
            "SELECT state, source_entry_count, directory_identity_scheme
             FROM library_metadata_inventory_spool_directories
             WHERE run_id = ?1 AND relative_directory = ?2",
            [run_id, relative_directory],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained directory state")
}

fn spool_root_identity(fixture: &InventoryFixture, run_id: &str) -> FileIdentityEvidence {
    rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("read generated spool evidence")
        .query_row(
            "SELECT root_identity_scheme, root_identity_value
             FROM library_metadata_inventory_spools WHERE run_id = ?1",
            [run_id],
            |row| {
                Ok(FileIdentityEvidence {
                    scheme: row.get(0)?,
                    value: row.get(1)?,
                })
            },
        )
        .expect("source identity retained by the live spool")
}

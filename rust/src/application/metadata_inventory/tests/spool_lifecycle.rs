use super::*;

mod retirement;
mod scope_isolation;

#[test]
fn subtree_spool_terminal_cleanup_removes_initial_entry_across_reopen() {
    assert_terminal_spool_cleanup(
        MetadataInventoryScope::Subtree {
            relative_path: "album".to_owned(),
        },
        MetadataInventoryRunStatus::Failed,
    );
}

#[test]
fn root_spool_terminal_cleanup_removes_directory_owned_entries_across_reopen() {
    assert_terminal_spool_cleanup(
        MetadataInventoryScope::Root,
        MetadataInventoryRunStatus::Failed,
    );
}

#[test]
fn subtree_spool_root_unregistration_removes_initial_entry_across_reopen() {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes before unregister");
    let run = stage_real_spool(
        &mut fixture,
        MetadataInventoryScope::Subtree {
            relative_path: "album".to_owned(),
        },
    );
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("real subtree inventory is valid before root unregistration");

    assert!(
        fixture
            .catalog
            .unregister_root(&fixture.root_id)
            .expect("unregister root with durable subtree spool")
    );
    let after_unregister = spool_rows(&fixture.catalog, &run.request.run_id);
    let recovery_after_unregister =
        recovery_retirement_evidence(&fixture.catalog, &run.request.run_id);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).unwrap_or_else(|error| {
        panic!(
            "reopen unregistered catalog: {error:?}; raw={after_unregister:?}; \
             authority(retired_unix_ms, queue_status, run_status)={recovery_after_unregister:?}"
        )
    });
    assert!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load removed root")
            .is_none()
    );
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&run.request.run_id)
            .expect("load removed inventory")
            .is_none()
    );
    let after_reopen = spool_rows(&fixture.catalog, &run.request.run_id);
    assert_eq!(
        fs::read(&source_path).expect("retained source bytes"),
        source_bytes
    );
    assert_eq!(
        (after_unregister, after_reopen),
        (SpoolRows::default(), SpoolRows::default()),
        "root removal must retire all raw rows, including the initial subtree entry; before={before:?}",
    );
}

fn assert_terminal_spool_cleanup(
    scope: MetadataInventoryScope,
    status: MetadataInventoryRunStatus,
) {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes before termination");
    let run = stage_real_spool(&mut fixture, scope);
    let before = spool_rows(&fixture.catalog, &run.request.run_id);
    let terminal = fixture
        .catalog
        .terminate_metadata_inventory(
            &run.request.run_id,
            status,
            Some(("fixture_failure", "controlled inventory failure")),
            6_000,
        )
        .expect("terminate real durable inventory");
    assert_eq!(terminal.status, status);
    assert!(!terminal.absence_authority);
    let after_terminal = spool_rows(&fixture.catalog, &run.request.run_id);

    let first = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(6_001, 1, 1)
        .expect("first bounded terminal cleanup");
    assert_eq!((first.removed_entry_count, first.removed_run_count), (1, 0));
    assert!(first.has_more);
    let second = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(6_001, 1, 1)
        .expect("second bounded terminal cleanup");
    assert_eq!(
        (second.removed_entry_count, second.removed_run_count),
        (1, 1)
    );
    assert!(!second.has_more);
    let after_cleanup = spool_rows(&fixture.catalog, &run.request.run_id);

    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen cleaned inventory catalog");
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&run.request.run_id)
            .expect("load expired inventory")
            .is_none()
    );
    assert!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load retained root")
            .is_some()
    );
    let after_reopen = spool_rows(&fixture.catalog, &run.request.run_id);
    assert_eq!(
        fs::read(&source_path).expect("retained source bytes"),
        source_bytes
    );
    assert_eq!(
        (after_terminal, after_cleanup, after_reopen),
        (
            SpoolRows::default(),
            SpoolRows::default(),
            SpoolRows::default()
        ),
        "terminal cleanup must not leave raw rows after reporting no more work; before={before:?}",
    );
}

fn stage_real_spool(
    fixture: &mut InventoryFixture,
    scope: MetadataInventoryScope,
) -> MetadataInventoryRun {
    stage_real_spool_with_lease(fixture, scope).0
}

fn stage_real_spool_with_lease(
    fixture: &mut InventoryFixture,
    scope: MetadataInventoryScope,
) -> (MetadataInventoryRun, LeasedLibraryChange) {
    stage_named_spool(fixture, scope, "spool-lifecycle")
}

fn stage_named_spool(
    fixture: &mut InventoryFixture,
    scope: MetadataInventoryScope,
    run_id: &str,
) -> (MetadataInventoryRun, LeasedLibraryChange) {
    let (kind, change_scope, relative_path) = match &scope {
        MetadataInventoryScope::Root => (
            LibraryChangeIntentKind::FreshnessUnknown,
            LibraryChangeScope::Root,
            String::new(),
        ),
        MetadataInventoryScope::Subtree { relative_path } => (
            LibraryChangeIntentKind::Reconcile,
            LibraryChangeScope::Subtree,
            relative_path.clone(),
        ),
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind,
                scope: change_scope,
                relative_path,
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 4_000,
                most_recent_observed_unix_ms: 4_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            4_000,
            queue_policy(),
        )
        .expect("enqueue real inventory scope");
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            4_000,
            queue_policy(),
        )
        .expect("lease scoped recovery")
        .expect("scoped recovery available");
    assert_eq!(
        super::super::metadata_inventory_scope(&leased).expect("production recovery scope"),
        scope,
    );
    authorize_leased_containment_recovery(fixture, &leased, run_id, 4_000);
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: run_id.to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: scope.clone(),
            started_unix_ms: 4_000,
        })
        .expect("begin real durable inventory");
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(fixture, &scope, &run, &leased);
    let mut ready = false;
    for _ in 0..8 {
        if matches!(
            source
                .prepare_next_page(4_095, &cancellation)
                .expect("prepare tiny durable source"),
            MetadataInventorySourcePreparation::Ready
        ) {
            ready = true;
            break;
        }
    }
    assert!(
        ready,
        "two-entry fixture must finish within eight bounded raw reads"
    );
    let page = source
        .next_page(4_095, &cancellation)
        .expect("read real spool page");
    assert!(page.is_complete);
    assert_eq!(page.entries.len(), 2);
    let run = fixture
        .catalog
        .stage_metadata_inventory_page(run_id, &page, 5_000)
        .expect("stage real durable page");
    assert!(run.enumeration_complete);
    let rows = spool_rows(&fixture.catalog, run_id);
    assert_eq!((rows.spools, rows.entries), (1, 2));
    (run, leased)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct SpoolRows {
    spools: i64,
    directories: i64,
    entries: i64,
    entries_without_directory: i64,
}

fn recovery_retirement_evidence(
    catalog: &SqliteCatalog,
    run_id: &str,
) -> Vec<(Option<i64>, String, Option<String>)> {
    let connection = rusqlite::Connection::open_with_flags(
        catalog.catalog_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open read-only recovery evidence");
    let mut statement = connection
        .prepare(
            "SELECT authority.retired_unix_ms, queue.status, run.status
             FROM library_recovery_authorities AS authority
             JOIN library_change_queue AS queue ON queue.id = authority.change_id
             LEFT JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
             WHERE authority.run_id = ?1 ORDER BY authority.change_id",
        )
        .expect("prepare recovery retirement evidence");
    statement
        .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("read recovery retirement evidence")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode recovery retirement evidence")
}

fn spool_rows(catalog: &SqliteCatalog, run_id: &str) -> SpoolRows {
    let connection = rusqlite::Connection::open_with_flags(
        catalog.catalog_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open read-only spool evidence");
    connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries
                WHERE run_id = ?1 AND directory_relative_path IS NULL)",
            [run_id],
            |row| {
                Ok(SpoolRows {
                    spools: row.get(0)?,
                    directories: row.get(1)?,
                    entries: row.get(2)?,
                    entries_without_directory: row.get(3)?,
                })
            },
        )
        .expect("count durable spool rows")
}

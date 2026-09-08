use super::*;
use crate::adapters::MetadataInventorySpoolExecution;
use rusqlite::OptionalExtension;

mod bounded_reset;
mod bounded_retirement;
mod execution_admission;
mod retirement;
mod scope_isolation;
mod source_execution;
mod stale_source;

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
    let (run, _, execution) = stage_named_spool_with_execution(
        &mut fixture,
        MetadataInventoryScope::Subtree {
            relative_path: "album".to_owned(),
        },
        "spool-lifecycle",
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
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    assert_execution_rejected(&fixture.catalog, &execution);
    let recovery_after_unregister =
        recovery_retirement_evidence(&fixture.catalog, &run.request.run_id);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone()).unwrap_or_else(|error| {
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
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    assert_execution_rejected(&fixture.catalog, &execution);
    cleanup_inventory_until_idle(&mut fixture, 6_001);
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("reopen fully reclaimed removed root");
    assert_eq!(
        spool_rows(&fixture.catalog, &run.request.run_id),
        SpoolRows::default()
    );
    assert_eq!(
        fs::read(&source_path).expect("retained source bytes"),
        source_bytes
    );
    assert_eq!(
        (after_unregister, after_reopen),
        (before, before),
        "root removal and reopen revoke authority without deleting raw observations",
    );
}

fn assert_terminal_spool_cleanup(
    scope: MetadataInventoryScope,
    status: MetadataInventoryRunStatus,
) {
    let mut fixture = InventoryFixture::new(&["album/retained.png"]);
    let source_path = fixture.source.path().join("album/retained.png");
    let source_bytes = fs::read(&source_path).expect("source bytes before termination");
    let (run, _, execution) =
        stage_named_spool_with_execution(&mut fixture, scope, "spool-lifecycle");
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
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    assert_execution_rejected(&fixture.catalog, &execution);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("reopen before physical cleanup");
    assert_retired_spool(&fixture.catalog, &run.request.run_id, &before);
    let cleanup = cleanup_inventory_until_idle(&mut fixture, 6_001);
    assert_eq!(
        cleanup.removed_entry_count, 4,
        "two raw and two logical entries share the budget"
    );
    assert_eq!(cleanup.removed_run_count, 1);
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
        (before, SpoolRows::default(), SpoolRows::default()),
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
    let (run, leased) = begin_named_spool(fixture, scope.clone(), run_id);
    stage_opened_spool(fixture, &scope, run, leased)
}

fn stage_named_spool_with_execution(
    fixture: &mut InventoryFixture,
    scope: MetadataInventoryScope,
    run_id: &str,
) -> (
    MetadataInventoryRun,
    LeasedLibraryChange,
    MetadataInventorySpoolExecution,
) {
    let (run, leased) = begin_named_spool(fixture, scope.clone(), run_id);
    stage_opened_spool_with_execution(fixture, &scope, run, leased)
}

fn begin_named_spool(
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
    (run, leased)
}

fn stage_opened_spool(
    fixture: &mut InventoryFixture,
    scope: &MetadataInventoryScope,
    run: MetadataInventoryRun,
    leased: LeasedLibraryChange,
) -> (MetadataInventoryRun, LeasedLibraryChange) {
    let (run, leased, _) = stage_opened_spool_with_execution(fixture, scope, run, leased);
    (run, leased)
}

fn stage_opened_spool_with_execution(
    fixture: &mut InventoryFixture,
    scope: &MetadataInventoryScope,
    run: MetadataInventoryRun,
    leased: LeasedLibraryChange,
) -> (
    MetadataInventoryRun,
    LeasedLibraryChange,
    MetadataInventorySpoolExecution,
) {
    let run_id = &run.request.run_id;
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(fixture, scope, &run, &leased);
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
    let identity = fixture
        .catalog
        .load_metadata_inventory_root_identity(run_id)
        .expect("read ready source identity")
        .expect("ready source identity");
    let execution = fixture
        .catalog
        .initialize_metadata_inventory_spool(&run, &leased, &identity, None, None, 5_000)
        .expect("capture the current ready execution before logical publication");
    fixture
        .catalog
        .validate_metadata_inventory_spool_execution(&execution)
        .expect("the original execution is current");
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
    (run, leased, execution)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SpoolRows {
    spools: i64,
    directories: i64,
    entries: i64,
    entries_without_directory: i64,
}

fn spool_state(catalog: &SqliteCatalog, run_id: &str) -> Option<String> {
    rusqlite::Connection::open_with_flags(
        catalog.catalog_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read spool state")
    .query_row(
        "SELECT state FROM library_metadata_inventory_spools WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )
    .optional()
    .expect("decode spool state")
}

fn assert_retired_spool(catalog: &SqliteCatalog, run_id: &str, before: &SpoolRows) {
    assert_eq!(
        spool_rows(catalog, run_id),
        *before,
        "retirement cannot reclaim raw payload"
    );
    assert_eq!(spool_state(catalog, run_id).as_deref(), Some("retired"));
}

fn assert_execution_rejected(catalog: &SqliteCatalog, execution: &MetadataInventorySpoolExecution) {
    assert!(
        catalog
            .validate_metadata_inventory_spool_execution(execution)
            .is_err()
    );
    assert!(
        catalog
            .metadata_inventory_spool_is_ready(execution)
            .is_err()
    );
    assert!(
        catalog
            .load_metadata_inventory_spool_page(execution, 4)
            .is_err()
    );
}

fn cleanup_inventory_until_idle(
    fixture: &mut InventoryFixture,
    terminal_before_unix_ms: i64,
) -> crate::domain::MetadataInventoryCleanupReport {
    let mut total = crate::domain::MetadataInventoryCleanupReport::default();
    for _ in 0..32 {
        let batch = fixture
            .catalog
            .cleanup_terminal_metadata_inventories(
                terminal_before_unix_ms,
                1,
                1,
                Default::default(),
            )
            .expect("one bounded raw and logical cleanup batch");
        assert!(
            batch.removed_entry_count <= 1,
            "raw and logical entries share one budget"
        );
        assert!(batch.removed_run_count <= 1);
        total.removed_entry_count += batch.removed_entry_count;
        total.removed_run_count += batch.removed_run_count;
        SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
            .expect("every partial cleanup state passes a fresh full open");
        if !batch.has_more {
            return total;
        }
    }
    panic!("tiny lifecycle fixture failed to finish within 32 bounded cleanup batches");
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

use super::super::cleanup::cleanup_terminal_inventory_batch;
use super::super::{InventoryExecution, finish_started_metadata_inventory};
use super::*;

#[test]
fn bounded_cleanup_retires_candidate_ownership_before_deleting_a_summary() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin owned inventory");
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    let mut connection =
        rusqlite::Connection::open(&catalog_path).expect("controlled metadata fixture");
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("retain foreign keys");
    let transaction = connection
        .transaction()
        .expect("candidate fixture transaction");
    for index in 0..257 {
        let path = format!("observed-{index:04}.txt");
        transaction.execute("INSERT INTO library_change_queue(
            id, root_id, root_generation, intent_kind, scope, relative_path, origin,
            first_observed_unix_ms, most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
            coalesced_observation_count, status, ready_unix_ms, catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms)
            VALUES (?1, ?2, 1, 'reconcile', 'path', ?3, 'metadata_inventory', 2100, 2100, '1', '1', 1, 'pending', 2100, 0, 2100, 2100)",
            rusqlite::params![10_000 + index, fixture.root_id, path]).expect("scoped candidate queue");
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_candidate_owners(
            run_id, candidate_key, change_id, candidate_role, relative_path, owned_unix_ms)
            VALUES (?1, ?2, ?3, 'present', ?2, 2100)",
                rusqlite::params![request.run_id, path, 10_000 + index],
            )
            .expect("candidate lineage");
    }
    transaction.commit().expect("persist candidate debt");
    drop(connection);
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "controlled failure")),
            2_200,
        )
        .expect("terminate owned inventory");
    let retained = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 100, 1, Default::default())
        .expect("retention protects candidate lineage");
    assert!(!retained.has_more);
    for (remaining, removed_runs) in [(157, 0), (57, 0), (0, 1)] {
        let report = fixture
            .catalog
            .cleanup_terminal_metadata_inventories(3_000, 100, 1, Default::default())
            .expect("bounded ownership retirement");
        assert_eq!(report.removed_entry_count, 0);
        assert_eq!(report.removed_run_count, removed_runs);
        assert_eq!(report.has_more, remaining > 0);
        let count: i64 = rusqlite::Connection::open(&catalog_path).expect("read retained owners")
            .query_row("SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = ?1",
                [&request.run_id], |row| row.get(0)).expect("owner count");
        assert_eq!(
            count, remaining,
            "run deletion cannot cascade the remaining owner set"
        );
        drop(fixture.catalog);
        fixture.catalog = SqliteCatalog::open(catalog_path.clone())
            .expect("FULL reopen of partially reclaimed lineage");
    }
}

#[test]
fn cancellation_reports_bounded_cleanup_pending_and_preserves_the_remaining_page() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin inventory");
    for (page_index, start, end) in [(1, 0, 4_096), (2, 4_096, 4_097)] {
        let cursor = format!("entry-{:05}.txt", end - 1);
        fixture
            .catalog
            .stage_metadata_inventory_page(
                &request.run_id,
                &MetadataInventoryPage {
                    page_index,
                    entries: (start..end)
                        .map(|index| metadata_entry(&format!("entry-{index:05}.txt")))
                        .collect(),
                    cursor: Some(cursor.clone()),
                    is_complete: false,
                    frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                        0,
                        "",
                        None,
                        Some(cursor),
                        end,
                    )],
                },
                2_100,
            )
            .expect("stage a bounded logical page");
    }
    let cancelled = AtomicBool::new(true);
    let report = finish_started_metadata_inventory(
        &mut fixture.catalog,
        Some(&mut FailingInventorySource),
        &request,
        InventoryExecution {
            authority: None,
            invalidates_unchanged_sources: false,
            yield_after_work_page: false,
            observed_unix_ms: 2_200,
            page_limit: 4_096,
            queue_policy: queue_policy(),
            control: MetadataInventoryWorkerControl::without_progress(&cancelled),
        },
    )
    .expect("cancellation preserves successful bounded cleanup progress");
    assert!(report.is_cancelled);
    assert!(report.cleanup_pending);
    let next = cleanup_terminal_inventory_batch(&mut fixture.catalog, 2_200, Default::default())
        .expect("a later owner consumes the remaining batch");
    assert_eq!(
        next.removed_entry_count, 1,
        "the application must not drain to exhaustion"
    );
    assert!(!next.has_more);
    assert_eq!(
        next.removed_run_count, 0,
        "the run retains its existing retention period"
    );
}

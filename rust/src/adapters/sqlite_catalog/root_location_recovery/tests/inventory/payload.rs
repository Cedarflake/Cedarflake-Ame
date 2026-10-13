use crate::domain::{
    MetadataInventoryComparisonStatus, MetadataInventoryComparisonUpdate,
    MetadataInventoryRunStatus, MetadataInventoryStartRequest,
};

use super::*;

mod native_fixture;

#[test]
fn root_location_recovery_cleans_logical_payload_and_retains_raw_storage_for_maintenance() {
    let fixture = Fixture::new();
    let source_bytes: Vec<_> = fixture
        .before
        .iter()
        .map(|asset| fs::read(fixture.moved.join(&asset.relative_path)).unwrap())
        .collect();
    let mut catalog = fixture.catalog();
    seed_inventory(&mut catalog, &fixture.root, 3_073);
    seed_raw_spool(&catalog, 3_073);
    seed_candidates(&mut catalog, &fixture.root, 2_305);
    drop(catalog);
    let mut catalog = fixture.catalog();
    let located = fixture.located();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let mut catalog = fixture.catalog();
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_entries"),
        0
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_candidate_owners"),
        0
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_spool_entries"),
        3_073
    );
    let history: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue
         WHERE root_id = ?1 AND root_generation = ?2 AND origin = 'metadata_inventory'
           AND status = 'superseded'",
            params![
                fixture.root.root_id,
                sqlite_integer(fixture.root.root_generation.value(), "fixture generation").unwrap()
            ],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(history, 2_305);
    let published = assets(&mut catalog);
    assert_eq!(published.len(), fixture.before.len());
    for ((old, new), original_bytes) in fixture.before.iter().zip(&published).zip(&source_bytes) {
        assert_eq!(old.asset_id, new.asset_id);
        assert_eq!(old.location_id, new.location_id);
        assert_eq!(old.source_generation, new.source_generation);
        assert_eq!(old.preview_path, new.preview_path);
        assert_eq!(fs::read(&new.absolute_path).unwrap(), *original_bytes);
    }
    let cleanup = catalog
        .cleanup_terminal_metadata_inventories(unix_time_ms(), 128, 1, Default::default())
        .unwrap();
    assert_eq!(cleanup.removed_entry_count, 128);
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_spool_entries"),
        2_945
    );
    drop(catalog);
    drop(fixture.catalog());
}

#[test]
fn root_location_recovery_cancelled_after_a_page_preserves_resumption_when_the_directory_returns() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_inventory(&mut catalog, &fixture.root, 3_073);
    seed_raw_spool(&catalog, 3_073);
    let located = fixture.located();
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    super::super::cost::observe_next_page(move || signal.store(true, Ordering::Release));
    assert_eq!(
        catalog
            .recover_located_root(&fixture.root, &located, &cancelled)
            .unwrap_err()
            .code,
        "root_location_recovery_cancelled"
    );
    assert!(cancelled.load(Ordering::Acquire));
    drop(catalog);
    drop(located);
    fs::rename(&fixture.moved, &fixture.original).unwrap();
    let mut catalog = fixture.catalog();
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_entries"),
        3_073
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_spool_entries"),
        3_073
    );
    let current = catalog
        .load_incremental_catalog_root(&fixture.root.root_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.root_path, fixture.root.root_path);
    assert_eq!(current.root_generation, fixture.root.root_generation);
    let resumed = catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "old-inventory".into(),
            root_id: fixture.root.root_id.clone(),
            root_generation: fixture.root.root_generation,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: unix_time_ms(),
        })
        .unwrap();
    assert_eq!(resumed.status, MetadataInventoryRunStatus::Comparing);
    catalog
        .record_metadata_inventory_comparisons(
            "old-inventory",
            &[MetadataInventoryComparisonUpdate {
                relative_path: "entry-3072.png".into(),
                status: MetadataInventoryComparisonStatus::Unchanged,
                candidate_previous_relative_path: None,
            }],
            unix_time_ms(),
        )
        .unwrap();
    drop(catalog);
    fs::rename(&fixture.original, &fixture.moved).unwrap();
    let mut catalog = fixture.catalog();
    catalog
        .recover_located_root(&fixture.root, &fixture.located(), &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    drop(fixture.catalog());
}

#[test]
fn root_location_recovery_rejects_stale_binding_before_inventory_payload_changes() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_inventory(&mut catalog, &fixture.root, 1_025);
    let mut stale = fixture.root.clone();
    stale.root_path.push_str("-different-binding");
    assert_eq!(
        catalog
            .recover_located_root(&stale, &fixture.located(), &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "catalog_root_relocation_stale"
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_entries"),
        1_025
    );
    assert_eq!(
        catalog
            .load_metadata_inventory_run("old-inventory")
            .unwrap()
            .unwrap()
            .status,
        MetadataInventoryRunStatus::Comparing
    );
    drop(catalog);
    drop(fixture.catalog());
}

fn payload_count(catalog: &SqliteCatalog, table: &str) -> i64 {
    catalog
        .connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE run_id = 'old-inventory'"),
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn seed_raw_spool(catalog: &SqliteCatalog, entry_count: i64) {
    catalog.connection.execute(
        "INSERT INTO library_metadata_inventory_spools(
           run_id, authority_change_id, root_id, root_generation, root_identity_scheme, root_identity_value,
           scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms)
         SELECT authority.run_id, authority.change_id, authority.root_id, authority.root_generation,
                namespace.identity_scheme, namespace.identity_value, 'root', '', 'ready',
                authority.authorized_unix_ms, authority.authorized_unix_ms
         FROM library_recovery_authorities AS authority
         JOIN library_root_publication_namespaces AS namespace ON namespace.root_id = authority.root_id
         WHERE authority.run_id = 'old-inventory'", [],
    ).unwrap();
    catalog.connection.execute(
        "INSERT INTO library_metadata_inventory_spool_directories(
           run_id, ordinal, relative_directory, state, directory_identity_scheme, directory_identity_value,
           source_entry_count, created_unix_ms, updated_unix_ms)
         SELECT run_id, 0, '', 'completed', root_identity_scheme, root_identity_value, ?1,
                created_unix_ms, updated_unix_ms
         FROM library_metadata_inventory_spools WHERE run_id = 'old-inventory'", [entry_count],
    ).unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO library_metadata_inventory_spool_entries(
           run_id, directory_relative_path, relative_path, entry_kind, file_size, modified_unix_ms,
           placeholder_state, is_reparse_point, staged_unix_ms)
         SELECT run_id, '', relative_path, entry_kind, file_size, modified_unix_ms,
                placeholder_state, is_reparse_point, staged_unix_ms
         FROM library_metadata_inventory_entries WHERE run_id = 'old-inventory'",
            [],
        )
        .unwrap();
}

fn seed_candidates(catalog: &mut SqliteCatalog, root: &IncrementalCatalogRoot, count: u32) {
    let now = unix_time_ms();
    let transaction = catalog.connection.transaction().unwrap();
    {
        let mut queue = transaction.prepare(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path, origin,
               first_observed_unix_ms, most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms)
             VALUES (?1, ?2, ?3, 'reconcile', 'path', ?4, 'metadata_inventory',
                     ?5, ?5, '1', '1', 1, 'pending', ?5, 0, ?5, ?5)"
        ).unwrap();
        let mut owner = transaction
            .prepare(
                "INSERT INTO library_metadata_inventory_candidate_owners(
               run_id, candidate_key, change_id, candidate_role, relative_path, owned_unix_ms)
             VALUES ('old-inventory', ?1, ?2, 'present', ?1, ?3)",
            )
            .unwrap();
        for index in 0..count {
            let change_id = i64::from(index) + 10_000;
            let path = format!("candidate-{index:05}.png");
            queue
                .execute(params![
                    change_id,
                    root.root_id,
                    sqlite_integer(root.root_generation.value(), "fixture generation").unwrap(),
                    path,
                    now
                ])
                .unwrap();
            owner.execute(params![path, change_id, now]).unwrap();
        }
    }
    transaction.commit().unwrap();
}

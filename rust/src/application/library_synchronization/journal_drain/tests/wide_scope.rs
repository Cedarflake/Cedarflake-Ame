use super::*;
use crate::application::incremental_library_changes::process_ready_metadata_inventory_recovery_candidates_cancellable;
use crate::application::metadata_inventory::{
    MetadataInventoryRecoveryExecution,
    process_leased_metadata_inventory_change_with_retained_source,
};

#[test]
fn oversized_directory_rename_recovers_only_two_subtrees_across_reopen() {
    let source = tempdir().unwrap();
    let derived = tempdir().unwrap();
    fs::create_dir(source.path().join("old")).unwrap();
    fs::create_dir(source.path().join("unrelated")).unwrap();
    write_image(&source.path().join("unrelated/retained.png"), [10, 20, 30]);
    for index in 0..160_u16 {
        write_image(
            &source.path().join(format!("old/{index:03}.png")),
            [40, 60, 80],
        );
    }
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog/ame.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "wide-journal-rename".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 96,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let root = catalog.load_incremental_catalog_roots().unwrap().remove(0);
    let before = catalog
        .load_incremental_locations_in_subtree(&root.root_id, "old", 256)
        .unwrap();
    assert_eq!(before.len(), 160);
    let bytes = fs::read(source.path().join("old/000.png")).unwrap();
    fs::rename(source.path().join("old"), source.path().join("new")).unwrap();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..Default::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                kind: LibraryChangeIntentKind::RenameCandidate,
                scope: LibraryChangeScope::Subtree,
                relative_path: "new".to_owned(),
                previous_relative_path: Some("old".to_owned()),
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .unwrap();
    let cancellation = AtomicBool::new(false);
    let promoted = process_ready(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        2_000,
        policy,
        AuthoritativeRecoveryPolicy::default(),
        &cancellation,
    )
    .unwrap();
    assert_eq!(promoted.completed_count, 0);
    assert_eq!(promoted.retried_count, 1);
    assert!(
        !catalog
            .has_ready_journal_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                2_000,
                policy
            )
            .unwrap()
    );
    drop(catalog);
    let mut catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("reopen after handoff");
    let mut retained_source = None;
    let mut reopened_during_inventory = false;
    let mut converged = false;
    for tick in 0..300_i64 {
        let now = 3_000 + tick * 1_000;
        process_ready_metadata_inventory_recovery_candidates_cancellable(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            now,
            policy,
            &cancellation,
        )
        .unwrap();
        if let Some(leased) = catalog
            .lease_metadata_inventory_recovery(&root.root_id, root.root_generation, now, policy)
            .unwrap()
        {
            let current_root = catalog
                .load_incremental_catalog_root(&root.root_id)
                .unwrap()
                .unwrap();
            let page = process_leased_metadata_inventory_change_with_retained_source(
                &mut catalog,
                &current_root,
                &leased,
                MetadataInventoryRecoveryExecution::without_progress(
                    now,
                    32,
                    policy,
                    &cancellation,
                ),
                retained_source.take(),
            )
            .unwrap();
            retained_source = page.retained_source;
            if !reopened_during_inventory && !page.report.inventory.is_complete {
                drop(retained_source.take());
                drop(catalog);
                catalog = SqliteCatalog::open(storage.catalog_path.clone())
                    .expect("full reopen during scoped inventory");
                reopened_during_inventory = true;
            }
        }
        process_ready(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            now,
            policy,
            AuthoritativeRecoveryPolicy::default(),
            &cancellation,
        )
        .unwrap();
        let connection = rusqlite::Connection::open(&storage.catalog_path).unwrap();
        let remaining: i64 = connection.query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE status IN ('pending','leased','retry_wait')",
            [], |row| row.get(0),
        ).unwrap();
        if remaining == 0 {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "oversized journal scope reaches terminal publication"
    );
    assert!(reopened_during_inventory);
    let after = catalog
        .load_incremental_locations_in_subtree(&root.root_id, "new", 256)
        .unwrap();
    assert_eq!(after.len(), 160);
    assert!(
        catalog
            .load_incremental_locations_in_subtree(&root.root_id, "old", 256)
            .unwrap()
            .is_empty()
    );
    for previous in before {
        let path = previous.relative_path.replacen("old/", "new/", 1);
        let current = after
            .iter()
            .find(|location| location.relative_path == path)
            .unwrap();
        assert_eq!(current.asset_id, previous.asset_id);
        assert_eq!(current.file_identity, previous.file_identity);
    }
    assert_eq!(fs::read(source.path().join("new/000.png")).unwrap(), bytes);
    for index in 0..160_u16 {
        assert_eq!(
            fs::read(source.path().join(format!("new/{index:03}.png"))).unwrap(),
            bytes
        );
    }
    assert_eq!(
        catalog
            .load_incremental_locations_in_subtree(&root.root_id, "unrelated", 256)
            .unwrap()
            .len(),
        1
    );
    let connection = rusqlite::Connection::open(&storage.catalog_path).unwrap();
    let scan_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(scan_count, 1);
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_relative_path FROM library_metadata_inventory_runs ORDER BY scope_relative_path",
    ).unwrap();
    let scopes = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        scopes,
        [
            ("subtree".to_owned(), "new".to_owned()),
            ("subtree".to_owned(), "old".to_owned())
        ]
    );
    drop(retained_source);
    drop(catalog);
    SqliteCatalog::open(storage.catalog_path).expect("full reopen after ordered journal recovery");
}

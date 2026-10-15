use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::adapters::SqliteCatalog;
use crate::domain::{
    CatalogDeltaBatch, LibraryChangeCompletion, LibraryChangeId, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration,
};
use crate::ports::LibraryChangeQueue;

#[test]
fn completing_one_recheck_does_not_walk_the_remaining_roster() {
    let mut counts = Vec::new();
    for entries in [64, 4096] {
        let directory = tempfile::tempdir().unwrap();
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        catalog
            .connection
            .execute_batch(
                "INSERT INTO library_roots(id,path,created_unix_ms) VALUES('root','C:\\fixture',1);
             INSERT INTO library_change_root_state(root_id,generation,is_active,updated_unix_ms)
               VALUES('root',1,1,1);
             INSERT INTO assets(id,created_unix_ms) VALUES('retained-asset',1);",
            )
            .unwrap();
        let transaction = catalog.connection.transaction().unwrap();
        for index in 0..entries {
            transaction
                .execute(
                    "INSERT INTO library_media_admission_rechecks(
                   root_id,root_generation,relative_path,location_id,asset_id,state)
                 VALUES('root',1,?1,?1,'retained-asset','pending')",
                    [format!("item-{index:05}.png")],
                )
                .unwrap();
        }
        transaction.commit().unwrap();
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: "root".to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "item-00000.png".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::ConsistencyAudit,
                    first_observed_unix_ms: 1000,
                    most_recent_observed_unix_ms: 1000,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                1000,
                LibraryChangeQueuePolicy::default(),
            )
            .unwrap();
        let change_id = catalog
            .connection
            .query_row(
                "SELECT id FROM library_change_queue WHERE relative_path='item-00000.png'",
                [],
                |row| row.get::<_, u64>(0),
            )
            .unwrap();
        let batch = CatalogDeltaBatch {
            root_id: "root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            expected_catalog_revision: 0,
            mutations: Vec::new(),
            terminal_media_evidence: Vec::new(),
            completions: vec![LibraryChangeCompletion {
                change_id: LibraryChangeId::new(change_id).unwrap(),
                lease_generation: 1,
                issue: None,
            }],
        };
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&steps);
        catalog
            .connection
            .progress_handler(
                1,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        let transaction = catalog.connection.transaction().unwrap();
        super::finish_publication(&transaction, &batch).unwrap();
        transaction.commit().unwrap();
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        counts.push(steps.load(Ordering::Relaxed));
        let remaining = catalog
            .connection
            .query_row(
                "SELECT count(*) FROM library_media_admission_rechecks",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(remaining, entries - 1);
    }
    assert!(
        counts[1] <= counts[0] * 2,
        "One completion must stay bounded as unrelated rechecks grow: {counts:?}"
    );
}

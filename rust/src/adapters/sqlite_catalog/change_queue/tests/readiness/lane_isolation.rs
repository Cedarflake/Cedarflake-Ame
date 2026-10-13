use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;

#[test]
fn foreground_readiness_work_is_independent_of_recovery_backlog() {
    let cases: [(LibraryChangeOrigin, LibraryChangeScope, Readiness); 3] = [
        (
            LibraryChangeOrigin::LiveNotification,
            LibraryChangeScope::Path,
            SqliteCatalog::has_ready_live_path_library_change,
        ),
        (
            LibraryChangeOrigin::StartupCatchUp,
            LibraryChangeScope::Path,
            SqliteCatalog::has_ready_journal_path_library_change,
        ),
        (
            LibraryChangeOrigin::LiveNotification,
            LibraryChangeScope::Root,
            SqliteCatalog::has_ready_live_authoritative_library_change,
        ),
    ];
    for (origin, scope, ready) in cases {
        for has_target in [false, true] {
            let mut costs = Vec::new();
            for backlog in [256, 3_072] {
                let directory = tempdir().expect("isolated queue");
                let path = directory.path().join("catalog.sqlite3");
                let mut catalog = queue_catalog(path.clone());
                seed_recovery_backlog(&catalog, backlog);
                let generation = LibraryRootGeneration::initial();
                let policy = LibraryChangeQueuePolicy {
                    max_unresolved_changes: 4_096,
                    ..immediate_policy()
                };
                if has_target {
                    let mut change = intent(
                        "root-a",
                        generation,
                        1,
                        1_000,
                        if scope == LibraryChangeScope::Path {
                            LibraryChangeIntentKind::Reconcile
                        } else {
                            LibraryChangeIntentKind::FreshnessUnknown
                        },
                        scope,
                        if scope == LibraryChangeScope::Path {
                            "new.jpg"
                        } else {
                            ""
                        },
                    );
                    change.origin = origin;
                    catalog
                        .enqueue_library_change_intents(&[change], 1_000, policy)
                        .expect("admit the foreground work after recovery");
                }
                drop(catalog);
                let catalog = SqliteCatalog::open(path).expect("full queue validation");
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
                    .expect("observe actual readiness queries");
                let result = ready(&catalog, "root-a", generation, 1_001, policy);
                catalog
                    .connection
                    .progress_handler(0, None::<fn() -> bool>)
                    .expect("retire observation");
                assert_eq!(result.expect("foreground readiness"), has_target);
                costs.push(steps.load(Ordering::Relaxed));
            }
            eprintln!(
                "readiness origin={origin:?} scope={scope:?} target={has_target} VM steps={costs:?}"
            );
            assert!(
                costs[1] <= costs[0] * 2,
                "unrelated recovery work must not grow foreground polling: {costs:?}"
            );
        }
    }
}

fn seed_recovery_backlog(catalog: &SqliteCatalog, entries: u32) {
    catalog
        .connection
        .execute(
            "WITH RECURSIVE entries(ordinal) AS (
           SELECT 1 UNION ALL SELECT ordinal + 1 FROM entries WHERE ordinal < ?1
         )
         INSERT INTO library_change_queue(
           root_id, root_generation, intent_kind, scope, relative_path, origin,
           first_observed_unix_ms, most_recent_observed_unix_ms,
           first_sequence, most_recent_sequence, coalesced_observation_count,
           status, ready_unix_ms, attempt_count, catalog_revision_at_enqueue,
           created_unix_ms, updated_unix_ms
         )
         SELECT 'root-a', 1, 'reconcile', 'path', 'backlog-' || ordinal || '.jpg',
           'consistency_audit', 1, 1, '1', '1', 1, 'pending', 1, 0, 0, 1, 1
         FROM entries",
            [entries],
        )
        .expect("retained recovery queue with real lane triggers");
}

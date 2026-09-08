use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;

#[derive(Clone, Copy, Debug)]
enum UnrelatedHistory {
    OtherRootTerminal,
    OldGenerationTerminal,
    OtherRootExhausted,
    OtherRootExplicitClaims,
}

#[test]
fn root_metrics_work_is_independent_of_other_root_terminal_history() {
    assert_root_work(UnrelatedHistory::OtherRootTerminal);
}

#[test]
fn root_metrics_work_is_independent_of_old_generation_history() {
    assert_root_work(UnrelatedHistory::OldGenerationTerminal);
}

#[test]
fn root_metrics_work_is_independent_of_newer_other_root_failures() {
    assert_root_work(UnrelatedHistory::OtherRootExhausted);
}

#[test]
fn root_metrics_work_is_independent_of_other_root_explicit_claims() {
    assert_root_work(UnrelatedHistory::OtherRootExplicitClaims);
}

fn assert_root_work(history: UnrelatedHistory) {
    for has_target in [true, false] {
        let mut counts = Vec::new();
        for entries in [256_u32, 4_096] {
            let directory = tempdir().expect("temporary directory");
            let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
            let generation = LibraryRootGeneration::new(2).expect("current generation");
            let policy = immediate_policy();
            let transaction = catalog
                .connection
                .transaction()
                .expect("generation transaction");
            assert!(matches!(
                establish_root_generation(&transaction, "root-a", generation, 2),
                Ok(GenerationDisposition::Current {
                    superseded_count: 0
                })
            ));
            transaction
                .commit()
                .expect("advance complete generation authority");
            if has_target {
                catalog
                    .enqueue_library_change_intents(
                        &[path_intent("root-a", generation, 1, 1_000, "current.png")],
                        1_000,
                        policy,
                    )
                    .expect("one current-root observation");
                if matches!(history, UnrelatedHistory::OtherRootExhausted) {
                    catalog
                        .connection
                        .execute_batch(
                            "UPDATE library_change_queue SET status = 'retry_wait',
                               attempt_count = 4, next_retry_unix_ms = 1,
                               last_failure_code = 'target-failure',
                               last_failure_message = 'Target failure'
                             WHERE root_id = 'root-a'",
                        )
                        .expect("target failure precedes unrelated failures");
                }
            }
            seed_history(&mut catalog, history, entries);
            drop(catalog);
            let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
                .expect("FULL reopen of the complete history fixture");
            let (metrics, steps) = measured_root_metrics(&catalog, generation, policy);
            let exhausted_target =
                has_target && matches!(history, UnrelatedHistory::OtherRootExhausted);
            assert_eq!(
                metrics.pending_count,
                u64::from(has_target && !exhausted_target)
            );
            assert_eq!(metrics.retry_wait_count, u64::from(exhausted_target));
            assert_eq!(metrics.completed_count, 0);
            assert_eq!(metrics.explicit_recovery_required_count, 0);
            assert_eq!(
                metrics.latest_exhausted_failure_code.as_deref(),
                exhausted_target.then_some("target-failure"),
            );
            let global = catalog
                .load_library_change_queue_metrics(1_001, policy)
                .expect("global metrics still include all roots and generations");
            match history {
                UnrelatedHistory::OtherRootTerminal | UnrelatedHistory::OldGenerationTerminal => {
                    assert_eq!(global.completed_count, u64::from(entries));
                }
                UnrelatedHistory::OtherRootExhausted => {
                    assert_eq!(
                        global.exhausted_retry_count,
                        u64::from(entries) + u64::from(has_target)
                    );
                    assert_eq!(
                        global.latest_exhausted_failure_code.as_deref(),
                        Some("other-failure")
                    );
                }
                UnrelatedHistory::OtherRootExplicitClaims => {
                    assert_eq!(global.explicit_recovery_required_count, u64::from(entries));
                }
            }
            counts.push(steps);
        }
        eprintln!("root metrics VM steps history={history:?} target={has_target}: {counts:?}");
        assert!(
            counts[1] <= counts[0] * 2,
            "one root must not traverse unrelated history: {history:?} target={has_target} {counts:?}"
        );
    }
}

#[test]
fn root_metrics_remain_readable_without_a_hard_eligible_index_dependency() {
    for has_wrong_definition in [false, true] {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = queue_catalog(path.clone());
        seed_history(&mut catalog, UnrelatedHistory::OtherRootExhausted, 4_096);
        catalog
            .connection
            .execute_batch("DROP INDEX library_change_queue_eligible")
            .expect("remove optional access path");
        if has_wrong_definition {
            catalog.connection.execute_batch(
                "CREATE INDEX library_change_queue_eligible ON library_change_queue(status, id)",
            ).expect("same-name different access path");
        }
        drop(catalog);
        let catalog = SqliteCatalog::open(path).expect("FULL accepts the existing index contract");
        let columns: Vec<String> = catalog
            .connection
            .prepare("PRAGMA index_info(library_change_queue_eligible)")
            .expect("index evidence")
            .query_map([], |row| row.get(2))
            .expect("index columns")
            .collect::<Result<_, _>>()
            .expect("complete index evidence");
        let expected = if has_wrong_definition {
            vec!["status", "id"]
        } else {
            Vec::new()
        };
        assert_eq!(
            columns, expected,
            "FULL did not silently recreate the index"
        );
        let (metrics, steps) = measured_root_metrics(
            &catalog,
            LibraryRootGeneration::initial(),
            immediate_policy(),
        );
        assert_eq!(metrics.pending_count, 0);
        assert_eq!(metrics.exhausted_retry_count, 0);
        assert_eq!(metrics.latest_exhausted_failure_code, None);
        eprintln!(
            "root metrics optional index wrong_definition={has_wrong_definition} VM steps={steps}"
        );
        assert!(
            steps < 512,
            "empty target must use another root-scoped access path: {steps}"
        );
    }
}

fn measured_root_metrics(
    catalog: &SqliteCatalog,
    generation: LibraryRootGeneration,
    policy: LibraryChangeQueuePolicy,
) -> (LibraryChangeQueueMetrics, usize) {
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
        .expect("count production-port root-query VM work");
    let result =
        catalog.load_library_change_root_queue_metrics("root-a", generation, 1_001, policy);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("remove VM observer");
    (
        result.expect("current root metrics"),
        steps.load(Ordering::Relaxed),
    )
}

fn seed_history(catalog: &mut SqliteCatalog, history: UnrelatedHistory, entries: u32) {
    let transaction = catalog
        .connection
        .transaction()
        .expect("fixture transaction");
    transaction
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('root-b', 'C:\\Other', 1)",
            [],
        )
        .expect("register other root");
    activate_root_change_queue(&transaction, "root-b", 1).expect("other root authority");
    let (root_id, shape) = match history {
        UnrelatedHistory::OtherRootTerminal => ("root-b", "terminal"),
        UnrelatedHistory::OldGenerationTerminal => ("root-a", "terminal"),
        UnrelatedHistory::OtherRootExhausted => ("root-b", "exhausted"),
        UnrelatedHistory::OtherRootExplicitClaims => ("root-b", "explicit"),
    };
    transaction
        .execute(
            "WITH RECURSIVE entries(ordinal) AS (
               SELECT 1 UNION ALL SELECT ordinal + 1 FROM entries WHERE ordinal < ?1
             )
             INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path, origin,
               first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message, catalog_revision_at_enqueue,
               catalog_revision_at_success, created_unix_ms, updated_unix_ms
             )
             SELECT ?2, 1,
               CASE WHEN ?3 = 'explicit' THEN 'freshness_unknown' ELSE 'reconcile' END,
               CASE WHEN ?3 = 'explicit' THEN 'root' ELSE 'path' END,
               CASE WHEN ?3 = 'explicit' THEN '' ELSE 'other-' || ordinal || '.png' END,
               CASE WHEN ?3 = 'explicit' THEN 'startup_catch_up' ELSE 'live_notification' END,
               1, 1, '1', '1', 1,
               CASE WHEN ?3 = 'terminal' THEN 'completed' ELSE 'retry_wait' END, 1,
               CASE WHEN ?3 = 'exhausted' THEN 4 ELSE 0 END,
               CASE WHEN ?3 = 'exhausted' THEN 1 ELSE NULL END,
               CASE ?3 WHEN 'exhausted' THEN 'other-failure'
                 WHEN 'explicit' THEN 'live_gap_v30_explicit_recovery_required' ELSE NULL END,
               CASE WHEN ?3 = 'terminal' THEN NULL ELSE 'Retained failure' END, 0,
               CASE WHEN ?3 = 'terminal' THEN 0 ELSE NULL END, 1, 1
             FROM entries",
            params![entries, root_id, shape],
        )
        .expect("retained unrelated history");
    if matches!(history, UnrelatedHistory::OtherRootExplicitClaims) {
        transaction
            .execute_batch(
                "INSERT INTO library_live_gap_recovery_claims(
                   gap_change_id, root_id, root_generation, consumer_kind, created_unix_ms
                 )
                 SELECT id, root_id, root_generation, 'explicit_recovery_required', 1
                 FROM library_change_queue WHERE root_id = 'root-b'",
            )
            .expect("owned explicit recovery claims");
    }
    transaction.commit().expect("commit unrelated history");
}

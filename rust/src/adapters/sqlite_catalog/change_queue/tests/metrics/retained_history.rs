use super::*;

#[test]
fn completed_root_history_preserves_exact_metrics_with_counting_work() {
    verify_terminal_history(true);
}

#[test]
fn idle_lane_readiness_does_not_walk_completed_root_history() {
    verify_terminal_history(false);
}

fn verify_terminal_history(check_metrics: bool) {
    for entries in [256_u32, 4_096] {
        let directory = tempdir().expect("temporary catalog");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = queue_catalog(path.clone());
        seed_history(&mut catalog, UnrelatedHistory::OtherRootTerminal, entries);
        let live_history: u32 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_change_queue_lanes AS lane
             JOIN library_change_queue AS queue ON queue.id = lane.change_id
             WHERE queue.root_id = 'root-b' AND lane.lane = 'p0_live'",
                [],
                |row| row.get(0),
            )
            .expect("insert trigger retained live lane ownership");
        assert_eq!(live_history, entries);
        drop(catalog);
        let catalog = SqliteCatalog::open(path).expect("FULL validates retained terminal history");
        let generation = LibraryRootGeneration::initial();
        let policy = immediate_policy();
        if check_metrics {
            let metrics = measured(
                &catalog,
                "metrics",
                entries,
                entries as usize * 24 + 1_024,
                || {
                    catalog
                        .load_library_change_root_queue_metrics("root-b", generation, 1_001, policy)
                },
            );
            assert_eq!(metrics.health, LibraryChangeQueueHealth::Idle);
            assert_eq!(metrics.completed_count, u64::from(entries));
            assert_eq!(
                metrics.pending_count + metrics.leased_count + metrics.retry_wait_count,
                0
            );
            assert_eq!(metrics.explicit_recovery_required_count, 0);
            assert_eq!(metrics.latest_exhausted_failure_code, None);
            continue;
        }
        assert!(!measured(&catalog, "live_path", entries, 1_024, || {
            catalog.has_ready_live_path_library_change("root-b", generation, 1_001, policy)
        }));
        assert!(!measured(
            &catalog,
            "live_authoritative",
            entries,
            1_024,
            || {
                catalog.has_ready_live_authoritative_library_change(
                    "root-b", generation, 1_001, policy,
                )
            }
        ));
        assert!(!measured(&catalog, "journal_path", entries, 1_024, || {
            catalog.has_ready_journal_path_library_change("root-b", generation, 1_001, policy)
        }));
        assert!(!measured(
            &catalog,
            "legacy_recovery",
            entries,
            1_024,
            || {
                catalog.has_ready_legacy_unowned_recovery_debt("root-b", generation, 1_001, policy)
            }
        ));
    }
}

fn measured<T>(
    catalog: &SqliteCatalog,
    operation: &str,
    entries: u32,
    maximum_steps: usize,
    query: impl FnOnce() -> Result<T, ScanError>,
) -> T {
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
        .expect("observe production query work");
    let result = query();
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire query observer");
    eprintln!(
        "controlled retained history operation={operation} entries={entries} steps={}",
        steps.load(Ordering::Relaxed)
    );
    let result = result.expect("production query");
    assert!(
        steps.load(Ordering::Relaxed) <= maximum_steps,
        "{operation} must not project active work through terminal history: entries={entries}, steps={}, maximum={maximum_steps}",
        steps.load(Ordering::Relaxed)
    );
    result
}

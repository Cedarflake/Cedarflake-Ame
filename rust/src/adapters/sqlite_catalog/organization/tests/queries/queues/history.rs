use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn queue_reads_do_not_scan_unrelated_admission_and_decision_history() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a"], 'a');
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
               VALUES(1000) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50999
             )
             INSERT INTO organization_admissions(source_generation) SELECT value FROM sequence;
             INSERT INTO organization_admission_sources
             SELECT admission_id, (
               SELECT root_key FROM organization_admission_roots WHERE root_id = 'root-a'
             ) FROM organization_admissions WHERE source_generation >= 1000;
             INSERT INTO organization_records
             SELECT 'history-' || source_generation, NULL, 'ignored', 0, 0, 0, 0, 0, NULL
             FROM organization_admissions WHERE source_generation >= 1000;
             INSERT INTO organization_record_groups
             SELECT record_id, record_id FROM organization_records
             WHERE record_id LIKE 'history-%';
             INSERT INTO organization_sources
             SELECT 'root-a', record_id FROM organization_records
             WHERE record_id LIKE 'history-%';
             INSERT INTO organization_bindings
             SELECT 'history-' || source_generation, 'asset-' || source_generation,
                    source_generation, 20, NULL, NULL, NULL, binding.fingerprint_scheme,
                    printf('%064x', source_generation)
             FROM organization_admissions, (
               SELECT fingerprint_scheme FROM organization_bindings LIMIT 1
             ) AS binding
             WHERE source_generation >= 1000;",
        )
        .unwrap();
    for (kind, expected, total) in [
        (OrganizationQueue::Pending, "c", 2),
        (OrganizationQueue::Ignored, "a", 1),
    ] {
        let ticks = Arc::new(AtomicUsize::new(0));
        let observed = ticks.clone();
        catalog
            .connection
            .progress_handler(
                100,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        let result = catalog.load_query_snapshot(1, &queue(kind), "history", None);
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        let result = result.unwrap();
        assert_eq!(result.snapshot.assets.len(), 1);
        assert_eq!(result.snapshot.assets[0].location_id, expected);
        assert_eq!(result.timeline.total_items, total);
        assert_eq!(
            result.organization_counts.unwrap(),
            OrganizationCounts {
                total: 3,
                pending: 2,
                ignored: 1,
                ..OrganizationCounts::default()
            }
        );
        let steps = ticks.load(Ordering::Relaxed) * 100;
        assert!(
            steps < 20_000,
            "a {kind:?} queue read used at least {steps} SQLite steps"
        );
    }
}

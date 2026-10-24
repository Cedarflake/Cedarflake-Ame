use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::ScanRequest;

use super::super::admission;
use super::*;

fn ordinal(catalog: &SqliteCatalog, location_id: &str) -> i64 {
    catalog
        .connection
        .query_row(
            "SELECT admitted.admission_id FROM asset_locations AS location
             JOIN library_roots AS root
               ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             JOIN organization_admissions AS admitted
               ON admitted.source_generation = location.source_generation
             WHERE location.location_id = ?1",
            [location_id],
            |row| row.get(0),
        )
        .unwrap()
}

fn stage(catalog: &mut SqliteCatalog, suffix: &str) -> (String, String) {
    let scan = format!("scan-{suffix}");
    let root = format!("root-{suffix}");
    let path = format!("C:\\{suffix}");
    let mut location = catalog.load_active_location("a").unwrap().unwrap();
    location.asset_id = format!("asset-{suffix}");
    location.location_id = suffix.into();
    location.root_id.clone_from(&root);
    location.scan_id.clone_from(&scan);
    location.relative_path = "image.png".into();
    location.absolute_path = format!("{path}\\image.png");
    location.display_path.clone_from(&location.absolute_path);
    location.source_generation = 0;
    location.source_revision = None;
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: scan.clone(),
                root_path: path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            },
            &root,
            &path,
        )
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test(&scan)
        .unwrap();
    catalog.stage_location(&scan, &root, &location).unwrap();
    catalog.flush_pending_locations().unwrap();
    (scan, root)
}

#[test]
fn concurrent_imports_enter_the_queue_in_publication_order_not_generation_order() {
    let (_directory, mut catalog) = fixture();
    let (early_scan, early_root) = stage(&mut catalog, "early");
    let (late_scan, late_root) = stage(&mut catalog, "late");
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_admissions"),
        3
    );
    let staged: Vec<i64> = catalog
        .connection
        .prepare(
            "SELECT source_generation FROM asset_locations
             WHERE location_id IN ('early', 'late') ORDER BY location_id",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(staged.len(), 2);
    assert!(staged[0] < staged[1]);
    catalog.publish_scan(&late_scan, &late_root, 1, 0).unwrap();
    catalog
        .publish_scan(&early_scan, &early_root, 1, 0)
        .unwrap();
    assert!(ordinal(&catalog, "late") < ordinal(&catalog, "early"));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_admissions"),
        5
    );
}

#[test]
fn unchanged_publication_keeps_ordinals_and_does_not_consume_sequence_values() {
    let (_directory, catalog) = fixture();
    let before = ordinal(&catalog, "a");
    let sequence = scalar(
        &catalog,
        "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'",
    );
    let generation = selected(&catalog, "a").source_generation;
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    admission::publish_root(&transaction, "root-a").unwrap();
    admission::publish_generation(&transaction, generation as i64).unwrap();
    admission::publish_root(&transaction, "missing-root").unwrap();
    transaction.commit().unwrap();
    assert_eq!(ordinal(&catalog, "a"), before);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'"
        ),
        sequence
    );
}

#[test]
fn admission_failure_rolls_back_the_scan_publication_and_retry_admits_once() {
    let (_directory, mut catalog) = fixture();
    let (scan, root) = stage(&mut catalog, "failure");
    let sequence = scalar(
        &catalog,
        "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'",
    );
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_admission BEFORE INSERT ON organization_admission_sources
         BEGIN SELECT RAISE(ABORT, 'injected admission failure'); END;",
        )
        .unwrap();
    assert!(catalog.publish_scan(&scan, &root, 1, 0).is_err());
    assert!(
        catalog
            .connection
            .query_row(
                "SELECT active_scan_id IS NULL FROM library_roots WHERE id = ?1",
                [&root],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_admissions"),
        3
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'"
        ),
        sequence
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_admission")
        .unwrap();
    catalog.publish_scan(&scan, &root, 1, 0).unwrap();
    assert_eq!(ordinal(&catalog, "failure"), sequence as i64 + 1);
}

#[test]
fn source_removal_keeps_shared_admissions_and_reimport_receives_a_new_ordinal() {
    let (_directory, mut catalog) = fixture();
    let generation = selected(&catalog, "a").source_generation;
    let original = ordinal(&catalog, "a");
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = ?1 WHERE location_id = 'c'",
            [generation as i64],
        )
        .unwrap();
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    admission::publish_root(&transaction, "root-c").unwrap();
    transaction.commit().unwrap();
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(ordinal(&catalog, "c"), original);
    catalog.unregister_root("root-c").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_admissions"),
        0
    );
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "reimport",
        "root-returned",
        "C:\\Returned",
        &[("returned", None, 1)],
    );
    assert!(ordinal(&catalog, "returned") > original);
}

#[test]
fn consolidation_transfers_historical_admissions_before_retiring_the_old_source() {
    let (directory, mut catalog) = fixture();
    let original = ordinal(&catalog, "a");
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    super::super::lifecycle::transfer_source(&transaction, "root-a", "root-c").unwrap();
    transaction.commit().unwrap();
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_admissions"),
        3
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT MIN(admission_id) FROM organization_admissions"
        ),
        original as u64
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_admission_roots WHERE root_id = 'root-a'"
        ),
        0
    );
}

#[test]
fn one_version_publication_does_not_scan_fifty_thousand_historical_admissions() {
    let (_directory, catalog) = fixture();
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
             ) FROM organization_admissions WHERE source_generation >= 1000;",
        )
        .unwrap();
    let generation = selected(&catalog, "a").source_generation;
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
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    let result = admission::publish_generation(&transaction, generation as i64);
    transaction.commit().unwrap();
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    result.unwrap();
    let steps = ticks.load(Ordering::Relaxed) * 100;
    assert!(
        steps < 20_000,
        "one-version admission used at least {steps} SQLite steps"
    );
}

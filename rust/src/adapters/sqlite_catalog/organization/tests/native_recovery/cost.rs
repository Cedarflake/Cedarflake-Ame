use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn organization_native_recovery_uses_indexed_evidence_among_fifty_thousand_records() {
    let (_directory, mut catalog) = native_fixture();
    let favorite = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &favorite, true)
        .unwrap();
    retire_derived_bindings(&catalog);
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50000)
         INSERT INTO organization_records
           SELECT 'unrelated-' || value, NULL, 'pending', 0, 0, 0, 1, 1, 1 FROM sequence;
         INSERT INTO organization_record_groups
           SELECT record_id, record_id FROM organization_records WHERE record_id LIKE 'unrelated-%';
         INSERT INTO organization_bindings
           SELECT record_id, record_id, 20000 + CAST(substr(record_id, 11) AS INTEGER), 20,
             'windows-file-id-128-v1', printf('%016x:%032x', 2, CAST(substr(record_id, 11) AS INTEGER)),
             'windows-file-change-time-100ns-v1:0000000000000001', NULL, NULL
           FROM organization_records WHERE record_id LIKE 'unrelated-%';
         UPDATE catalog_state SET next_source_generation = 100000;",
    ).unwrap();
    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(100, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    publish_root(&catalog, "root-a").unwrap();
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    let used = steps.load(Ordering::Relaxed);
    assert!(used < 30_000, "native association used {used} SQLite steps");
    assert_eq!(decision(&catalog, "a"), (None, "pending".into(), true));
}

#[test]
fn organization_native_recovery_does_not_multiply_current_and_historical_aliases() {
    let (_directory, mut catalog) = native_fixture();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    retire_derived_bindings(&catalog);
    let aliases: Vec<_> = (0..1000).map(|index| format!("alias-{index}")).collect();
    let entries: Vec<_> = aliases.iter().map(|id| (id.as_str(), None, 1)).collect();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-aliases",
        "root-aliases",
        "C:\\Aliases",
        &entries,
    );
    catalog.connection.execute_batch(
        "UPDATE asset_locations SET
           (file_identity_scheme, file_identity_value, source_revision_token, source_generation,
            modified_unix_ms) =
           (SELECT file_identity_scheme, file_identity_value, source_revision_token, source_generation,
                   modified_unix_ms FROM asset_locations WHERE location_id = 'a')
         WHERE root_id = 'root-aliases';
         WITH RECURSIVE sequence(value) AS
           (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 1000)
         INSERT INTO organization_bindings
         SELECT binding.record_id, 'history-' || value, 20000 + value, binding.byte_length,
           binding.file_identity_scheme, binding.file_identity_value, binding.source_revision_token,
           binding.fingerprint_scheme, binding.digest
         FROM (SELECT * FROM organization_bindings LIMIT 1) AS binding CROSS JOIN sequence;
         UPDATE catalog_state SET next_source_generation = 100000;",
    ).unwrap();
    for _ in 0..2 {
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&steps);
        catalog
            .connection
            .progress_handler(
                100,
                Some(move || {
                    observed.fetch_add(100, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        publish_root(&catalog, "root-aliases").unwrap();
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        let used = steps.load(Ordering::Relaxed);
        assert!(used < 1_000_000, "alias recovery used {used} SQLite steps");
    }
    assert_eq!(active_bindings(&catalog), 1001);
    assert_eq!(
        decision(&catalog, "alias-999"),
        (None, "pending".into(), true)
    );
}

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::queries::publish_batch;
use super::*;

#[test]
fn confirmed_group_commands_do_not_visit_unrelated_same_size_locations() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let prepared = prepare(&catalog, &["a"]);
    assert_eq!(prepared.groups[0].location_count, 2);
    let key = &prepared.groups[0].key;
    let (before, read_steps) = measured_steps(&mut catalog, |catalog| {
        selection::target(&catalog.connection, key).unwrap()
    });
    let (receipt, write_steps) = measured_steps(&mut catalog, |catalog| {
        catalog
            .set_organization_favorite("before-growth", &prepared, true)
            .unwrap()
    });
    assert_eq!(receipt.changed_groups, 1);

    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE copies(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM copies WHERE value < 2048
         )
         INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
           absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
           width, height, preview_status, source_generation,
           file_identity_scheme, file_identity_value, source_revision_token)
         SELECT current.scan_id, current.asset_id, 'unrelated-' || value, current.root_id,
           current.absolute_path, current.relative_path, '', current.file_size, 0,
           40, 50, 'pending', current.source_generation,
           current.file_identity_scheme, current.file_identity_value, current.source_revision_token
         FROM copies CROSS JOIN asset_locations AS current
         WHERE current.location_id = 'b'",
        )
        .unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(DISTINCT file_size) FROM asset_locations"
        ),
        1
    );
    let (after, grown_read_steps) = measured_steps(&mut catalog, |catalog| {
        selection::target(&catalog.connection, key).unwrap()
    });
    assert_eq!(after, before);
    assert!(
        grown_read_steps <= read_steps + 100,
        "member read grew from {read_steps} to {grown_read_steps} steps"
    );
    let prepared = prepare(&catalog, &["c"]);
    let (receipt, grown_write_steps) = measured_steps(&mut catalog, |catalog| {
        catalog
            .set_organization_favorite("after-growth", &prepared, false)
            .unwrap()
    });
    assert_eq!(receipt.changed_groups, 1);
    assert!(
        grown_write_steps <= write_steps + 500,
        "group write grew from {write_steps} to {grown_write_steps} steps"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_bindings"),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
        ),
        0
    );
}

fn measured_steps<T>(
    catalog: &mut SqliteCatalog,
    action: impl FnOnce(&mut SqliteCatalog) -> T,
) -> (T, usize) {
    let steps = Arc::new(AtomicUsize::new(0));
    let observer = Arc::clone(&steps);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observer.fetch_add(100, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let result = action(catalog);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    (result, steps.load(Ordering::Relaxed))
}

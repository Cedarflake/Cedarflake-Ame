use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn gallery_time_query_uses_its_ordering_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT location_id FROM asset_locations
                 ORDER BY (COALESCE(capture_local_time, file_local_time) IS NULL),
                          IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                          modified_unix_ms DESC, root_id, location_id
                 LIMIT 100",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_gallery_time")),
        "unexpected gallery-time query plan: {details:?}"
    );
}

#[test]
fn active_relative_path_lookup_uses_its_complete_lookup_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
             SELECT locations.location_id
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
             WHERE locations.root_id = ?1 AND locations.relative_path = ?2
             ORDER BY locations.location_id
             LIMIT 1",
        )
        .expect("query plan")
        .query_map(["root-a", "album/photo.jpg"], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_root_relative")),
        "unexpected active path query plan: {details:?}"
    );
}

#[test]
fn orphan_cleanup_plan_uses_the_asset_identity_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT 1 FROM assets
                 WHERE NOT EXISTS (
                   SELECT 1 FROM asset_locations
                   WHERE asset_locations.asset_id = assets.id
                 )",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_asset_id")),
        "unexpected orphan-cleanup query plan: {details:?}"
    );
}

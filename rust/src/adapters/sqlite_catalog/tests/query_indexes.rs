use rusqlite::{StatementStatus, params_from_iter};
use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;
use crate::domain::GalleryDuplicateMode;

#[test]
fn merged_page_without_fingerprints_does_not_group_the_whole_library() {
    let mut costs = Vec::new();
    for count in [256, 4096] {
        let directory = tempdir().expect("temporary catalog");
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
            .expect("real catalog schema");
        let ids = (0..count)
            .map(|index| format!("image-{index:05}"))
            .collect::<Vec<_>>();
        let locations = ids
            .iter()
            .map(|id| (id.as_str(), Some("2026-01-01T00:00:00"), 1))
            .collect::<Vec<_>>();
        super::publish_gallery_fixture(&mut catalog, "scan", "root", "C:\\Fixture", &locations);
        let query = GalleryQuery {
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        };
        let built = gallery::build_gallery_asset_query(&query, None, None, None, 32)
            .expect("production page query");
        let mut statement = catalog
            .connection
            .prepare(&built.sql)
            .expect("page statement");
        let page = statement
            .query_map(params_from_iter(built.parameters), |row| {
                row.get::<_, String>(1)
            })
            .expect("page rows")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("page identities");
        assert_eq!(page, ids[..32]);
        costs.push(statement.get_status(StatementStatus::VmStep));
    }
    assert!(
        costs[1] < costs[0] * 2,
        "a fixed page must not scan sixteen times as many unfingerprinted images: {costs:?}"
    );
}

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

use crate::adapters::sqlite_catalog::{SqliteCatalog, tests::publish_gallery_fixture};
use crate::domain::GalleryQuery;
use crate::ports::CatalogRepository;
use crate::ports::organization::OrganizationRepository;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:\\Fixture",
        &[("山水画", None, 1)],
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album("create", revision, "Retained")
        .unwrap();
    downgrade_to_v60_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn upgrade_indexes_existing_paths_without_changing_decisions_and_reopens() {
    let (directory, mut catalog) = prior();
    let history = catalog.load_organization_history().unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 61).unwrap();
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Retained"
    );
    drop(catalog);
    let mut reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let page = reopened
        .load_snapshot(
            10,
            &GalleryQuery {
                search_text: "山水画".into(),
                ..GalleryQuery::default()
            },
            "search",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(page.assets.len(), 1);
    assert_eq!(page.assets[0].location_id, "山水画");
    reopened
        .connection
        .execute_batch(
            "INSERT INTO gallery_search_terms(gallery_search_terms) VALUES ('integrity-check');",
        )
        .unwrap();
}

#[test]
fn failed_backfill_rolls_back_index_and_version_before_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_search BEFORE UPDATE ON schema_info
         WHEN NEW.version = 61 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 60).unwrap();
    let objects: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name GLOB 'gallery_search_*'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(objects, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_search")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate(&catalog.connection).unwrap();
}

#[test]
fn admission_rejects_missing_or_altered_index_contracts() {
    for damage in [
        "DROP TABLE gallery_search_terms;",
        "DROP TRIGGER gallery_search_update;",
        "DROP TRIGGER gallery_search_insert;
         CREATE TRIGGER gallery_search_insert AFTER INSERT ON asset_locations BEGIN SELECT 1; END;",
    ] {
        let (directory, mut catalog) = prior();
        migrate(&mut catalog.connection).unwrap();
        catalog.connection.execute_batch(damage).unwrap();
        drop(catalog);
        let error = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
            .err()
            .unwrap();
        assert_eq!(error.code, "catalog_search_index_invalid");
    }
}

#[test]
fn selective_search_reduces_work_after_catalog_only_backfill() {
    use std::time::Instant;

    use crate::adapters::sqlite_catalog::gallery;
    use rusqlite::StatementStatus;

    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 10000
         )
         INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
           absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
           width, height, preview_status, source_generation)
         SELECT current.scan_id, current.asset_id, printf('generated-%05d', value), current.root_id,
           printf('C:/Fixture/Synthetic-%05d.png', value), printf('Synthetic-%05d.png', value),
           '', 20, value, 40, 50, 'pending', current.source_generation
         FROM sequence CROSS JOIN asset_locations AS current WHERE current.location_id = '山水画';",
        )
        .unwrap();
    let bytes = |connection: &Connection| -> i64 {
        connection
            .query_row(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                [],
                |row| row.get(0),
            )
            .unwrap()
    };
    let before_bytes = bytes(&catalog.connection);
    let upgrade_start = Instant::now();
    migrate(&mut catalog.connection).unwrap();
    let upgrade_ms = upgrade_start.elapsed().as_millis();
    let added_bytes = bytes(&catalog.connection) - before_bytes;

    let query = gallery::build_gallery_asset_query(
        &GalleryQuery {
            search_text: "Synthetic-08765".into(),
            ..GalleryQuery::default()
        },
        None,
        None,
        None,
        20,
    )
    .unwrap();
    let mut indexed = catalog.connection.prepare(&query.sql).unwrap();
    let hits = indexed
        .query_map(rusqlite::params_from_iter(query.parameters), |_| Ok(()))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let indexed_steps = indexed.get_status(StatementStatus::VmStep);
    let mut literal = catalog
        .connection
        .prepare(
            "SELECT COUNT(*) FROM asset_locations WHERE
         instr(lower(absolute_path), lower(?1)) > 0 OR instr(lower(relative_path), lower(?1)) > 0",
        )
        .unwrap();
    let literal_hits: i64 = literal
        .query_row(["Synthetic-08765"], |row| row.get(0))
        .unwrap();
    let literal_steps = literal.get_status(StatementStatus::VmStep);
    assert_eq!(hits.len(), 1);
    assert_eq!(literal_hits, 1);
    assert!(
        indexed_steps * 10 < literal_steps,
        "indexed={indexed_steps}, literal={literal_steps}"
    );
    drop((indexed, literal));

    let rename_start = Instant::now();
    let renamed = catalog.connection.execute(
        "UPDATE asset_locations SET relative_path = replace(relative_path, 'Synthetic-', 'Renamed-'),
         absolute_path = replace(absolute_path, 'Synthetic-', 'Renamed-') WHERE location_id LIKE 'generated-%'",
        [],
    ).unwrap();
    assert_eq!(renamed, 10000);
    println!(
        "search_cost rows=10001 upgrade_ms={upgrade_ms} added_bytes={added_bytes} indexed_vm_steps={indexed_steps} literal_vm_steps={literal_steps} rename_10000_ms={}",
        rename_start.elapsed().as_millis()
    );
}

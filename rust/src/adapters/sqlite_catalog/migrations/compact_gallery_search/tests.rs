use crate::adapters::sqlite_catalog::{SqliteCatalog, tests::publish_gallery_fixture};
use crate::ports::organization::OrganizationRepository;

use super::*;

fn documents(connection: &Connection) -> Vec<(i64, String, String)> {
    let sql = if schema_shape::schema_version(connection).unwrap() >= 63 {
        "SELECT gallery_search_id, scan_id, location_id FROM asset_locations ORDER BY gallery_search_id"
    } else {
        "SELECT id, scan_id, location_id FROM gallery_search_documents ORDER BY id"
    };
    connection
        .prepare(sql)
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn current_schema_rejects_position_index_substituted_for_compact_contract() {
    let (directory, catalog) = prior();

    catalog
        .connection
        .execute_batch("DROP TABLE gallery_search_terms;")
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "CREATE VIRTUAL TABLE gallery_search_terms USING fts5(
            absolute_path, relative_path, tokenize='trigram', content='', contentless_delete=1)",
        )
        .unwrap();

    drop(catalog);
    let error = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
        .err()
        .unwrap();
    assert_eq!(error.code, "catalog_search_index_invalid");
}

#[test]
fn deleted_integer_key_can_be_reused_without_old_terms_and_upserts_keep_the_key() {
    let (_directory, catalog) = prior();

    let last = documents(&catalog.connection).pop().unwrap();
    assert_eq!(last.2, "photo");
    catalog
        .connection
        .execute_batch(
            "BEGIN;
         DELETE FROM asset_locations WHERE location_id = 'photo';
         ROLLBACK;",
        )
        .unwrap();
    assert_eq!(documents(&catalog.connection).last(), Some(&last));
    catalog
        .connection
        .execute_batch(
            "DELETE FROM asset_locations WHERE location_id = 'photo';
         INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id, absolute_path,
           relative_path, preview_path, file_size, modified_unix_ms, width, height, preview_status,
           source_generation)
         SELECT scan_id, asset_id, 'replacement', root_id, 'C:/Fixture/Unique.png', 'Unique.png',
           '', 20, 100, 40, 50, 'pending', source_generation
         FROM asset_locations LIMIT 1;",
        )
        .unwrap();
    let replacement = documents(&catalog.connection).pop().unwrap();
    assert_eq!(replacement.0, last.0);
    assert_eq!(replacement.2, "replacement");
    let hits = |term| -> i64 {
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM gallery_search_terms WHERE gallery_search_terms MATCH ?1",
                [term],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(hits("pho"), 0);
    assert_eq!(hits("uni"), 1);
    catalog
        .connection
        .execute_batch(
            "INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id, absolute_path,
           relative_path, preview_path, file_size, modified_unix_ms, width, height, preview_status,
           source_generation)
         SELECT scan_id, asset_id, location_id, root_id, 'C:/Fixture/Renamed.png', 'Renamed.png',
           '', 20, 100, 40, 50, 'pending', source_generation
         FROM asset_locations WHERE location_id = 'replacement'
         ON CONFLICT(scan_id, location_id) DO UPDATE SET
           relative_path = excluded.relative_path, absolute_path = excluded.absolute_path;
         VACUUM;",
        )
        .unwrap();
    assert_eq!(documents(&catalog.connection).last(), Some(&replacement));
    assert_eq!(hits("uni"), 0);
    assert_eq!(hits("ren"), 1);
    assert!(catalog.connection.execute(
        "UPDATE asset_locations SET gallery_search_id = NULL WHERE location_id = 'replacement'", [],
    ).is_err());
    validate_rows(&catalog.connection).unwrap();
}

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:/Fixture",
        &[("山水画", None, 1), ("photo", None, 2)],
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album("create", revision, "Retained", Default::default())
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET camera_make = 'Fixture Camera', camera_model = 'Model 2'
             WHERE location_id = '山水画';",
        )
        .unwrap();

    (directory, catalog)
}

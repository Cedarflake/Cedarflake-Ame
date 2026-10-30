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
        "C:/Fixture",
        &[("山水画", None, 1), ("photo", None, 2)],
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album("create", revision, "Retained")
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET camera_make = 'Fixture Camera', camera_model = 'Model 2'
             WHERE location_id = '山水画';",
        )
        .unwrap();
    downgrade_to_v62_for_test(&catalog.connection);
    (directory, catalog)
}

fn documents(connection: &Connection) -> Vec<(i64, String, String)> {
    let sql = if schema_shape::schema_version(connection).unwrap() == 63 {
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
fn compact_upgrade_preserves_document_identity_metadata_and_decisions() {
    let (directory, mut catalog) = prior();
    let before_documents = documents(&catalog.connection);
    let history = catalog.load_organization_history().unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 63).unwrap();
    assert_eq!(documents(&catalog.connection), before_documents);
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Retained"
    );
    drop(catalog);
    let mut reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let snapshot = reopened
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
    assert_eq!(snapshot.assets.len(), 1);
    assert_eq!(snapshot.assets[0].location_id, "山水画");
    assert_eq!(
        snapshot.assets[0].camera.make.as_deref(),
        Some("Fixture Camera")
    );
    assert_eq!(snapshot.assets[0].camera.model.as_deref(), Some("Model 2"));
    reopened
        .connection
        .execute_batch(
            "INSERT INTO gallery_search_terms(gallery_search_terms) VALUES ('integrity-check');",
        )
        .unwrap();
}

#[test]
fn failed_compaction_rolls_back_terms_and_version_before_retry() {
    let (_directory, mut catalog) = prior();
    let before_documents = documents(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_compaction BEFORE UPDATE ON schema_info
             WHEN NEW.version = 63 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 62).unwrap();
    assert_eq!(documents(&catalog.connection), before_documents);
    let hits: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM gallery_search_terms WHERE gallery_search_terms MATCH '\"photo\"'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_compaction")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 63).unwrap();
}

#[test]
fn current_schema_rejects_position_index_substituted_for_compact_contract() {
    let (directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DROP TABLE gallery_search_terms;")
        .unwrap();
    catalog
        .connection
        .execute_batch(super::super::gallery_search::TERMS)
        .unwrap();
    backfill_terms(&catalog.connection).unwrap();
    drop(catalog);
    let error = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
        .err()
        .unwrap();
    assert_eq!(error.code, "catalog_search_index_invalid");
}

#[test]
fn missing_document_mapping_aborts_upgrade_without_assigning_a_different_identity() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute(
            "DELETE FROM gallery_search_documents WHERE location_id = 'photo'",
            [],
        )
        .unwrap();
    let error = migrate(&mut catalog.connection).unwrap_err();
    assert_eq!(error.code, "catalog_search_index_invalid");
    current_schema::validate_schema_version(&catalog.connection, 62).unwrap();
    assert_eq!(documents(&catalog.connection).len(), 1);
}

#[test]
fn deleted_integer_key_can_be_reused_without_old_terms_and_upserts_keep_the_key() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
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

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
        &[("photo", Some("2026-10-30T12:00:00"), 1)],
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album("create", revision, "Retained")
        .unwrap();
    downgrade_to_v61_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn camera_metadata_upgrade_preserves_catalog_decisions_and_unknown_evidence() {
    let (directory, mut catalog) = prior();
    let history = catalog.load_organization_history().unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Retained"
    );
    let snapshot = catalog
        .load_snapshot(10, &GalleryQuery::default(), "camera", None, None, None)
        .unwrap();
    assert_eq!(snapshot.assets.len(), 1);
    assert_eq!(snapshot.assets[0].camera, Default::default());
    assert_eq!(
        snapshot.assets[0].capture_time.as_ref().unwrap().local_time,
        "2026-10-30T12:00:00"
    );
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn camera_metadata_migration_rolls_back_every_payload_before_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_camera BEFORE UPDATE ON schema_info
             WHEN NEW.version = 62
             BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 61).unwrap();
    for table in TABLES {
        let count: i64 = catalog
            .connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name LIKE 'camera_%'"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_camera")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 62).unwrap();
}

#[test]
fn camera_metadata_admission_requires_bounded_columns_on_all_payload_tables() {
    for table in TABLES {
        let (_directory, mut catalog) = prior();
        migrate(&mut catalog.connection).unwrap();
        catalog
            .connection
            .execute_batch(&format!(
                "ALTER TABLE {table} DROP COLUMN camera_model;
                 ALTER TABLE {table} ADD COLUMN camera_model TEXT;"
            ))
            .unwrap();
        assert_eq!(
            validate(&catalog.connection).unwrap_err().code,
            "catalog_camera_metadata_invalid"
        );
    }
}

#[test]
fn camera_metadata_columns_reject_empty_and_oversized_values() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    for value in [String::new(), "x".repeat(129)] {
        assert!(
            catalog
                .connection
                .execute("UPDATE asset_locations SET camera_make = ?1", [value])
                .is_err()
        );
    }
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET camera_make = ?1, camera_model = ?2",
            ["制造商", "Camera 2"],
        )
        .unwrap();
    let snapshot = catalog
        .load_snapshot(10, &GalleryQuery::default(), "camera", None, None, None)
        .unwrap();
    assert_eq!(snapshot.assets[0].camera.make.as_deref(), Some("制造商"));
    assert_eq!(snapshot.assets[0].camera.model.as_deref(), Some("Camera 2"));
}

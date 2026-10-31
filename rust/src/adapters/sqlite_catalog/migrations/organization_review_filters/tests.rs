use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::domain::{GalleryQuery, organization_queue::OrganizationQueue};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .begin_organization_review(&GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            search_text: "Retained".into(),
            ..Default::default()
        })
        .unwrap();
    downgrade_to_v63_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn upgrade_supplies_empty_filters_without_widening_existing_scope() {
    let (directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    let (search, payload): (String, String) = catalog.connection.query_row(
        "SELECT search_text, payload FROM organization_review_scopes JOIN organization_review_filters ON scope_slot = slot WHERE slot = 'active'", [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(search, "Retained");
    assert_eq!(
        serde_json::from_str::<GalleryMetadataFilter>(&payload).unwrap(),
        GalleryMetadataFilter::default()
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    validate_rows(&catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DELETE FROM organization_review_scopes WHERE slot = 'active'")
        .unwrap();
    let count: i64 = catalog
        .connection
        .query_row(
            "SELECT count(*) FROM organization_review_filters",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn failed_upgrade_rolls_back_and_missing_or_invalid_payload_fails_validation() {
    let (_directory, mut catalog) = prior();
    catalog.connection.execute_batch("CREATE TRIGGER reject_filter_version BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 64 BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 63).unwrap();
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_filter_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("UPDATE organization_review_filters SET payload = '{}'")
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog
        .connection
        .execute_batch("DELETE FROM organization_review_filters")
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
}

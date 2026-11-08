use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::domain::GalleryQuery;
use crate::domain::gallery_metadata_filter::{GalleryMetadataFilter, GalleryNumberRange};
use crate::domain::organization_queue::OrganizationQueue;
use crate::ports::organization_review::OrganizationReviewRepository;

#[test]
fn format_upgrade_preserves_saved_scope_and_rolls_back_on_failure() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let filter = GalleryMetadataFilter {
        extensions: vec!["png".into()],
        width: GalleryNumberRange {
            minimum: Some(1920),
            maximum: None,
        },
        camera_text: "Retained camera".into(),
        ..Default::default()
    };
    catalog
        .begin_organization_review(&GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            search_text: "Retained".into(),
            metadata_filter: filter.clone(),
            ..Default::default()
        })
        .unwrap();
    downgrade_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_format_upgrade BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 77 BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 76).unwrap();
    let legacy: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM organization_review_filters WHERE scope_slot = 'active'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(metadata_filter_payload::decode(&legacy, 1).unwrap(), filter);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_format_upgrade;")
        .unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(path).unwrap();
    let (search, version, payload): (String, i64, String) = catalog.connection.query_row("SELECT search_text, filters.version, payload FROM organization_review_scopes JOIN organization_review_filters AS filters ON scope_slot = slot WHERE slot = 'active'", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!(search, "Retained");
    assert_eq!(version, 2);
    assert_eq!(
        metadata_filter_payload::decode(&payload, version).unwrap(),
        filter
    );
}

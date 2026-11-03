use crate::domain::gallery_metadata_filter::{GalleryMetadataFilter, GalleryNumberRange};

use super::*;

#[test]
fn saved_filters_survive_restart_and_constrain_continuation() {
    let (directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET camera_make = 'Selected' WHERE location_id IN ('a', 'b');
        UPDATE asset_locations SET camera_make = 'Other' WHERE location_id = 'c';",
        )
        .unwrap();
    let query = GalleryQuery {
        metadata_filter: GalleryMetadataFilter {
            extension: "png".into(),
            camera_text: "Selected".into(),
            width: GalleryNumberRange {
                minimum: Some(1),
                maximum: None,
            },
            ..Default::default()
        },
        sort_key: crate::domain::GallerySortKey::FileName,
        sort_direction: crate::domain::GallerySortDirection::Ascending,
        ..pending()
    };
    let session = catalog.begin_organization_review(&query).unwrap();
    checkpoint(&mut catalog, &session, 1, "a");
    change(&mut catalog, "handle", &["a"], OrganizationAction::Ignore);
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let restored = catalog.load_organization_continuation(None).unwrap();
    assert_eq!(
        restored.query.unwrap().metadata_filter,
        query.metadata_filter
    );
    assert_eq!(restored.target.unwrap().location_id, "b");
}

#[test]
fn current_scope_clears_retired_conditions_before_resolving_continuation() {
    let (directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET camera_make = 'Selected' WHERE location_id = 'a';",
        )
        .unwrap();
    let saved = GalleryQuery {
        duplicate_mode: crate::domain::GalleryDuplicateMode::Merged,
        metadata_filter: GalleryMetadataFilter {
            camera_text: "Selected".into(),
            ..Default::default()
        },
        sort_key: crate::domain::GallerySortKey::FileName,
        sort_direction: crate::domain::GallerySortDirection::Ascending,
        ..pending()
    };
    let session = catalog.begin_organization_review(&saved).unwrap();
    checkpoint(&mut catalog, &session, 1, "a");
    change(&mut catalog, "handle", &["a"], OrganizationAction::Ignore);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let current = GalleryQuery {
        metadata_filter: GalleryMetadataFilter::default(),
        ..saved.clone()
    };
    let resumed = catalog
        .load_organization_continuation(Some(&current))
        .unwrap();
    assert_eq!(resumed.status, Status::Ready);
    assert_eq!(resumed.query.unwrap(), current);
    assert_eq!(resumed.target.unwrap().location_id, "b");
    assert_eq!(
        catalog
            .load_organization_continuation(None)
            .unwrap()
            .query
            .unwrap(),
        saved
    );
}

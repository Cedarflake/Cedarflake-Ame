use std::sync::{Arc, atomic::AtomicBool};

use super::*;
use crate::adapters::sqlite_catalog::file_operations::SqliteFileOperationSelection;
use crate::domain::file_operation_exports::{
    FileOrganizationDestination as Destination, FileOrganizationRule as Rule,
};
use crate::domain::file_operation_selection::FileOperationSelectionScope;
use crate::domain::organization::{OrganizationAction, OrganizationAlbumIcon};
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

#[test]
fn file_operation_rules_freeze_effective_classification_and_skip_ambiguous_albums() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    catalog
        .publish_exact_analysis_batch(
            "organization-fixture",
            &[selected(&catalog, "a").source_generation],
        )
        .unwrap();
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "initial", "drawing");
    let target = selected(&catalog, "a");
    let mut selection = SqliteFileOperationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        FileOperationSelectionScope::Locations,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    selection
        .append_locations(std::slice::from_ref(&target))
        .unwrap();
    selection.prepare().unwrap();
    selection.freeze_rule(Rule::Category).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "drawing")
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .change_classification(
            &id("legacy-choice"),
            &prepared,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "drawing")
    );
    selection.freeze_rule(Rule::Category).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "photo")
    );

    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET capture_local_time = '2024-06-02T12:00:00',
         capture_time_source = 'exif_original', capture_raw_value = '2024:06:02 12:00:00'
         WHERE location_id = 'a'",
            [],
        )
        .unwrap();
    selection.freeze_rule(Rule::Date).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "2024-06")
    );
    selection.freeze_rule(Rule::Source).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "Fixture")
    );
    selection.freeze_rule(Rule::Album).unwrap();
    assert!(matches!(
        &selection.page(None).unwrap()[0].organization_destination,
        Destination::Skip(_)
    ));

    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    let album = catalog
        .create_organization_album("album", revision, "旅行", OrganizationAlbumIcon::Travel)
        .unwrap();
    let prepared = catalog
        .prepare_organization_selection(std::slice::from_ref(&target))
        .unwrap();
    catalog
        .change_organization(
            "assign",
            &prepared,
            &OrganizationAction::AssignAlbum {
                album_id: album.album_id.clone(),
            },
        )
        .unwrap();
    selection.freeze_rule(Rule::Album).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Folder(name) if name == "旅行")
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    let second = catalog
        .create_organization_album("second", revision, "画作", OrganizationAlbumIcon::Art)
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO organization_conflicts(record_id, album_id, decision_revision)
        SELECT record_id, ?1, organization_revision FROM organization_records WHERE album_id = ?2",
            rusqlite::params![second.album_id, album.album_id],
        )
        .unwrap();
    selection.freeze_rule(Rule::Album).unwrap();
    assert!(
        matches!(&selection.page(None).unwrap()[0].organization_destination, Destination::Skip(reason) if reason.contains("多个相册"))
    );
    assert!(
        catalog
            .inspect_organization_selection(&[target])
            .unwrap()
            .decisions[0]
            .has_conflict
    );
}

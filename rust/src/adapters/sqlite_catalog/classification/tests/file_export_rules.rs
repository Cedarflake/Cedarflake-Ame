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
fn album_year_destinations_page_large_selections_and_preserve_cancelled_freezes() {
    use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
    use std::sync::atomic::Ordering;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let ids: Vec<_> = (0..513).map(|index| format!("year-{index:04}")).collect();
    let photos: Vec<_> = ids
        .iter()
        .map(|id| (id.as_str(), Some("2024-07-15T10:00:00"), 1))
        .collect();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "years",
        "year-root",
        "C:\\YearFixture",
        &photos,
    );
    catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
           '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;
         UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001';"
    ).unwrap();
    let album = catalog
        .create_organization_album("album", 0, "旅行", Default::default())
        .unwrap();
    let targets: Vec<_> = ids.iter().map(|id| selected(&catalog, id)).collect();
    for (index, page) in targets.chunks(FILE_OPERATION_PAGE_SIZE).enumerate() {
        let prepared = catalog.prepare_organization_selection(page).unwrap();
        catalog
            .change_organization(
                &format!("assign-{index}"),
                &prepared,
                &OrganizationAction::AssignAlbum {
                    album_id: album.album_id.clone(),
                },
            )
            .unwrap();
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut selection = SqliteFileOperationSelection::new(
        SqliteCatalog::open(path).unwrap(),
        FileOperationSelectionScope::Locations,
        Arc::clone(&cancelled),
    )
    .unwrap();
    for page in targets.chunks(FILE_OPERATION_PAGE_SIZE) {
        selection.append_locations(page).unwrap();
    }
    assert_eq!(selection.prepare().unwrap().file_count, 513);
    selection.freeze_rule(Rule::AlbumYear).unwrap();
    cancelled.store(true, Ordering::Release);
    assert!(selection.freeze_rule(Rule::Direct).is_err());
    cancelled.store(false, Ordering::Release);
    let mut after = None;
    let mut observed = std::collections::BTreeSet::new();
    loop {
        let page = selection.page(after).unwrap();
        if page.is_empty() {
            break;
        }
        assert!(page.len() <= FILE_OPERATION_PAGE_SIZE);
        for item in &page {
            assert!(matches!(&item.organization_destination,
                Destination::AlbumYear { album, year } if album == "旅行" && year == "2024"));
            assert!(observed.insert(item.location.location_id.clone()));
        }
        after = page.last().map(|item| item.ordinal);
    }
    assert_eq!(observed, ids.into_iter().collect());
}

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
    for (capture, file_date, year) in [
        (
            Some("2024-12-31T23:59:59"),
            Some("2025-01-01T08:00:00"),
            Some("2024"),
        ),
        (None, Some("2023-01-01T00:00:00"), Some("2023")),
        (None, None, None),
    ] {
        catalog
            .connection
            .execute(
                "UPDATE asset_locations SET capture_local_time = ?1, file_local_time = ?2,
             capture_time_source = CASE WHEN ?1 IS NULL THEN NULL ELSE 'exif_original' END,
             capture_raw_value = ?1 WHERE location_id = 'a'",
                rusqlite::params![capture, file_date],
            )
            .unwrap();
        selection.freeze_rule(Rule::AlbumYear).unwrap();
        let page = selection.page(None).unwrap();
        match (&page[0].organization_destination, year) {
            (
                Destination::AlbumYear {
                    album,
                    year: actual,
                },
                Some(expected),
            ) => {
                assert_eq!(album, "旅行");
                assert_eq!(actual, expected);
            }
            (Destination::Skip(reason), None) => assert_eq!(reason, "没有可用的图库日期"),
            other => panic!("Unexpected year destination: {other:?}"),
        }
    }
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
    selection.freeze_rule(Rule::AlbumYear).unwrap();
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

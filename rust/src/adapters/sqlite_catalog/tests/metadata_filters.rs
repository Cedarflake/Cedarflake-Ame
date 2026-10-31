use crate::adapters::sqlite_catalog::tests::support::publish_gallery_query_fixture;
use crate::adapters::sqlite_catalog::*;
use crate::domain::gallery_metadata_filter::*;
use crate::domain::{GallerySortDirection, GallerySortKey};
use crate::ports::GalleryQueryRepository;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_query_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:/Fixture",
        &[
            ("a", "Camera/a.JPG", Some("2024-02-29T23:59:59"), None, 10),
            ("b", "Camera/b.jpg", Some("2024-03-01T00:00:00"), None, 20),
            ("c", "Camera/c.png", None, Some(1_709_164_800_000), 30),
        ],
    );
    catalog.connection.execute_batch("UPDATE asset_locations SET file_size = 100, width = 300, height = 200, camera_make = 'Maker%_One', camera_model = 'Model 2' WHERE location_id IN ('a', 'b');
        UPDATE asset_locations SET file_size = 99, width = 0, height = 0, camera_make = NULL, camera_model = NULL WHERE location_id = 'c';").unwrap();
    (directory, catalog)
}

fn ids(catalog: &mut SqliteCatalog, filter: GalleryMetadataFilter) -> Vec<String> {
    catalog
        .load_snapshot(
            10,
            &GalleryQuery {
                metadata_filter: filter,
                sort_key: GallerySortKey::FileName,
                sort_direction: GallerySortDirection::Ascending,
                ..Default::default()
            },
            "filters",
            None,
            None,
            None,
        )
        .unwrap()
        .assets
        .into_iter()
        .map(|asset| asset.location_id)
        .collect()
}

#[test]
fn compound_filters_share_counts_layout_pages_and_scope() {
    let (_directory, mut catalog) = fixture();
    let query = GalleryQuery {
        root_id: Some("root".into()),
        folder_relative_path: Some("Camera".into()),
        search_text: "Camera".into(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        metadata_filter: GalleryMetadataFilter {
            extension: "jpg".into(),
            camera_text: "%_one model".into(),
            bytes: GalleryNumberRange {
                minimum: Some(100),
                maximum: Some(100),
            },
            width: GalleryNumberRange {
                minimum: Some(300),
                maximum: Some(300),
            },
            height: GalleryNumberRange {
                minimum: Some(200),
                maximum: Some(200),
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let first = catalog
        .load_query_snapshot(1, &query, "filters", None)
        .unwrap();
    assert_eq!(first.timeline.total_items, 2);
    assert_eq!(first.snapshot.assets[0].location_id, "a");
    let second = catalog
        .load_snapshot(
            1,
            &query,
            "filters",
            first.snapshot.next_cursor.as_ref(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(second.assets[0].location_id, "b");
    assert!(second.next_cursor.is_none());
    let layout = catalog
        .load_gallery_layout_manifest_chunk(10, &query, "filters", None)
        .unwrap();
    assert_eq!(layout.location_ids, ["a", "b"]);
    assert_eq!(
        catalog
            .load_query_snapshot(
                10,
                &GalleryQuery {
                    root_id: Some("absent".into()),
                    ..query
                },
                "absent",
                None
            )
            .unwrap()
            .timeline
            .total_items,
        0
    );
}

#[test]
fn capture_dates_include_the_full_day_without_filesystem_fallback() {
    let (_directory, mut catalog) = fixture();
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                date: GalleryDateRange {
                    start: Some("2024-02-29".into()),
                    end: Some("2024-02-29".into()),
                    ..Default::default()
                },
                ..Default::default()
            }
        ),
        ["a"]
    );
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                date: GalleryDateRange {
                    start: Some("2024-03-01".into()),
                    ..Default::default()
                },
                ..Default::default()
            }
        ),
        ["b"]
    );
}

#[test]
fn unknown_metadata_and_literal_camera_characters_are_not_wildcards() {
    let (_directory, mut catalog) = fixture();
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                width: GalleryNumberRange {
                    minimum: None,
                    maximum: Some(1000)
                },
                ..Default::default()
            }
        ),
        ["a", "b"]
    );
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                extension: "png".into(),
                ..Default::default()
            }
        ),
        ["c"]
    );
    assert!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                camera_text: "MakerX_One".into(),
                ..Default::default()
            }
        )
        .is_empty()
    );
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                width: GalleryNumberRange {
                    minimum: Some(1),
                    maximum: None
                },
                ..Default::default()
            }
        ),
        ["a", "b"]
    );
    assert_eq!(
        ids(
            &mut catalog,
            GalleryMetadataFilter {
                bytes: GalleryNumberRange {
                    minimum: None,
                    maximum: Some(99)
                },
                ..Default::default()
            }
        ),
        ["c"]
    );
}

use crate::adapters::sqlite_catalog::tests::support::publish_gallery_query_fixture;
use crate::adapters::sqlite_catalog::*;
use crate::domain::{GallerySortDirection, GallerySortKey};
use crate::ports::GalleryQueryRepository;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_query_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:\\Fixture",
        &[
            ("landscape", "山水画/Photo10.png", None, None, 10),
            ("photo", "Album/Photo2.png", None, None, 20),
            ("literal", "Album/100%_done[1].png", None, None, 30),
            ("quote", "Album/ab\"cd.png", None, None, 40),
            ("unicode", "Album/Été.png", None, None, 50),
        ],
    );
    (directory, catalog)
}

fn ids(catalog: &mut SqliteCatalog, text: &str) -> Vec<String> {
    catalog
        .load_snapshot(
            20,
            &GalleryQuery {
                search_text: text.into(),
                sort_key: GallerySortKey::FileName,
                sort_direction: GallerySortDirection::Ascending,
                ..GalleryQuery::default()
            },
            "search",
            None,
            None,
            None,
        )
        .unwrap()
        .assets
        .into_iter()
        .map(|a| a.location_id)
        .collect()
}

#[test]
fn literal_search_preserves_short_unicode_punctuation_and_case_behavior() {
    let (_directory, mut catalog) = fixture();
    for (text, expected) in [
        (" PHOTO ", vec!["photo", "landscape"]),
        ("山", vec!["landscape"]),
        ("山水", vec!["landscape"]),
        ("山水画", vec!["landscape"]),
        ("100%_", vec!["literal"]),
        ("[1]", vec!["literal"]),
        ("ab\"cd", vec!["quote"]),
        ("Été", vec!["unicode"]),
        ("été", vec![]),
        ("photo OR done", vec![]),
        ("pho\0to", vec![]),
        ("absent", vec![]),
    ] {
        assert_eq!(ids(&mut catalog, text), expected, "query {text:?}");
    }
}

#[test]
fn paths_and_document_keys_follow_transactional_changes_and_vacuum() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute_batch(
        r"BEGIN;
         UPDATE asset_locations SET relative_path = 'Discarded.png', absolute_path = 'C:\Discarded.png'
           WHERE location_id = 'photo';
         ROLLBACK;"
    ).unwrap();
    assert!(ids(&mut catalog, "discarded").is_empty());
    assert_eq!(ids(&mut catalog, "photo2"), ["photo"]);
    catalog.connection.execute_batch(
        r"UPDATE asset_locations SET relative_path = 'Renamed.png', absolute_path = 'C:\Renamed.png',
             location_id = 'renamed' WHERE location_id = 'photo';
         DELETE FROM asset_locations WHERE location_id = 'literal';
         VACUUM;"
    ).unwrap();
    assert_eq!(ids(&mut catalog, "renamed"), ["renamed"]);
    assert!(ids(&mut catalog, "photo2").is_empty());
    assert!(ids(&mut catalog, "100%").is_empty());
    assert_eq!(ids(&mut catalog, "山水画"), ["landscape"]);
    let counts: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM asset_locations),
                (SELECT COUNT(*) FROM gallery_search_documents),
                (SELECT COUNT(*) FROM gallery_search_terms)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(counts, (4, 4, 4));
    catalog
        .connection
        .execute_batch(
            "INSERT INTO gallery_search_terms(gallery_search_terms) VALUES ('integrity-check');",
        )
        .unwrap();
}

#[test]
fn search_identity_includes_scan_and_excludes_retired_names() {
    let (_directory, mut catalog) = fixture();
    publish_gallery_query_fixture(
        &mut catalog,
        "other-scan",
        "other-root",
        "C:\\Other",
        &[("photo", "Unrelated.png", None, None, 1)],
    );
    assert_eq!(ids(&mut catalog, "photo2"), ["photo"]);
    publish_gallery_query_fixture(
        &mut catalog,
        "replacement",
        "root",
        "C:\\Fixture",
        &[("photo", "Newname.png", None, None, 1)],
    );
    assert!(ids(&mut catalog, "photo2").is_empty());
    assert_eq!(ids(&mut catalog, "newname"), ["photo"]);
    assert_eq!(ids(&mut catalog, "unrelated"), ["photo"]);
}

#[test]
fn indexed_search_preserves_scoped_counts_layout_and_keyset_pages() {
    let (_directory, mut catalog) = fixture();
    let query = GalleryQuery {
        search_text: "photo".into(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let first = catalog
        .load_query_snapshot(1, &query, "search", None)
        .unwrap();
    assert_eq!(first.timeline.total_items, 2);
    assert_eq!(first.snapshot.assets[0].location_id, "photo");
    let second = catalog
        .load_snapshot(
            1,
            &query,
            "search",
            first.snapshot.next_cursor.as_ref(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(second.assets[0].location_id, "landscape");
    assert!(second.next_cursor.is_none());
    let layout = catalog
        .load_gallery_layout_manifest_chunk(10, &query, "search", None)
        .unwrap();
    assert_eq!(layout.location_ids, ["photo", "landscape"]);
    let scoped = GalleryQuery {
        root_id: Some("root".into()),
        folder_relative_path: Some("Album".into()),
        include_descendants: false,
        ..query
    };
    let page = catalog
        .load_query_snapshot(10, &scoped, "scoped", None)
        .unwrap();
    assert_eq!(page.timeline.total_items, 1);
    assert_eq!(page.snapshot.assets[0].location_id, "photo");
}

#[test]
fn gallery_plan_uses_fts_candidates_for_long_text() {
    let (_directory, catalog) = fixture();
    let query = gallery::build_gallery_asset_query(
        &GalleryQuery {
            search_text: "photo".into(),
            ..GalleryQuery::default()
        },
        None,
        None,
        None,
        20,
    )
    .unwrap();
    let plan = catalog
        .connection
        .prepare(&format!("EXPLAIN QUERY PLAN {}", query.sql))
        .unwrap()
        .query_map(rusqlite::params_from_iter(query.parameters), |row| {
            row.get::<_, String>(3)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        plan.iter().any(
            |detail| detail.contains("gallery_search_terms VIRTUAL TABLE INDEX")
                && detail.contains('M')
        ),
        "{plan:?}"
    );
}

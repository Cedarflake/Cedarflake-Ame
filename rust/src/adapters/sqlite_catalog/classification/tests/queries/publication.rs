use crate::adapters::sqlite_catalog::gallery_snapshot::set_after_query_snapshot_read_hook;
use crate::domain::GalleryQueryAnchor;
use crate::ports::IncrementalCatalogRepository;

use super::*;

#[test]
fn classification_publication_keeps_counts_and_pages_on_one_snapshot() {
    let (directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first-a", "drawing");
    predict(&mut catalog, &run, "b", "first-b", "drawing");
    let query = query(Some("drawing"), ClassificationReviewFilter::All);
    let mut writer = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    set_after_query_snapshot_read_hook(move || {
        predict(&mut writer, &run, "a", "replacement-a", "photo");
    });

    let previous = catalog
        .load_query_snapshot(1, &query, "drawing", None)
        .unwrap();
    assert_eq!(previous.snapshot.assets[0].location_id, "a");
    assert_eq!(
        (
            previous.timeline.total_items,
            previous.timeline.total_source_items
        ),
        (2, 3)
    );
    assert_eq!(previous.timeline.revision, previous.snapshot.revision);
    let current = catalog
        .load_query_snapshot(1, &query, "drawing", None)
        .unwrap();
    assert_eq!(current.snapshot.assets[0].location_id, "b");
    assert_eq!(current.timeline.total_items, 1);
    assert!(current.snapshot.revision > previous.snapshot.revision);
    assert_eq!(
        catalog
            .load_snapshot(
                1,
                &query,
                "drawing",
                previous.snapshot.next_cursor.as_ref(),
                None,
                None,
            )
            .unwrap_err()
            .code,
        "catalog_cursor_stale"
    );
    let roots = catalog.load_incremental_catalog_roots().unwrap();
    assert!(!roots.is_empty());
    assert!(
        roots
            .iter()
            .all(|root| root.catalog_revision == current.snapshot.revision)
    );
}

#[test]
fn a_suggestion_arriving_before_the_current_image_retains_the_logical_anchor() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "b", "first-b", "drawing");
    let query = query(Some("drawing"), ClassificationReviewFilter::All);
    let previous = catalog
        .load_query_snapshot(1, &query, "drawing", None)
        .unwrap();
    assert_eq!(previous.snapshot.assets[0].location_id, "b");
    let anchor = GalleryQueryAnchor {
        requested_location_id: "b".into(),
        asset_id: Some(previous.snapshot.assets[0].asset_id.clone()),
        fallback_ordinal: 0,
    };
    predict(&mut catalog, &run, "a", "arriving-a", "drawing");
    let current = catalog
        .load_query_snapshot(1, &query, "drawing", Some(&anchor))
        .unwrap();
    assert_eq!(current.timeline.total_items, 2);
    assert_eq!(current.snapshot.assets[0].location_id, "b");
    assert_eq!(
        current
            .snapshot
            .query_anchor_resolution
            .unwrap()
            .location_id
            .as_deref(),
        Some("b")
    );
}

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn category_snapshot_reads_only_its_page_and_timeline() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "prediction-a", "drawing");
    predict(&mut catalog, &run, "b", "prediction-b", "drawing");
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE copies(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM copies WHERE value < 2048
         )
         INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
           absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
           width, height, preview_status, source_generation,
           file_identity_scheme, file_identity_value, source_revision_token)
         SELECT current.scan_id, current.asset_id, 'copy-' || value, current.root_id,
           current.absolute_path, current.relative_path, '', current.file_size, 0,
           40, 50, 'pending', current.source_generation,
           current.file_identity_scheme, current.file_identity_value, current.source_revision_token
         FROM copies CROSS JOIN asset_locations AS current
         WHERE current.location_id = 'a'",
        )
        .unwrap();
    let scope = query(Some("drawing"), ClassificationReviewFilter::All);
    let (page, page_steps) = measured_steps(&mut catalog, |catalog| {
        catalog.load_snapshot(1, &scope, "category", None, None, None)
    });
    let (timeline, timeline_steps) = measured_steps(&mut catalog, |catalog| {
        catalog.load_gallery_timeline(&scope, "category")
    });
    let (combined, combined_steps) = measured_steps(&mut catalog, |catalog| {
        catalog.load_query_snapshot(1, &scope, "category", None)
    });
    let page = page.unwrap();
    let timeline = timeline.unwrap();
    let combined = combined.unwrap();
    assert_eq!(
        combined.snapshot.assets[0].location_id,
        page.assets[0].location_id
    );
    assert_eq!(combined.timeline.total_items, timeline.total_items);
    assert_eq!(combined.timeline.total_source_items, 2051);
    assert!(
        combined_steps <= page_steps + timeline_steps + 100,
        "combined={combined_steps}, page={page_steps}, timeline={timeline_steps}"
    );
}

fn measured_steps<T>(
    catalog: &mut SqliteCatalog,
    read: impl FnOnce(&mut SqliteCatalog) -> T,
) -> (T, usize) {
    let steps = Arc::new(AtomicUsize::new(0));
    let observer = Arc::clone(&steps);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observer.fetch_add(100, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let result = read(catalog);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    (result, steps.load(Ordering::Relaxed))
}

#[test]
fn timeline_counts_follow_group_category_and_physical_scope() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "prediction-a", "drawing");
    predict(&mut catalog, &run, "b", "prediction-b", "drawing");
    correct(&mut catalog, "confirm-b", "b", "drawing");

    for (review, groups, files) in [
        (ClassificationReviewFilter::All, 2, 3),
        (ClassificationReviewFilter::Pending, 1, 2),
        (ClassificationReviewFilter::Confirmed, 1, 1),
    ] {
        let scope = query(Some("drawing"), review);
        let snapshot = catalog
            .load_query_snapshot(1, &scope, "counts", None)
            .unwrap();
        assert_eq!(
            (
                snapshot.timeline.total_items,
                snapshot.timeline.total_source_items
            ),
            (groups, files)
        );
    }

    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        search_text: "c.png".into(),
        ..query(Some("drawing"), ClassificationReviewFilter::All)
    };
    let snapshot = catalog
        .load_query_snapshot(1, &scoped, "physical", None)
        .unwrap();
    assert_eq!(
        (
            snapshot.timeline.total_items,
            snapshot.timeline.total_source_items
        ),
        (1, 1)
    );
    correct(&mut catalog, "move-a", "a", "photo");
    let snapshot = catalog
        .load_query_snapshot(1, &scoped, "physical", None)
        .unwrap();
    assert_eq!(
        (
            snapshot.timeline.total_items,
            snapshot.timeline.total_source_items
        ),
        (0, 0)
    );
}

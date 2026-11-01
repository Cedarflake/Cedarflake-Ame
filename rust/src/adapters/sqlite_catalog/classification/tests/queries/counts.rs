use crate::domain::classification_review::ClassificationCounts;

use super::*;

#[test]
fn summary_counts_share_group_category_and_physical_scope_across_review_filters() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "prediction-a", "drawing");
    predict(&mut catalog, &run, "b", "prediction-b", "drawing");
    correct(&mut catalog, "confirm-b", "b", "drawing");

    for review in [
        ClassificationReviewFilter::All,
        ClassificationReviewFilter::Pending,
        ClassificationReviewFilter::Confirmed,
    ] {
        let scope = query(Some("drawing"), review);
        let snapshot = catalog
            .load_query_snapshot(1, &scope, "counts", None)
            .unwrap();
        assert_eq!(
            snapshot.classification_counts,
            Some(ClassificationCounts {
                total: 2,
                pending: 1,
                confirmed: 1,
            })
        );
        assert_eq!(
            snapshot.timeline.total_items,
            if review == ClassificationReviewFilter::All {
                2
            } else {
                1
            }
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
        snapshot.classification_counts,
        Some(ClassificationCounts {
            total: 1,
            pending: 1,
            confirmed: 0,
        })
    );
    correct(&mut catalog, "move-a", "a", "photo");
    let snapshot = catalog
        .load_query_snapshot(1, &scoped, "physical", None)
        .unwrap();
    assert_eq!(
        snapshot.classification_counts,
        Some(ClassificationCounts {
            total: 0,
            pending: 0,
            confirmed: 0,
        })
    );
    let ordinary = catalog
        .load_query_snapshot(1, &GalleryQuery::default(), "library", None)
        .unwrap();
    assert!(ordinary.classification_counts.is_none());
}

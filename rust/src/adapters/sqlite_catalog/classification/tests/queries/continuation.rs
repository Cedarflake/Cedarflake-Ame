use crate::domain::classification_review::{
    ClassificationContinuationStatus as Status, ClassificationReviewScope,
};
use crate::ports::classification_review::ClassificationReviewRepository;

use super::*;

#[test]
fn confirmation_continues_to_the_next_pending_group_and_survives_restart() {
    let (directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first-a", "drawing");
    predict(&mut catalog, &run, "b", "first-b", "drawing");
    assert_eq!(
        catalog.load_classification_continuation().unwrap().status,
        Status::NoActivity
    );
    let scope = ClassificationReviewScope {
        gallery: GalleryQuery {
            sort_key: GallerySortKey::FileName,
            sort_direction: GallerySortDirection::Ascending,
            ..ClassificationReviewScope::default().gallery
        },
        category: Some(category("drawing")),
        review: ClassificationReviewFilter::Pending,
    };
    let mut session = catalog.begin_classification_review(&scope).unwrap();
    session.position_sequence = 1;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "c"))
        .unwrap();
    assert_eq!(
        catalog
            .load_classification_continuation()
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "a"
    );
    let prepared = prepare(&catalog, &["a"]);
    let receipt = catalog
        .change_current_classification_review(
            &id("confirm"),
            &session,
            &prepared,
            &ClassificationAction::Confirm,
        )
        .unwrap();
    let next = catalog.load_classification_continuation().unwrap();
    assert_eq!(next.status, Status::Ready);
    assert_eq!(next.target.as_ref().unwrap().location_id, "b");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_continuation().unwrap(), next);
    let mut session = receipt.session;
    session.position_sequence += 1;
    catalog
        .checkpoint_classification_review(&session, next.target.as_ref().unwrap())
        .unwrap();
    let prepared = prepare(&catalog, &["b"]);
    catalog
        .change_current_classification_review(
            &id("confirm-b"),
            &session,
            &prepared,
            &ClassificationAction::Confirm,
        )
        .unwrap();
    assert_eq!(
        catalog.load_classification_continuation().unwrap().status,
        Status::EmptyScope
    );
}

#[test]
fn deferral_advances_all_results_without_wrapping_back_to_the_same_group() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let scope = ClassificationReviewScope {
        gallery: GalleryQuery {
            sort_key: GallerySortKey::FileName,
            sort_direction: GallerySortDirection::Ascending,
            ..ClassificationReviewScope::default().gallery
        },
        ..Default::default()
    };
    let mut session = catalog.begin_classification_review(&scope).unwrap();
    session.position_sequence = 1;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "c"))
        .unwrap();
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .change_current_classification_review(
            &id("defer"),
            &session,
            &prepared,
            &ClassificationAction::Defer,
        )
        .unwrap();
    let result = catalog.load_classification_continuation().unwrap();
    assert_eq!(result.status, Status::EndOfScope);
    assert!(result.target.is_none());
}

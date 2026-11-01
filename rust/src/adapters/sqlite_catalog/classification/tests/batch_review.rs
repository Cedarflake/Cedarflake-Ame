use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::domain::classification_review::{ClassificationReviewFilter, ClassificationReviewScope};
use crate::domain::{GalleryQuery, GallerySortDirection, GallerySortKey};
use crate::ports::classification_review::ClassificationReviewRepository;
use crate::ports::classification_selection::{
    ClassificationSelectionControl, ClassificationSelectionRepository,
};
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

fn draft(
    directory: &tempfile::TempDir,
    catalog: &SqliteCatalog,
    images: &[&str],
) -> (
    SqliteClassificationSelection,
    Arc<ClassificationSelectionControl>,
) {
    let control = Arc::new(ClassificationSelectionControl::new());
    let mut draft = SqliteClassificationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        GalleryQuery::default(),
        scalar(catalog, "SELECT revision FROM catalog_state") as u64,
        false,
        Arc::clone(&control),
    )
    .unwrap();
    draft
        .append_targets(
            &images
                .iter()
                .map(|image| selected(catalog, image))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    draft.prepare().unwrap();
    (draft, control)
}

#[test]
fn selected_draft_deferral_commits_continuation_and_preserves_it_on_refresh_and_restart() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    publish(&mut catalog, "b", 'b');
    let generations = ["a", "b", "c"].map(|name| selected(&catalog, name).source_generation);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "a-prediction", "drawing");
    predict(&mut catalog, &run, "b", "b-prediction", "drawing");
    let r4 = catalog.load_organization_continuation(None).unwrap();
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
    session.position_sequence = 2;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "c"))
        .unwrap();
    let (mut selected_draft, _) = draft(&directory, &catalog, &["c"]);
    let receipt = selected_draft
        .apply_review(&id("defer-current"), &session, &ClassificationAction::Defer)
        .unwrap();
    assert_eq!(receipt.operation.changed_groups, 1);
    assert_eq!(
        selected_draft
            .apply_review(&id("defer-current"), &session, &ClassificationAction::Defer)
            .unwrap(),
        receipt
    );
    let next = catalog.load_classification_continuation().unwrap();
    assert_eq!(next.target.as_ref().unwrap().location_id, "b");
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().review,
        ClassificationReview::Deferred
    );
    let (refreshed, _) = draft(&directory, &catalog, &["c"]);
    assert_eq!(catalog.load_classification_continuation().unwrap(), next);
    assert_eq!(catalog.load_organization_continuation(None).unwrap(), r4);
    drop((selected_draft, refreshed, catalog));
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_continuation().unwrap(), next);
}

#[test]
fn frozen_review_draft_rejects_changed_evidence_and_retired_session_without_advancing() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let mut session = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    session.position_sequence = 2;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "a"))
        .unwrap();
    let (mut selected_draft, _) = draft(&directory, &catalog, &["a"]);
    let bookmark = catalog.load_classification_bookmark().unwrap();
    predict(&mut catalog, &run, "a", "replacement", "drawing");
    assert!(
        selected_draft
            .apply_review(
                &id("late-confirm"),
                &session,
                &ClassificationAction::Confirm
            )
            .is_err()
    );
    assert_eq!(catalog.load_classification_bookmark().unwrap(), bookmark);
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
    let second = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    catalog.end_classification_review(&session.id).unwrap();
    assert!(
        selected_draft
            .apply_review(
                &id("retired-correction"),
                &session,
                &ClassificationAction::Correct(category("photo"))
            )
            .is_err()
    );
    assert_ne!(session.id, second.id);
    assert_eq!(catalog.load_classification_bookmark().unwrap(), bookmark);
}

#[test]
fn review_draft_rejects_multiple_groups_and_cancelled_writes() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut session = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    session.position_sequence = 2;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "a"))
        .unwrap();
    let (mut multiple, _) = draft(&directory, &catalog, &["a", "b"]);
    assert!(
        multiple
            .apply_review(&id("multiple"), &session, &ClassificationAction::Defer)
            .is_err()
    );
    let (mut single, control) = draft(&directory, &catalog, &["a"]);
    control.cancelled.store(true, Ordering::Release);
    assert_eq!(
        single
            .apply_review(&id("cancelled"), &session, &ClassificationAction::Defer)
            .unwrap_err()
            .code,
        "classification_selection_cancelled"
    );
    assert!(
        !catalog
            .load_classification_bookmark()
            .unwrap()
            .unwrap()
            .after_current
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_operations WHERE operation_id IN ('multiple', 'cancelled')"
        ),
        0
    );
}

#[test]
fn retained_image_correction_updates_its_resume_boundary_with_the_latest_receipt() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut session = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    session.position_sequence = 2;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "a"))
        .unwrap();
    let (mut first, _) = draft(&directory, &catalog, &["a"]);
    let deferred = first
        .apply_review(&id("defer"), &session, &ClassificationAction::Defer)
        .unwrap();
    assert!(
        catalog
            .load_classification_bookmark()
            .unwrap()
            .unwrap()
            .after_current
    );
    let (mut refreshed, _) = draft(&directory, &catalog, &["a"]);
    assert!(
        catalog
            .load_classification_bookmark()
            .unwrap()
            .unwrap()
            .after_current
    );
    assert!(
        refreshed
            .apply_review(
                &id("old-position"),
                &session,
                &ClassificationAction::Correct(category("photo"))
            )
            .is_err()
    );
    let corrected = refreshed
        .apply_review(
            &id("correct"),
            &deferred.session,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert!(
        !catalog
            .load_classification_bookmark()
            .unwrap()
            .unwrap()
            .after_current
    );
    assert_eq!(corrected.session.position_sequence, 4);
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("photo"))
    );
    assert_eq!(
        refreshed
            .apply_review(
                &id("correct"),
                &deferred.session,
                &ClassificationAction::Correct(category("photo"))
            )
            .unwrap(),
        corrected
    );
}

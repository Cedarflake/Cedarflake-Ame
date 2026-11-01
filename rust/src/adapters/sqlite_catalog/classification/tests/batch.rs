use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::domain::GalleryQuery;
use crate::domain::classification_selection::ClassificationSelectionPhase;
use crate::ports::classification_selection::{
    ClassificationSelectionControl, ClassificationSelectionRepository,
};

use super::*;

fn draft(
    directory: &tempfile::TempDir,
    catalog: &SqliteCatalog,
) -> (
    SqliteClassificationSelection,
    Arc<ClassificationSelectionControl>,
) {
    let control = Arc::new(ClassificationSelectionControl::new());
    let batch = SqliteClassificationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        GalleryQuery::default(),
        scalar(catalog, "SELECT revision FROM catalog_state") as u64,
        true,
        Arc::clone(&control),
    )
    .unwrap();
    (batch, control)
}

#[test]
fn mixed_batch_confirms_each_prediction_once_and_undo_preserves_favorites() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    publish(&mut catalog, "b", 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "prediction-a", "drawing");
    predict(&mut catalog, &run, "b", "prediction-b", "photo");
    let (mut batch, _) = draft(&directory, &catalog);
    let summary = batch.prepare().unwrap();
    assert_eq!(summary.group_count, 2);
    assert_eq!(summary.confirmable_count, 2);
    assert!(summary.common_category_id.is_none());
    let favorite = catalog
        .prepare_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    catalog
        .set_organization_favorite("favorite-a", &favorite, true)
        .unwrap();
    let receipt = batch
        .apply(&id("batch-confirm"), &ClassificationAction::Confirm)
        .unwrap();
    assert_eq!(receipt.changed_groups, 2);
    assert_eq!(
        batch
            .apply(&id("batch-confirm"), &ClassificationAction::Confirm)
            .unwrap(),
        receipt
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("drawing"))
    );
    assert_eq!(
        prepare(&catalog, &["b"]).groups[0].effective().category,
        Some(category("photo"))
    );
    catalog
        .undo_classification(&id("undo-batch"), &id("batch-confirm"))
        .unwrap();
    for image in ["a", "b", "c"] {
        assert_eq!(
            prepare(&catalog, &[image]).groups[0].human,
            HumanClassification::Unreviewed
        );
    }
    assert!(
        catalog
            .inspect_organization_selection(&[selected(&catalog, "a")])
            .unwrap()
            .decisions[0]
            .favorite
    );
}

#[test]
fn frozen_predictions_reject_confirmation_but_allow_explicit_correction() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "b", 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first-a", "drawing");
    predict(&mut catalog, &run, "b", "first-b", "drawing");
    let (mut batch, _) = draft(&directory, &catalog);
    batch.prepare().unwrap();
    predict(&mut catalog, &run, "b", "new-b", "drawing");
    assert!(
        batch
            .apply(&id("stale-batch"), &ClassificationAction::Confirm)
            .is_err()
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_operations WHERE operation_id = 'stale-batch'"
        ),
        0
    );
    assert_eq!(
        batch
            .apply(
                &id("correction"),
                &ClassificationAction::Correct(category("photo"))
            )
            .unwrap()
            .changed_groups,
        3
    );
}

#[test]
fn exclusions_freeze_targets_and_later_human_choice_aborts_the_whole_batch() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let (mut batch, _) = draft(&directory, &catalog);
    batch
        .append_ids(&[selected(&catalog, "c").asset_id])
        .unwrap();
    assert_eq!(batch.prepare().unwrap().group_count, 2);
    let selection = prepare(&catalog, &["b"]);
    catalog
        .change_classification(
            &id("new-choice"),
            &selection,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert!(
        batch
            .apply(
                &id("stale-correction"),
                &ClassificationAction::Correct(category("drawing"))
            )
            .is_err()
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
    assert_eq!(
        prepare(&catalog, &["c"]).groups[0].human,
        HumanClassification::Unreviewed
    );
}

#[test]
fn cancelling_a_prepared_selection_never_commits() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let (mut batch, control) = draft(&directory, &catalog);
    batch.prepare().unwrap();
    control.cancelled.store(true, Ordering::Release);
    assert_eq!(
        batch
            .apply(
                &id("cancelled"),
                &ClassificationAction::Correct(category("photo"))
            )
            .unwrap_err()
            .code,
        "classification_selection_cancelled"
    );
    assert_eq!(
        control.snapshot().phase,
        ClassificationSelectionPhase::Cancelled
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_operations WHERE operation_id = 'cancelled'"
        ),
        0
    );
}

#[test]
fn retained_viewer_target_cannot_be_replaced_with_the_current_asset_version() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut target = selected(&catalog, "a");
    target.source_generation += 1;
    let (mut batch, _) = draft(&directory, &catalog);
    assert!(batch.append_targets(&[target]).is_err());
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_decisions"),
        0
    );
}

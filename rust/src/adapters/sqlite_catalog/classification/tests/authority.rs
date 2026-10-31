use super::*;

#[test]
fn favorite_changes_do_not_invalidate_classification_or_undo() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let old = prepare(&catalog, &["a"]);
    let r4 = catalog
        .prepare_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &r4, true)
        .unwrap();
    catalog
        .change_classification(
            &id("correct"),
            &old,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let r4_after = catalog
        .inspect_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    assert!(r4_after.decisions[0].favorite);
    catalog
        .undo_classification(&id("undo-correct"), &id("correct"))
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
    assert!(
        catalog
            .inspect_organization_selection(&[selected(&catalog, "a")])
            .unwrap()
            .decisions[0]
            .favorite
    );
}

#[test]
fn old_click_cannot_confirm_new_prediction_even_when_category_agrees() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let old = prepare(&catalog, &["a"]);
    predict(&mut catalog, &run, "a", "second", "drawing");
    assert!(
        catalog
            .change_classification(&id("stale"), &old, &ClassificationAction::Confirm)
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_decisions"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_operations WHERE operation_id = 'stale'"
        ),
        0
    );
    catalog
        .change_classification(
            &id("correct"),
            &old,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("photo"))
    );
}

#[test]
fn failed_batch_is_atomic_and_later_field_choice_blocks_old_undo() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let both = prepare(&catalog, &["a", "b"]);
    catalog
        .change_classification(
            &id("first-b"),
            &prepare(&catalog, &["b"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert!(
        catalog
            .change_classification(
                &id("stale-batch"),
                &both,
                &ClassificationAction::Correct(category("drawing"))
            )
            .is_err()
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
    let both = prepare(&catalog, &["a", "b"]);
    catalog
        .change_classification(
            &id("batch"),
            &both,
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    catalog
        .change_classification(
            &id("later-b"),
            &prepare(&catalog, &["b"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert!(
        catalog
            .undo_classification(&id("stale-undo"), &id("batch"))
            .is_err()
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("drawing"))
    );
    assert_eq!(
        prepare(&catalog, &["b"]).groups[0].effective().category,
        Some(category("photo"))
    );
}

#[test]
fn confirmation_restores_deferred_review_on_operation_specific_undo() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    catalog
        .change_classification(
            &id("defer"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Defer,
        )
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().review,
        ClassificationReview::Deferred
    );
    catalog
        .change_classification(
            &id("confirm"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Confirm,
        )
        .unwrap();
    let receipt = catalog
        .undo_classification(&id("undo"), &id("confirm"))
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().review,
        ClassificationReview::Deferred
    );
    assert_eq!(
        catalog
            .undo_classification(&id("undo"), &id("confirm"))
            .unwrap(),
        receipt
    );
    catalog
        .undo_classification(&id("redo"), &id("undo"))
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().review,
        ClassificationReview::Confirmed
    );
}

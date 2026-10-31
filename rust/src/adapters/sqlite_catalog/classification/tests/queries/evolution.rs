use super::*;

#[test]
fn late_group_conflict_has_no_category_and_local_intent_precedes_restored_choices() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    correct(&mut catalog, "a-choice", "a", "drawing");
    correct(&mut catalog, "c-choice", "c", "photo");
    publish_group(&mut catalog, &["a", "c"], 'a');
    assert_eq!(
        ids(
            &mut catalog,
            &query(None, ClassificationReviewFilter::Pending)
        ),
        ["a"]
    );
    for name in ["drawing", "photo"] {
        assert!(
            ids(
                &mut catalog,
                &query(Some(name), ClassificationReviewFilter::All)
            )
            .is_empty()
        );
    }
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().review,
        ClassificationReview::Conflict
    );
    catalog
        .connection
        .execute(
            "UPDATE classification_decisions SET choice_restored = 1 WHERE category_id = 'photo'",
            [],
        )
        .unwrap();
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("drawing"), ClassificationReviewFilter::Confirmed)
        ),
        ["a"]
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("drawing"))
    );
    catalog
        .connection
        .execute(
            "UPDATE classification_decisions SET choice_restored = 1",
            [],
        )
        .unwrap();
    assert_eq!(
        ids(
            &mut catalog,
            &query(None, ClassificationReviewFilter::Pending)
        ),
        ["a"]
    );
    correct(&mut catalog, "resolve", "c", "photo");
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        ),
        ["a"]
    );
}

#[test]
fn changed_content_loses_category_while_a_surviving_copy_keeps_its_decision() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    setup(&mut catalog);
    correct(&mut catalog, "choice", "c", "photo");
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'a'",
            [],
        )
        .unwrap();
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        ),
        ["c"]
    );
    assert_eq!(
        ids(&mut catalog, &query(None, ClassificationReviewFilter::All)),
        ["a", "b", "c"]
    );
    catalog.unregister_root("root-c").unwrap();
    assert!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        )
        .is_empty()
    );
    assert_eq!(
        ids(&mut catalog, &query(None, ClassificationReviewFilter::All)),
        ["a", "b"]
    );
}

#[test]
fn semantic_rule_replacement_hides_old_predictions_but_keeps_human_categories() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    correct(&mut catalog, "human", "b", "photo");
    let replacement = Taxonomy::new(
        taxonomy()
            .categories()
            .iter()
            .map(|definition| {
                CategoryDefinition::new(
                    definition.id().clone(),
                    definition.label().into(),
                    vec![format!("Changed description for {}", definition.label())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let revision = catalog.load_classification_state().unwrap().revision;
    catalog
        .configure_classification(&id("rules"), revision, &replacement)
        .unwrap();
    assert!(
        ids(
            &mut catalog,
            &query(Some("drawing"), ClassificationReviewFilter::All)
        )
        .is_empty()
    );
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        ),
        ["b"]
    );
    assert_eq!(
        ids(&mut catalog, &query(None, ClassificationReviewFilter::All)),
        ["a", "b", "c"]
    );
}

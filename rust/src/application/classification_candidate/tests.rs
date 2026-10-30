use super::*;

fn model() -> ModelCompatibility {
    let model = &candidate().model;
    ModelCompatibility::new(
        ClassificationRevision::from_bytes(model.embedding_space),
        ClassificationRevision::from_bytes(model.image_pipeline),
        ClassificationRevision::from_bytes(model.text_pipeline),
        model.dimensions,
    )
    .unwrap()
}

fn id() -> ClassificationRecordId {
    ClassificationRecordId::new("candidate".into()).unwrap()
}

#[test]
fn measured_candidate_keeps_both_rule_sets_and_exact_category_thresholds() {
    let setup = prepare(id(), initial_categories().unwrap(), model()).unwrap();
    let (rules, profile) = setup.refinement().unwrap();
    assert_eq!(rules.categories().len(), 6);
    assert_ne!(rules.revision(), setup.primary().revision());
    assert_eq!(profile.identity().model, model());
    assert_eq!(
        profile.calibration(),
        ClassificationRevision::from_bytes(candidate().calibration)
    );
    assert_eq!(
        setup.run().refinement().unwrap().profile,
        profile.revision()
    );
    let illustration = profile
        .thresholds_for(&CategoryId::new("illustration".into()).unwrap())
        .unwrap();
    assert_eq!(
        illustration.rescreen_below().to_bits(),
        0.000_995_636_1_f32.to_bits()
    );
    assert_eq!(
        illustration.review_below().unwrap().to_bits(),
        0.010_479_377_f32.to_bits()
    );
    assert!(
        profile
            .thresholds_for(&CategoryId::new("comic".into()).unwrap())
            .unwrap()
            .review_below()
            .is_none()
    );
}

#[test]
fn display_renames_retain_calibration_but_semantic_edits_and_model_changes_do_not() {
    let primary = initial_categories().unwrap();
    let renamed = Taxonomy::new(
        primary
            .categories()
            .iter()
            .map(|category| {
                CategoryDefinition::new(
                    category.id().clone(),
                    format!("Renamed {}", category.label()),
                    category.descriptions().to_vec(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    assert!(
        prepare(id(), renamed, model())
            .unwrap()
            .refinement()
            .is_some()
    );
    let edited = Taxonomy::new(
        primary
            .categories()
            .iter()
            .map(|category| {
                CategoryDefinition::new(
                    category.id().clone(),
                    category.label().into(),
                    vec![format!("Different {}", category.id().as_str())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    assert!(
        prepare(id(), edited, model())
            .unwrap()
            .refinement()
            .is_none()
    );
    let model = ModelCompatibility::new(
        model().image_identity().embedding_space,
        ClassificationRevision::from_bytes([0; 32]),
        model().text_identity().pipeline,
        768,
    )
    .unwrap();
    assert!(
        prepare(id(), primary, model)
            .unwrap()
            .refinement()
            .is_none()
    );
}

#[test]
fn custom_categories_are_usable_without_borrowed_calibration() {
    let primary = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("custom".into()).unwrap(),
            "Custom".into(),
            vec!["A subject outside the initial categories".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    let setup = prepare(id(), primary.clone(), model()).unwrap();
    assert_eq!(setup.primary(), &primary);
    assert!(setup.refinement().is_none());
}

#[cfg(all(windows, feature = "classification-worker"))]
#[test]
fn frozen_native_pipeline_matches_the_measured_candidate() {
    assert_eq!(
        model(),
        crate::adapters::classification_runtime::selected_model_compatibility()
    );
}

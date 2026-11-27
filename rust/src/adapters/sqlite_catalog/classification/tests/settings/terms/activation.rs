use super::*;

#[test]
fn activating_a_builtin_preserves_later_edits_pause_and_image_feature_compatibility() {
    let (_directory, mut catalog) = fixture();
    let initial = taxonomy();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([4; 32]),
        ClassificationRevision::from_bytes([5; 32]),
        ClassificationRevision::from_bytes([6; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id("start"), model, initial.revision(), None).unwrap();
    let mut start = request(
        &catalog,
        "start",
        initial.clone(),
        ClassificationConfigurationAction::Enable,
    );
    start.setup = ClassificationAnalysisSetup::new(run, initial, None).unwrap();
    catalog.save_classification_configuration(&start).unwrap();

    let builtin = CategoryDefinition::new(
        category("managed-cats"),
        "Cats".into(),
        vec!["A photograph of cats".into()],
    )
    .unwrap();
    let mut activation = edit(&catalog, "activate-cats", None, Some(builtin.clone()));
    activation.bundled_default = Some(builtin.clone());
    assert!(activation.changes_matching());
    let later = edit(&catalog, "later-category", None, Some(custom()));
    let before = catalog.save_classification_term(&later).unwrap();
    catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();

    let saved = catalog.save_classification_term(&activation).unwrap();
    assert_eq!(saved.dispatch.mode, ClassificationDispatchMode::Paused);
    assert_eq!(
        saved.catalog.active_run_id.as_deref(),
        Some("activate-cats")
    );
    let categories = saved.catalog.taxonomy.as_ref().unwrap().categories();
    assert_eq!(categories.len(), 4);
    assert!(categories.contains(&builtin));
    for retained in before.catalog.taxonomy.as_ref().unwrap().categories() {
        assert!(categories.contains(retained));
    }
    let setup = catalog
        .load_classification_setup(&id("activate-cats"))
        .unwrap()
        .unwrap();
    assert_eq!(setup.run().model(), model);
    assert_eq!(setup.primary(), saved.catalog.taxonomy.as_ref().unwrap());
    assert_ne!(saved.catalog.configuration, before.catalog.configuration);
}

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::classification_configuration::ClassificationTermsReset;

use super::*;

fn reset(
    catalog: &SqliteCatalog,
    operation: &str,
    expected: Vec<CategoryDefinition>,
) -> ClassificationTermsReset {
    let defaults = taxonomy()
        .categories()
        .iter()
        .map(|category| (category.id().clone(), category.clone()))
        .collect::<BTreeMap<_, _>>();
    ClassificationTermsReset::new(
        request(
            catalog,
            operation,
            taxonomy(),
            ClassificationConfigurationAction::Save,
        )
        .setup,
        expected,
        &defaults,
    )
    .unwrap()
}

fn modified(semantic: bool) -> Vec<CategoryDefinition> {
    taxonomy()
        .categories()
        .iter()
        .enumerate()
        .map(|(index, category)| {
            if semantic && index == 1 {
                CategoryDefinition::new(
                    category.id().clone(),
                    "Holiday photos".into(),
                    vec!["A holiday photograph".into()],
                )
                .unwrap()
            } else {
                category
                    .renamed(format!("My {}", category.label()))
                    .unwrap()
            }
        })
        .collect()
}

#[test]
fn mixed_batch_preserves_active_ids_later_definitions_pause_and_encoder() {
    let (_directory, mut catalog) = fixture();
    let expected = modified(true);
    let mut categories = expected.clone();
    categories.push(custom());
    let initial = request(
        &catalog,
        "start",
        Taxonomy::new(categories).unwrap(),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&initial).unwrap();
    let batch = reset(&catalog, "restore-all", expected);
    let later_custom = custom().renamed("Later custom label".into()).unwrap();
    catalog
        .save_classification_term(&edit(
            &catalog,
            "later-custom",
            Some(custom()),
            Some(later_custom.clone()),
        ))
        .unwrap();
    let unrelated = CategoryDefinition::new(
        category("managed-unrelated"),
        "Background category".into(),
        vec!["A background image".into()],
    )
    .unwrap();
    let mut categories = catalog
        .load_classification_configuration()
        .unwrap()
        .catalog
        .taxonomy
        .unwrap()
        .categories()
        .to_vec();
    categories.push(unrelated.clone());
    let background = request(
        &catalog,
        "background",
        Taxonomy::new(categories).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    let before = catalog
        .save_classification_configuration(&background)
        .unwrap();
    catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let after = catalog.reset_all_classification_terms(&batch).unwrap();
    assert_eq!(after.dispatch.mode, ClassificationDispatchMode::Paused);
    assert_eq!(after.catalog.active_run_id.as_deref(), Some("restore-all"));
    let categories = after.catalog.taxonomy.as_ref().unwrap().categories();
    let before_ids = before
        .catalog
        .taxonomy
        .as_ref()
        .unwrap()
        .categories()
        .iter()
        .map(CategoryDefinition::id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        categories
            .iter()
            .map(CategoryDefinition::id)
            .collect::<BTreeSet<_>>(),
        before_ids
    );
    assert!(categories.contains(&later_custom));
    assert!(categories.contains(&unrelated));
    for default in taxonomy().categories() {
        assert!(categories.contains(default));
    }
    let restored_setup = catalog
        .load_classification_setup(&id("restore-all"))
        .unwrap()
        .unwrap();
    assert_eq!(restored_setup.run().model(), background.setup.run().model());
    let historical = catalog
        .load_classification_setup(&id("background"))
        .unwrap()
        .unwrap();
    assert_eq!(historical.run(), background.setup.run());
    assert_eq!(
        historical.primary().revision(),
        background.setup.primary().revision()
    );
    assert_eq!(historical.refinement(), background.setup.refinement());
    assert_eq!(
        historical
            .primary()
            .categories()
            .iter()
            .map(|category| (category.id(), category.descriptions()))
            .collect::<Vec<_>>(),
        background
            .setup
            .primary()
            .categories()
            .iter()
            .map(|category| (category.id(), category.descriptions()))
            .collect::<Vec<_>>()
    );
    // Labels are current category metadata; run rules retain versioned descriptions.
    for category in historical.primary().categories() {
        let current = categories
            .iter()
            .find(|current| current.id() == category.id())
            .unwrap();
        assert_eq!(category.label(), current.label());
    }
    let stopped = catalog
        .change_classification_dispatch(
            after.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    assert_eq!(
        catalog
            .reset_all_classification_terms(&batch)
            .unwrap()
            .dispatch,
        stopped
    );
}

#[test]
fn names_only_batch_retains_run_and_replay_does_not_undo_later_edits() {
    let (_directory, mut catalog) = fixture();
    let expected = modified(false);
    let initial = request(
        &catalog,
        "start",
        Taxonomy::new(expected.clone()).unwrap(),
        ClassificationConfigurationAction::Enable,
    );
    let before = catalog.save_classification_configuration(&initial).unwrap();
    let batch = reset(&catalog, "restore-all", expected.clone());
    assert!(!batch.changes_matching());
    let after = catalog.reset_all_classification_terms(&batch).unwrap();
    assert_eq!(after.catalog.active_run_id, before.catalog.active_run_id);
    assert_eq!(after.catalog.configuration, before.catalog.configuration);
    assert_eq!(after.dispatch, before.dispatch);
    let original = taxonomy().categories()[0].clone();
    let later = edit(
        &catalog,
        "after-reset",
        Some(original.clone()),
        Some(original.renamed("Later choice".into()).unwrap()),
    );
    let later_state = catalog.save_classification_term(&later).unwrap();
    let reordered = reset(
        &catalog,
        "restore-all",
        expected.into_iter().rev().collect(),
    );
    assert_eq!(
        catalog.reset_all_classification_terms(&reordered).unwrap(),
        later_state
    );
    let conflicting = reset(&catalog, "restore-all", taxonomy().categories().to_vec());
    assert_eq!(
        catalog
            .reset_all_classification_terms(&conflicting)
            .unwrap_err()
            .code,
        "classification_operation_conflict"
    );
    assert_eq!(
        catalog.load_classification_configuration().unwrap(),
        later_state
    );
}

#[test]
fn stale_target_rolls_back_every_definition_and_leaves_operation_retryable() {
    let (_directory, mut catalog) = fixture();
    let expected = modified(true);
    let initial = request(
        &catalog,
        "start",
        Taxonomy::new(expected.clone()).unwrap(),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&initial).unwrap();
    let batch = reset(&catalog, "restore-all", expected.clone());
    let concurrent = expected[1].renamed("Concurrent name".into()).unwrap();
    let later = edit(
        &catalog,
        "concurrent",
        Some(expected[1].clone()),
        Some(concurrent.clone()),
    );
    let before = catalog.save_classification_term(&later).unwrap();
    assert_eq!(
        catalog
            .reset_all_classification_terms(&batch)
            .unwrap_err()
            .code,
        "classification_term_changed"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert!(
        catalog
            .load_classification_setup(&id("restore-all"))
            .unwrap()
            .is_none()
    );
    let reviewed = reset(
        &catalog,
        "restore-all",
        vec![expected[0].clone(), concurrent],
    );
    let after = catalog.reset_all_classification_terms(&reviewed).unwrap();
    assert_eq!(after.catalog.taxonomy, Some(taxonomy()));
}

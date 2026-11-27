use crate::domain::classification_configuration::ClassificationTermChange;
use crate::ports::classification_configuration::ClassificationTermRepository;

use super::*;

mod activation;

fn edit(
    catalog: &SqliteCatalog,
    operation: &str,
    expected: Option<CategoryDefinition>,
    replacement: Option<CategoryDefinition>,
) -> ClassificationTermChange {
    ClassificationTermChange {
        expected,
        replacement,
        bundled_default: None,
        baseline: request(
            catalog,
            operation,
            taxonomy(),
            ClassificationConfigurationAction::Save,
        )
        .setup,
    }
}

fn custom() -> CategoryDefinition {
    CategoryDefinition::new(
        category("custom-travel"),
        "旅途".into(),
        vec!["A travel photograph".into()],
    )
    .unwrap()
}

#[test]
fn term_edit_merges_later_categories_and_control_without_restarting_the_run() {
    let (_directory, mut catalog) = fixture();
    let initial = taxonomy();
    let selected = initial.categories()[0].clone();
    let rename = edit(
        &catalog,
        "rename",
        Some(selected.clone()),
        Some(selected.renamed("My art".into()).unwrap()),
    );
    let start = request(
        &catalog,
        "start",
        initial,
        ClassificationConfigurationAction::Enable,
    );
    let mut later = start.setup.primary().categories().to_vec();
    later.push(custom());
    catalog.save_classification_configuration(&start).unwrap();
    let background = request(
        &catalog,
        "later-category",
        Taxonomy::new(later).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    let before = catalog
        .save_classification_configuration(&background)
        .unwrap();
    let paused = catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let saved = catalog.save_classification_term(&rename).unwrap();
    assert_eq!(saved.dispatch, paused);
    assert_eq!(saved.catalog.active_run_id, before.catalog.active_run_id);
    assert_eq!(saved.catalog.configuration, before.catalog.configuration);
    let categories = saved.catalog.taxonomy.as_ref().unwrap().categories();
    assert!(categories.contains(&custom()));
    assert_eq!(
        categories
            .iter()
            .find(|category| category.id() == selected.id())
            .unwrap()
            .label(),
        "My art"
    );
    let stale = edit(
        &catalog,
        "stale-edit",
        Some(selected.clone()),
        Some(selected.renamed("Old editor".into()).unwrap()),
    );
    assert_eq!(
        catalog.save_classification_term(&stale).unwrap_err().code,
        "classification_term_changed"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), saved);
    let stopped = catalog
        .change_classification_dispatch(paused.revision, &ClassificationDispatchCommand::Stop)
        .unwrap();
    assert_eq!(
        catalog.save_classification_term(&rename).unwrap().dispatch,
        stopped
    );
}

#[test]
fn renamed_category_retains_an_active_encoder_different_from_the_editor_baseline() {
    let (_directory, mut catalog) = fixture();
    let original = taxonomy();
    let selected = original.categories()[0].clone();
    let rename = edit(
        &catalog,
        "rename",
        Some(selected.clone()),
        Some(selected.renamed("作品".into()).unwrap()),
    );
    let mut start = request(
        &catalog,
        "start",
        original.clone(),
        ClassificationConfigurationAction::Enable,
    );
    let alternate_model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([4; 32]),
        ClassificationRevision::from_bytes([5; 32]),
        ClassificationRevision::from_bytes([6; 32]),
        2,
    )
    .unwrap();
    let run =
        ClassificationRun::new(id("start"), alternate_model, original.revision(), None).unwrap();
    start.setup = ClassificationAnalysisSetup::new(run, original, None).unwrap();
    let before = catalog.save_classification_configuration(&start).unwrap();
    let after = catalog.save_classification_term(&rename).unwrap();
    assert_eq!(after.dispatch, before.dispatch);
    assert_eq!(after.catalog.configuration, before.catalog.configuration);
    assert_eq!(after.catalog.active_run_id, before.catalog.active_run_id);
}

#[test]
fn custom_term_lifecycle_preserves_other_definitions_and_rejects_builtin_removal() {
    let (_directory, mut catalog) = fixture();
    let start = request(
        &catalog,
        "start",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    let original = catalog.save_classification_configuration(&start).unwrap();
    let add = edit(&catalog, "add", None, Some(custom()));
    let added = catalog.save_classification_term(&add).unwrap();
    assert_eq!(added.catalog.active_run_id.as_deref(), Some("add"));
    assert_eq!(
        added.catalog.taxonomy.as_ref().unwrap().categories().len(),
        3
    );
    let builtin = original.catalog.taxonomy.as_ref().unwrap().categories()[0].clone();
    let remove_builtin = edit(&catalog, "remove-builtin", Some(builtin), None);
    assert_eq!(
        catalog
            .save_classification_term(&remove_builtin)
            .unwrap_err()
            .code,
        "classification_term_invalid"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), added);
    let remove = edit(&catalog, "remove", Some(custom()), None);
    let removed = catalog.save_classification_term(&remove).unwrap();
    assert_eq!(removed.catalog.taxonomy, original.catalog.taxonomy);
    assert_eq!(removed.catalog.active_run_id.as_deref(), Some("remove"));
    assert_eq!(catalog.save_classification_term(&remove).unwrap(), removed);
}

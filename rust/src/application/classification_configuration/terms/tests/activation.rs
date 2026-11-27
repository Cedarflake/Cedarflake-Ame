use super::*;
use crate::domain::classification::Taxonomy;
use crate::domain::classification::work::ClassificationDispatchMode;

fn cats(storage: &StoragePaths) -> CategoryDefinition {
    let selected = load_with_storage(storage)
        .unwrap()
        .into_iter()
        .find(|term| term.category.label == "猫咪")
        .unwrap();
    assert!(!selected.is_configured);
    definition(selected.category).unwrap()
}

#[test]
fn first_curated_save_configures_only_starters_and_the_selected_term() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let default = cats(&storage);
    let replacement = default.renamed("我的猫咪".into()).unwrap();
    let activation = change("activate-cats".into(), None, Some(replacement.clone())).unwrap();
    assert_eq!(activation.bundled_default.as_ref(), Some(&default));
    assert!(activation.changes_matching());

    let saved = save_with_storage(&storage, &activation).unwrap();
    assert_eq!(saved.categories.len(), 7);
    assert_eq!(saved.mode, ClassificationDispatchMode::Disabled);
    assert!(saved.run_id.is_none());
    assert!(!saved.installation_selected);
    let terms = load_with_storage(&storage).unwrap();
    assert_eq!(terms.iter().filter(|term| term.is_configured).count(), 6);
    let configured = terms
        .iter()
        .find(|term| term.category.id == default.id().as_str())
        .unwrap();
    assert_eq!(configured.category, input(&replacement));
    assert_eq!(configured.default_category.as_ref(), Some(&input(&default)));
    assert!(configured.is_configured);

    let stale = change("duplicate-activation".into(), None, Some(replacement)).unwrap();
    assert_eq!(
        save_with_storage(&storage, &stale).unwrap_err().code,
        "classification_term_changed"
    );
    assert_eq!(
        super::super::super::load_with_storage(&storage).unwrap(),
        saved
    );
}

#[test]
fn activation_merges_current_definitions_without_restoring_missing_starters() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let first = CategoryDefinition::new(
        CategoryId::new("custom-first".into()).unwrap(),
        "First".into(),
        vec!["A photograph of a forest".into()],
    )
    .unwrap();
    let mut initial = initial_change(ClassificationConfigurationAction::Save);
    initial.setup = classification_candidate::prepare(
        initial.operation_id.clone(),
        Taxonomy::new(vec![first.clone()]).unwrap(),
        model_package().compatibility(),
    )
    .unwrap();
    super::super::super::save_with_storage(&storage, &initial).unwrap();
    let selected = cats(&storage);
    let activation = change("activate-cats".into(), None, Some(selected.clone())).unwrap();
    let later = CategoryDefinition::new(
        CategoryId::new("custom-later".into()).unwrap(),
        "Later".into(),
        vec!["A photograph of a mountain".into()],
    )
    .unwrap();
    save_with_storage(
        &storage,
        &change("add-later".into(), None, Some(later.clone())).unwrap(),
    )
    .unwrap();

    let saved = save_with_storage(&storage, &activation).unwrap();
    assert_eq!(saved.categories.len(), 3);
    for expected in [first, later, selected] {
        let category = saved
            .categories
            .iter()
            .find(|category| category.id == expected.id().as_str())
            .unwrap();
        assert_eq!(category.label, expected.label());
        assert_eq!(category.descriptions.as_slice(), expected.descriptions());
    }
}

#[test]
fn unlisted_or_hidden_builtin_ids_cannot_initialize_a_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let before = super::super::super::load_with_storage(&storage).unwrap();
    for id in ["managed-unlisted-topic", "other"] {
        let replacement = CategoryDefinition::new(
            CategoryId::new(id.into()).unwrap(),
            "Unlisted".into(),
            vec!["An unrelated image".into()],
        )
        .unwrap();
        let request = change(format!("reject-{id}"), None, Some(replacement)).unwrap();
        assert_eq!(
            save_with_storage(&storage, &request).unwrap_err().code,
            "classification_term_invalid"
        );
        assert_eq!(
            super::super::super::load_with_storage(&storage).unwrap(),
            before
        );
    }
}

use super::super::tests::{change as initial_change, storage};
use super::*;
use crate::domain::classification_configuration::ClassificationConfigurationAction;

mod initialization;

#[test]
fn editor_includes_empty_definitions_and_restores_the_bundled_definition() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    super::super::save_with_storage(
        &storage,
        &initial_change(ClassificationConfigurationAction::Save),
    )
    .unwrap();
    let terms = load_with_storage(&storage).unwrap();
    assert_eq!(terms.len(), 5);
    assert!(terms.iter().all(|term| term.category.id != "other"));
    let original = terms
        .iter()
        .find(|term| term.category.id == "photo")
        .unwrap();
    let expected = definition(original.category.clone()).unwrap();
    let replacement = CategoryDefinition::new(
        expected.id().clone(),
        "旅行".into(),
        vec!["A photograph of mountains.".into()],
    )
    .unwrap();
    let edited = save_with_storage(
        &storage,
        &change(
            "edit".into(),
            Some(expected.clone()),
            Some(replacement.clone()),
        )
        .unwrap(),
    )
    .unwrap();
    let category = edited
        .categories
        .iter()
        .find(|category| category.id == "photo")
        .unwrap();
    assert_eq!(category.label, "旅行");
    assert_eq!(category.default_label.as_deref(), Some("实拍"));
    let defaults = load_with_storage(&storage)
        .unwrap()
        .into_iter()
        .find(|term| term.category.id == "photo")
        .unwrap()
        .default_category
        .unwrap();
    let restored = save_with_storage(
        &storage,
        &change(
            "restore".into(),
            Some(replacement),
            Some(definition(defaults).unwrap()),
        )
        .unwrap(),
    )
    .unwrap();
    let category = restored
        .categories
        .iter()
        .find(|category| category.id == "photo")
        .unwrap();
    assert_eq!(category.label, expected.label());
    assert_eq!(category.descriptions.as_slice(), expected.descriptions());
    assert!(
        !storage
            .catalog_path
            .with_file_name("classification-features")
            .exists()
    );
}

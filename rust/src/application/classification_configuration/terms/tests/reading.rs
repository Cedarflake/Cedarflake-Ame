use super::*;
use crate::domain::classification::Taxonomy;

#[test]
fn reading_the_complete_directory_does_not_initialize_or_dispatch_classification() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery).unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    assert!(before.catalog.taxonomy.is_none());

    let terms = load_with_storage(&storage).unwrap();
    assert_eq!(terms.len(), 56);
    assert!(terms.iter().all(|term| {
        !term.is_configured
            && !term.is_custom
            && term.default_category.as_ref() == Some(&term.category)
    }));
    assert!(terms.iter().any(|term| term.category.label == "猫咪"));
    assert!(terms.iter().all(|term| term.category.id != "other"));
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

#[test]
fn configured_definitions_override_defaults_without_configuring_the_remaining_directory() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let photo = CategoryDefinition::new(
        CategoryId::new("photo".into()).unwrap(),
        "旅行照片".into(),
        vec!["A photograph of a holiday.".into()],
    )
    .unwrap();
    let custom = CategoryDefinition::new(
        CategoryId::new("custom-hiking".into()).unwrap(),
        "登山".into(),
        vec!["Hikers walking on a mountain trail.".into()],
    )
    .unwrap();
    let mut configuration = initial_change(ClassificationConfigurationAction::Save);
    configuration.setup = classification_candidate::prepare(
        configuration.operation_id.clone(),
        Taxonomy::new(vec![photo.clone(), custom.clone()]).unwrap(),
        model_package().compatibility(),
    )
    .unwrap();
    super::super::super::save_with_storage(&storage, &configuration).unwrap();
    let catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery).unwrap();
    let before = catalog.load_classification_configuration().unwrap();

    let terms = load_with_storage(&storage).unwrap();
    assert_eq!(terms.len(), 57);
    assert_eq!(terms.iter().filter(|term| term.is_configured).count(), 2);
    let configured_photo = terms
        .iter()
        .find(|term| term.category.id == "photo")
        .unwrap();
    assert_eq!(configured_photo.category, input(&photo));
    assert!(configured_photo.is_configured);
    assert!(!configured_photo.is_custom);
    assert_eq!(
        configured_photo.default_category.as_ref().unwrap().label,
        "实拍"
    );
    let configured_custom = terms
        .iter()
        .find(|term| term.category.id == "custom-hiking")
        .unwrap();
    assert_eq!(configured_custom.category, input(&custom));
    assert!(configured_custom.is_configured);
    assert!(configured_custom.is_custom);
    assert!(configured_custom.default_category.is_none());
    let missing_builtin = terms
        .iter()
        .find(|term| term.category.id == "illustration")
        .unwrap();
    assert!(!missing_builtin.is_configured);
    assert_eq!(
        missing_builtin.default_category.as_ref(),
        Some(&missing_builtin.category),
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

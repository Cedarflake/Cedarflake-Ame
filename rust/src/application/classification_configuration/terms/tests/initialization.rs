use super::*;
use crate::domain::classification::work::ClassificationDispatchMode;

#[test]
fn first_custom_add_initializes_starters_and_can_be_deleted_without_models() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let draft = load_with_storage(&storage).unwrap();
    assert_eq!(draft.len(), 5);
    assert!(
        super::super::super::load_with_storage(&storage)
            .unwrap()
            .categories
            .is_empty()
    );
    let custom = CategoryDefinition::new(
        CategoryId::new("custom-first".into()).unwrap(),
        "我的照片".into(),
        vec!["A photograph of a family holiday".into()],
    )
    .unwrap();
    let added = save_with_storage(
        &storage,
        &change("add-first".into(), None, Some(custom.clone())).unwrap(),
    )
    .unwrap();
    assert_eq!(added.categories.len(), 7);
    assert_eq!(added.mode, ClassificationDispatchMode::Disabled);
    assert!(added.run_id.is_none());
    assert!(!added.installation_selected);
    let removed = save_with_storage(
        &storage,
        &change("remove-first".into(), Some(custom), None).unwrap(),
    )
    .unwrap();
    assert_eq!(removed.categories.len(), 6);
    assert_eq!(load_with_storage(&storage).unwrap(), draft);
}

#[test]
fn first_builtin_edit_initializes_once_and_preserves_the_reviewed_definition() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let draft = load_with_storage(&storage).unwrap();
    let selected = draft
        .iter()
        .find(|term| term.category.id == "photo")
        .unwrap();
    let expected = definition(selected.category.clone()).unwrap();
    let renamed = expected.renamed("My photographs".into()).unwrap();
    let saved = save_with_storage(
        &storage,
        &change("first-rename".into(), Some(expected), Some(renamed.clone())).unwrap(),
    )
    .unwrap();
    assert_eq!(saved.categories.len(), 6);
    let photo = saved
        .categories
        .iter()
        .find(|category| category.id == "photo")
        .unwrap();
    assert_eq!(photo.label, renamed.label());
    assert_eq!(photo.descriptions.as_slice(), renamed.descriptions());
}

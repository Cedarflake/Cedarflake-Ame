use super::*;

#[test]
fn batch_reset_uses_bundled_defaults_without_activating_dormant_terms_or_changing_customs() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    super::super::super::save_with_storage(
        &storage,
        &initial_change(ClassificationConfigurationAction::Save),
    )
    .unwrap();
    let defaults = defaults::categories().unwrap();
    let photo = defaults
        .get(&CategoryId::new("photo".into()).unwrap())
        .unwrap();
    let comic = defaults
        .get(&CategoryId::new("comic".into()).unwrap())
        .unwrap();
    let renamed = photo.renamed("My photos".into()).unwrap();
    let redefined = CategoryDefinition::new(
        comic.id().clone(),
        "My comics".into(),
        vec!["A comic strip with several panels".into()],
    )
    .unwrap();
    let custom = CategoryDefinition::new(
        CategoryId::new("custom-kept".into()).unwrap(),
        "旅行".into(),
        vec!["A photograph from a holiday".into()],
    )
    .unwrap();
    for (operation, expected, replacement) in [
        ("photo", Some(photo.clone()), renamed.clone()),
        ("comic", Some(comic.clone()), redefined.clone()),
        ("custom", None, custom.clone()),
    ] {
        save_with_storage(
            &storage,
            &change(operation.into(), expected, Some(replacement)).unwrap(),
        )
        .unwrap();
    }
    let before = load_with_storage(&storage).unwrap();
    let reset = reset_change("reset-all".into(), vec![input(&renamed), input(&redefined)]).unwrap();
    assert!(reset.changes_matching());
    reset_with_storage(&storage, &reset).unwrap();
    let after = load_with_storage(&storage).unwrap();
    for original in before {
        let current = after
            .iter()
            .find(|term| term.category.id == original.category.id)
            .unwrap();
        assert_eq!(current.is_configured, original.is_configured);
        if ["photo", "comic"].contains(&current.category.id.as_str()) {
            assert_eq!(Some(&current.category), current.default_category.as_ref());
        } else {
            assert_eq!(current, &original);
        }
    }
    assert_eq!(after.iter().filter(|term| term.is_configured).count(), 6);
    assert!(
        !storage
            .catalog_path
            .with_file_name("classification-features")
            .exists()
    );
}

#[test]
fn reset_admission_rejects_custom_unknown_hidden_and_duplicate_targets() {
    let photo = defaults::categories()
        .unwrap()
        .get(&CategoryId::new("photo".into()).unwrap())
        .unwrap();
    for id in ["custom-photo", "unknown-topic", "other"] {
        let mut target = input(photo);
        target.id = id.into();
        assert_eq!(
            reset_change(format!("reject-{id}"), vec![target])
                .err()
                .unwrap()
                .code,
            "classification_term_invalid"
        );
    }
    assert_eq!(
        reset_change("duplicate".into(), vec![input(photo), input(photo)])
            .err()
            .unwrap()
            .code,
        "classification_term_invalid"
    );
}

#[test]
fn empty_reset_is_read_only_and_reviewed_dormant_terms_cannot_initialize_the_catalog() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let before = super::super::super::load_with_storage(&storage).unwrap();
    let empty = reset_change("empty".into(), vec![]).unwrap();
    assert!(!empty.changes_matching());
    assert_eq!(reset_with_storage(&storage, &empty).unwrap(), before);
    let photo = defaults::categories()
        .unwrap()
        .get(&CategoryId::new("photo".into()).unwrap())
        .unwrap();
    let absent = reset_change("absent".into(), vec![input(photo)]).unwrap();
    assert_eq!(
        reset_with_storage(&storage, &absent).unwrap_err().code,
        "classification_term_changed"
    );
    assert_eq!(
        super::super::super::load_with_storage(&storage).unwrap(),
        before
    );
    let original = save_with_storage(
        &storage,
        &change("initial".into(), None, Some(photo.clone())).unwrap(),
    )
    .unwrap();
    let unchanged = reset_change("unchanged".into(), vec![input(photo)]).unwrap();
    assert!(!unchanged.changes_matching());
    assert_eq!(reset_with_storage(&storage, &unchanged).unwrap(), original);
    assert_eq!(reset_with_storage(&storage, &empty).unwrap(), original);
}

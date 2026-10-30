use super::*;
use crate::domain::classification::work::ClassificationDispatchMode;

fn storage(root: &std::path::Path) -> StoragePaths {
    StoragePaths {
        catalog_path: root.join("catalog.sqlite3"),
        preview_root: root.join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: root.join("settings.sqlite3"),
    }
}

fn change(action: ClassificationConfigurationAction) -> ClassificationConfigurationChange {
    let id = ClassificationRecordId::new("configure".into()).unwrap();
    ClassificationConfigurationChange {
        operation_id: id.clone(),
        expected_catalog_revision: 0,
        expected_dispatch_revision: 0,
        setup: classification_candidate::prepare(
            id,
            classification_candidate::initial_categories().unwrap(),
            selected_model_compatibility(),
        )
        .unwrap(),
        action,
    }
}

#[test]
fn categories_can_be_saved_without_models_but_enablement_requires_verified_installation() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let before = load_with_storage(&storage).unwrap();
    assert!(before.categories.is_empty());
    assert!(!before.installation_selected);
    assert_eq!(
        save_with_storage(&storage, &change(ClassificationConfigurationAction::Enable))
            .unwrap_err()
            .code,
        "classification_installation_missing"
    );
    assert_eq!(load_with_storage(&storage).unwrap(), before);
    let saved =
        save_with_storage(&storage, &change(ClassificationConfigurationAction::Save)).unwrap();
    assert_eq!(saved.categories.len(), 6);
    assert_eq!(saved.mode, ClassificationDispatchMode::Disabled);
    assert!(saved.run_id.is_none());
    assert!(!saved.installation_selected);
    assert!(
        !storage
            .catalog_path
            .with_file_name("classification-features")
            .exists()
    );
}

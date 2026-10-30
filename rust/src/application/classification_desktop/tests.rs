use std::sync::Arc;

use crate::adapters::SqliteCatalog;
use crate::domain::classification::work::ClassificationDispatchCommand;
use crate::domain::classification::*;
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_setup::ClassificationSetupRepository;

use super::*;

#[test]
fn disabled_startup_does_not_require_or_create_model_installation() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    assert!(
        open_with_storage(&storage, directory.path().join("app.exe"))
            .unwrap()
            .is_none()
    );
    assert!(!installation_path(&storage).exists());
    assert!(
        !storage
            .catalog_path
            .with_file_name("classification-features.sqlite3")
            .exists()
    );
}

#[test]
fn paused_startup_close_before_run_and_joined_shutdown_preserve_intent_without_loading_models() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let executable = directory.path().join("app.exe");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let taxonomy = Taxonomy::new(
        ["drawing", "photo"]
            .map(|label| {
                CategoryDefinition::new(
                    CategoryId::new(label.into()).unwrap(),
                    label.into(),
                    vec![format!("A {label}")],
                )
                .unwrap()
            })
            .into(),
    )
    .unwrap();
    catalog
        .configure_classification(&id("configure"), 0, &taxonomy)
        .unwrap();
    let run = ClassificationRun::new(
        id("desktop-run"),
        ModelCompatibility::new(
            ClassificationRevision::from_bytes([1; 32]),
            ClassificationRevision::from_bytes([2; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            2,
        )
        .unwrap(),
        taxonomy.revision(),
        None,
    )
    .unwrap();
    let setup = ClassificationAnalysisSetup::new(run, taxonomy, None).unwrap();
    let revision = catalog.load_classification_state().unwrap().revision;
    let active = catalog
        .activate_classification_setup(revision, 0, &setup, ClassificationSetupActivation::Enable)
        .unwrap();
    catalog
        .change_classification_dispatch(active.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    let error = open_with_storage(&storage, executable.clone())
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_installation_missing");
    ClassificationInstallationStore::save(
        &installation_path(&storage),
        &ClassificationInstallation {
            models: directory.path().join("absent-models"),
            runtime: directory.path().join("absent-runtime"),
        },
    )
    .unwrap();
    let session = open_with_storage(&storage, executable.clone())
        .unwrap()
        .unwrap();
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
    assert_eq!(
        open_with_storage(&storage, executable.clone())
            .err()
            .unwrap()
            .code,
        "classification_runtime_active"
    );
    session.close().unwrap();
    assert_eq!(
        session.run().unwrap_err().code,
        "classification_session_started"
    );
    drop(session);

    let session = Arc::new(open_with_storage(&storage, executable).unwrap().unwrap());
    session
        .foreground_changed(ClassificationForegroundState::default())
        .unwrap();
    let running = session.clone();
    let thread = std::thread::spawn(move || running.run());
    session.request_close().unwrap();
    thread.join().unwrap().unwrap();
    session.close().unwrap();
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
    assert!(
        !storage
            .catalog_path
            .with_file_name("classification-features.sqlite3")
            .exists()
    );
}

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn storage(root: &std::path::Path) -> StoragePaths {
    StoragePaths {
        catalog_path: root.join("catalog.sqlite3"),
        preview_root: root.join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: root.join("settings.sqlite3"),
    }
}

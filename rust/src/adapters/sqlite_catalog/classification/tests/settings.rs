use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchMode,
};
use crate::domain::classification::{ClassificationAnalysisSetup, Taxonomy};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::ClassificationConfigurationRepository;
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::*;

fn request(
    catalog: &SqliteCatalog,
    operation: &str,
    rules: Taxonomy,
    action: ClassificationConfigurationAction,
) -> ClassificationConfigurationChange {
    let current = catalog.load_classification_configuration().unwrap();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id(operation), model, rules.revision(), None).unwrap();
    ClassificationConfigurationChange {
        operation_id: id(operation),
        expected_catalog_revision: current.catalog.revision,
        expected_dispatch_revision: current.dispatch.revision,
        setup: ClassificationAnalysisSetup::new(run, rules, None).unwrap(),
        action,
    }
}

fn edited_rules(label: &str, description: &str) -> Taxonomy {
    Taxonomy::new(vec![
        CategoryDefinition::new(category("drawing"), label.into(), vec![description.into()])
            .unwrap(),
    ])
    .unwrap()
}

#[test]
fn saving_categories_keeps_classification_disabled_until_explicit_enablement() {
    let (_directory, mut catalog) = fixture();
    let save = request(
        &catalog,
        "save",
        taxonomy(),
        ClassificationConfigurationAction::Save,
    );
    let saved = catalog.save_classification_configuration(&save).unwrap();
    assert_eq!(saved.catalog.taxonomy.as_ref(), Some(save.setup.primary()));
    assert_eq!(saved.dispatch.mode, ClassificationDispatchMode::Disabled);
    assert!(saved.catalog.active_run_id.is_none());
    assert!(
        catalog
            .load_classification_setup(save.setup.run().id())
            .unwrap()
            .is_none()
    );

    let enable = request(
        &catalog,
        "enable",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    let enabled = catalog.save_classification_configuration(&enable).unwrap();
    assert_eq!(enabled.dispatch.mode, ClassificationDispatchMode::Automatic);
    assert_eq!(enabled.catalog.active_run_id.as_deref(), Some("enable"));
    assert_eq!(
        catalog
            .load_classification_setup(enable.setup.run().id())
            .unwrap(),
        Some(enable.setup)
    );
}

#[test]
fn semantic_changes_replace_the_run_but_preserve_pause_and_survive_reopening() {
    let (directory, mut catalog) = fixture();
    let start = request(
        &catalog,
        "start",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    let started = catalog.save_classification_configuration(&start).unwrap();
    catalog
        .change_classification_dispatch(
            started.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let edit = request(
        &catalog,
        "edit",
        edited_rules("drawing", "A standalone drawing"),
        ClassificationConfigurationAction::Save,
    );
    let saved = catalog.save_classification_configuration(&edit).unwrap();
    assert_eq!(saved.dispatch.mode, ClassificationDispatchMode::Paused);
    assert_eq!(saved.catalog.active_run_id.as_deref(), Some("edit"));
    assert_eq!(saved.dispatch.run_id.as_ref(), Some(edit.setup.run().id()));
    assert_eq!(
        catalog
            .load_classification_setup(start.setup.run().id())
            .unwrap(),
        Some(start.setup)
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_configuration().unwrap(), saved);
    assert_eq!(
        catalog
            .load_classification_setup(edit.setup.run().id())
            .unwrap(),
        Some(edit.setup)
    );
}

#[test]
fn display_renames_preserve_the_run_and_dispatch_revision() {
    let (_directory, mut catalog) = fixture();
    let start = request(
        &catalog,
        "start",
        edited_rules("drawing", "A drawing"),
        ClassificationConfigurationAction::Enable,
    );
    let before = catalog.save_classification_configuration(&start).unwrap();
    let rename = request(
        &catalog,
        "rename",
        edited_rules("Artwork", "A drawing"),
        ClassificationConfigurationAction::Save,
    );
    let after = catalog.save_classification_configuration(&rename).unwrap();
    assert_eq!(after.dispatch, before.dispatch);
    assert_eq!(after.catalog.active_run_id, before.catalog.active_run_id);
    assert_eq!(after.catalog.configuration, before.catalog.configuration);
    assert_eq!(
        after.catalog.taxonomy.unwrap().categories()[0].label(),
        "Artwork"
    );
}

#[test]
fn repeating_configuration_preserves_later_stop_and_rejects_operation_reuse() {
    let (_directory, mut catalog) = fixture();
    let mut start = request(
        &catalog,
        "start",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    let started = catalog.save_classification_configuration(&start).unwrap();
    let stopped = catalog
        .change_classification_dispatch(
            started.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    assert_eq!(
        catalog
            .save_classification_configuration(&start)
            .unwrap()
            .dispatch,
        stopped
    );
    start.action = ClassificationConfigurationAction::Save;
    assert_eq!(
        catalog
            .save_classification_configuration(&start)
            .unwrap_err()
            .code,
        "classification_operation_conflict"
    );
    let edit = request(
        &catalog,
        "edit-stopped",
        edited_rules("drawing", "A new drawing"),
        ClassificationConfigurationAction::Save,
    );
    let saved = catalog.save_classification_configuration(&edit).unwrap();
    assert_eq!(saved.dispatch, stopped);
    assert!(saved.catalog.active_run_id.is_none());
}

#[test]
fn stale_dispatch_and_failed_activation_roll_back_category_changes() {
    let (_directory, mut catalog) = fixture();
    let start = request(
        &catalog,
        "start",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    let started = catalog.save_classification_configuration(&start).unwrap();
    let stale = request(
        &catalog,
        "stale",
        edited_rules("drawing", "A drawing"),
        ClassificationConfigurationAction::Save,
    );
    catalog
        .change_classification_dispatch(
            started.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    assert!(catalog.save_classification_configuration(&stale).is_err());
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);

    catalog.connection.execute_batch("CREATE TEMP TRIGGER reject_setup BEFORE INSERT ON classification_analysis_setups BEGIN SELECT RAISE(ABORT, 'injected setup failure'); END;").unwrap();
    let failed = request(
        &catalog,
        "failed",
        edited_rules("drawing", "A drawing"),
        ClassificationConfigurationAction::Save,
    );
    assert!(catalog.save_classification_configuration(&failed).is_err());
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_setup;")
        .unwrap();
    assert!(catalog.save_classification_configuration(&failed).is_ok());
}

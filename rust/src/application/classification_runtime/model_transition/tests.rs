use super::*;
use crate::adapters::SqliteCatalog;
use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchMode,
};
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRevision, Taxonomy,
};
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

#[test]
fn device_transition_preserves_rules_pause_and_old_runs_and_rejects_a_stale_setup() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(database.clone()).unwrap();
    let model = |pipeline| {
        ModelCompatibility::new(
            ClassificationRevision::from_bytes([1; 32]),
            ClassificationRevision::from_bytes([pipeline; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            768,
        )
        .unwrap()
    };
    let rules = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("drawing".into()).unwrap(),
            "绘画".into(),
            vec!["a drawing".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    let cpu =
        crate::application::classification_candidate::prepare(id("cpu"), rules.clone(), model(2))
            .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let enabled = catalog
        .save_classification_configuration(&ClassificationConfigurationChange {
            operation_id: id("enable"),
            expected_catalog_revision: before.catalog.revision,
            expected_dispatch_revision: before.dispatch.revision,
            setup: cpu.clone(),
            action: ClassificationConfigurationAction::Enable,
        })
        .unwrap();
    catalog
        .change_classification_dispatch(
            enabled.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let renamed_rules = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("drawing".into()).unwrap(),
            "保留名称".into(),
            vec!["a drawing".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    let before_rename = catalog.load_classification_configuration().unwrap();
    catalog
        .save_classification_configuration(&ClassificationConfigurationChange {
            operation_id: id("rename"),
            expected_catalog_revision: before_rename.catalog.revision,
            expected_dispatch_revision: before_rename.dispatch.revision,
            setup: crate::application::classification_candidate::prepare(
                id("rename"),
                renamed_rules.clone(),
                model(2),
            )
            .unwrap(),
            action: ClassificationConfigurationAction::Save,
        })
        .unwrap();
    let gpu = replace_model(&mut catalog, cpu.clone(), model(4), &id("gpu")).unwrap();
    assert_eq!(gpu.primary(), &renamed_rules);
    assert_ne!(
        gpu.run().model().image_identity(),
        cpu.run().model().image_identity()
    );
    assert_eq!(
        gpu.run().model().text_identity(),
        cpu.run().model().text_identity()
    );
    let transitioned = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        transitioned.dispatch.mode,
        ClassificationDispatchMode::Paused
    );
    assert_eq!(transitioned.dispatch.run_id.as_ref(), Some(gpu.run().id()));
    assert!(replace_model(&mut catalog, cpu.clone(), model(4), &id("stale")).is_err());
    assert_eq!(
        catalog.load_classification_configuration().unwrap(),
        transitioned
    );
    assert_eq!(
        replace_model(&mut catalog, gpu.clone(), model(4), &id("same")).unwrap(),
        gpu
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(database).unwrap();
    assert_eq!(
        catalog.load_classification_setup(cpu.run().id()).unwrap(),
        Some(
            ClassificationAnalysisSetup::new(cpu.run().clone(), renamed_rules.clone(), None)
                .unwrap()
        )
    );
    assert_eq!(
        catalog.load_classification_setup(gpu.run().id()).unwrap(),
        Some(gpu.clone())
    );
    let restored = replace_model(&mut catalog, gpu, model(2), &id("cpu-again")).unwrap();
    assert_eq!(restored.run().model(), cpu.run().model());
    assert_eq!(restored.primary(), &renamed_rules);
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
}

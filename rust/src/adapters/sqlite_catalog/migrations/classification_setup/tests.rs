use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::{fixture, selected};
use crate::domain::classification::work::ClassificationDispatchCommand;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationAction, ClassificationRecordId,
    ClassificationRevision, ClassificationRun, ModelCompatibility, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;
use crate::ports::organization::OrganizationRepository;

use super::*;

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

#[test]
fn upgrade_preserves_legacy_pause_human_decisions_and_source_boundary() {
    let (directory, mut catalog) = fixture();
    let selection = selected(&catalog, "a");
    let category = CategoryId::new("photo".into()).unwrap();
    let taxonomy = Taxonomy::new(vec![
        CategoryDefinition::new(
            category.clone(),
            "Photo".into(),
            vec!["A photograph".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    catalog
        .configure_classification(&id("configure"), 0, &taxonomy)
        .unwrap();
    let run = ClassificationRun::new(
        id("legacy"),
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
    catalog.begin_classification_run(&run, None).unwrap();
    let enabled = catalog
        .change_classification_dispatch(
            0,
            &ClassificationDispatchCommand::Enable {
                run_id: run.id().clone(),
            },
        )
        .unwrap();
    let paused = catalog
        .change_classification_dispatch(enabled.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    let prepared = catalog
        .prepare_classification_selection(std::slice::from_ref(&selection))
        .unwrap();
    catalog
        .change_classification(
            &id("human"),
            &prepared,
            &ClassificationAction::Correct(category),
        )
        .unwrap();
    catalog
        .create_organization_album("album", 0, "Retained")
        .unwrap();
    let selected_r4 = catalog
        .prepare_organization_selection(std::slice::from_ref(&selection))
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &selected_r4, true)
        .unwrap();
    let before = catalog
        .prepare_classification_selection(std::slice::from_ref(&selection))
        .unwrap();
    let before_r4 = catalog
        .inspect_organization_selection(std::slice::from_ref(&selection))
        .unwrap();
    downgrade_to_v66_for_test(&catalog.connection);
    migrate(&mut catalog.connection).unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.classification_dispatch_state().unwrap(), paused);
    assert!(
        catalog
            .load_classification_setup(run.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        catalog
            .prepare_classification_selection(std::slice::from_ref(&selection))
            .unwrap(),
        before
    );
    assert_eq!(
        catalog
            .inspect_organization_selection(&[selection])
            .unwrap(),
        before_r4
    );
    let (ceiling, next): (i64, i64) = catalog.connection.query_row(
        "SELECT backlog_ceiling, next_source_generation FROM classification_dispatch_admission CROSS JOIN catalog_state",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(ceiling, next - 1);
}

#[test]
fn failed_upgrade_rolls_back_setup_tables_and_version() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v66_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_setup_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 67 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        66
    );
    for (name, _) in OBJECTS {
        let exists: bool = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists, "{name}");
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_setup_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn startup_rejects_a_missing_or_impossible_source_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute(
            "UPDATE classification_dispatch_state SET source_cursor = 1",
            [],
        )
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog
        .connection
        .execute(
            "UPDATE classification_dispatch_state SET source_cursor = 0",
            [],
        )
        .unwrap();
    catalog.connection.execute_batch(
        "UPDATE classification_dispatch_admission SET arrival_cursor = (SELECT next_source_generation FROM catalog_state);",
    ).unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_admission", [])
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
}

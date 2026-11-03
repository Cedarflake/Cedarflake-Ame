use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::{fixture, publish, selected};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationAction, ClassificationRecordId, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_work::ClassificationWorkRepository;
use crate::ports::organization::OrganizationRepository;

use super::*;

#[test]
fn upgrade_preserves_classification_decisions_without_enabling_analysis() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    catalog
        .create_organization_album("retained", 0, "Retained")
        .unwrap();
    let category = CategoryId::new("drawing".into()).unwrap();
    let taxonomy = Taxonomy::new(vec![
        CategoryDefinition::new(category.clone(), "Drawing".into(), vec!["A drawing".into()])
            .unwrap(),
    ])
    .unwrap();
    catalog
        .configure_classification(
            &ClassificationRecordId::new("configure".into()).unwrap(),
            0,
            &taxonomy,
        )
        .unwrap();
    let selection = selected(&catalog, "a");
    let prepared = catalog
        .prepare_classification_selection(std::slice::from_ref(&selection))
        .unwrap();
    catalog
        .change_classification(
            &ClassificationRecordId::new("correct".into()).unwrap(),
            &prepared,
            &ClassificationAction::Correct(category.clone()),
        )
        .unwrap();
    let before = catalog
        .prepare_classification_selection(std::slice::from_ref(&selection))
        .unwrap();
    downgrade_to_v65_for_test(&catalog.connection);
    migrate(&mut catalog.connection).unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let after = catalog
        .prepare_classification_selection(&[selection])
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(after.groups[0].effective().category, Some(category));
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Disabled
    );
    let albums: i64 = catalog
        .connection
        .query_row("SELECT count(*) FROM organization_albums", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(albums, 1);
}

#[test]
fn failed_upgrade_rolls_back_queue_objects_and_version() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v65_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_dispatch_upgrade BEFORE UPDATE OF version ON schema_info
        WHEN NEW.version = 66 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        65
    );
    for (_, name, _) in schema::OBJECTS {
        let present: bool = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!present, "{name}");
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_dispatch_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn startup_rejects_missing_dispatch_state() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_admission", [])
        .unwrap();
    catalog
        .connection
        .execute("DELETE FROM classification_dispatch_state", [])
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
}

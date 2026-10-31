use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::*;

#[test]
fn upgrade_retains_legacy_definitions_without_claiming_automatic_ownership() {
    let (directory, mut catalog) = fixture();
    let rules = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("manual".into()).unwrap(),
            "My category".into(),
            vec!["my description".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    catalog
        .configure_classification(
            &ClassificationRecordId::new("save".into()).unwrap(),
            0,
            &rules,
        )
        .unwrap();
    let before = catalog.load_classification_state().unwrap();
    let dispatch = catalog.classification_dispatch_state().unwrap();
    downgrade_to_v70_for_test(&catalog.connection);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_state().unwrap(), before);
    assert_eq!(catalog.classification_dispatch_state().unwrap(), dispatch);
    validate(&catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
    let counts: (i64, i64) = catalog.connection.query_row("SELECT (SELECT COUNT(*) FROM classification_managed_categories), (SELECT COUNT(*) FROM classification_maintenance_evidence)", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(counts, (0, 0));
}

#[test]
fn failed_upgrade_rolls_back_both_tables_and_retries() {
    let (_directory, mut catalog) = fixture();
    downgrade_to_v70_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_maintenance_upgrade BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 71 BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        70
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
        assert!(!exists);
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_maintenance_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

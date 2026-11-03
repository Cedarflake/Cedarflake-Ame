use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, ClassificationRevision,
    ClassificationRun, ModelCompatibility, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;

use super::*;

fn old_evidence(catalog: &mut SqliteCatalog) -> String {
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
    let run = ClassificationRun::new(
        ClassificationRecordId::new("run".into()).unwrap(),
        ModelCompatibility::new(
            ClassificationRevision::from_bytes([1; 32]),
            ClassificationRevision::from_bytes([2; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            2,
        )
        .unwrap(),
        rules.revision(),
        None,
    )
    .unwrap();
    catalog.begin_classification_run(&run, None).unwrap();
    let payload = "{\"version\":1,\"retained\":\"原有证据\"}".to_owned();
    catalog
        .connection
        .execute(
            "INSERT INTO classification_maintenance_evidence VALUES ('save', 'run', ?1, ?2)",
            rusqlite::params!["a".repeat(64), payload],
        )
        .unwrap();
    payload
}

#[test]
fn retirement_upgrade_preserves_existing_evidence_and_configuration() {
    let (directory, mut catalog) = fixture();
    let payload = old_evidence(&mut catalog);
    let before = catalog.load_classification_state().unwrap();
    downgrade_to_v71_for_test(&catalog.connection);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        crate::adapters::sqlite_catalog::SCHEMA_VERSION
    );
    assert_eq!(catalog.load_classification_state().unwrap(), before);
    let retained: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, payload);
    classification_maintenance::validate_rows(&catalog.connection).unwrap();
    for invalid in [
        r#"{"version":2,"kind":"other","retirements":[{}]}"#,
        r#"{"version":2,"kind":"equivalent_category_retirement","retirements":[]}"#,
    ] {
        assert!(
            catalog
                .connection
                .execute(
                    "UPDATE classification_maintenance_evidence SET payload = ?1",
                    [invalid]
                )
                .is_err()
        );
    }
}

#[test]
fn failed_retirement_upgrade_keeps_the_old_table_and_can_retry() {
    let (_directory, mut catalog) = fixture();
    let payload = old_evidence(&mut catalog);
    downgrade_to_v71_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_retirement_upgrade BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 72 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        71
    );
    classification_maintenance::validate(&catalog.connection).unwrap();
    let retained: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, payload);
    let temporary: bool = catalog.connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'classification_maintenance_evidence_v71')", [], |row| row.get(0)).unwrap();
    assert!(!temporary);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_retirement_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        72
    );
    classification_maintenance::validate_rows(&catalog.connection).unwrap();
}

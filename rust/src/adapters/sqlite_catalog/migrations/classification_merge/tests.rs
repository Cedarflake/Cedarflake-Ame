use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, ClassificationRevision,
    ClassificationRun, ModelCompatibility, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;

use super::*;

fn old_records(catalog: &mut SqliteCatalog) -> Vec<String> {
    let id = |name: &str| ClassificationRecordId::new(name.into()).unwrap();
    let rules = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("drawing".into()).unwrap(),
            "Drawing".into(),
            vec!["a drawing".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    catalog
        .configure_classification(&id("save"), 0, &rules)
        .unwrap();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id("run"), model, rules.revision(), None).unwrap();
    catalog.begin_classification_run(&run, None).unwrap();
    let revision = catalog.load_classification_state().unwrap().revision;
    catalog
        .configure_classification(&id("retirement"), revision, &rules)
        .unwrap();
    let records = vec![
        r#"{"version":1,"retained":"原有依据"}"#.to_owned(),
        r#"{"version":2,"kind":"equivalent_category_retirement","retirements":[{"category":"old","retained":"drawing"}]}"#.to_owned(),
    ];
    for (operation, payload) in ["save", "retirement"].iter().zip(&records) {
        catalog
            .connection
            .execute(
                "INSERT INTO classification_maintenance_evidence VALUES (?1, 'run', ?2, ?3)",
                rusqlite::params![operation, "a".repeat(64), payload],
            )
            .unwrap();
    }
    records
}

#[test]
fn purpose_merge_upgrade_preserves_both_prior_evidence_formats_and_configuration() {
    let (directory, mut catalog) = fixture();
    let expected = old_records(&mut catalog);
    let before = catalog.load_classification_state().unwrap();
    downgrade_to_v72_for_test(&catalog.connection);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        super::super::schema_shape::schema_version(&catalog.connection).unwrap(),
        73
    );
    assert_eq!(catalog.load_classification_state().unwrap(), before);
    for (operation, payload) in ["save", "retirement"].iter().zip(expected) {
        let retained: String = catalog
            .connection
            .query_row(
                "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = ?1",
                [operation],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained, payload);
    }
    super::super::classification_maintenance::validate_rows(&catalog.connection).unwrap();
    for invalid in [
        r#"{"version":3,"kind":"other","merge":{"category":"old","purpose":"drawing"}}"#,
        r#"{"version":3,"kind":"purpose_category_merge","merge":[]}"#,
        r#"{"version":3,"kind":"purpose_category_merge","merge":{"purpose":"drawing"}}"#,
    ] {
        assert!(catalog.connection.execute("UPDATE classification_maintenance_evidence SET payload = ?1 WHERE operation_id = 'save'", [invalid]).is_err());
    }
}

#[test]
fn failed_purpose_merge_upgrade_rolls_back_and_retries_without_losing_evidence() {
    let (_directory, mut catalog) = fixture();
    old_records(&mut catalog);
    downgrade_to_v72_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_merge_upgrade BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 73 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        super::super::schema_shape::schema_version(&catalog.connection).unwrap(),
        72
    );
    super::super::classification_maintenance::validate(&catalog.connection).unwrap();
    let (count, temporary): (i64, bool) = catalog.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM classification_maintenance_evidence),
                EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'classification_maintenance_evidence_v72')",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(count, 2);
    assert!(!temporary);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_merge_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(
        super::super::schema_shape::schema_version(&catalog.connection).unwrap(),
        73
    );
    super::super::classification_maintenance::validate_rows(&catalog.connection).unwrap();
}

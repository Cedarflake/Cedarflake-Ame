use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::{fixture, selected};
use crate::domain::classification::work::*;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, ClassificationRevision,
    ClassificationRun, ModelCompatibility, Taxonomy,
};
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_work::ClassificationWorkRepository;
use crate::ports::organization::OrganizationRepository;

use super::*;

#[test]
fn failed_availability_upgrade_rolls_back_tables_and_can_retry() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v67_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_availability_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 68 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        67
    );
    for (_, name, _) in OBJECTS {
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
        .execute_batch("DROP TRIGGER reject_availability_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn upgrade_reconsiders_legacy_waiting_work_without_resuming_pause_or_changing_favorites() {
    let (directory, mut catalog) = fixture();
    let id = |value: &str| ClassificationRecordId::new(value.into()).unwrap();
    let taxonomy = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("photo".into()).unwrap(),
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
    let session = id("session");
    catalog
        .open_classification_session(enabled.revision, &session)
        .unwrap();
    let selected_a = selected(&catalog, "a");
    catalog
        .request_classification_sources(
            &session,
            std::slice::from_ref(&selected_a),
            ClassificationWorkPriority::Requested,
            10,
        )
        .unwrap();
    let claim = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &claim,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Unavailable,
                code: "offline".into(),
                retry_unix_ms: 0,
            },
        )
        .unwrap();
    let revision = catalog.classification_dispatch_state().unwrap().revision;
    let paused = catalog
        .change_classification_dispatch(revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    let prepared = catalog
        .prepare_organization_selection(std::slice::from_ref(&selected_a))
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let human = catalog
        .inspect_organization_selection(std::slice::from_ref(&selected_a))
        .unwrap();
    let configuration = catalog.load_classification_state().unwrap();
    downgrade_to_v67_for_test(&catalog.connection);
    migrate(&mut catalog.connection).unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.classification_dispatch_state().unwrap(), paused);
    assert_eq!(catalog.load_classification_state().unwrap(), configuration);
    assert_eq!(
        catalog
            .inspect_organization_selection(&[selected_a])
            .unwrap(),
        human
    );
    let phase: String = catalog
        .connection
        .query_row("SELECT phase FROM classification_work_items", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(phase, "queued");
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        68
    );
    validate_rows(&catalog.connection).unwrap();
}

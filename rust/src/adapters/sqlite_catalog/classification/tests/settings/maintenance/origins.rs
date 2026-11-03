use crate::domain::classification::maintenance::ManagedCategoryOrigin;
use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::*;

fn origin(catalog: &SqliteCatalog, identity: &CategoryId) -> ManagedCategoryOrigin {
    catalog
        .load_category_expansion_snapshot(std::slice::from_ref(identity))
        .unwrap()
        .managed_categories
        .remove(identity)
        .unwrap()
}

fn count(catalog: &SqliteCatalog, table: &str) -> i64 {
    catalog
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn origin_and_complete_parameters_survive_restart_and_committed_retry() {
    let (directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    let identity = plan.additions()[0].clone();
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let before = origin(&catalog, &identity);
    assert!(!before.protected);
    assert_eq!(before.admitted_label, "Architecture");
    assert_eq!(before.operation_id, id("expand"));
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'expand'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["criteria"]["minimum_members"], 2);
    assert_eq!(payload["bounds"]["entry_points"], 1);
    assert_eq!(payload["members"].as_array().unwrap().len(), 2);
    assert_eq!(payload["sources"].as_array().unwrap().len(), 3);
    assert_eq!(
        payload["candidates"][0]["supporting_contents"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(payload["run"]["id"], "expand");
    assert_eq!(
        payload["comparison_rules"],
        codec::revision(edited_rules("Drawing", "baseline").revision())
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(origin(&catalog, &identity), before);
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(count(&catalog, "classification_maintenance_evidence"), 1);
    assert_eq!(count(&catalog, "classification_managed_categories"), 1);
}

#[test]
fn explicit_edits_and_retirement_remain_protected_after_reverting_wording() {
    for retire in [false, true] {
        let (_directory, mut catalog) = super::super::super::fixture();
        start(&mut catalog);
        let plan = plan(&catalog, &evidence(&catalog));
        catalog
            .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
            .unwrap();
        let identity = plan.additions()[0].clone();
        let initial = catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .taxonomy
            .unwrap();
        let mut categories = initial.categories().to_vec();
        if retire {
            categories.retain(|category| category.id() != &identity);
        } else {
            let category = categories
                .iter_mut()
                .find(|category| category.id() == &identity)
                .unwrap();
            *category = category.renamed("My buildings".into()).unwrap();
        }
        let edit = request(
            &catalog,
            "manual-edit",
            Taxonomy::new(categories).unwrap(),
            ClassificationConfigurationAction::Save,
        );
        catalog.save_classification_configuration(&edit).unwrap();
        assert!(origin(&catalog, &identity).protected);
        let reset = request(
            &catalog,
            "reset-wording",
            initial,
            ClassificationConfigurationAction::Save,
        );
        catalog.save_classification_configuration(&reset).unwrap();
        catalog
            .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
            .unwrap();
        assert!(origin(&catalog, &identity).protected);
    }
}

#[test]
fn direct_configuration_protects_edits_but_replayed_old_save_does_not_claim_new_categories() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let saved_rules = catalog
        .load_classification_configuration()
        .unwrap()
        .catalog
        .taxonomy
        .unwrap();
    let unchanged = request(
        &catalog,
        "before-expansion",
        saved_rules,
        ClassificationConfigurationAction::Save,
    );
    catalog
        .save_classification_configuration(&unchanged)
        .unwrap();
    let plan = plan(&catalog, &evidence(&catalog));
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let identity = plan.additions()[0].clone();
    catalog
        .save_classification_configuration(&unchanged)
        .unwrap();
    assert!(!origin(&catalog, &identity).protected);
    let current = catalog.load_classification_state().unwrap();
    catalog
        .configure_classification(
            &id("direct-retirement"),
            current.revision,
            &edited_rules("Drawing", "baseline"),
        )
        .unwrap();
    assert!(origin(&catalog, &identity).protected);
}

#[test]
fn provenance_failure_rolls_back_definitions_run_and_receipt() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    let before = catalog.load_classification_configuration().unwrap();
    catalog.connection.execute_batch("CREATE TEMP TRIGGER reject_origin BEFORE INSERT ON classification_managed_categories BEGIN SELECT RAISE(ABORT, 'injected origin failure'); END;").unwrap();
    assert!(
        catalog
            .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
            .is_err()
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert_eq!(count(&catalog, "classification_maintenance_evidence"), 0);
    assert_eq!(count(&catalog, "classification_managed_categories"), 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_origin")
        .unwrap();
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
}

#[test]
fn backup_restores_origins_and_protection_without_models_or_discovery_samples() {
    let (directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let identity = plan.additions()[0].clone();
    let automatic = directory.path().join("automatic.ame-backup");
    catalog
        .export_organization_archive(&automatic, &mut |_| true)
        .unwrap();
    let edit = request(
        &catalog,
        "retire",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&edit).unwrap();
    let backup = directory.path().join("protected.ame-backup");
    catalog
        .export_organization_archive(&backup, &mut |_| true)
        .unwrap();

    let target_path = directory.path().join("restored.sqlite3");
    let mut target = SqliteCatalog::open(target_path.clone()).unwrap();
    super::super::super::archives::restore(&mut target, &automatic, "restore-automatic").unwrap();
    assert!(!origin(&target, &identity).protected);
    assert_eq!(count(&target, "classification_maintenance_evidence"), 0);
    assert_eq!(count(&target, "classification_runs"), 0);
    super::super::super::archives::restore(&mut target, &backup, "restore-protected").unwrap();
    assert!(origin(&target, &identity).protected);
    super::super::super::archives::restore(&mut target, &automatic, "restore-old").unwrap();
    assert!(origin(&target, &identity).protected);
    drop(target);
    let target = SqliteCatalog::open(target_path).unwrap();
    assert!(origin(&target, &identity).protected);
    let reexport = directory.path().join("reexport.ame-backup");
    target
        .export_organization_archive(&reexport, &mut |_| true)
        .unwrap();
    let mut third = SqliteCatalog::open(directory.path().join("third.sqlite3")).unwrap();
    super::super::super::archives::restore(&mut third, &reexport, "restore-third").unwrap();
    assert!(origin(&third, &identity).protected);
}

#[test]
fn restored_origin_does_not_claim_an_existing_category_without_recorded_ownership() {
    let (directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let identity = plan.additions()[0].clone();
    let automatic = directory.path().join("automatic.ame-backup");
    catalog
        .export_organization_archive(&automatic, &mut |_| true)
        .unwrap();

    let mut target = SqliteCatalog::open(directory.path().join("existing.sqlite3")).unwrap();
    let existing = plan
        .configuration()
        .setup
        .primary()
        .categories()
        .iter()
        .find(|category| category.id() == &identity)
        .unwrap()
        .renamed("My buildings".into())
        .unwrap();
    let rules = Taxonomy::new(vec![existing.clone()]).unwrap();
    let before = target.load_classification_state().unwrap();
    target
        .configure_classification(&id("local-definition"), before.revision, &rules)
        .unwrap();
    super::super::super::archives::restore(&mut target, &automatic, "restore-existing").unwrap();
    assert!(origin(&target, &identity).protected);
    assert_eq!(
        target.load_classification_state().unwrap().taxonomy,
        Some(rules)
    );
}

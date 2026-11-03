use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::application::classification_discovery::prepare_category_catalog_reconciliation;
use crate::domain::classification::maintenance::{
    CategoryCatalogReconciliationPlan, ManagedCategoryCatalog,
};
use crate::ports::classification_configuration::{
    CategoryCatalogRepository, CategoryExpansionRepository,
};

use super::*;

fn plan(catalog: &SqliteCatalog, allowed: Vec<CategoryId>) -> CategoryCatalogReconciliationPlan {
    let target = ManagedCategoryCatalog::new(
        ClassificationRevision::from_bytes([7; 32]),
        allowed.into_iter().collect(),
    )
    .unwrap();
    prepare_category_catalog_reconciliation(
        id("reconcile"),
        &target,
        catalog,
        &AtomicBool::new(false),
    )
    .unwrap()
    .unwrap()
}

fn evidence_count(catalog: &SqliteCatalog) -> i64 {
    catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM classification_maintenance_evidence",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn reconciliation_retires_only_unprotected_out_of_catalog_categories_and_preserves_history() {
    let (directory, mut catalog) = fixture();
    let identities = super::retirement::seed(&mut catalog, 3);
    let before = catalog.load_classification_configuration().unwrap();
    let proposal = plan(&catalog, vec![identities[0].clone()]);
    assert_eq!(
        proposal.retired(),
        &BTreeSet::from([identities[1].clone(), identities[2].clone()])
    );
    let saved = catalog
        .apply_category_catalog_reconciliation(&proposal, &AtomicBool::new(false))
        .unwrap();
    let rules = saved.catalog.taxonomy.as_ref().unwrap();
    assert_eq!(rules.categories().len(), 2);
    assert!(
        rules
            .categories()
            .iter()
            .any(|definition| definition.id() == &identities[0])
    );
    assert_eq!(
        catalog
            .load_classification_setup(&id("start"))
            .unwrap()
            .unwrap()
            .primary(),
        before.catalog.taxonomy.as_ref().unwrap()
    );
    let known = catalog
        .load_category_expansion_snapshot(&identities)
        .unwrap();
    assert_eq!(known.known_categories.len(), 3);
    assert!(
        known
            .managed_categories
            .values()
            .all(|origin| !origin.protected)
    );
    let payload: String = catalog.connection.query_row(
        "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'reconcile'", [], |row| row.get(0),
    ).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["kind"], "managed_catalog_reconciliation");
    assert_eq!(payload["retired"].as_array().unwrap().len(), 2);
    assert_eq!(evidence_count(&catalog), 1);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_configuration().unwrap(), saved);
    assert!(
        prepare_category_catalog_reconciliation(
            id("again"),
            &proposal.catalog,
            &catalog,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none()
    );
    catalog
        .change_classification_dispatch(
            saved.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let stopped = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        catalog
            .apply_category_catalog_reconciliation(&proposal, &AtomicBool::new(false))
            .unwrap(),
        stopped
    );
    assert_eq!(evidence_count(&catalog), 1);
}

#[test]
fn human_choices_and_modified_definitions_survive_a_catalog_update() {
    let (_directory, mut catalog) = fixture();
    let identities = super::retirement::seed(&mut catalog, 4);
    publish(&mut catalog, "a", 'a');
    let observed = super::super::prepare(&catalog, &["a"]);
    catalog
        .change_classification(
            &id("choose"),
            &observed,
            &ClassificationAction::Correct(identities[0].clone()),
        )
        .unwrap();
    let human = super::super::prepare(&catalog, &["a"]).groups[0]
        .human
        .clone();
    let current = catalog.load_classification_configuration().unwrap();
    let edited = Taxonomy::new(
        current
            .catalog
            .taxonomy
            .unwrap()
            .categories()
            .iter()
            .map(|definition| {
                if definition.id() == &identities[1] {
                    definition.renamed("My category".into()).unwrap()
                } else {
                    definition.clone()
                }
            })
            .collect(),
    )
    .unwrap();
    catalog
        .save_classification_configuration(&request(
            &catalog,
            "edit",
            edited,
            ClassificationConfigurationAction::Save,
        ))
        .unwrap();
    let proposal = plan(&catalog, vec![identities[2].clone()]);
    assert_eq!(proposal.retired(), &BTreeSet::from([identities[3].clone()]));
    catalog
        .apply_category_catalog_reconciliation(&proposal, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        super::super::prepare(&catalog, &["a"]).groups[0].human,
        human
    );
    let categories = catalog
        .load_classification_configuration()
        .unwrap()
        .catalog
        .taxonomy
        .unwrap();
    assert!(
        categories
            .categories()
            .iter()
            .any(|definition| definition.label() == "My category")
    );
}

#[test]
fn concurrent_pause_protection_or_human_choice_rejects_the_prepared_update() {
    for race in ["pause", "protection", "choice"] {
        let (_directory, mut catalog) = fixture();
        let identities = super::retirement::seed(&mut catalog, 2);
        let proposal = plan(&catalog, vec![identities[0].clone()]);
        match race {
            "pause" => {
                let state = catalog.load_classification_configuration().unwrap();
                catalog
                    .change_classification_dispatch(
                        state.dispatch.revision,
                        &ClassificationDispatchCommand::Pause,
                    )
                    .unwrap();
            }
            "protection" => {
                catalog
                    .connection
                    .execute(
                        "UPDATE classification_managed_categories SET protected = 1",
                        [],
                    )
                    .unwrap();
            }
            "choice" => {
                publish(&mut catalog, "a", 'a');
                let observed = super::super::prepare(&catalog, &["a"]);
                catalog
                    .change_classification(
                        &id("choose"),
                        &observed,
                        &ClassificationAction::Correct(identities[1].clone()),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let before = catalog.load_classification_configuration().unwrap();
        assert_eq!(
            catalog
                .apply_category_catalog_reconciliation(&proposal, &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "classification_changed"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(evidence_count(&catalog), 0);
    }
}

#[test]
fn cancellation_and_evidence_failure_roll_back_category_and_run_changes() {
    for cancel in [false, true] {
        let (_directory, mut catalog) = fixture();
        let identities = super::retirement::seed(&mut catalog, 2);
        let proposal = plan(&catalog, vec![identities[0].clone()]);
        let before = catalog.load_classification_configuration().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        if cancel {
            let flag = cancelled.clone();
            catalog
                .connection
                .update_hook(Some(
                    move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                        if table == "classification_runs" {
                            flag.store(true, Ordering::Release);
                        }
                    },
                ))
                .unwrap();
        } else {
            catalog.connection.execute_batch("CREATE TRIGGER reject_catalog_update BEFORE INSERT ON classification_maintenance_evidence BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        }
        let error = catalog
            .apply_category_catalog_reconciliation(&proposal, &cancelled)
            .unwrap_err();
        if cancel {
            assert_eq!(error.code, "classification_cancelled");
        } else {
            assert!(error.message.contains("fixture"));
        }
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(evidence_count(&catalog), 0);
        assert!(
            catalog
                .load_classification_setup(&id("reconcile"))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn cancellation_before_preparation_keeps_the_configuration_and_noop_does_not_start_a_run() {
    let (_directory, mut catalog) = fixture();
    let identities = super::retirement::seed(&mut catalog, 2);
    let target = ManagedCategoryCatalog::new(
        ClassificationRevision::from_bytes([7; 32]),
        identities.into_iter().collect(),
    )
    .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    assert!(
        prepare_category_catalog_reconciliation(
            id("same"),
            &target,
            &catalog,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(
        prepare_category_catalog_reconciliation(
            id("cancelled"),
            &target,
            &catalog,
            &AtomicBool::new(true)
        )
        .err()
        .unwrap()
        .code,
        "classification_cancelled"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert_eq!(evidence_count(&catalog), 0);
}

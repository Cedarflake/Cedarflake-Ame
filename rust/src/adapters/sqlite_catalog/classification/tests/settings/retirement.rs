use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rusqlite::params;

use crate::application::classification_discovery::prepare_equivalent_category_retirement;
use crate::domain::classification::discovery::ConceptId;
use crate::domain::classification::maintenance::{
    CategoryRetirementPlan, managed_category_identity,
};
use crate::ports::classification_configuration::{
    CategoryExpansionRepository, CategoryRetirementRepository,
};

use super::*;

pub(super) fn seed(catalog: &mut SqliteCatalog, count: u8) -> Vec<CategoryId> {
    let mut definitions = edited_rules("Drawing", "a drawing").categories().to_vec();
    let mut identities = Vec::new();
    for index in 0..count {
        let concept = ConceptId::from_bytes([index + 1; 32]);
        let identity = managed_category_identity(concept, None).unwrap();
        definitions.push(
            CategoryDefinition::new(
                identity.clone(),
                format!("Generated {index}"),
                vec!["a city illustration".into()],
            )
            .unwrap(),
        );
        identities.push(identity);
    }
    let setup = request(
        catalog,
        "start",
        Taxonomy::new(definitions).unwrap(),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&setup).unwrap();
    for (index, identity) in identities.iter().enumerate() {
        let definition = setup
            .setup
            .primary()
            .categories()
            .iter()
            .find(|category| category.id() == identity)
            .unwrap();
        catalog.connection.execute(
            "INSERT INTO classification_managed_categories(category_id, concept_id, purpose_category_id,
                 operation_id, admitted_label, admitted_semantic_revision, protected)
             VALUES (?1, ?2, NULL, 'start', ?3, ?4, 0)",
            params![identity.as_str(), codec::digest(&[index as u8 + 1; 32]), definition.label(), codec::revision(definition.semantic_revision())],
        ).unwrap();
    }
    identities
}

fn plan(catalog: &SqliteCatalog, operation: &str) -> CategoryRetirementPlan {
    prepare_equivalent_category_retirement(id(operation), catalog, &AtomicBool::new(false))
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
fn equivalent_retirement_keeps_history_and_replays_without_resuming_a_stopped_run() {
    let (directory, mut catalog) = fixture();
    let identities = seed(&mut catalog, 3);
    let before = catalog.load_classification_configuration().unwrap();
    let proposal = plan(&catalog, "retire");
    assert_eq!(proposal.retirements().len(), 2);
    let saved = catalog
        .apply_category_retirement(&proposal, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        saved.catalog.taxonomy.as_ref().unwrap().categories().len(),
        2
    );
    assert_eq!(saved.dispatch.run_id, Some(id("retire")));
    assert_eq!(
        catalog
            .load_classification_setup(&id("start"))
            .unwrap()
            .unwrap()
            .primary(),
        before.catalog.taxonomy.as_ref().unwrap()
    );
    assert!(
        catalog
            .load_classification_setup(&id("retire"))
            .unwrap()
            .unwrap()
            .refinement()
            .is_none()
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
    assert!(
        prepare_equivalent_category_retirement(id("again"), &catalog, &AtomicBool::new(false))
            .unwrap()
            .is_none()
    );
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'retire'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["kind"], "equivalent_category_retirement");
    assert_eq!(payload["retirements"].as_array().unwrap().len(), 2);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_configuration().unwrap(), saved);
    catalog
        .change_classification_dispatch(
            saved.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let stopped = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        catalog
            .apply_category_retirement(&proposal, &AtomicBool::new(false))
            .unwrap(),
        stopped
    );
    assert_eq!(evidence_count(&catalog), 1);
}

#[test]
fn a_human_choice_keeps_its_category_and_favorite_unchanged() {
    let (_directory, mut catalog) = fixture();
    let identities = seed(&mut catalog, 3);
    publish(&mut catalog, "a", 'a');
    let chosen = identities.iter().max().unwrap();
    let observed = super::super::prepare(&catalog, &["a"]);
    catalog
        .change_classification(
            &id("choose"),
            &observed,
            &ClassificationAction::Correct(chosen.clone()),
        )
        .unwrap();
    let human = super::super::prepare(&catalog, &["a"]).groups[0]
        .human
        .clone();
    let proposal = plan(&catalog, "retire");
    assert!(
        proposal
            .retirements()
            .iter()
            .all(|entry| &entry.retained_id == chosen)
    );
    let selection = catalog
        .prepare_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &selection, true)
        .unwrap();
    catalog
        .apply_category_retirement(&proposal, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        super::super::prepare(&catalog, &["a"]).groups[0].human,
        human
    );
    assert!(
        catalog
            .inspect_organization_selection(&[selected(&catalog, "a")])
            .unwrap()
            .decisions[0]
            .favorite
    );
}

#[test]
fn manual_legacy_and_changed_definitions_are_not_retired() {
    for protection in ["manual", "legacy", "label", "descriptions"] {
        let (_directory, mut catalog) = fixture();
        let identities = seed(&mut catalog, 2);
        let protected = identities.iter().max().unwrap();
        match protection {
            "manual" => {
                catalog.connection.execute("UPDATE classification_managed_categories SET protected = 1 WHERE category_id = ?1", [protected.as_str()]).unwrap();
            }
            "legacy" => {
                catalog
                    .connection
                    .execute(
                        "DELETE FROM classification_managed_categories WHERE category_id = ?1",
                        [protected.as_str()],
                    )
                    .unwrap();
            }
            "label" | "descriptions" => {
                let current = catalog.load_classification_configuration().unwrap();
                let definitions = current
                    .catalog
                    .taxonomy
                    .unwrap()
                    .categories()
                    .iter()
                    .map(|category| {
                        if category.id() != protected {
                            return category.clone();
                        }
                        if protection == "label" {
                            category.renamed("My city pictures".into()).unwrap()
                        } else {
                            CategoryDefinition::new(
                                category.id().clone(),
                                category.label().into(),
                                vec!["a night city photograph".into()],
                            )
                            .unwrap()
                        }
                    })
                    .collect();
                let edit = request(
                    &catalog,
                    "edit",
                    Taxonomy::new(definitions).unwrap(),
                    ClassificationConfigurationAction::Save,
                );
                catalog.save_classification_configuration(&edit).unwrap();
            }
            _ => unreachable!(),
        }
        let result =
            prepare_equivalent_category_retirement(id("retire"), &catalog, &AtomicBool::new(false))
                .unwrap();
        if protection == "descriptions" {
            assert!(result.is_none());
        } else {
            let proposal = result.unwrap();
            assert_eq!(proposal.retirements().len(), 1);
            assert_eq!(&proposal.retirements()[0].retained_id, protected);
            catalog
                .apply_category_retirement(&proposal, &AtomicBool::new(false))
                .unwrap();
        }
    }
}

#[test]
fn pause_protection_and_new_human_choice_reject_an_old_plan() {
    for race in ["pause", "protection", "choice"] {
        let (_directory, mut catalog) = fixture();
        let identities = seed(&mut catalog, 2);
        let proposal = plan(&catalog, "retire");
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
                        &ClassificationAction::Correct(identities[0].clone()),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let before = catalog.load_classification_configuration().unwrap();
        assert_eq!(
            catalog
                .apply_category_retirement(&proposal, &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "classification_changed"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(evidence_count(&catalog), 0);
    }
}

#[test]
fn cancellation_during_run_replacement_and_failed_evidence_insertion_roll_back() {
    for cancel in [false, true] {
        let (_directory, mut catalog) = fixture();
        seed(&mut catalog, 2);
        let proposal = plan(&catalog, "retire");
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
            catalog.connection.execute_batch(
                "CREATE TRIGGER reject_retirement BEFORE INSERT ON classification_maintenance_evidence BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
            ).unwrap();
        }
        let error = catalog
            .apply_category_retirement(&proposal, &cancelled)
            .unwrap_err();
        if cancel {
            assert_eq!(error.code, "classification_cancelled");
        } else {
            assert!(error.message.contains("fixture"));
        }
        assert_eq!(cancelled.load(Ordering::Acquire), cancel);
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(evidence_count(&catalog), 0);
        assert!(
            catalog
                .load_classification_setup(&id("retire"))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn retirement_is_bounded_and_leaves_a_stable_survivor_across_rounds() {
    let (_directory, mut catalog) = fixture();
    let identities = seed(&mut catalog, 66);
    let survivor = identities.iter().min().unwrap();
    let first = plan(&catalog, "round-one");
    assert_eq!(first.retirements().len(), 64);
    assert!(
        first
            .retirements()
            .iter()
            .all(|entry| &entry.retained_id == survivor)
    );
    catalog
        .apply_category_retirement(&first, &AtomicBool::new(false))
        .unwrap();
    let second = plan(&catalog, "round-two");
    assert_eq!(second.retirements().len(), 1);
    assert_eq!(&second.retirements()[0].retained_id, survivor);
    catalog
        .apply_category_retirement(&second, &AtomicBool::new(false))
        .unwrap();
    assert!(
        prepare_equivalent_category_retirement(
            id("round-three"),
            &catalog,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn purpose_categories_remain_active_and_different_purposes_do_not_merge() {
    let (_directory, mut catalog) = fixture();
    let identities = seed(&mut catalog, 2);
    let parent = identities.iter().max().unwrap();
    let concept = ConceptId::from_bytes([200; 32]);
    let child = CategoryDefinition::new(
        managed_category_identity(concept, Some(parent)).unwrap(),
        "Purpose refinement".into(),
        vec!["a city illustration".into()],
    )
    .unwrap();
    let mut definitions = catalog
        .load_classification_configuration()
        .unwrap()
        .catalog
        .taxonomy
        .unwrap()
        .categories()
        .to_vec();
    definitions.push(child.clone());
    let change = request(
        &catalog,
        "add-child",
        Taxonomy::new(definitions).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&change).unwrap();
    catalog.connection.execute(
        "INSERT INTO classification_managed_categories(category_id, concept_id, purpose_category_id,
             operation_id, admitted_label, admitted_semantic_revision, protected)
         VALUES (?1, ?2, ?3, 'add-child', ?4, ?5, 0)",
        params![child.id().as_str(), codec::digest(concept.bytes()), parent.as_str(), child.label(), codec::revision(child.semantic_revision())],
    ).unwrap();
    let proposal = plan(&catalog, "retire");
    assert_eq!(proposal.retirements().len(), 1);
    assert_eq!(&proposal.retirements()[0].retained_id, parent);
    let result = catalog
        .apply_category_retirement(&proposal, &AtomicBool::new(false))
        .unwrap();
    let retained: Vec<_> = result
        .catalog
        .taxonomy
        .unwrap()
        .categories()
        .iter()
        .map(|category| category.id().clone())
        .collect();
    assert!(retained.contains(parent));
    assert!(retained.contains(child.id()));
}

#[test]
fn preparation_rejects_cancelled_and_obsolete_active_run_authority() {
    let (_directory, mut catalog) = fixture();
    seed(&mut catalog, 2);
    let cancelled =
        prepare_equivalent_category_retirement(id("retire"), &catalog, &AtomicBool::new(true));
    assert_eq!(cancelled.err().unwrap().code, "classification_cancelled");
    let before = catalog.load_classification_state().unwrap();
    let mut definitions = before.taxonomy.unwrap().categories().to_vec();
    definitions.push(
        CategoryDefinition::new(
            category("added"),
            "Added".into(),
            vec!["an unrelated image".into()],
        )
        .unwrap(),
    );
    catalog
        .configure_classification(
            &id("direct-change"),
            before.revision,
            &Taxonomy::new(definitions).unwrap(),
        )
        .unwrap();
    assert!(
        catalog
            .load_category_retirement_snapshot()
            .unwrap()
            .model
            .is_none()
    );
    let stale =
        prepare_equivalent_category_retirement(id("retire"), &catalog, &AtomicBool::new(false));
    assert_eq!(stale.err().unwrap().code, "classification_changed");
}

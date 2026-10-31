use std::sync::atomic::AtomicBool;

use crate::application::classification_discovery::CategoryAdmissionDisposition;
use crate::domain::classification::discovery::VocabularyConcept;
use crate::ports::classification_configuration::CategoryExpansionRepository;

use super::*;

mod fixture;
mod origins;
mod preservation;
use fixture::{assess, evidence, plan, start};

#[test]
fn candidate_addition_replaces_the_run_atomically_and_survives_restart() {
    let (directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    let plan = plan(&catalog, &evidence);
    assert_eq!(plan.additions().len(), 1);
    let saved = catalog
        .apply_category_expansion(&plan, &AtomicBool::new(false))
        .unwrap();
    let rules = saved.catalog.taxonomy.as_ref().unwrap();
    assert_eq!(rules.categories().len(), 2);
    assert!(
        rules
            .categories()
            .iter()
            .any(|category| category.label() == "Architecture")
    );
    assert!(
        rules
            .categories()
            .contains(&edited_rules("Drawing", "baseline").categories()[0])
    );
    assert_eq!(saved.dispatch.run_id, Some(id("expand")));
    let setup = catalog
        .load_classification_setup(&id("expand"))
        .unwrap()
        .unwrap();
    assert!(setup.refinement().is_none());
    assert_eq!(setup.primary(), rules);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_configuration().unwrap(), saved);
    assert!(
        catalog
            .load_classification_setup(&id("start"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn current_sources_include_comparison_samples_and_require_published_exact_evidence() {
    for invalidation in [
        "DELETE FROM gallery_content_fingerprints",
        "UPDATE asset_locations SET file_size = file_size + 1 WHERE location_id = 'a'",
        "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = 'c'",
        "UPDATE library_change_root_state SET is_active = 0 WHERE root_id = 'root-c'",
    ] {
        let (_directory, mut catalog) = super::super::fixture();
        start(&mut catalog);
        let plan = plan(&catalog, &evidence(&catalog));
        let before = catalog.load_classification_configuration().unwrap();
        catalog.connection.execute_batch(invalidation).unwrap();
        assert_eq!(
            catalog
                .apply_category_expansion(&plan, &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "classification_discovery_sources_changed"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        let count: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM classification_operations WHERE operation_id = 'expand'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[test]
fn concurrent_pause_or_edit_rejects_uncommitted_expansion() {
    for pause in [true, false] {
        let (_directory, mut catalog) = super::super::fixture();
        start(&mut catalog);
        let evidence = evidence(&catalog);
        let plan = plan(&catalog, &evidence);
        let before = catalog.load_classification_configuration().unwrap();
        if pause {
            catalog
                .change_classification_dispatch(
                    before.dispatch.revision,
                    &ClassificationDispatchCommand::Pause,
                )
                .unwrap();
        } else {
            let edit = request(
                &catalog,
                "rename",
                edited_rules("My drawings", "baseline"),
                ClassificationConfigurationAction::Save,
            );
            catalog.save_classification_configuration(&edit).unwrap();
        }
        let changed = catalog.load_classification_configuration().unwrap();
        assert_eq!(
            catalog
                .apply_category_expansion(&plan, &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "classification_changed"
        );
        assert_eq!(
            catalog.load_classification_configuration().unwrap(),
            changed
        );
        if pause {
            assert_eq!(
                assess(&catalog, &evidence, "after-pause")
                    .err()
                    .unwrap()
                    .code,
                "classification_changed"
            );
        }
    }
}

#[test]
fn direct_rule_replacement_cannot_reuse_the_old_automatic_run_for_expansion() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    let before = catalog.load_classification_state().unwrap();
    catalog
        .configure_classification(
            &id("direct-rule-edit"),
            before.revision,
            &edited_rules("Drawing", "changed description"),
        )
        .unwrap();
    let current = catalog.load_classification_configuration().unwrap();
    assert!(current.catalog.active_run_id.is_none());
    assert!(current.dispatch.run_id.is_some());
    assert_eq!(
        assess(&catalog, &evidence, "after-direct-edit")
            .err()
            .unwrap()
            .code,
        "classification_changed"
    );
    assert_eq!(
        catalog.load_classification_configuration().unwrap(),
        current
    );
}

#[test]
fn committed_retry_preserves_later_stop_and_removed_sources() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let mut plan = plan(&catalog, &evidence(&catalog));
    let saved = catalog
        .apply_category_expansion(&plan, &AtomicBool::new(false))
        .unwrap();
    let stopped = catalog
        .change_classification_dispatch(
            saved.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    catalog
        .connection
        .execute_batch("DELETE FROM gallery_content_fingerprints")
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(false))
            .unwrap()
            .dispatch,
        stopped
    );
    plan.evidence = ClassificationRevision::from_bytes([99; 32]);
    assert_eq!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_operation_conflict"
    );
}

#[test]
fn failed_activation_rolls_back_the_category_and_receipt_before_retry() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    let before = catalog.load_classification_configuration().unwrap();
    catalog.connection.execute_batch("CREATE TEMP TRIGGER reject_expansion BEFORE INSERT ON classification_analysis_setups BEGIN SELECT RAISE(ABORT, 'injected setup failure'); END;").unwrap();
    assert!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert!(
        catalog
            .load_category_expansion_snapshot(plan.additions())
            .unwrap()
            .known_categories
            .is_empty()
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_expansion")
        .unwrap();
    assert!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(false))
            .is_ok()
    );
}

#[test]
fn cancellation_does_not_save_categories() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    let before = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(true))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

#[test]
fn known_and_retired_categories_keep_their_identity_and_manual_edits() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let mut evidence = evidence(&catalog);
    let plan = plan(&catalog, &evidence);
    let saved = catalog
        .apply_category_expansion(&plan, &AtomicBool::new(false))
        .unwrap();
    let identity = plan.additions()[0].clone();
    let mut rules = saved.catalog.taxonomy.unwrap().categories().to_vec();
    let entry = rules
        .iter_mut()
        .find(|category| category.id() == &identity)
        .unwrap();
    *entry = CategoryDefinition::new(
        identity.clone(),
        "My collection".into(),
        vec!["my meaning".into()],
    )
    .unwrap();
    let rename = request(
        &catalog,
        "manual-edit",
        Taxonomy::new(rules.clone()).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&rename).unwrap();
    let concept = &mut evidence.discovery.candidates[0].candidate.concept;
    *concept = VocabularyConcept::new(
        concept.id(),
        "Buildings".into(),
        concept.description().into(),
    )
    .unwrap();
    evidence.discovery.vocabulary_revision = ClassificationRevision::from_bytes([80; 32]);
    let outcome = assess(&catalog, &evidence, "renamed-candidate").unwrap();
    assert!(outcome.plan.is_none());
    assert_eq!(outcome.already_known, vec![identity.clone()]);
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .taxonomy,
        Some(Taxonomy::new(rules).unwrap())
    );

    let retire = request(
        &catalog,
        "retire",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&retire).unwrap();
    let outcome = assess(&catalog, &evidence, "retired-candidate").unwrap();
    assert!(outcome.plan.is_none());
    assert_eq!(outcome.already_known, vec![identity]);
}

#[test]
fn insufficient_recurrence_cannot_be_published() {
    let (_directory, mut catalog) = super::super::fixture();
    start(&mut catalog);
    let mut evidence = evidence(&catalog);
    let criteria = evidence.policy.criteria();
    evidence.policy = crate::application::classification_discovery::CategoryAdmissionPolicy::new(
        ClassificationRevision::from_bytes([88; 32]),
        crate::application::classification_discovery::CategoryAdmissionCriteria {
            minimum_members: 3,
            ..criteria
        },
    )
    .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let outcome = assess(&catalog, &evidence, "weak").unwrap();
    assert!(outcome.plan.is_none());
    assert_eq!(
        outcome.admission.candidates[0].disposition,
        CategoryAdmissionDisposition::InsufficientMembers
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

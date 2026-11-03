use crate::application::classification_discovery::prepare_equivalent_category_retirement;
use crate::domain::classification::maintenance::{CategoryActivation, ManagedCategoryOrigin};
use crate::ports::classification_configuration::CategoryRetirementRepository;

use super::*;

fn retired(catalog: &mut SqliteCatalog) -> (fixture::Evidence, CategoryId) {
    start(catalog);
    let evidence = evidence(catalog);
    let addition = plan(catalog, &evidence);
    let identity = addition.additions()[0].clone();
    let saved = catalog
        .apply_category_expansion(&addition, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let mut definitions = saved.catalog.taxonomy.unwrap().categories().to_vec();
    let baseline = definitions
        .iter_mut()
        .find(|entry| entry.id() == &category("drawing"))
        .unwrap();
    *baseline = edited_rules("Drawing", "architectural drawing").categories()[0].clone();
    let alias = request(
        catalog,
        "alias",
        Taxonomy::new(definitions).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&alias).unwrap();
    let retirement = prepare_equivalent_category_retirement(
        id("retire-equivalent"),
        catalog,
        &AtomicBool::new(false),
    )
    .unwrap()
    .unwrap();
    assert_eq!(retirement.retirements().len(), 1);
    assert_eq!(retirement.retirements()[0].category_id, identity);
    catalog
        .apply_category_retirement(&retirement, &AtomicBool::new(false))
        .unwrap();
    let changed = request(
        catalog,
        "change-baseline",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&changed).unwrap();
    assert!(!origin(catalog, &identity).protected);
    (evidence, identity)
}

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
fn automatic_reactivation_reuses_identity_origin_history_and_committed_receipt() {
    let (directory, mut catalog) = super::super::super::fixture();
    let (evidence, identity) = retired(&mut catalog);
    let previous_origin = origin(&catalog, &identity);
    let category_count = count(&catalog, "classification_categories");
    let version_count = count(&catalog, "classification_category_versions");
    let restore = assess(&catalog, &evidence, "restore-generated")
        .unwrap()
        .plan
        .unwrap();
    assert_eq!(restore.additions(), std::slice::from_ref(&identity));
    assert!(
        matches!(&restore.provenance.candidates[0].activation, CategoryActivation::Reactivate(previous) if previous == &previous_origin)
    );
    let saved = catalog
        .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(saved.dispatch.run_id, Some(id("restore-generated")));
    assert!(
        saved
            .catalog
            .taxonomy
            .as_ref()
            .unwrap()
            .categories()
            .iter()
            .any(|entry| entry.id() == &identity)
    );
    assert_eq!(origin(&catalog, &identity), previous_origin);
    assert_eq!(count(&catalog, "classification_managed_categories"), 1);
    assert_eq!(count(&catalog, "classification_categories"), category_count);
    assert_eq!(
        count(&catalog, "classification_category_versions"),
        version_count
    );
    assert_eq!(count(&catalog, "classification_maintenance_evidence"), 3);
    let payload: String = catalog.connection.query_row(
        "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'restore-generated'", [], |row| row.get(0),
    ).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["candidates"][0]["activation"]["kind"], "reactivate");
    assert_eq!(
        payload["candidates"][0]["activation"]["original_operation"],
        "expand"
    );
    assert_eq!(payload["candidates"][0]["activation"]["protected"], false);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
            .unwrap(),
        saved
    );
    assert_eq!(origin(&catalog, &identity), previous_origin);
    let stopped = catalog
        .change_classification_dispatch(
            saved.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let replayed = catalog
        .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(replayed.dispatch, stopped);
    assert_eq!(replayed.dispatch.mode, ClassificationDispatchMode::Stopped);
    assert_eq!(count(&catalog, "classification_maintenance_evidence"), 3);
}

#[test]
fn changed_or_protected_retained_definitions_do_not_become_new_categories() {
    for change in [
        "UPDATE classification_managed_categories SET protected = 1",
        "UPDATE classification_categories SET label = 'Manual label' WHERE label = 'Architecture'",
        "DELETE FROM classification_managed_categories",
    ] {
        let (_directory, mut catalog) = super::super::super::fixture();
        let (evidence, identity) = retired(&mut catalog);
        catalog.connection.execute_batch(change).unwrap();
        let result = assess(&catalog, &evidence, "restore-generated").unwrap();
        assert!(result.plan.is_none());
        assert_eq!(result.already_known, vec![identity]);
    }
    for changed_description in [false, true] {
        let (_directory, mut catalog) = super::super::super::fixture();
        let (mut evidence, identity) = retired(&mut catalog);
        let concept = &evidence.discovery.candidates[0].candidate.concept;
        evidence.discovery.candidates[0].candidate.concept = VocabularyConcept::new(
            concept.id(),
            if changed_description {
                "Architecture"
            } else {
                "Different name"
            }
            .into(),
            if changed_description {
                "different meaning"
            } else {
                "architectural drawing"
            }
            .into(),
        )
        .unwrap();
        let result = assess(&catalog, &evidence, "restore-generated").unwrap();
        assert!(result.plan.is_none());
        assert_eq!(result.already_known, vec![identity]);
    }
}

#[test]
fn pending_reactivation_revalidates_origin_and_retained_label_without_revision_changes() {
    for change in [
        "UPDATE classification_managed_categories SET protected = 1",
        "UPDATE classification_managed_categories SET operation_id = 'start'",
        "UPDATE classification_categories SET label = 'Changed' WHERE label = 'Architecture'",
        "DELETE FROM classification_managed_categories",
    ] {
        let (_directory, mut catalog) = super::super::super::fixture();
        let (evidence, _) = retired(&mut catalog);
        let restore = assess(&catalog, &evidence, "restore-generated")
            .unwrap()
            .plan
            .unwrap();
        let before = catalog.load_classification_configuration().unwrap();
        catalog.connection.execute_batch(change).unwrap();
        if change.contains("operation_id") {
            let fresh = assess(&catalog, &evidence, "restore-generated")
                .unwrap()
                .plan
                .unwrap();
            assert_ne!(fresh.evidence, restore.evidence);
        }
        assert_eq!(
            catalog
                .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "classification_changed"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(count(&catalog, "classification_maintenance_evidence"), 2);
        assert!(
            catalog
                .load_classification_setup(&id("restore-generated"))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn reactivation_rollback_and_cancellation_preserve_the_retired_identity() {
    let (_directory, mut catalog) = super::super::super::fixture();
    let (evidence, identity) = retired(&mut catalog);
    let restore = assess(&catalog, &evidence, "restore-generated")
        .unwrap()
        .plan
        .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let old_origin = origin(&catalog, &identity);
    catalog.connection.execute_batch("CREATE TEMP TRIGGER reject_restore BEFORE INSERT ON classification_maintenance_evidence BEGIN SELECT RAISE(ABORT, 'injected evidence failure'); END;").unwrap();
    assert!(
        catalog
            .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
            .is_err()
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert_eq!(origin(&catalog, &identity), old_origin);
    assert!(
        catalog
            .load_classification_setup(&id("restore-generated"))
            .unwrap()
            .is_none()
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_restore")
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    catalog
        .connection
        .execute_batch("DELETE FROM gallery_content_fingerprints")
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
            .unwrap_err()
            .code,
        "classification_discovery_sources_changed"
    );
    assert_eq!(origin(&catalog, &identity), old_origin);
    assert_eq!(count(&catalog, "classification_maintenance_evidence"), 2);
}

#[test]
fn reactivation_consumes_a_generated_slot_and_still_uses_fresh_admission() {
    let (_directory, mut catalog) = super::super::super::fixture();
    let (evidence, identity) = retired(&mut catalog);
    let mut other = fixture::evidence(&catalog);
    other.discovery.candidates[0].candidate.concept = VocabularyConcept::new(
        crate::domain::classification::discovery::ConceptId::from_bytes([12; 32]),
        "Gardens".into(),
        "garden drawing".into(),
    )
    .unwrap();
    let another = assess(&catalog, &other, "another").unwrap().plan.unwrap();
    catalog
        .apply_category_expansion(&another, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let limited = fixture::assess_with_budget(&catalog, &evidence, "restore-generated", 1).unwrap();
    assert!(limited.plan.is_none());
    assert_eq!(limited.capacity_deferred, vec![identity.clone()]);
    let mut unsupported = fixture::evidence(&catalog);
    let scores = &mut unsupported.discovery.candidates[0].candidate.scores;
    scores.context_similarity = scores.similarity;
    scores.contrast = 0.0;
    let rejected = assess(&catalog, &unsupported, "unsupported").unwrap();
    assert!(rejected.plan.is_none());
    assert_eq!(
        rejected.admission.candidates[0].disposition,
        CategoryAdmissionDisposition::WeakContext
    );
    let restore = fixture::assess_with_budget(&catalog, &evidence, "restore-generated", 2)
        .unwrap()
        .plan
        .unwrap();
    catalog
        .apply_category_expansion(&restore, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(origin(&catalog, &identity).operation_id, id("expand"));
    assert_eq!(count(&catalog, "classification_managed_categories"), 2);
}

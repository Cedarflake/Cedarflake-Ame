use crate::application::classification_discovery::CategorySchemeDisposition;
use crate::domain::classification::discovery::ConceptId;

use super::fixture::{Evidence, assess_with_budget};
use super::*;

#[test]
fn joint_selection_publishes_only_the_supported_winner_when_one_slot_remains() {
    use crate::application::classification_discovery::{
        CategoryAdmissionCriteria, CategoryAdmissionPolicy, ConceptMatch, ConsolidatedConcept,
    };
    use crate::domain::classification::discovery::{DiscoveryCohort, DiscoverySample};
    use crate::domain::classification::{Embedding, ImageFeature, TextFeature};
    use crate::ports::exact_duplicates::ExactDuplicateRepository;

    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-extra",
        "root-extra",
        "E:\\Fixture",
        &[("d", None, 4), ("e", None, 5)],
    );
    catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state WHERE root_id = 'root-extra';
         UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'
         WHERE root_id = 'root-extra';",
    ).unwrap();
    for (name, digest) in [("d", 'd'), ("e", 'e')] {
        publish(&mut catalog, name, digest);
    }
    let generations = ["d", "e"].map(|name| selected(&catalog, name).source_generation);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let mut evidence = evidence(&catalog);
    let model = evidence.cohort.model();
    let samples = ["a", "b", "c", "d", "e"]
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let prepared = super::super::super::prepare(&catalog, &[name]);
            let crate::domain::classification::ClassificationSubject::Exact(content) =
                &prepared.groups[0].target.subject
            else {
                panic!("Expected exact content")
            };
            let values = match index {
                0 | 1 => vec![1.0, 0.0],
                2 | 3 => vec![0.6, 0.8],
                _ => vec![-1.0, 0.0],
            };
            DiscoverySample {
                content: content.clone(),
                feature: ImageFeature {
                    identity: model.image_identity(),
                    embedding: Embedding::normalized(values).unwrap(),
                },
            }
        })
        .collect::<Vec<_>>();
    evidence.cohort = DiscoveryCohort::new(model, &samples[..4], &samples[4..]).unwrap();
    evidence.discovery.cohort_revision = evidence.cohort.revision();
    evidence.discovery.candidates = [
        ([8; 32], "Architecture", vec![1.0, 0.0]),
        ([9; 32], "Landscapes", vec![0.6, 0.8]),
    ]
    .into_iter()
    .map(|(id, label, values)| {
        let concept = VocabularyConcept::new(
            ConceptId::from_bytes(id),
            label.into(),
            format!("drawing of {label}"),
        )
        .unwrap();
        let feature = TextFeature {
            identity: model.text_identity(),
            description: ClassificationRevision::description(concept.description()),
            embedding: Embedding::normalized(values).unwrap(),
        };
        ConsolidatedConcept {
            candidate: ConceptMatch {
                scores: evidence.cohort.score(&feature).unwrap(),
                concept,
            },
            member_similarities: evidence.cohort.member_similarities(&feature).unwrap(),
            narrower_concepts: vec![],
            comparison: None,
        }
    })
    .collect();
    evidence.policy = CategoryAdmissionPolicy::new(
        ClassificationRevision::from_bytes([9; 32]),
        CategoryAdmissionCriteria {
            coverage_per_thousand: 500,
            ..evidence.policy.criteria()
        },
    )
    .unwrap();
    let selected = assess_with_budget(&catalog, &evidence, "joint", 1).unwrap();
    assert_eq!(selected.capacity_deferred.len(), 1);
    let plan = selected.plan.unwrap();
    let chosen = plan
        .configuration()
        .setup
        .primary()
        .categories()
        .iter()
        .find(|category| plan.additions().contains(category.id()))
        .unwrap();
    assert_eq!(chosen.label(), "Architecture");
    assert_eq!(
        plan.provenance.candidates[0].supporting_contents,
        samples[..2]
            .iter()
            .map(|sample| sample.content.clone())
            .collect::<Vec<_>>()
    );
    evidence.discovery.candidates.reverse();
    let reversed = assess_with_budget(&catalog, &evidence, "joint", 1)
        .unwrap()
        .plan
        .unwrap();
    assert_eq!(reversed.additions(), plan.additions());
    assert_eq!(
        reversed.configuration().setup.primary(),
        plan.configuration().setup.primary()
    );
    assert_eq!(
        reversed.configuration().setup.run().configuration(),
        plan.configuration().setup.run().configuration()
    );
    assert_eq!(reversed.evidence, plan.evidence);
    catalog
        .apply_category_expansion(&plan, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .taxonomy
            .unwrap()
            .categories()
            .len(),
        2
    );
}

fn next_evidence(catalog: &SqliteCatalog, sequence: u8) -> Evidence {
    let mut next = evidence(catalog);
    next.discovery.candidates[0].candidate.concept = VocabularyConcept::new(
        ConceptId::from_bytes([sequence; 32]),
        format!("Category {sequence}"),
        format!("description {sequence}"),
    )
    .unwrap();
    next
}

#[test]
fn cumulative_generated_budget_survives_restarts_and_protected_edits() {
    let (directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let first = assess_with_budget(&catalog, &evidence(&catalog), "first", 1)
        .unwrap()
        .plan
        .unwrap();
    let first_id = first.additions()[0].clone();
    catalog
        .apply_category_expansion(&first, &AtomicBool::new(false))
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE classification_managed_categories SET protected = 1 WHERE category_id = ?1",
            [first_id.as_str()],
        )
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let next = next_evidence(&catalog, 9);
    let before = catalog.load_classification_configuration().unwrap();
    let full = assess_with_budget(&catalog, &next, "full", 1).unwrap();
    assert!(full.plan.is_none());
    assert_eq!(full.capacity_deferred.len(), 1);
    assert_eq!(
        full.scheme_candidates[0].disposition,
        CategorySchemeDisposition::AdditionBudget
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    let allowed = assess_with_budget(&catalog, &next, "second", 2)
        .unwrap()
        .plan
        .unwrap();
    catalog
        .apply_category_expansion(&allowed, &AtomicBool::new(false))
        .unwrap();
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'second'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["selection"]["maximum_generated"], 2);
    assert_eq!(
        payload["selection"]["active_generated"],
        serde_json::json!([first_id.as_str()])
    );
    assert!(
        assess_with_budget(&catalog, &next_evidence(&catalog, 10), "third", 2)
            .unwrap()
            .plan
            .is_none()
    );
}

#[test]
fn retired_categories_release_capacity_without_becoming_new_candidates() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let original = evidence(&catalog);
    let first = assess_with_budget(&catalog, &original, "first", 1)
        .unwrap()
        .plan
        .unwrap();
    let first_id = first.additions()[0].clone();
    catalog
        .apply_category_expansion(&first, &AtomicBool::new(false))
        .unwrap();
    let remove = request(
        &catalog,
        "retire",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&remove).unwrap();
    let old = assess_with_budget(&catalog, &original, "old", 1).unwrap();
    assert!(old.plan.is_none());
    assert_eq!(old.already_known, vec![first_id]);
    let replacement = assess_with_budget(&catalog, &next_evidence(&catalog, 9), "replacement", 1)
        .unwrap()
        .plan
        .unwrap();
    catalog
        .apply_category_expansion(&replacement, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .taxonomy
            .unwrap()
            .categories()
            .len(),
        2
    );
}

#[test]
fn changed_origin_membership_rejects_a_prepared_budget_without_catalog_revision_change() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let first = plan(&catalog, &evidence(&catalog));
    catalog
        .apply_category_expansion(&first, &AtomicBool::new(false))
        .unwrap();
    let pending = assess_with_budget(&catalog, &next_evidence(&catalog, 9), "pending", 2)
        .unwrap()
        .plan
        .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    catalog
        .connection
        .execute(
            "DELETE FROM classification_managed_categories WHERE category_id = ?1",
            [first.additions()[0].as_str()],
        )
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&pending, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_changed"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    let saved: bool = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM classification_operations WHERE operation_id = 'pending')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!saved);
}

#[test]
fn receipt_binds_the_budget_even_when_the_selected_addition_is_identical() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    let first = assess_with_budget(&catalog, &evidence, "same-operation", 1)
        .unwrap()
        .plan
        .unwrap();
    let changed = assess_with_budget(&catalog, &evidence, "same-operation", 2)
        .unwrap()
        .plan
        .unwrap();
    assert_eq!(first.additions(), changed.additions());
    catalog
        .apply_category_expansion(&first, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&changed, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_operation_conflict"
    );
}

#[test]
fn equivalent_current_definitions_are_not_added_under_another_name() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let mut evidence = evidence(&catalog);
    evidence.discovery.candidates[0].candidate.concept = VocabularyConcept::new(
        ConceptId::from_bytes([8; 32]),
        "Different label".into(),
        "baseline".into(),
    )
    .unwrap();
    let result = assess_with_budget(&catalog, &evidence, "duplicate-description", 8).unwrap();
    assert!(result.plan.is_none());
    assert_eq!(
        result.scheme_candidates[0].disposition,
        CategorySchemeDisposition::ExistingDefinition(category("drawing"))
    );
}

#[test]
fn hard_taxonomy_capacity_and_invalid_budgets_do_not_change_existing_categories() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    for budget in [0, 257] {
        assert_eq!(
            assess_with_budget(&catalog, &evidence, "invalid", budget)
                .err()
                .unwrap()
                .code,
            "classification_category_limit"
        );
    }
    let rules = Taxonomy::new(
        (0..256)
            .map(|index| {
                CategoryDefinition::new(
                    category(&format!("manual-{index}")),
                    format!("Manual {index}"),
                    vec!["baseline".into()],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let save = request(
        &catalog,
        "full-taxonomy",
        rules,
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&save).unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let full = assess_with_budget(&catalog, &evidence, "full", 8).unwrap();
    assert!(full.plan.is_none());
    assert_eq!(full.capacity_deferred.len(), 1);
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

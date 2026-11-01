use std::sync::atomic::AtomicBool;

use crate::application::classification_discovery::*;
use crate::application::classification_matching::CategoryMatcher;
use crate::domain::classification::discovery::*;
use crate::domain::classification::maintenance::CategoryExpansionPlan;
use crate::domain::classification::{Embedding, ImageFeature, TextFeature};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

pub(super) struct Evidence {
    pub cohort: DiscoveryCohort,
    pub discovery: ConceptConsolidation,
    pub policy: CategoryAdmissionPolicy,
}

pub(super) fn start(catalog: &mut SqliteCatalog) {
    for (name, digest) in [("a", 'a'), ("b", 'b'), ("c", 'c')] {
        publish(catalog, name, digest);
    }
    let generations = ["a", "b", "c"].map(|name| selected(catalog, name).source_generation);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let start = request(
        catalog,
        "start",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&start).unwrap();
}

pub(super) fn evidence(catalog: &SqliteCatalog) -> Evidence {
    let setup = catalog
        .load_classification_setup(&id("start"))
        .unwrap()
        .unwrap();
    let model = setup.run().model();
    let samples: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|name| {
            let prepared = super::super::super::prepare(catalog, &[name]);
            let crate::domain::classification::ClassificationSubject::Exact(content) =
                &prepared.groups[0].target.subject
            else {
                panic!("Expected exact fixture content")
            };
            DiscoverySample {
                content: content.clone(),
                feature: ImageFeature {
                    identity: model.image_identity(),
                    embedding: Embedding::normalized(if *name == "c" {
                        vec![-1.0, 0.0]
                    } else {
                        vec![1.0, 0.0]
                    })
                    .unwrap(),
                },
            }
        })
        .collect();
    let cohort = DiscoveryCohort::new(model, &samples[..2], &samples[2..]).unwrap();
    let concept = VocabularyConcept::new(
        ConceptId::from_bytes([8; 32]),
        "Architecture".into(),
        "architectural drawing".into(),
    )
    .unwrap();
    let feature = TextFeature {
        identity: model.text_identity(),
        description: ClassificationRevision::description(concept.description()),
        embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
    };
    let discovery = ConceptConsolidation {
        naming: CategoryNaming::Unconditioned,
        model,
        cohort_revision: cohort.revision(),
        vocabulary_revision: ClassificationRevision::from_bytes([4; 32]),
        retrieval_revision: ClassificationRevision::from_bytes([5; 32]),
        retrieval_limits: ConceptSearchLimits {
            entry_points: 1,
            expanded_concepts: 8,
            seeds: 1,
            hops: 2,
            edges: 16,
            results: 8,
        },
        frontier_revision: ClassificationRevision::from_bytes([6; 32]),
        consolidation_revision: ClassificationRevision::from_bytes([7; 32]),
        limits: ConceptConsolidationLimits {
            hops: 2,
            concepts: 8,
            edges: 16,
        },
        candidates: vec![ConsolidatedConcept {
            candidate: ConceptMatch {
                concept,
                scores: cohort.score(&feature).unwrap(),
            },
            member_similarities: cohort.member_similarities(&feature).unwrap(),
            narrower_concepts: vec![],
            comparison: None,
        }],
        comparison_policy: ContextComparisonPolicy::SelectedContexts,
        unresolved_concepts: vec![],
        visited_edges: 1,
    };
    Evidence {
        cohort,
        discovery,
        policy: CategoryAdmissionPolicy::new(
            ClassificationRevision::from_bytes([9; 32]),
            CategoryAdmissionCriteria {
                minimum_members: 2,
                coverage_per_thousand: 1000,
                minimum_improvement: 0.1,
                minimum_separation: 0.1,
                minimum_context_contrast: 0.1,
                max_proposals: 2,
            },
        )
        .unwrap(),
    }
}

pub(super) fn assess(
    catalog: &SqliteCatalog,
    evidence: &Evidence,
    operation: &str,
) -> Result<CategoryExpansionAssessment, ScanError> {
    assess_with_budget(catalog, evidence, operation, 8)
}

pub(super) fn assess_with_budget(
    catalog: &SqliteCatalog,
    evidence: &Evidence,
    operation: &str,
    maximum_generated_categories: usize,
) -> Result<CategoryExpansionAssessment, ScanError> {
    let current = catalog.load_classification_configuration().unwrap();
    let taxonomy = current.catalog.taxonomy.unwrap();
    let model = evidence.cohort.model();
    let features = taxonomy
        .description_revisions()
        .into_iter()
        .map(|description| TextFeature {
            identity: model.text_identity(),
            description,
            embedding: Embedding::normalized(vec![0.0, 1.0]).unwrap(),
        })
        .collect();
    let matcher = CategoryMatcher::new(taxonomy, model, features).unwrap();
    prepare_category_expansion(
        CategoryExpansionRequest {
            operation_id: id(operation),
            cohort: &evidence.cohort,
            discovery: &evidence.discovery,
            matcher: &matcher,
            policy: &evidence.policy,
            maximum_generated_categories,
        },
        catalog,
        &AtomicBool::new(false),
    )
}

pub(super) fn plan(catalog: &SqliteCatalog, evidence: &Evidence) -> CategoryExpansionPlan {
    assess(catalog, evidence, "expand").unwrap().plan.unwrap()
}

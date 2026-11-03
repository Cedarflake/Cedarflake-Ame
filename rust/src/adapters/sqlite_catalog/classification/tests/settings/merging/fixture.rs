use rusqlite::params;
use serde_json::json;

use crate::domain::classification::ClassificationSubject;
use crate::domain::classification::discovery::ConceptId;
use crate::domain::classification::maintenance::{
    CategoryCoverageSummary, managed_category_identity,
};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

pub(super) fn start(catalog: &mut SqliteCatalog) -> (ClassificationRun, CategoryId) {
    for (name, digest) in [("a", 'a'), ("b", 'b'), ("c", 'c')] {
        publish(catalog, name, digest);
    }
    let generations = ["a", "b", "c"].map(|name| selected(catalog, name).source_generation);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let parent = taxonomy()
        .categories()
        .iter()
        .find(|definition| definition.id().as_str() == "drawing")
        .unwrap()
        .clone();
    let concept = ConceptId::from_bytes([9; 32]);
    let child = CategoryDefinition::new(
        managed_category_identity(concept, Some(parent.id())).unwrap(),
        "City drawing".into(),
        vec!["a drawing of a city".into()],
    )
    .unwrap();
    let mut categories = taxonomy().categories().to_vec();
    categories.push(child.clone());
    let enable = request(
        catalog,
        "enable",
        Taxonomy::new(categories).unwrap(),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&enable).unwrap();
    let payload = json!({
        "version": 1,
        "naming": {"kind": "purpose", "category": parent.id().as_str(), "category_revision": codec::revision(parent.semantic_revision()),
            "label_pattern": "{subject} drawing", "comparison_pattern": "a drawing of {subject}"},
        "criteria": {"minimum_members": 3},
        "candidates": [{"category": child.id().as_str(), "concept": codec::digest(concept.bytes())}],
    }).to_string();
    catalog
        .connection
        .execute(
            "INSERT INTO classification_maintenance_evidence VALUES ('enable', 'enable', ?1, ?2)",
            params![
                blake3::hash(payload.as_bytes()).to_hex().to_string(),
                payload
            ],
        )
        .unwrap();
    catalog.connection.execute(
        "INSERT INTO classification_managed_categories VALUES (?1, ?2, 'drawing', 'enable', ?3, ?4, 0)",
        params![child.id().as_str(), codec::digest(concept.bytes()), child.label(), codec::revision(child.semantic_revision())],
    ).unwrap();
    for name in ["a", "b", "c"] {
        predict_child(
            catalog,
            enable.setup.run(),
            child.id(),
            name,
            &format!("initial-{name}"),
            true,
        );
    }
    (enable.setup.run().clone(), child.id().clone())
}

pub(super) fn predict_child(
    catalog: &mut SqliteCatalog,
    run: &ClassificationRun,
    child: &CategoryId,
    image: &str,
    prediction_id: &str,
    is_child: bool,
) {
    let prepared = super::super::super::prepare(catalog, &[image]);
    let ClassificationSubject::Exact(content) = &prepared.groups[0].target.subject else {
        panic!("expected exact content")
    };
    let prediction = PredictionEvidence::new(
        id(prediction_id),
        run.clone(),
        content.clone(),
        RankedPrediction::new(
            run.primary_rules(),
            run.model(),
            vec![
                CategoryScore {
                    category_id: child.clone(),
                    similarity: if is_child { 0.8 } else { 0.5 },
                },
                CategoryScore {
                    category_id: category("drawing"),
                    similarity: if is_child { 0.6 } else { 0.8 },
                },
                CategoryScore {
                    category_id: category("photo"),
                    similarity: 0.1,
                },
            ],
        )
        .unwrap(),
        None,
        ReviewRecommendation::Uncalibrated,
    )
    .unwrap();
    assert!(
        catalog
            .publish_classification_prediction(&selected(catalog, image), &prediction)
            .unwrap()
    );
}

pub(super) fn survey(catalog: &SqliteCatalog) -> CategoryCoverageSummary {
    survey_category_coverage(catalog, 2, 64, &Arc::new(AtomicBool::new(false))).unwrap()
}

pub(super) fn proposal(catalog: &SqliteCatalog) -> CategoryMergePlan {
    prepare_purpose_category_merge(
        id("merge"),
        survey(catalog),
        catalog,
        &Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
    .unwrap()
}

pub(super) fn sparse(catalog: &mut SqliteCatalog) -> CategoryId {
    let (run, child) = start(catalog);
    predict_child(catalog, &run, &child, "c", "new-purpose", false);
    child
}

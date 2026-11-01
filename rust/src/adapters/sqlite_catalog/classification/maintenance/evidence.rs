use serde_json::{Value, json};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationContent;
use crate::domain::classification::discovery::CategoryNaming;
use crate::domain::classification::maintenance::CategoryExpansionPlan;

use super::super::{codec, invalid_record};

pub(in crate::adapters::sqlite_catalog::classification) fn encode(
    plan: &CategoryExpansionPlan,
) -> Result<String, ScanError> {
    let evidence = &plan.provenance;
    let criteria = &evidence.criteria;
    let bounds = &evidence.bounds;
    let naming = match &evidence.naming {
        CategoryNaming::Unconditioned => json!({"kind": "unconditioned"}),
        CategoryNaming::WithinCategory(purpose) => json!({
            "kind": "purpose", "category": purpose.category().as_str(),
            "category_revision": codec::revision(purpose.category_revision()),
            "label_pattern": purpose.label_pattern(), "comparison_pattern": purpose.comparison_pattern(),
        }),
    };
    let candidates = evidence.candidates.iter().map(|candidate| {
        let category = plan.configuration.setup.primary().categories().iter()
            .find(|category| category.id() == &candidate.category_id).ok_or_else(invalid_record)?;
        Ok(json!({
            "category": candidate.category_id.as_str(), "concept": codec::digest(candidate.concept_id.bytes()),
            "label": category.label(), "descriptions": category.descriptions(),
            "supporting_contents": candidate.supporting_contents.iter().map(content).collect::<Vec<_>>(),
            "mean_supporting_improvement": candidate.mean_supporting_improvement,
        }))
    }).collect::<Result<Vec<_>, ScanError>>()?;
    let payload = json!({
        "version": 1,
        "selection": {
            "algorithm": "joint-admitted-support-v1",
            "maximum_generated": evidence.capacity.maximum_generated,
            "active_generated": evidence.capacity.active_generated.iter().map(|category| category.as_str()).collect::<Vec<_>>(),
        },
        "run": serde_json::from_str::<Value>(&codec::encode_run(plan.configuration.setup.run())?).map_err(|_| invalid_record())?,
        "naming": naming,
        "vocabulary": codec::revision(evidence.vocabulary), "cohort": codec::revision(evidence.cohort),
        "retrieval": codec::revision(evidence.retrieval), "frontier": codec::revision(evidence.frontier),
        "consolidation": codec::revision(evidence.consolidation), "policy": codec::revision(evidence.policy),
        "comparison_rules": codec::revision(evidence.comparison_rules),
        "comparison_names": codec::revision(evidence.comparison_names),
        "criteria": {
            "minimum_members": criteria.minimum_members, "coverage_per_thousand": criteria.coverage_per_thousand,
            "minimum_improvement": criteria.minimum_improvement, "minimum_separation": criteria.minimum_separation,
            "minimum_context_contrast": criteria.minimum_context_contrast, "max_proposals": criteria.max_proposals,
            "minimum_context_relatedness": criteria.minimum_context_relatedness,
        },
        "bounds": {
            "entry_points": bounds.entry_points, "expanded_concepts": bounds.expanded_concepts,
            "retrieval_edges": bounds.retrieval_edges, "retrieval_hops": bounds.retrieval_hops,
            "seeds": bounds.seeds, "results": bounds.results, "consolidation_hops": bounds.consolidation_hops,
            "consolidation_concepts": bounds.consolidation_concepts, "consolidation_edges": bounds.consolidation_edges,
        },
        "members": evidence.members.iter().map(content).collect::<Vec<_>>(),
        "sources": plan.sources.iter().map(content).collect::<Vec<_>>(),
        "candidates": candidates,
    });
    codec::encode(&payload, 524288)
}

fn content(content: &ClassificationContent) -> Value {
    json!({"scheme": content.scheme.as_str(), "digest": codec::digest(&content.digest), "byte_length": content.byte_length})
}

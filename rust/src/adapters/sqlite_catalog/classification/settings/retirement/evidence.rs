use serde_json::{Value, json};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::CategoryRetirementPlan;

use super::super::super::invalid_record;
use super::super::codec;

pub(super) fn encode(plan: &CategoryRetirementPlan) -> Result<String, ScanError> {
    let before = plan
        .before
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(invalid_record)?;
    let definitions = before
        .categories()
        .iter()
        .map(|category| (category.id(), category))
        .collect::<std::collections::BTreeMap<_, _>>();
    let retirements = plan.retirements.iter().map(|retirement| {
        let category = definitions.get(&retirement.category_id).ok_or_else(invalid_record)?;
        let retained = definitions.get(&retirement.retained_id).ok_or_else(invalid_record)?;
        let origin = plan.before.origins.get(&retirement.category_id).ok_or_else(invalid_record)?;
        Ok(json!({
            "category": category.id().as_str(), "label": category.label(),
            "semantic_revision": codec::revision(category.semantic_revision()),
            "retained_category": retained.id().as_str(), "retained_label": retained.label(),
            "retained_semantic_revision": codec::revision(retained.semantic_revision()),
            "origin": {
                "operation": origin.operation_id.as_str(), "concept": codec::digest(origin.concept_id.bytes()),
                "purpose": origin.purpose.as_ref().map(|purpose| purpose.as_str()),
                "admitted_label": origin.admitted_label,
                "admitted_semantic_revision": codec::revision(origin.admitted_semantic_revision),
                "protected": origin.protected,
            },
        }))
    }).collect::<Result<Vec<_>, ScanError>>()?;
    codec::encode(
        &json!({
            "version": 2, "kind": "equivalent_category_retirement", "algorithm": "exact-descriptions-v1",
            "run": serde_json::from_str::<Value>(&codec::encode_run(plan.configuration.setup.run())?).map_err(|_| invalid_record())?,
            "previous_rules": codec::revision(before.revision()),
            "human_categories": plan.before.human_categories.iter().map(|category| category.as_str()).collect::<Vec<_>>(),
            "retirements": retirements,
        }),
        524288,
    )
}

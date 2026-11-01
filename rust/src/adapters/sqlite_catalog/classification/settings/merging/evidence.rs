use serde_json::{Value, json};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::CategoryMergePlan;

use super::super::super::{codec, invalid_record};

pub(super) fn encode(plan: &CategoryMergePlan) -> Result<String, ScanError> {
    let basis = plan
        .before
        .bases
        .get(&plan.merged.category_id)
        .ok_or_else(invalid_record)?;
    let origin = plan
        .before
        .categories
        .origins
        .get(&plan.merged.category_id)
        .ok_or_else(invalid_record)?;
    let support = plan
        .coverage
        .categories
        .get(&plan.merged.category_id)
        .ok_or_else(invalid_record)?;
    let members = support.retained_members.iter().map(|member| {
        let alternative = member.alternative.as_ref().ok_or_else(invalid_record)?;
        Ok(json!({
            "scheme": member.content.scheme.as_str(), "digest": codec::digest(&member.content.digest),
            "byte_length": member.content.byte_length,
            "alternative": alternative.category_id.as_str(), "similarity": alternative.similarity,
        }))
    }).collect::<Result<Vec<_>, ScanError>>()?;
    codec::encode(
        &json!({
            "version": 3, "kind": "purpose_category_merge", "algorithm": "complete-purpose-support-v1",
            "run": serde_json::from_str::<Value>(&codec::encode_run(plan.configuration.setup.run())?).map_err(|_| invalid_record())?,
            "coverage": {
                "catalog_revision": plan.coverage.snapshot.catalog_revision,
                "configuration": plan.coverage.snapshot.configuration.catalog.configuration.map(codec::revision),
                "evidence": codec::revision(plan.coverage.evidence),
                "classified_contents": plan.coverage.classified_contents,
            },
            "merge": {
                "category": plan.merged.category_id.as_str(), "purpose": plan.merged.retained_id.as_str(),
                "purpose_revision": codec::revision(basis.naming.category_revision()),
                "label": origin.admitted_label, "semantic_revision": codec::revision(origin.admitted_semantic_revision),
                "origin_operation": origin.operation_id.as_str(), "origin_evidence": codec::revision(basis.evidence),
                "minimum_members": basis.minimum_members, "contents": support.contents, "members": members,
            },
        }),
        524288,
    )
}

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::{
    CategoryAdmissionCriteria, CategoryAdmissionPolicy, CategoryDefinition, CategoryId,
    CategoryMatcher, ClassificationEncoderFactory, ClassificationEncoderResidency,
    ClassificationFeatureRepository, ClassificationRevision, ConceptConsolidation, DiscoveryCohort,
    ResidencyClock, Taxonomy, assess_category_candidates, prepare_classification_matcher,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::Result;

#[derive(Deserialize)]
pub(super) struct Fixture {
    categories: Vec<Category>,
    calibration: [u8; 32],
    minimum_members: usize,
    coverage_per_thousand: u16,
    minimum_improvement: f32,
    minimum_separation: f32,
    minimum_context_contrast: f32,
    max_proposals: usize,
}

#[derive(Deserialize)]
struct Category {
    id: String,
    label: String,
    descriptions: Vec<String>,
}

pub(super) struct PreparedAdmission {
    pub(super) matcher: CategoryMatcher,
    pub(super) policy: CategoryAdmissionPolicy,
}

pub(super) fn prepare<F: ClassificationEncoderFactory, C: ResidencyClock>(
    fixture: Fixture,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
) -> Result<PreparedAdmission> {
    let policy = CategoryAdmissionPolicy::new(
        ClassificationRevision::from_bytes(fixture.calibration),
        CategoryAdmissionCriteria {
            minimum_members: fixture.minimum_members,
            coverage_per_thousand: fixture.coverage_per_thousand,
            minimum_improvement: fixture.minimum_improvement,
            minimum_separation: fixture.minimum_separation,
            minimum_context_contrast: fixture.minimum_context_contrast,
            max_proposals: fixture.max_proposals,
        },
    )?;
    let categories = fixture
        .categories
        .into_iter()
        .map(|category| {
            CategoryDefinition::new(
                CategoryId::new(category.id)?,
                category.label,
                category.descriptions,
            )
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let before = cache.feature_cache_stats()?;
    let started = Instant::now();
    let matcher = prepare_classification_matcher(
        Taxonomy::new(categories)?,
        residency,
        cache,
        &AtomicBool::new(false),
    )?;
    println!(
        "{}",
        json!({
            "stage": "admission_baseline",
            "seconds": started.elapsed().as_secs_f64(),
            "cache_entries_before": before.entries,
            "cache_entries_after": cache.feature_cache_stats()?.entries,
            "worker_retired": residency.resident_kind().is_none(),
        })
    );
    Ok(PreparedAdmission { matcher, policy })
}

impl PreparedAdmission {
    pub(super) fn assess(
        &self,
        cohort: &DiscoveryCohort,
        discovery: &ConceptConsolidation,
        member_ids: &BTreeMap<[u8; 32], &String>,
    ) -> Result<Value> {
        let started = Instant::now();
        let result = assess_category_candidates(
            cohort,
            discovery,
            &self.matcher,
            &self.policy,
            &AtomicBool::new(false),
        )?;
        let labels: BTreeMap<_, _> = discovery
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.candidate.concept.id(),
                    candidate.candidate.concept.label(),
                )
            })
            .collect();
        Ok(json!({
            "seconds": started.elapsed().as_secs_f64(),
            "policy_revision": result.policy_revision.bytes(),
            "taxonomy_revision": result.taxonomy_revision.bytes(),
            "taxonomy_names_revision": result.taxonomy_names_revision.bytes(),
            "minimum_context_relatedness": result.minimum_context_relatedness,
            "candidates": result.candidates.iter().map(|candidate| json!({
                "label": labels[&candidate.concept_id],
                "disposition": format!("{:?}", candidate.disposition),
                "supporting_members": candidate.supporting_members,
                "mean_supporting_improvement": candidate.mean_supporting_improvement,
                "members": candidate.members.iter().map(|member| json!({
                    "id": member_ids[&member.content.digest],
                    "current_category": member.current.category_id.as_str(),
                    "current_similarity": member.current.similarity,
                    "similarity": member.similarity,
                    "improvement": member.improvement,
                    "separation": member.separation,
                    "disposition": format!("{:?}", member.disposition),
                    "alternative_context": member.alternative_context.map(|alternative| json!({
                        "concept": alternative.concept.bytes(),
                        "similarity": alternative.similarity,
                        "relatedness": alternative.relatedness,
                    })),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))
    }
}

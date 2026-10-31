use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use rust_lib_cedarflake_ame::classification::{
    ClassificationEncoderFactory, ClassificationEncoderResidency, ClassificationFeatureRepository,
    ClassificationVocabulary, ConceptConsolidationLimits, ConceptConsolidationRequest, ConceptId,
    ConceptNominationRequest, DiscoveryCandidates, DiscoveryCohort, ResidencyClock,
    consolidate_classification_concepts, nominate_classification_contexts,
};
use serde_json::{Value, json};

use super::{Result, admission};

enum Strategy {
    Disabled,
    Frontier(BTreeSet<ConceptId>),
    Automatic {
        capacity: usize,
        minimum_relatedness: f32,
    },
}

pub(super) struct Naming {
    strategy: Strategy,
    assessment: Option<admission::PreparedAdmission>,
}

impl Naming {
    pub(super) fn prepare<F: ClassificationEncoderFactory, C: ResidencyClock>(
        frontier: BTreeSet<ConceptId>,
        automatic: Option<usize>,
        minimum_relatedness: Option<f32>,
        fixture: Option<admission::Fixture>,
        residency: &mut ClassificationEncoderResidency<F, C>,
        cache: &mut impl ClassificationFeatureRepository,
    ) -> Result<Self> {
        let strategy = if let Some(capacity) = automatic {
            if !frontier.is_empty() || fixture.is_none() || !(1..=64).contains(&capacity) {
                return Err(
                    "Automatic naming needs an admission profile and no supplied frontier".into(),
                );
            }
            let minimum_relatedness = minimum_relatedness
                .ok_or("Automatic naming needs an explicit context comparison cutoff")?;
            if !minimum_relatedness.is_finite() || !(0.0..=1.0).contains(&minimum_relatedness) {
                return Err("Context comparison cutoff is outside its bounds".into());
            }
            Strategy::Automatic {
                capacity,
                minimum_relatedness,
            }
        } else if frontier.is_empty() {
            Strategy::Disabled
        } else {
            Strategy::Frontier(frontier)
        };
        let assessment = fixture
            .map(|fixture| admission::prepare(fixture, residency, cache))
            .transpose()?;
        Ok(Self {
            strategy,
            assessment,
        })
    }

    pub(super) fn enabled(&self) -> bool {
        !matches!(self.strategy, Strategy::Disabled)
    }

    pub(super) fn resolve<F: ClassificationEncoderFactory, C: ResidencyClock>(
        &self,
        cohort: &DiscoveryCohort,
        retrieved: &DiscoveryCandidates,
        vocabulary: &mut impl ClassificationVocabulary,
        residency: &mut ClassificationEncoderResidency<F, C>,
        cache: &mut impl ClassificationFeatureRepository,
        member_ids: &BTreeMap<[u8; 32], &String>,
    ) -> Result<Option<Value>> {
        let limits = ConceptConsolidationLimits {
            hops: 4,
            concepts: 4096,
            edges: 65_536,
        };
        let cancelled = AtomicBool::new(false);
        let (result, nomination) = match &self.strategy {
            Strategy::Disabled => return Ok(None),
            Strategy::Frontier(frontier) => (
                consolidate_classification_concepts(
                    ConceptConsolidationRequest {
                        cohort,
                        retrieved,
                        frontier,
                        limits,
                    },
                    vocabulary,
                    residency,
                    cache,
                    &cancelled,
                )?,
                None,
            ),
            Strategy::Automatic {
                capacity,
                minimum_relatedness,
            } => {
                let assessment = self.assessment.as_ref().ok_or("Missing naming profile")?;
                let result = nominate_classification_contexts(
                    ConceptNominationRequest {
                        cohort,
                        retrieved,
                        matcher: &assessment.matcher,
                        policy: &assessment.policy,
                        limits,
                        max_candidates: *capacity,
                        minimum_context_relatedness: *minimum_relatedness,
                        naming: &assessment.naming,
                    },
                    vocabulary,
                    residency,
                    cache,
                    &cancelled,
                )?;
                let counts = json!({
                    "considered_concepts": result.considered_concepts,
                    "naming_concepts": result.naming_concepts,
                    "eligible_concepts": result.eligible_concepts,
                    "subsumed_concepts": result.subsumed_concepts,
                    "candidate_limit_reached": result.candidate_limit_reached,
                });
                (result.contexts, Some(counts))
            }
        };
        let assessment = self
            .assessment
            .as_ref()
            .map(|prepared| prepared.assess(cohort, &result, member_ids))
            .transpose()?;
        Ok(Some(json!({
            "naming_revision": result.naming.revision().bytes(),
            "nomination": nomination,
            "admission": assessment,
            "candidates": result.candidates.iter().map(|candidate| json!({
                "label": candidate.candidate.concept.label(),
                "description": candidate.candidate.concept.description(),
                "similarity": candidate.candidate.scores.similarity,
                "contrast": candidate.candidate.scores.contrast,
                "narrower_concepts": candidate.narrower_concepts.len(),
                "members": candidate.member_similarities.iter().map(|member| json!({
                    "id": member_ids[&member.content.digest], "similarity": member.similarity,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "unresolved_concepts": result.unresolved_concepts.len(),
            "visited_edges": result.visited_edges,
        })))
    }
}

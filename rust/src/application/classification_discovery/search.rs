use std::collections::{BTreeMap, BTreeSet};

use crate::domain::ScanError;
use crate::domain::classification::discovery::{ConceptId, DiscoveryCohort, VocabularyConcept};
use crate::domain::classification::{ClassificationRevision, TextFeature};

use super::ranking::{CandidateRankings, ConceptMatch, DiscoveryCandidates};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConceptSearchLimits {
    pub entry_points: usize,
    pub expanded_concepts: usize,
    pub edges: usize,
    pub hops: usize,
    pub seeds: usize,
    pub results: usize,
}

impl ConceptSearchLimits {
    pub(super) fn validate(&self) -> Result<(), ScanError> {
        if !(1..=4096).contains(&self.entry_points)
            || self.expanded_concepts > 4096
            || self.edges > 65_536
            || self.hops > 4
            || !(1..=64).contains(&self.seeds)
            || !(1..=64).contains(&self.results)
        {
            return Err(ScanError::new(
                "classification_discovery_limit",
                "The concept search exceeds its resource limits",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConceptSearchBound {
    EntryPoints,
    ExpandedConcepts,
    Edges,
}

pub(super) struct ConceptSearch {
    limits: ConceptSearchLimits,
    seen: BTreeMap<ConceptId, ClassificationRevision>,
    rankings: CandidateRankings,
    bounds: BTreeSet<ConceptSearchBound>,
}

impl ConceptSearch {
    pub fn new(limits: ConceptSearchLimits) -> Self {
        Self {
            limits,
            seen: BTreeMap::new(),
            rankings: CandidateRankings::new(limits.seeds.max(limits.results)),
            bounds: BTreeSet::new(),
        }
    }

    pub fn scored_concepts(&self) -> usize {
        self.seen.len()
    }

    pub fn has_concept(&self, concept: &VocabularyConcept) -> Result<bool, ScanError> {
        let Some(revision) = self.seen.get(&concept.id()) else {
            return Ok(false);
        };
        if *revision != concept.revision() {
            return Err(ScanError::new(
                "classification_vocabulary_changed",
                "A concept changed within the discovery vocabulary",
            ));
        }
        Ok(true)
    }

    pub fn rank(
        &mut self,
        concept: VocabularyConcept,
        cohort: &DiscoveryCohort,
        feature: &TextFeature,
    ) -> Result<(), ScanError> {
        if feature.description != ClassificationRevision::description(concept.description()) {
            return Err(ScanError::new(
                "classification_feature_mismatch",
                "The concept description changed",
            ));
        }
        let scores = cohort.score(feature).map_err(|error| {
            ScanError::new("classification_feature_mismatch", error.to_string())
        })?;
        self.seen.insert(concept.id(), concept.revision());
        self.rankings.insert(ConceptMatch { concept, scores });
        Ok(())
    }

    pub fn seeds(&self) -> Vec<ConceptId> {
        self.rankings
            .distinctive
            .iter()
            .take(self.limits.seeds)
            .map(|candidate| candidate.concept.id())
            .collect()
    }

    pub fn reached(&mut self, bound: ConceptSearchBound) {
        self.bounds.insert(bound);
    }

    pub fn finish(
        mut self,
        cohort: &DiscoveryCohort,
        vocabulary_revision: ClassificationRevision,
        visited_edges: usize,
    ) -> DiscoveryCandidates {
        self.rankings.nearest.truncate(self.limits.results);
        self.rankings.distinctive.truncate(self.limits.results);
        DiscoveryCandidates {
            model: cohort.model(),
            vocabulary_revision,
            cohort_revision: cohort.revision(),
            retrieval_revision: ClassificationRevision::derive(
                "ame.classification.concept-retrieval.v1",
                std::iter::empty(),
            ),
            limits: self.limits,
            nearest: self.rankings.nearest,
            distinctive: self.rankings.distinctive,
            scored_concepts: self.seen.len(),
            visited_edges,
            bounds_reached: self.bounds.into_iter().collect(),
        }
    }
}

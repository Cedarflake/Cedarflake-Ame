use crate::domain::classification::discovery::{ConceptSimilarity, VocabularyConcept};
use crate::domain::classification::{ClassificationRevision, ModelCompatibility};

use super::{ConceptSearchBound, ConceptSearchLimits};

#[derive(Clone, Debug, PartialEq)]
pub struct ConceptMatch {
    pub concept: VocabularyConcept,
    pub scores: ConceptSimilarity,
}

pub struct DiscoveryCandidates {
    pub model: ModelCompatibility,
    pub vocabulary_revision: ClassificationRevision,
    pub cohort_revision: ClassificationRevision,
    pub retrieval_revision: ClassificationRevision,
    pub limits: ConceptSearchLimits,
    pub nearest: Vec<ConceptMatch>,
    pub distinctive: Vec<ConceptMatch>,
    pub scored_concepts: usize,
    pub visited_edges: usize,
    pub bounds_reached: Vec<ConceptSearchBound>,
}

pub(super) struct CandidateRankings {
    pub nearest: Vec<ConceptMatch>,
    pub distinctive: Vec<ConceptMatch>,
    capacity: usize,
}

impl CandidateRankings {
    pub fn new(capacity: usize) -> Self {
        Self {
            nearest: Vec::new(),
            distinctive: Vec::new(),
            capacity,
        }
    }

    pub fn insert(&mut self, candidate: ConceptMatch) {
        self.nearest.push(candidate.clone());
        self.nearest.sort_by(|left, right| {
            right
                .scores
                .similarity
                .total_cmp(&left.scores.similarity)
                .then_with(|| left.concept.id().cmp(&right.concept.id()))
        });
        self.nearest.truncate(self.capacity);
        self.distinctive.push(candidate);
        self.distinctive.sort_by(|left, right| {
            right
                .scores
                .contrast
                .total_cmp(&left.scores.contrast)
                .then_with(|| right.scores.similarity.total_cmp(&left.scores.similarity))
                .then_with(|| left.concept.id().cmp(&right.concept.id()))
        });
        self.distinctive.truncate(self.capacity);
    }
}

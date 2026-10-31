use std::collections::BTreeSet;

use crate::domain::classification::discovery::ConceptSimilarity;

use super::*;

struct Harness {
    _directory: tempfile::TempDir,
    calls: Rc<Calls>,
    residency: ClassificationEncoderResidency<Factory, Clock>,
    cache: SqliteClassificationFeatureCache,
    vocabulary: Vocabulary,
    cohort: DiscoveryCohort,
    retrieved: DiscoveryCandidates,
}

impl Harness {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
        let calls = Rc::new(Calls::default());
        let residency = ClassificationEncoderResidency::new(
            Factory(calls.clone()),
            Clock,
            Duration::from_secs(60),
        );
        let mut vocabulary = vocabulary();
        vocabulary.concepts.insert(
            id(6),
            VocabularyConcept::new(id(6), "unrelated".into(), "an unrelated subject".into())
                .unwrap(),
        );
        vocabulary.links = BTreeMap::from([(id(1), vec![id(2)]), (id(3), vec![id(2)])]);
        let cohort = cohort();
        let nearest = [1, 3, 6]
            .map(|value| ConceptMatch {
                concept: vocabulary.concepts[&id(value)].clone(),
                scores: ConceptSimilarity {
                    similarity: 1.0,
                    context_similarity: 0.0,
                    contrast: 1.0,
                },
            })
            .to_vec();
        let retrieved = DiscoveryCandidates {
            model: model(),
            vocabulary_revision: revision(4),
            cohort_revision: cohort.revision(),
            retrieval_revision: revision(5),
            limits: limits(),
            nearest: nearest.clone(),
            distinctive: nearest,
            scored_concepts: 3,
            visited_edges: 0,
            bounds_reached: vec![],
        };
        Self {
            _directory: directory,
            calls,
            residency,
            cache,
            vocabulary,
            cohort,
            retrieved,
        }
    }

    fn resolve(
        &mut self,
        frontier: &[u8],
        limits: ConceptConsolidationLimits,
        cancelled: &AtomicBool,
    ) -> Result<ConceptConsolidation, ScanError> {
        consolidate_classification_concepts(
            ConceptConsolidationRequest {
                cohort: &self.cohort,
                retrieved: &self.retrieved,
                frontier: &frontier.iter().map(|&value| id(value)).collect(),
                limits,
            },
            &mut self.vocabulary,
            &mut self.residency,
            &mut self.cache,
            cancelled,
        )
    }
}

fn consolidation_limits() -> ConceptConsolidationLimits {
    ConceptConsolidationLimits {
        hops: 4,
        concepts: 16,
        edges: 32,
    }
}

#[test]
fn shared_context_is_rescored_instead_of_inheriting_narrow_names_or_duplicate_votes() {
    let mut harness = Harness::new();
    let result = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result.candidates.len(), 1);
    let candidate = &result.candidates[0];
    assert_eq!(candidate.candidate.concept.id(), id(2));
    assert_eq!(candidate.narrower_concepts, vec![id(1), id(3)]);
    assert_eq!(candidate.candidate.scores.similarity, 0.0);
    assert_eq!(candidate.candidate.scores.contrast, -1.0);
    assert_eq!(result.unresolved_concepts, vec![id(6)]);
    assert_eq!(&*harness.calls.encoded.borrow(), &["a room"]);
    assert_eq!(harness.calls.retires.get(), 1);
    let repeated = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(repeated.candidates, result.candidates);
    assert_eq!(harness.calls.starts.get(), 1);
    assert!(
        harness
            .vocabulary
            .requests
            .iter()
            .all(|(kind, limit)| matches!(kind, ConceptNeighborhood::Broader(_)) && *limit <= 64)
    );
}

#[test]
fn cycles_and_unreachable_frontiers_return_unresolved_seeds_without_encoding() {
    let mut harness = Harness::new();
    harness.vocabulary.links = BTreeMap::from([(id(1), vec![id(3)]), (id(3), vec![id(1)])]);
    let result = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .unwrap();
    assert!(result.candidates.is_empty());
    assert_eq!(result.unresolved_concepts, vec![id(1), id(3), id(6)]);
    assert_eq!(result.visited_edges, 4);
    assert_eq!(harness.calls.starts.get(), 0);
}

#[test]
fn graph_budgets_and_partial_retrieval_reject_the_proposal_without_publishing_a_partial_frontier() {
    let mut harness = Harness::new();
    for limits in [
        ConceptConsolidationLimits {
            concepts: 3,
            ..consolidation_limits()
        },
        ConceptConsolidationLimits {
            edges: 1,
            ..consolidation_limits()
        },
    ] {
        let error = harness
            .resolve(&[2], limits, &AtomicBool::new(false))
            .err()
            .unwrap();
        assert_eq!(error.code, "classification_discovery_limit");
    }
    harness
        .retrieved
        .bounds_reached
        .push(ConceptSearchBound::Edges);
    let error = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_discovery_limit");
    assert_eq!(harness.calls.starts.get(), 0);
}

#[test]
fn changed_cohort_or_vocabulary_is_rejected_before_model_work() {
    let mut harness = Harness::new();
    harness.retrieved.cohort_revision = revision(9);
    let error = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_feature_mismatch");
    assert!(harness.vocabulary.requests.is_empty());
    harness.retrieved.cohort_revision = harness.cohort.revision();
    harness.vocabulary.changed_on_read = true;
    let error = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_vocabulary_changed");
    assert_eq!(harness.calls.starts.get(), 0);
}

#[test]
fn conflicting_definitions_under_one_revision_are_rejected() {
    let mut harness = Harness::new();
    harness.vocabulary.links = BTreeMap::from([(id(3), vec![id(1)])]);
    harness.vocabulary.concepts.insert(
        id(1),
        VocabularyConcept::new(id(1), "changed".into(), "changed meaning".into()).unwrap(),
    );
    let error = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_vocabulary_changed");
    assert_eq!(harness.calls.starts.get(), 0);
}

#[test]
fn cancellation_during_rescoring_retires_the_worker_and_rejects_late_features() {
    let mut harness = Harness::new();
    harness.calls.cancel_at.set(Some(1));
    let error = harness
        .resolve(&[2], consolidation_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_cancelled");
    assert_eq!(harness.calls.retires.get(), 1);
    assert_eq!(harness.residency.resident_kind(), None);
    assert_eq!(harness.cache.feature_cache_stats().unwrap().entries, 0);
}

#[test]
fn naming_frontier_stops_traversal_and_keeps_its_identity_independent_of_order() {
    let mut harness = Harness::new();
    let result = harness
        .resolve(&[3, 1], consolidation_limits(), &AtomicBool::new(false))
        .unwrap();
    let repeated = harness
        .resolve(&[1, 3], consolidation_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result.frontier_revision, repeated.frontier_revision);
    assert_eq!(result.candidates, repeated.candidates);
    assert_eq!(result.candidates[0].candidate.concept.id(), id(3));
    assert_eq!(result.retrieval_revision, revision(5));
    assert_eq!(result.retrieval_limits, limits());
    let queried: BTreeSet<_> = harness
        .vocabulary
        .requests
        .iter()
        .filter_map(|(kind, _)| match kind {
            ConceptNeighborhood::Broader(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(queried, BTreeSet::from([id(6)]));
}

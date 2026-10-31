use crate::application::classification_matching::CategoryMatcher;
use crate::domain::classification::discovery::{CategoryNaming, ConceptNamingScope};
use crate::domain::classification::{CategoryDefinition, CategoryId, Taxonomy};

use super::*;

mod purpose;

struct Harness {
    _directory: tempfile::TempDir,
    calls: Rc<Calls>,
    residency: ClassificationEncoderResidency<Factory, Clock>,
    cache: SqliteClassificationFeatureCache,
    vocabulary: Vocabulary,
    cohort: DiscoveryCohort,
    retrieved: DiscoveryCandidates,
    matcher: CategoryMatcher,
    policy: CategoryAdmissionPolicy,
    minimum_relatedness: f32,
    naming: CategoryNaming,
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
            id(4),
            VocabularyConcept::new(id(4), "detail".into(), "a detail".into()).unwrap(),
        );
        vocabulary.links = BTreeMap::from([
            (id(1), vec![id(2)]),
            (id(2), vec![id(3)]),
            (id(4), vec![id(1)]),
        ]);
        let sample = |identity, values| DiscoverySample {
            content: ClassificationContent {
                scheme: ClassificationRecordId::new("nomination-fixture".into()).unwrap(),
                digest: [identity; 32],
                byte_length: 100,
            },
            feature: ImageFeature {
                identity: model().image_identity(),
                embedding: Embedding::normalized(values).unwrap(),
            },
        };
        let cohort = DiscoveryCohort::new(
            model(),
            &[
                sample(1, vec![1.0, 0.0]),
                sample(2, vec![1.0, 0.0]),
                sample(3, vec![0.0, 1.0]),
            ],
            &[sample(9, vec![-1.0, 0.0])],
        )
        .unwrap();
        let nearest: Vec<_> = [1, 4]
            .into_iter()
            .map(|value| ConceptMatch {
                concept: vocabulary.concepts[&id(value)].clone(),
                scores: crate::domain::classification::discovery::ConceptSimilarity {
                    similarity: 1.0,
                    context_similarity: 0.0,
                    contrast: 1.0,
                },
            })
            .collect();
        let retrieved = DiscoveryCandidates {
            model: model(),
            vocabulary_revision: revision(4),
            cohort_revision: cohort.revision(),
            retrieval_revision: revision(5),
            limits: limits(),
            nearest: nearest.clone(),
            distinctive: nearest,
            scored_concepts: 2,
            visited_edges: 0,
            bounds_reached: vec![],
        };
        let policy = CategoryAdmissionPolicy::new(
            revision(8),
            CategoryAdmissionCriteria {
                minimum_members: 2,
                coverage_per_thousand: 500,
                minimum_improvement: 0.0,
                minimum_separation: 0.0,
                minimum_context_contrast: 0.0,
                max_proposals: 8,
            },
        )
        .unwrap();
        Self {
            _directory: directory,
            calls,
            residency,
            cache,
            vocabulary,
            cohort,
            retrieved,
            matcher: matcher("a room", vec![0.0, 1.0]),
            policy,
            minimum_relatedness: 0.9,
            naming: CategoryNaming::Unconditioned,
        }
    }

    fn nominate(
        &mut self,
        max_candidates: usize,
        limits: ConceptConsolidationLimits,
        cancelled: &AtomicBool,
    ) -> Result<ConceptNomination, ScanError> {
        nominate_classification_contexts(
            ConceptNominationRequest {
                cohort: &self.cohort,
                retrieved: &self.retrieved,
                matcher: &self.matcher,
                policy: &self.policy,
                limits,
                max_candidates,
                minimum_context_relatedness: self.minimum_relatedness,
                naming: &self.naming,
            },
            &mut self.vocabulary,
            &mut self.residency,
            &mut self.cache,
            cancelled,
        )
    }
}

fn matcher(description: &str, values: Vec<f32>) -> CategoryMatcher {
    CategoryMatcher::new(
        Taxonomy::new(vec![
            CategoryDefinition::new(
                CategoryId::new("existing".into()).unwrap(),
                "Existing".into(),
                vec![description.into()],
            )
            .unwrap(),
        ])
        .unwrap(),
        model(),
        vec![TextFeature {
            identity: model().text_identity(),
            description: ClassificationRevision::description(description),
            embedding: Embedding::normalized(values).unwrap(),
        }],
    )
    .unwrap()
}

fn context_limits() -> ConceptConsolidationLimits {
    ConceptConsolidationLimits {
        hops: 4,
        concepts: 16,
        edges: 32,
    }
}

#[test]
fn nomination_reaches_contexts_without_a_supplied_frontier_and_reuses_cached_evidence() {
    let mut harness = Harness::new();
    let first = harness
        .nominate(1, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(first.considered_concepts, 4);
    assert_eq!(first.eligible_concepts, 2);
    assert!(!first.candidate_limit_reached);
    assert_eq!(first.contexts.candidates.len(), 1);
    let candidate = &first.contexts.candidates[0];
    assert_eq!(candidate.candidate.concept.id(), id(3));
    assert_eq!(candidate.narrower_concepts, vec![id(1), id(4)]);
    assert_eq!(candidate.member_similarities.len(), 3);
    assert_eq!(candidate.member_similarities[2].similarity, 0.0);
    assert!(first.contexts.unresolved_concepts.is_empty());
    let encoded = harness.calls.encoded.borrow().len();
    let repeated = harness
        .nominate(1, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(repeated.contexts.candidates, first.contexts.candidates);
    assert_eq!(
        repeated.contexts.frontier_revision,
        first.contexts.frontier_revision
    );
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
    assert!(
        harness.vocabulary.requests.iter().all(|(kind, limit)| {
            matches!(kind, ConceptNeighborhood::Broader(_)) && *limit <= 64
        })
    );
}

#[test]
fn current_taxonomy_can_retain_all_members_without_inventing_a_context() {
    let mut harness = Harness::new();
    harness.matcher = matcher("a classroom", vec![1.0, 0.0]);
    let result = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result.eligible_concepts, 0);
    assert!(!result.candidate_limit_reached);
    assert!(result.contexts.candidates.is_empty());
    assert_eq!(result.contexts.unresolved_concepts, vec![id(1), id(4)]);
}

#[test]
fn traversal_only_contexts_keep_the_path_to_a_name_without_encoding_their_descriptions() {
    let mut harness = Harness::new();
    let intermediate = harness.vocabulary.concepts[&id(2)]
        .clone()
        .with_naming_scope(ConceptNamingScope::TraversalOnly);
    harness.vocabulary.concepts.insert(id(2), intermediate);
    let result = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result.considered_concepts, 4);
    assert_eq!(result.naming_concepts, 3);
    assert_eq!(result.contexts.candidates[0].candidate.concept.id(), id(3));
    assert_eq!(
        result.contexts.candidates[0].narrower_concepts,
        vec![id(1), id(4)]
    );
    assert!(
        !harness
            .calls
            .encoded
            .borrow()
            .iter()
            .any(|text| text == "a room")
    );
}

#[test]
fn a_pruned_distinct_context_still_prevents_false_member_support() {
    let mut harness = Harness::new();
    let members: Vec<_> = harness
        .cohort
        .member_samples()
        .iter()
        .zip([vec![1.0, 0.0], vec![0.8, 0.6], vec![0.0, 1.0]])
        .map(|(sample, values)| DiscoverySample {
            content: sample.content.clone(),
            feature: ImageFeature {
                identity: model().image_identity(),
                embedding: Embedding::normalized(values).unwrap(),
            },
        })
        .collect();
    let mut comparison = members[0].clone();
    comparison.content.digest = [9; 32];
    comparison.feature.embedding = Embedding::normalized(vec![-1.0, 0.0]).unwrap();
    harness.cohort = DiscoveryCohort::new(model(), &members, &[comparison]).unwrap();
    harness.retrieved.cohort_revision = harness.cohort.revision();
    harness
        .cache
        .store_text_feature(
            model(),
            &TextFeature {
                identity: model().text_identity(),
                description: ClassificationRevision::description("a detail"),
                embedding: Embedding::normalized(vec![0.8, 0.6]).unwrap(),
            },
        )
        .unwrap();

    let nomination = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(nomination.contexts.candidates.len(), 1);
    assert_eq!(
        nomination.contexts.candidates[0].candidate.concept.id(),
        id(3)
    );
    let assessment = assess_category_candidates(
        &harness.cohort,
        &nomination.contexts,
        &harness.matcher,
        &harness.policy,
        &AtomicBool::new(false),
    )
    .unwrap();
    let candidate = &assessment.candidates[0];
    assert_eq!(candidate.supporting_members, 1);
    assert_eq!(
        candidate.disposition,
        CategoryAdmissionDisposition::InsufficientMembers
    );
    assert_eq!(
        candidate.members[1].disposition,
        MemberAdmission::CompetingContext
    );
    assert_eq!(
        candidate.members[1].alternative_context.unwrap().concept,
        id(4)
    );

    let encoded = harness.calls.encoded.borrow().len();
    harness.minimum_relatedness = 0.55;
    let related = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_ne!(
        nomination.contexts.frontier_revision,
        related.contexts.frontier_revision
    );
    let assessment = assess_category_candidates(
        &harness.cohort,
        &related.contexts,
        &harness.matcher,
        &harness.policy,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(assessment.candidates[0].supporting_members, 2);
    assert_eq!(
        assessment.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
}

#[test]
fn capacity_and_taxonomy_changes_bind_nomination_without_reencoding_descriptions() {
    let mut harness = Harness::new();
    let first = harness
        .nominate(1, context_limits(), &AtomicBool::new(false))
        .unwrap();
    let encoded = harness.calls.encoded.borrow().len();
    let wider = harness
        .nominate(2, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(wider.contexts.candidates.len(), 1);
    assert!(!wider.candidate_limit_reached);
    assert_ne!(
        first.contexts.frontier_revision,
        wider.contexts.frontier_revision
    );
    harness.matcher = matcher("a classroom", vec![1.0, 0.0]);
    let changed = harness
        .nominate(2, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_ne!(
        wider.contexts.frontier_revision,
        changed.contexts.frontier_revision
    );
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
}

#[test]
fn cancellation_during_nomination_retires_the_worker_and_retains_only_completed_features() {
    let mut harness = Harness::new();
    harness.calls.cancel_at.set(Some(2));
    let cancelled = AtomicBool::new(false);
    let error = harness
        .nominate(64, context_limits(), &cancelled)
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_cancelled");
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
    assert_eq!(harness.cache.feature_cache_stats().unwrap().entries, 1);
}

#[test]
fn exhausted_graph_work_rejects_nomination_before_encoding() {
    let mut harness = Harness::new();
    for limits in [
        ConceptConsolidationLimits {
            concepts: 2,
            ..context_limits()
        },
        ConceptConsolidationLimits {
            edges: 1,
            ..context_limits()
        },
    ] {
        let error = harness
            .nominate(64, limits, &AtomicBool::new(false))
            .err()
            .unwrap();
        assert_eq!(error.code, "classification_discovery_limit");
    }
    assert_eq!(harness.calls.starts.get(), 0);
}

#[test]
fn exhausted_ancestor_selection_retires_the_worker_without_publishing_partial_names() {
    let mut harness = Harness::new();
    harness
        .retrieved
        .nearest
        .retain(|candidate| candidate.concept.id() == id(4));
    harness.retrieved.distinctive = harness.retrieved.nearest.clone();
    for description in ["a detail", "a workshop", "a room", "a classroom"] {
        harness
            .calls
            .features
            .borrow_mut()
            .insert(description.into(), vec![1.0, 0.0]);
    }
    let error = harness
        .nominate(
            64,
            ConceptConsolidationLimits {
                edges: 4,
                ..context_limits()
            },
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_discovery_limit");
    assert_eq!(harness.calls.encoded.borrow().len(), 4);
    assert_eq!(harness.cache.feature_cache_stats().unwrap().entries, 4);
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
}

#[test]
fn changed_retrieval_or_vocabulary_cannot_nominate_categories() {
    let mut harness = Harness::new();
    harness.retrieved.cohort_revision = revision(9);
    assert!(
        harness
            .nominate(64, context_limits(), &AtomicBool::new(false))
            .is_err()
    );
    harness.retrieved.cohort_revision = harness.cohort.revision();
    harness
        .retrieved
        .bounds_reached
        .push(ConceptSearchBound::Edges);
    assert!(
        harness
            .nominate(64, context_limits(), &AtomicBool::new(false))
            .is_err()
    );
    harness.retrieved.bounds_reached.clear();
    harness.vocabulary.changed_on_read = true;
    let error = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_vocabulary_changed");
    assert_eq!(harness.calls.starts.get(), 0);
}

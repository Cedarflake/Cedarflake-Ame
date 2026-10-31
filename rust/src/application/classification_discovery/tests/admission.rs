use crate::application::classification_matching::CategoryMatcher;
use crate::domain::classification::{CategoryDefinition, CategoryId, Taxonomy, TextFeature};

use super::*;

struct AdmissionFixture {
    cohort: DiscoveryCohort,
    discovery: ConceptConsolidation,
    matcher: CategoryMatcher,
}

impl AdmissionFixture {
    fn new() -> Self {
        Self::with_existing_name("Existing")
    }

    fn with_existing_name(label: &str) -> Self {
        let members: Vec<_> = (1..=4)
            .map(|value| {
                let values = if value <= 2 {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                };
                DiscoverySample {
                    content: ClassificationContent {
                        scheme: ClassificationRecordId::new("admission-fixture".into()).unwrap(),
                        digest: [value; 32],
                        byte_length: 100,
                    },
                    feature: ImageFeature {
                        identity: model().image_identity(),
                        embedding: Embedding::normalized(values).unwrap(),
                    },
                }
            })
            .collect();
        let comparison = DiscoverySample {
            content: ClassificationContent {
                digest: [9; 32],
                ..members[0].content.clone()
            },
            feature: ImageFeature {
                identity: model().image_identity(),
                embedding: Embedding::normalized(vec![-1.0, -1.0]).unwrap(),
            },
        };
        let cohort = DiscoveryCohort::new(model(), &members, &[comparison]).unwrap();
        let candidates = [(1, "First", vec![1.0, 0.0]), (2, "Second", vec![0.0, 1.0])]
            .into_iter()
            .map(|(identity, label, values)| {
                let feature = TextFeature {
                    identity: model().text_identity(),
                    description: ClassificationRevision::description(label),
                    embedding: Embedding::normalized(values).unwrap(),
                };
                ConsolidatedConcept {
                    candidate: ConceptMatch {
                        concept: VocabularyConcept::new(id(identity), label.into(), label.into())
                            .unwrap(),
                        scores: cohort.score(&feature).unwrap(),
                    },
                    member_similarities: cohort.member_similarities(&feature).unwrap(),
                    narrower_concepts: vec![],
                }
            })
            .collect();
        let discovery = ConceptConsolidation {
            model: model(),
            cohort_revision: cohort.revision(),
            vocabulary_revision: revision(4),
            retrieval_revision: revision(5),
            retrieval_limits: limits(),
            frontier_revision: revision(6),
            consolidation_revision: revision(7),
            limits: ConceptConsolidationLimits {
                hops: 4,
                concepts: 16,
                edges: 32,
            },
            candidates,
            unresolved_concepts: vec![],
            visited_edges: 2,
        };
        let taxonomy = Taxonomy::new(vec![
            CategoryDefinition::new(
                CategoryId::new("existing".into()).unwrap(),
                label.into(),
                vec!["baseline".into()],
            )
            .unwrap(),
        ])
        .unwrap();
        let matcher = CategoryMatcher::new(
            taxonomy,
            model(),
            vec![TextFeature {
                identity: model().text_identity(),
                description: ClassificationRevision::description("baseline"),
                embedding: Embedding::normalized(vec![-1.0, -1.0]).unwrap(),
            }],
        )
        .unwrap();
        Self {
            cohort,
            discovery,
            matcher,
        }
    }

    fn assess(&self, criteria: CategoryAdmissionCriteria) -> Result<CategoryAdmission, ScanError> {
        assess_category_candidates(
            &self.cohort,
            &self.discovery,
            &self.matcher,
            &CategoryAdmissionPolicy::new(revision(8), criteria).unwrap(),
            &AtomicBool::new(false),
        )
    }
}

fn criteria() -> CategoryAdmissionCriteria {
    CategoryAdmissionCriteria {
        minimum_members: 2,
        coverage_per_thousand: 500,
        minimum_improvement: 0.75,
        minimum_separation: 0.25,
        minimum_context_contrast: 0.25,
        max_proposals: 2,
    }
}

#[test]
fn proposals_retain_supported_members_and_the_images_that_keep_their_current_category() {
    let fixture = AdmissionFixture::new();
    let result = fixture.assess(criteria()).unwrap();
    assert_eq!(result.candidates.len(), 2);
    let first = &result.candidates[0];
    assert_eq!(first.disposition, CategoryAdmissionDisposition::Proposed);
    assert_eq!(first.supporting_members, 2);
    let supporting: Vec<_> = first
        .members
        .iter()
        .filter(|member| member.disposition == MemberAdmission::Supports)
        .map(|member| member.content.digest)
        .collect();
    assert_eq!(supporting, vec![[1; 32], [2; 32]]);
    assert_eq!(first.members[2].disposition, MemberAdmission::NoImprovement);
    assert_eq!(
        result.taxonomy_revision,
        fixture.matcher.taxonomy().revision()
    );
    assert_eq!(result.cohort_revision, fixture.cohort.revision());
    assert_eq!(result.retrieval_limits, fixture.discovery.retrieval_limits);
    assert_eq!(result.consolidation_limits, fixture.discovery.limits);
}

#[test]
fn coverage_and_repeat_support_are_independent_of_a_good_average() {
    let fixture = AdmissionFixture::new();
    let count = fixture
        .assess(CategoryAdmissionCriteria {
            minimum_members: 3,
            ..criteria()
        })
        .unwrap();
    assert!(count.candidates.iter().all(|candidate| {
        candidate.disposition == CategoryAdmissionDisposition::InsufficientMembers
    }));
    let coverage = fixture
        .assess(CategoryAdmissionCriteria {
            coverage_per_thousand: 501,
            ..criteria()
        })
        .unwrap();
    assert!(coverage.candidates.iter().all(|candidate| {
        candidate.disposition == CategoryAdmissionDisposition::InsufficientCoverage
    }));
}

#[test]
fn equal_competing_contexts_do_not_gain_support_from_identity_tiebreaks() {
    let mut fixture = AdmissionFixture::new();
    let scores = fixture.discovery.candidates[0].member_similarities.clone();
    fixture.discovery.candidates[1].member_similarities = scores;
    fixture.discovery.candidates[1].candidate.scores =
        fixture.discovery.candidates[0].candidate.scores;
    let result = fixture
        .assess(CategoryAdmissionCriteria {
            minimum_separation: 0.0,
            ..criteria()
        })
        .unwrap();
    for candidate in result.candidates {
        assert_eq!(candidate.supporting_members, 0);
        assert_eq!(
            candidate.members[0].disposition,
            MemberAdmission::CompetingContext
        );
    }
}

#[test]
fn budget_and_name_conflicts_defer_proposals_without_dropping_evidence() {
    let mut fixture = AdmissionFixture::new();
    let result = fixture
        .assess(CategoryAdmissionCriteria {
            max_proposals: 1,
            ..criteria()
        })
        .unwrap();
    assert_eq!(
        result.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    assert_eq!(
        result.candidates[1].disposition,
        CategoryAdmissionDisposition::ProposalLimit
    );
    fixture.discovery.candidates[0].candidate.concept =
        VocabularyConcept::new(id(1), "EXISTING".into(), "First".into()).unwrap();
    let result = fixture.assess(criteria()).unwrap();
    assert_eq!(
        result.candidates[0].disposition,
        CategoryAdmissionDisposition::NameConflict
    );
    assert_eq!(
        result.candidates[1].disposition,
        CategoryAdmissionDisposition::Proposed
    );
}

#[test]
fn reordered_candidates_and_member_rows_keep_the_same_assessment() {
    let mut fixture = AdmissionFixture::new();
    let original = fixture.assess(criteria()).unwrap();
    fixture.discovery.candidates.reverse();
    for candidate in &mut fixture.discovery.candidates {
        candidate.member_similarities.reverse();
    }
    assert_eq!(fixture.assess(criteria()).unwrap(), original);
}

#[test]
fn renaming_an_existing_category_invalidates_name_admission_without_rematching_semantics() {
    let original = AdmissionFixture::new().assess(criteria()).unwrap();
    let renamed = AdmissionFixture::with_existing_name("First")
        .assess(criteria())
        .unwrap();
    assert_eq!(original.taxonomy_revision, renamed.taxonomy_revision);
    assert_ne!(
        original.taxonomy_names_revision,
        renamed.taxonomy_names_revision
    );
    assert_eq!(
        original.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    assert_eq!(
        renamed.candidates[0].disposition,
        CategoryAdmissionDisposition::NameConflict
    );
    assert_eq!(
        original.candidates[0].members,
        renamed.candidates[0].members
    );
}

#[test]
fn invalid_or_stale_evidence_is_rejected_before_admission() {
    let mut fixture = AdmissionFixture::new();
    fixture.discovery.cohort_revision = revision(9);
    assert!(fixture.assess(criteria()).is_err());
    fixture.discovery.cohort_revision = fixture.cohort.revision();
    let original = fixture.discovery.candidates[0].member_similarities.clone();
    fixture.discovery.candidates[0].member_similarities.pop();
    assert!(fixture.assess(criteria()).is_err());
    fixture.discovery.candidates[0].member_similarities = original.clone();
    fixture.discovery.candidates[0].member_similarities[1] = original[0].clone();
    assert!(fixture.assess(criteria()).is_err());
    fixture.discovery.candidates[0].member_similarities = original;
    fixture.discovery.candidates[0].member_similarities[0].similarity = f32::NAN;
    assert!(fixture.assess(criteria()).is_err());
}

#[test]
fn weak_context_and_cancellation_do_not_publish_a_proposal() {
    let mut fixture = AdmissionFixture::new();
    let scores = &mut fixture.discovery.candidates[0].candidate.scores;
    scores.context_similarity = scores.similarity;
    scores.contrast = 0.0;
    let result = fixture.assess(criteria()).unwrap();
    assert_eq!(
        result.candidates[0].disposition,
        CategoryAdmissionDisposition::WeakContext
    );
    let error = assess_category_candidates(
        &fixture.cohort,
        &fixture.discovery,
        &fixture.matcher,
        &CategoryAdmissionPolicy::new(revision(8), criteria()).unwrap(),
        &AtomicBool::new(true),
    )
    .unwrap_err();
    assert_eq!(error.code, "classification_cancelled");
}

#[test]
fn policy_bounds_and_revisions_preserve_the_measured_parameters() {
    for invalid in [
        CategoryAdmissionCriteria {
            minimum_members: 1,
            ..criteria()
        },
        CategoryAdmissionCriteria {
            coverage_per_thousand: 0,
            ..criteria()
        },
        CategoryAdmissionCriteria {
            max_proposals: 65,
            ..criteria()
        },
        CategoryAdmissionCriteria {
            minimum_separation: f32::NAN,
            ..criteria()
        },
    ] {
        assert!(CategoryAdmissionPolicy::new(revision(8), invalid).is_err());
    }
    let original = CategoryAdmissionPolicy::new(revision(8), criteria()).unwrap();
    assert_ne!(
        original.revision(),
        CategoryAdmissionPolicy::new(revision(9), criteria())
            .unwrap()
            .revision()
    );
    assert_ne!(
        original.revision(),
        CategoryAdmissionPolicy::new(
            revision(8),
            CategoryAdmissionCriteria {
                max_proposals: 1,
                ..criteria()
            }
        )
        .unwrap()
        .revision()
    );
}

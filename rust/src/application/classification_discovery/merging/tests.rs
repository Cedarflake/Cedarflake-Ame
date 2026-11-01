use std::collections::{BTreeMap, BTreeSet};

use crate::domain::classification::discovery::{CategoryNamingPurpose, ConceptId};
use crate::domain::classification::maintenance::{
    CategoryCoverageMember, CategoryCoverageSnapshot, CategoryCoverageSupport, CategoryMergeBasis,
    CategoryRetirementSnapshot, ManagedCategoryOrigin, managed_category_identity,
};
use crate::domain::classification::work::ClassificationDispatchState;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScore, ClassificationContent, ClassificationRevision,
    ModelCompatibility,
};
use crate::domain::classification_catalog::ClassificationCatalogState;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

use super::*;

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn fixture() -> (CategoryMergeSnapshot, CategoryCoverageSummary, CategoryId) {
    let parent = CategoryDefinition::new(
        CategoryId::new("drawing".into()).unwrap(),
        "Drawing".into(),
        vec!["a drawing".into()],
    )
    .unwrap();
    let concept = ConceptId::from_bytes([5; 32]);
    let child = CategoryDefinition::new(
        managed_category_identity(concept, Some(parent.id())).unwrap(),
        "City drawing".into(),
        vec!["a drawing of a city".into()],
    )
    .unwrap();
    let taxonomy = Taxonomy::new(vec![parent.clone(), child.clone()]).unwrap();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id("run"), model, taxonomy.revision(), None).unwrap();
    let configuration = ClassificationConfigurationSnapshot {
        catalog: ClassificationCatalogState {
            revision: 4,
            taxonomy: Some(taxonomy),
            configuration: Some(run.configuration()),
            active_run_id: Some("run".into()),
        },
        dispatch: ClassificationDispatchState {
            revision: 3,
            epoch: 1,
            mode: ClassificationDispatchMode::Automatic,
            run_id: Some(id("run")),
            session_id: None,
        },
    };
    let origin = ManagedCategoryOrigin {
        category_id: child.id().clone(),
        concept_id: concept,
        purpose: Some(parent.id().clone()),
        operation_id: id("create"),
        admitted_label: child.label().into(),
        admitted_semantic_revision: child.semantic_revision(),
        protected: false,
    };
    let basis = CategoryMergeBasis {
        naming: CategoryNamingPurpose::new(
            &parent,
            "{subject} drawing".into(),
            "a drawing of {subject}".into(),
        )
        .unwrap(),
        minimum_members: 3,
        evidence: ClassificationRevision::from_bytes([9; 32]),
    };
    let snapshot = CategoryMergeSnapshot {
        categories: CategoryRetirementSnapshot {
            configuration: configuration.clone(),
            model: Some(model),
            origins: BTreeMap::from([(child.id().clone(), origin)]),
            human_categories: BTreeSet::new(),
        },
        bases: BTreeMap::from([(child.id().clone(), basis)]),
    };
    let members = (0..2)
        .map(|index| CategoryCoverageMember {
            content: ClassificationContent {
                scheme: id("exact"),
                digest: [index; 32],
                byte_length: 20,
            },
            alternative: Some(CategoryScore {
                category_id: parent.id().clone(),
                similarity: 0.6,
            }),
        })
        .collect();
    let coverage = CategoryCoverageSummary {
        snapshot: CategoryCoverageSnapshot {
            catalog_revision: 6,
            configuration,
            model,
            unverified_sources: false,
        },
        categories: BTreeMap::from([
            (
                parent.id().clone(),
                CategoryCoverageSupport {
                    contents: 10,
                    retained_members: Vec::new(),
                },
            ),
            (
                child.id().clone(),
                CategoryCoverageSupport {
                    contents: 2,
                    retained_members: members,
                },
            ),
        ]),
        classified_contents: 12,
        unclassified_contents: 0,
        evidence: ClassificationRevision::from_bytes([8; 32]),
    };
    (snapshot, coverage, child.id().clone())
}

#[test]
fn merge_requires_lost_recurrence_and_the_original_purpose_for_every_content() {
    let (snapshot, mut coverage, child) = fixture();
    let cancelled = AtomicBool::new(false);
    let candidate = select_purpose_category_merge(&snapshot, &coverage, &cancelled)
        .unwrap()
        .unwrap();
    assert_eq!(candidate.category_id, child);
    assert_eq!(candidate.retained_id.as_str(), "drawing");
    coverage.categories.get_mut(&child).unwrap().contents = 3;
    assert!(
        select_purpose_category_merge(&snapshot, &coverage, &cancelled)
            .unwrap()
            .is_none()
    );
    let support = coverage.categories.get_mut(&child).unwrap();
    support.contents = 2;
    support.retained_members[0]
        .alternative
        .as_mut()
        .unwrap()
        .category_id = child;
    assert!(
        select_purpose_category_merge(&snapshot, &coverage, &cancelled)
            .unwrap()
            .is_none()
    );
}

#[test]
fn incomplete_empty_truncated_and_tied_evidence_preserve_the_category() {
    for case in ["missing", "unverified", "empty", "truncated", "tie"] {
        let (snapshot, mut coverage, child) = fixture();
        match case {
            "missing" => coverage.unclassified_contents = 1,
            "unverified" => coverage.snapshot.unverified_sources = true,
            "empty" => coverage.classified_contents = 0,
            "truncated" => {
                coverage
                    .categories
                    .get_mut(&child)
                    .unwrap()
                    .retained_members
                    .pop();
            }
            "tie" => {
                coverage
                    .categories
                    .get_mut(&child)
                    .unwrap()
                    .retained_members[0]
                    .alternative = None
            }
            _ => unreachable!(),
        }
        assert!(
            select_purpose_category_merge(&snapshot, &coverage, &AtomicBool::new(false))
                .unwrap()
                .is_none(),
            "{case}"
        );
    }
    let (snapshot, mut coverage, child) = fixture();
    let support = coverage.categories.get_mut(&child).unwrap();
    support.contents = 0;
    support.retained_members.clear();
    coverage.classified_contents = 10;
    assert!(
        select_purpose_category_merge(&snapshot, &coverage, &AtomicBool::new(false))
            .unwrap()
            .is_some()
    );
}

#[test]
fn saved_intent_changed_meaning_and_missing_original_evidence_preserve_refinements() {
    for case in [
        "manual", "human", "name", "meaning", "basis", "parent", "child",
    ] {
        let (mut snapshot, coverage, child) = fixture();
        match case {
            "manual" => {
                snapshot
                    .categories
                    .origins
                    .get_mut(&child)
                    .unwrap()
                    .protected = true
            }
            "human" => {
                snapshot.categories.human_categories.insert(child.clone());
            }
            "name" => {
                snapshot
                    .categories
                    .origins
                    .get_mut(&child)
                    .unwrap()
                    .admitted_label = "old name".into()
            }
            "meaning" => {
                snapshot
                    .categories
                    .origins
                    .get_mut(&child)
                    .unwrap()
                    .admitted_semantic_revision = ClassificationRevision::from_bytes([0; 32])
            }
            "basis" => snapshot.bases.clear(),
            "parent" => {
                let changed = CategoryDefinition::new(
                    CategoryId::new("drawing".into()).unwrap(),
                    "Drawing".into(),
                    vec!["a different purpose".into()],
                )
                .unwrap();
                snapshot.bases.get_mut(&child).unwrap().naming = CategoryNamingPurpose::new(
                    &changed,
                    "{subject} drawing".into(),
                    "a drawing of {subject}".into(),
                )
                .unwrap();
            }
            "child" => {
                let mut dependent = snapshot.categories.origins[&child].clone();
                dependent.category_id = CategoryId::new("dependent".into()).unwrap();
                dependent.purpose = Some(child.clone());
                snapshot
                    .categories
                    .origins
                    .insert(dependent.category_id.clone(), dependent);
            }
            _ => unreachable!(),
        }
        assert!(
            select_purpose_category_merge(&snapshot, &coverage, &AtomicBool::new(false))
                .unwrap()
                .is_none(),
            "{case}"
        );
    }
}

#[test]
fn changed_authority_and_cancellation_reject_preparation() {
    let (snapshot, mut coverage, _) = fixture();
    assert_eq!(
        select_purpose_category_merge(&snapshot, &coverage, &AtomicBool::new(true))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    coverage.snapshot.configuration.catalog.revision += 1;
    assert_eq!(
        select_purpose_category_merge(&snapshot, &coverage, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_changed"
    );
}

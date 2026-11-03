use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::CategoryNamingPurpose;

use super::*;

#[path = "../../../../../test_support/classification_merge_fixture.rs"]
mod support;
use support::fixture;

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

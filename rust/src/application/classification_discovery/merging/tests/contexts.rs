use crate::application::classification_discovery::{NamingContext, NamingContextCatalog};
use crate::domain::classification::discovery::VocabularyConcept;

use super::*;

fn concept(index: u8, label: &str, subject: &str) -> VocabularyConcept {
    VocabularyConcept::new(
        ConceptId::from_bytes([index; 32]),
        label.into(),
        format!("an image of {subject}"),
    )
    .unwrap()
    .with_subject(subject.into())
    .unwrap()
}

fn contexts(relation: bool, cycle: bool) -> NamingContextCatalog {
    NamingContextCatalog::new(vec![
        NamingContext {
            concept: concept(5, "City", "a city"),
            cues: vec![],
            broader: if relation {
                vec![ConceptId::from_bytes([6; 32])]
            } else {
                vec![]
            },
        },
        NamingContext {
            concept: concept(6, "Urban scene", "an urban scene"),
            cues: vec![],
            broader: if cycle {
                vec![ConceptId::from_bytes([5; 32])]
            } else {
                vec![]
            },
        },
    ])
    .unwrap()
}

fn context_fixture() -> (
    CategoryMergeSnapshot,
    CategoryCoverageSummary,
    CategoryId,
    CategoryId,
) {
    let (mut snapshot, mut coverage, child) = fixture();
    let parent = snapshot.bases[&child].naming.category().clone();
    let concept = ConceptId::from_bytes([6; 32]);
    let target = CategoryDefinition::new(
        managed_category_identity(concept, Some(&parent)).unwrap(),
        "Urban scene drawing".into(),
        vec!["a drawing of an urban scene".into()],
    )
    .unwrap();
    let mut categories = snapshot
        .categories
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .unwrap()
        .categories()
        .to_vec();
    categories.push(target.clone());
    let taxonomy = Taxonomy::new(categories).unwrap();
    let configuration = &mut snapshot.categories.configuration;
    configuration.catalog.taxonomy = Some(taxonomy.clone());
    configuration.catalog.configuration = Some(
        ClassificationRun::new(
            id("run"),
            snapshot.categories.model.unwrap(),
            taxonomy.revision(),
            None,
        )
        .unwrap()
        .configuration(),
    );
    coverage.snapshot.configuration = configuration.clone();
    snapshot.categories.origins.insert(
        target.id().clone(),
        ManagedCategoryOrigin {
            category_id: target.id().clone(),
            concept_id: concept,
            purpose: Some(parent),
            operation_id: id("broader-created"),
            admitted_label: target.label().into(),
            admitted_semantic_revision: target.semantic_revision(),
            protected: false,
        },
    );
    snapshot
        .bases
        .insert(target.id().clone(), snapshot.bases[&child].clone());
    coverage.categories.insert(
        target.id().clone(),
        CategoryCoverageSupport {
            contents: 4,
            retained_members: vec![],
        },
    );
    coverage.classified_contents += 4;
    for member in &mut coverage
        .categories
        .get_mut(&child)
        .unwrap()
        .retained_members
    {
        member.alternative.as_mut().unwrap().category_id = target.id().clone();
    }
    (snapshot, coverage, child, target.id().clone())
}

#[test]
fn an_existing_broader_context_can_retain_the_whole_sparse_category() {
    let (snapshot, coverage, child, target) = context_fixture();
    let signal = AtomicBool::new(false);
    assert!(
        select_purpose_category_merge(&snapshot, &coverage, &signal)
            .unwrap()
            .is_none()
    );
    let merge = super::super::contexts::select_context_category_merge(
        &snapshot,
        &coverage,
        &contexts(true, false),
        &signal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(merge.category_id, child);
    assert_eq!(merge.retained_id, target);
    let proof = merge.context.as_ref().unwrap();
    assert_eq!(
        proof.path,
        vec![
            ConceptId::from_bytes([5; 32]),
            ConceptId::from_bytes([6; 32])
        ]
    );
    assert!(validate_category_merge(&snapshot, &coverage, &merge, &signal).unwrap());
}

#[test]
fn similar_names_and_cycles_do_not_establish_semantic_containment() {
    let (snapshot, coverage, _, _) = context_fixture();
    for catalog in [contexts(false, false), contexts(true, true)] {
        assert!(
            super::super::contexts::select_context_category_merge(
                &snapshot,
                &coverage,
                &catalog,
                &AtomicBool::new(false)
            )
            .unwrap()
            .is_none()
        );
    }
}

#[test]
fn missing_support_changed_meaning_and_human_intent_preserve_the_distinction() {
    for case in [
        "target-support",
        "source-support",
        "missing",
        "target-basis",
        "pattern",
        "human",
        "protected",
        "alternative",
        "meaning",
    ] {
        let (mut snapshot, mut coverage, child, target) = context_fixture();
        match case {
            "target-support" => coverage.categories.get_mut(&target).unwrap().contents = 2,
            "source-support" => coverage.categories.get_mut(&child).unwrap().contents = 3,
            "missing" => coverage.unclassified_contents = 1,
            "target-basis" => {
                snapshot.bases.remove(&target);
            }
            "pattern" => {
                let parent = snapshot
                    .categories
                    .configuration
                    .catalog
                    .taxonomy
                    .as_ref()
                    .unwrap()
                    .categories()
                    .iter()
                    .find(|category| category.id().as_str() == "drawing")
                    .unwrap();
                snapshot.bases.get_mut(&target).unwrap().naming = CategoryNamingPurpose::new(
                    parent,
                    "{subject} drawing".into(),
                    "a different purpose for {subject}".into(),
                )
                .unwrap();
            }
            "human" => {
                snapshot.categories.human_categories.insert(child.clone());
            }
            "protected" => {
                snapshot
                    .categories
                    .origins
                    .get_mut(&child)
                    .unwrap()
                    .protected = true
            }
            "alternative" => {
                coverage
                    .categories
                    .get_mut(&child)
                    .unwrap()
                    .retained_members[0]
                    .alternative = None
            }
            "meaning" => {
                let catalog = NamingContextCatalog::new(vec![
                    NamingContext {
                        concept: concept(5, "City", "a person"),
                        cues: vec![],
                        broader: vec![ConceptId::from_bytes([6; 32])],
                    },
                    NamingContext {
                        concept: concept(6, "Urban scene", "an urban scene"),
                        cues: vec![],
                        broader: vec![],
                    },
                ])
                .unwrap();
                assert!(
                    super::super::contexts::select_context_category_merge(
                        &snapshot,
                        &coverage,
                        &catalog,
                        &AtomicBool::new(false)
                    )
                    .unwrap()
                    .is_none()
                );
                continue;
            }
            _ => unreachable!(),
        }
        assert!(
            super::super::contexts::select_context_category_merge(
                &snapshot,
                &coverage,
                &contexts(true, false),
                &AtomicBool::new(false)
            )
            .unwrap()
            .is_none(),
            "{case}"
        );
    }
}

#[test]
fn broader_relation_traversal_is_bounded_and_cancellable() {
    let catalog = |length: u8, reverse: bool| {
        let mut nodes = (1..=length)
            .map(|index| NamingContext {
                concept: concept(index, "Scene", "a scene"),
                cues: vec![],
                broader: if index < length {
                    vec![ConceptId::from_bytes([index + 1; 32])]
                } else {
                    vec![]
                },
            })
            .collect::<Vec<_>>();
        if reverse {
            nodes.reverse();
        }
        NamingContextCatalog::new(nodes).unwrap()
    };
    let signal = AtomicBool::new(false);
    let from = ConceptId::from_bytes([1; 32]);
    let to = ConceptId::from_bytes([5; 32]);
    let first = catalog(5, false)
        .broader_merge_context(from, to, &signal)
        .unwrap()
        .unwrap();
    assert_eq!(
        Some(first),
        catalog(5, true)
            .broader_merge_context(from, to, &signal)
            .unwrap()
    );
    assert!(
        catalog(6, false)
            .broader_merge_context(from, ConceptId::from_bytes([6; 32]), &signal)
            .unwrap()
            .is_none()
    );
    let cycle = NamingContextCatalog::new(
        (1..=7)
            .map(|index| NamingContext {
                concept: concept(index, "Scene", "a scene"),
                cues: vec![],
                broader: vec![ConceptId::from_bytes(
                    [if index == 7 { 1 } else { index + 1 }; 32],
                )],
            })
            .collect(),
    )
    .unwrap();
    assert!(
        cycle
            .broader_merge_context(from, ConceptId::from_bytes([2; 32]), &signal)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        catalog(5, false)
            .broader_merge_context(from, to, &AtomicBool::new(true))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
}

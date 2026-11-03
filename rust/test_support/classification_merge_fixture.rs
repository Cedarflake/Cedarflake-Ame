use std::collections::{BTreeMap, BTreeSet};

use crate::domain::classification::discovery::{CategoryNamingPurpose, ConceptId};
use crate::domain::classification::maintenance::{
    CategoryCoverageMember, CategoryCoverageSnapshot, CategoryCoverageSummary,
    CategoryCoverageSupport, CategoryMergeBasis, CategoryMergeSnapshot, CategoryRetirementSnapshot,
    ManagedCategoryOrigin, managed_category_identity,
};
use crate::domain::classification::work::{
    ClassificationDispatchMode, ClassificationDispatchState,
};
use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScore, ClassificationContent, ClassificationRecordId,
    ClassificationRevision, ClassificationRun, ModelCompatibility, Taxonomy,
};
use crate::domain::classification_catalog::ClassificationCatalogState;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

pub(crate) fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

pub(crate) fn fixture() -> (CategoryMergeSnapshot, CategoryCoverageSummary, CategoryId) {
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

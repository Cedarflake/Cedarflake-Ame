use std::sync::Arc;

use crate::domain::classification::maintenance::{
    CategoryCoverageEntry, CategoryCoveragePage, CategoryCoverageSnapshot, CategoryExpansionPlan,
    CategoryExpansionSnapshot,
};
use crate::domain::classification::work::{
    ClassificationDispatchMode, ClassificationDispatchState,
};
use crate::domain::classification::{CategoryDefinition, CategoryId, ClassificationRun, Taxonomy};
use crate::domain::classification_catalog::ClassificationCatalogState;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::{
    CategoryCoverageRepository, CategoryExpansionRepository,
};

use super::*;

struct Catalog {
    snapshot: CategoryCoverageSnapshot,
    contents: Vec<ClassificationContent>,
    validations: Cell<usize>,
    fail_validation: usize,
    begins: Cell<usize>,
}

impl CategoryCoverageRepository for Catalog {
    fn begin_category_coverage(
        &self,
        _: &Arc<AtomicBool>,
    ) -> Result<CategoryCoverageSnapshot, ScanError> {
        self.begins.set(self.begins.get() + 1);
        Ok(self.snapshot.clone())
    }
    fn read_category_coverage(
        &self,
        _: &CategoryCoverageSnapshot,
        after: Option<&ClassificationContent>,
        limit: usize,
        _: &Arc<AtomicBool>,
    ) -> Result<CategoryCoveragePage, ScanError> {
        let remaining: Vec<_> = self
            .contents
            .iter()
            .filter(|content| after.is_none_or(|previous| content.digest > previous.digest))
            .collect();
        Ok(CategoryCoveragePage {
            entries: remaining
                .iter()
                .take(limit)
                .map(|content| CategoryCoverageEntry {
                    content: (*content).clone(),
                    prediction: None,
                })
                .collect(),
            has_more: remaining.len() > limit,
        })
    }
    fn validate_category_coverage(
        &self,
        _: &CategoryCoverageSnapshot,
        _: &Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        self.validations.set(self.validations.get() + 1);
        if self.validations.get() == self.fail_validation {
            Err(ScanError::new(
                "classification_changed",
                "Publication changed",
            ))
        } else {
            Ok(())
        }
    }
}

impl CategoryExpansionRepository for Catalog {
    fn load_category_expansion_snapshot(
        &self,
        _: &[CategoryId],
    ) -> Result<CategoryExpansionSnapshot, ScanError> {
        Ok(CategoryExpansionSnapshot {
            configuration: self.snapshot.configuration.clone(),
            model: Some(model()),
            known_categories: self
                .snapshot
                .configuration
                .catalog
                .taxonomy
                .as_ref()
                .unwrap()
                .categories()
                .iter()
                .map(|definition| (definition.id().clone(), definition.label().to_owned()))
                .collect(),
            managed_categories: BTreeMap::new(),
        })
    }
    fn apply_category_expansion(
        &mut self,
        _: &CategoryExpansionPlan,
        _: &Arc<AtomicBool>,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        panic!("Preparation must not publish a configuration")
    }
}

struct Harness {
    catalog: Catalog,
    calls: Rc<Calls>,
    residency: ClassificationEncoderResidency<Factory, Clock>,
    vocabulary: Vocabulary,
    cache: SqliteClassificationFeatureCache,
    cancelled: Arc<AtomicBool>,
    _directory: tempfile::TempDir,
}

impl Harness {
    fn new(cached: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let calls = Rc::new(Calls::default());
        calls.features.borrow_mut().extend([
            ("a photograph".into(), vec![0.6, 0.8]),
            ("other content".into(), vec![-1.0, 0.0]),
        ]);
        let definitions = [("photo", "a photograph"), ("other", "other content")];
        let taxonomy = Taxonomy::new(
            definitions
                .into_iter()
                .map(|(name, description)| {
                    CategoryDefinition::new(
                        CategoryId::new(name.into()).unwrap(),
                        name.into(),
                        vec![description.into()],
                    )
                    .unwrap()
                })
                .collect(),
        )
        .unwrap();
        let run = ClassificationRun::new(
            ClassificationRecordId::new("initial".into()).unwrap(),
            model(),
            taxonomy.revision(),
            None,
        )
        .unwrap();
        let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
        let contents = (1..=4)
            .map(|index| {
                let content = ClassificationContent {
                    scheme: ClassificationRecordId::new("fixture-v1".into()).unwrap(),
                    digest: [index; 32],
                    byte_length: 100,
                };
                if cached {
                    let coordinates = if index == 4 {
                        vec![0.0, 1.0]
                    } else {
                        vec![1.0, 0.0]
                    };
                    cache
                        .store_image_feature(
                            model(),
                            &content,
                            &ImageFeature {
                                identity: model().image_identity(),
                                embedding: Embedding::normalized(coordinates).unwrap(),
                            },
                        )
                        .unwrap();
                }
                content
            })
            .collect();
        Self {
            catalog: Catalog {
                snapshot: CategoryCoverageSnapshot {
                    catalog_revision: 1,
                    configuration: ClassificationConfigurationSnapshot {
                        catalog: ClassificationCatalogState {
                            revision: 1,
                            taxonomy: Some(taxonomy),
                            configuration: Some(run.configuration()),
                            active_run_id: Some(run.id().as_str().into()),
                        },
                        dispatch: ClassificationDispatchState {
                            revision: 1,
                            epoch: 1,
                            mode: ClassificationDispatchMode::Automatic,
                            run_id: Some(run.id().clone()),
                            session_id: None,
                        },
                    },
                    model: model(),
                    unverified_sources: true,
                },
                contents,
                validations: Cell::new(0),
                fail_validation: usize::MAX,
                begins: Cell::new(0),
            },
            calls: calls.clone(),
            residency: ClassificationEncoderResidency::new(
                Factory(calls),
                Clock,
                Duration::from_secs(60),
            ),
            vocabulary: vocabulary(),
            cache,
            cancelled: Arc::new(AtomicBool::new(false)),
            _directory: directory,
        }
    }

    fn prepare(&mut self, maximum_contents: usize) -> Result<PreparedDiscoveryWindow, ScanError> {
        let admission = CategoryAdmissionPolicy::new(
            revision(8),
            CategoryAdmissionCriteria {
                minimum_members: 3,
                coverage_per_thousand: 500,
                minimum_improvement: 0.01,
                minimum_separation: 0.01,
                minimum_context_contrast: 0.01,
                max_proposals: 4,
            },
        )?;
        let naming = [crate::domain::classification::discovery::CategoryNaming::Unconditioned];
        let policy = DiscoveryPreparationPolicy {
            maximum_contents,
            cohorts: DiscoveryCohortPolicy {
                minimum_pair_cosine: 0.75,
                minimum_members: 3,
                maximum_cohorts: 64,
            },
            retrieval: limits(),
            consolidation: ConceptConsolidationLimits {
                hops: 2,
                concepts: 16,
                edges: 32,
            },
            maximum_candidates: 16,
            minimum_context_relatedness: 0.9,
            admission: &admission,
            naming: &naming,
        };
        prepare_discovery_window(
            DiscoveryPreparationRequest {
                operation_id: ClassificationRecordId::new("prepared".into()).unwrap(),
                cursor: None,
                policy: &policy,
            },
            &self.catalog,
            &mut self.vocabulary,
            &mut self.residency,
            &mut self.cache,
            &self.cancelled,
        )
    }
}

#[test]
fn cached_content_produces_a_guarded_expansion_without_catalog_writes_or_image_inference() {
    let mut harness = Harness::new(true);
    let result = harness.prepare(512).unwrap();
    assert_eq!(result.examined_contents, 4);
    assert_eq!(result.missing_features, 0);
    assert_eq!(result.considered_cohorts, 1);
    assert_eq!(result.deferred_cohorts, 1);
    assert!(result.next.is_none());
    let plan = result.expansion.unwrap();
    assert_eq!(plan.additions().len(), 1);
    assert_eq!(plan.configuration().expected_catalog_revision, 1);
    assert_eq!(plan.configuration().expected_dispatch_revision, 1);
    let generated = plan
        .configuration()
        .setup
        .primary()
        .categories()
        .iter()
        .find(|category| plan.additions().contains(category.id()))
        .unwrap();
    assert_eq!(generated.label(), "a classroom");
    assert_eq!(harness.catalog.snapshot.configuration.catalog.revision, 1);
    let encoded = harness.calls.encoded.borrow().len();
    assert!(encoded > 0);
    let retry = harness.prepare(512).unwrap().expansion.unwrap();
    assert_eq!(retry.additions(), plan.additions());
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
}

#[test]
fn absent_image_features_never_start_an_encoder_or_survey_the_whole_catalog() {
    let mut harness = Harness::new(false);
    let result = harness.prepare(512).unwrap();
    assert_eq!(result.missing_features, 4);
    assert!(result.expansion.is_none());
    assert_eq!(harness.calls.starts.get(), 0);
    assert_eq!(harness.catalog.begins.get(), 1);
}

#[test]
fn cancelled_text_work_does_not_return_a_plan() {
    let mut harness = Harness::new(true);
    harness.calls.cancel_at.set(Some(2));
    assert_eq!(
        harness.prepare(512).err().unwrap().code,
        "classification_cancelled"
    );
    assert!(harness.cancelled.load(Ordering::Acquire));
    assert_eq!(harness.catalog.snapshot.configuration.catalog.revision, 1);
}

#[test]
fn publication_changes_after_assessment_reject_the_prepared_result() {
    let mut harness = Harness::new(true);
    harness.catalog.fail_validation = 3;
    assert_eq!(
        harness.prepare(512).err().unwrap().code,
        "classification_changed"
    );
    assert_eq!(harness.catalog.validations.get(), 3);
}

#[test]
fn an_invalid_policy_is_rejected_before_catalog_and_model_work() {
    let mut harness = Harness::new(true);
    assert_eq!(
        harness.prepare(513).err().unwrap().code,
        "classification_discovery_policy_invalid"
    );
    assert_eq!(harness.catalog.begins.get(), 0);
    assert_eq!(harness.calls.starts.get(), 0);
}

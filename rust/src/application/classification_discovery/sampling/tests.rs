use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use crate::domain::classification::maintenance::{CategoryCoverageEntry, CategoryCoveragePage};
use crate::domain::classification::work::ClassificationDispatchState;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, ClassificationRevision,
    ClassificationRun, Embedding, ImageFeature, ModelCompatibility, Taxonomy, TextFeature,
};
use crate::domain::classification_catalog::ClassificationCatalogState;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_features::ClassificationFeatureCacheStats;

use super::*;

#[derive(Clone, Copy)]
enum Fault {
    None,
    Duplicate,
    Oversized,
    EmptyMore,
    FinalChange,
}

struct Catalog {
    snapshot: CategoryCoverageSnapshot,
    contents: Vec<ClassificationContent>,
    limits: RefCell<Vec<usize>>,
    begins: Cell<usize>,
    fault: Fault,
}

struct Cache {
    reads: usize,
    missing: BTreeSet<usize>,
    wrong_dimensions: bool,
    cancel: Option<Arc<AtomicBool>>,
}

fn fixture(count: usize) -> (Catalog, Cache, Arc<AtomicBool>) {
    let revision = ClassificationRevision::from_bytes([1; 32]);
    let model = ModelCompatibility::new(revision, revision, revision, 2).unwrap();
    let taxonomy = Taxonomy::new(
        ["photo", "other"]
            .into_iter()
            .map(|name| {
                CategoryDefinition::new(
                    CategoryId::new(name.into()).unwrap(),
                    name.into(),
                    vec![name.into()],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let run = ClassificationRun::new(
        ClassificationRecordId::new("fixture-run".into()).unwrap(),
        model,
        taxonomy.revision(),
        None,
    )
    .unwrap();
    let contents = (0..count)
        .map(|index| {
            let mut digest = [0; 32];
            digest[..8].copy_from_slice(&(index as u64).to_be_bytes());
            ClassificationContent {
                scheme: ClassificationRecordId::new("fixture".into()).unwrap(),
                digest,
                byte_length: 100,
            }
        })
        .collect();
    (
        Catalog {
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
                model,
                unverified_sources: true,
            },
            contents,
            limits: RefCell::new(Vec::new()),
            begins: Cell::new(0),
            fault: Fault::None,
        },
        Cache {
            reads: 0,
            missing: BTreeSet::new(),
            wrong_dimensions: false,
            cancel: None,
        },
        Arc::new(AtomicBool::new(false)),
    )
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
        self.limits.borrow_mut().push(limit);
        let start = self
            .contents
            .partition_point(|content| after.is_some_and(|last| key(content) <= key(last)));
        let end = (start + limit).min(self.contents.len());
        let mut entries: Vec<_> = self.contents[start..end]
            .iter()
            .map(|content| CategoryCoverageEntry {
                content: content.clone(),
                prediction: None,
            })
            .collect();
        let mut has_more = end < self.contents.len();
        match self.fault {
            Fault::Duplicate => entries[1] = entries[0].clone(),
            Fault::Oversized => entries.push(entries[0].clone()),
            Fault::EmptyMore => {
                entries.clear();
                has_more = true;
            }
            Fault::None | Fault::FinalChange => {}
        }
        Ok(CategoryCoveragePage { entries, has_more })
    }

    fn validate_category_coverage(
        &self,
        _: &CategoryCoverageSnapshot,
        _: &Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        if matches!(self.fault, Fault::FinalChange) {
            Err(stale())
        } else {
            Ok(())
        }
    }
}

impl ClassificationFeatureRepository for Cache {
    fn load_image_feature(
        &mut self,
        model: ModelCompatibility,
        _: &ClassificationContent,
    ) -> Result<Option<ImageFeature>, ScanError> {
        self.reads += 1;
        if let Some(cancelled) = &self.cancel {
            cancelled.store(true, Ordering::Release);
        }
        if self.missing.contains(&self.reads) {
            return Ok(None);
        }
        let values = if self.wrong_dimensions {
            vec![1.0]
        } else {
            vec![1.0, 1.0]
        };
        Ok(Some(ImageFeature {
            identity: model.image_identity(),
            embedding: Embedding::normalized(values).unwrap(),
        }))
    }

    fn store_image_feature(
        &mut self,
        _: ModelCompatibility,
        _: &ClassificationContent,
        _: &ImageFeature,
    ) -> Result<(), ScanError> {
        panic!("Sampling must not write features")
    }
    fn load_text_feature(
        &mut self,
        _: ModelCompatibility,
        _: ClassificationRevision,
    ) -> Result<Option<TextFeature>, ScanError> {
        panic!("Sampling must not perform text analysis")
    }
    fn store_text_feature(
        &mut self,
        _: ModelCompatibility,
        _: &TextFeature,
    ) -> Result<(), ScanError> {
        panic!("Sampling must not write features")
    }
    fn feature_cache_stats(&self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        panic!("Sampling must not enumerate the cache")
    }
    fn clear_feature_cache_batch(&mut self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        panic!("Sampling must not remove cached work")
    }
}

#[test]
fn samples_use_bounded_current_content_windows_and_exact_continuation() {
    let (catalog, mut cache, cancelled) = fixture(130);
    cache.missing.insert(2);
    let first = read_discovery_samples(&catalog, &mut cache, None, 65, &cancelled).unwrap();
    assert_eq!(first.examined_contents, 65);
    assert_eq!(first.samples.len(), 64);
    assert_eq!(first.missing_features, 1);
    assert!(first.snapshot.unverified_sources);
    assert_eq!(*catalog.limits.borrow(), [64, 1]);
    let second =
        read_discovery_samples(&catalog, &mut cache, first.next.as_ref(), 65, &cancelled).unwrap();
    assert_eq!(second.samples.len(), 65);
    assert!(second.next.is_none());
    assert_eq!(second.samples[0].content, catalog.contents[65]);
    assert_eq!(second.samples[64].content, catalog.contents[129]);
    assert_eq!(cache.reads, 130);
}

#[test]
fn missing_features_still_consume_the_read_budget() {
    let (catalog, mut cache, cancelled) = fixture(600);
    cache.missing.extend(1..=512);
    let result = read_discovery_samples(&catalog, &mut cache, None, 512, &cancelled).unwrap();
    assert!(result.samples.is_empty());
    assert_eq!(result.missing_features, 512);
    assert_eq!(cache.reads, 512);
    assert_eq!(*catalog.limits.borrow(), [64; 8]);
    assert!(result.next.is_some());
}

#[test]
fn changed_publication_and_manual_pause_reject_continuation_before_cache_reads() {
    let (mut catalog, mut cache, cancelled) = fixture(5);
    let first = read_discovery_samples(&catalog, &mut cache, None, 2, &cancelled).unwrap();
    catalog.snapshot.catalog_revision += 1;
    assert_eq!(
        read_discovery_samples(&catalog, &mut cache, first.next.as_ref(), 2, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_samples_changed"
    );
    assert_eq!(cache.reads, 2);
    catalog.snapshot.configuration.dispatch.mode = ClassificationDispatchMode::Paused;
    assert_eq!(
        read_discovery_samples(&catalog, &mut cache, None, 2, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_samples_changed"
    );
    assert_eq!(cache.reads, 2);
}

#[test]
fn cancellation_and_final_revalidation_never_return_partial_samples() {
    let (mut catalog, mut cache, cancelled) = fixture(5);
    cache.cancel = Some(cancelled.clone());
    assert_eq!(
        read_discovery_samples(&catalog, &mut cache, None, 5, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_cancelled"
    );
    assert_eq!(cache.reads, 1);
    cancelled.store(false, Ordering::Release);
    cache.cancel = None;
    catalog.fault = Fault::FinalChange;
    assert_eq!(
        read_discovery_samples(&catalog, &mut cache, None, 5, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_samples_changed"
    );
}

#[test]
fn invalid_pages_and_incompatible_features_are_rejected() {
    for fault in [Fault::Duplicate, Fault::Oversized, Fault::EmptyMore] {
        let (mut catalog, mut cache, cancelled) = fixture(5);
        catalog.fault = fault;
        assert_eq!(
            read_discovery_samples(&catalog, &mut cache, None, 5, &cancelled)
                .err()
                .unwrap()
                .code,
            "classification_samples_invalid"
        );
    }
    let (catalog, mut cache, cancelled) = fixture(1);
    cache.wrong_dimensions = true;
    assert_eq!(
        read_discovery_samples(&catalog, &mut cache, None, 1, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_samples_invalid"
    );
}

#[test]
fn empty_windows_finish_and_invalid_limits_do_not_open_the_catalog() {
    let (catalog, mut cache, cancelled) = fixture(0);
    for limit in [0, 513] {
        assert!(read_discovery_samples(&catalog, &mut cache, None, limit, &cancelled).is_err());
    }
    assert_eq!(catalog.begins.get(), 0);
    let result = read_discovery_samples(&catalog, &mut cache, None, 64, &cancelled).unwrap();
    assert_eq!(result.examined_contents, 0);
    assert!(result.samples.is_empty());
    assert!(result.next.is_none());
    assert_eq!(cache.reads, 0);
}

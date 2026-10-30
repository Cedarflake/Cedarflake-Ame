use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::adapters::SqliteClassificationFeatureCache;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationContent, ClassificationRecordId, Embedding,
    ImageFeature, ModelCompatibility, TextFeature,
};
use crate::ports::classification_encoder::{ClassificationEncoder, ClassificationEncoderKind};

use super::*;

#[derive(Default)]
struct Calls {
    descriptions: RefCell<Vec<String>>,
    images: Cell<usize>,
    started: Cell<usize>,
    retired: Cell<usize>,
    cancel_at: Cell<Option<usize>>,
    wrong_description: Cell<bool>,
}

struct Factory(Rc<Calls>);
struct Encoder {
    calls: Rc<Calls>,
    retired: bool,
}
struct Clock;

impl ResidencyClock for Clock {
    fn elapsed(&self) -> Duration {
        Duration::ZERO
    }
}

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

impl ClassificationEncoderFactory for Factory {
    type Encoder = Encoder;
    fn compatibility(&self) -> ModelCompatibility {
        model()
    }
    fn create(&self, _kind: ClassificationEncoderKind) -> Result<Encoder, ScanError> {
        Ok(Encoder {
            calls: self.0.clone(),
            retired: false,
        })
    }
}

impl ClassificationEncoder for Encoder {
    fn initialize(&mut self, _cancelled: &AtomicBool) -> Result<(), ScanError> {
        self.calls.started.set(self.calls.started.get() + 1);
        Ok(())
    }
    fn encode_prepared_image(
        &mut self,
        _pixels: &[f32],
        _cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        self.calls.images.set(self.calls.images.get() + 1);
        Ok(image())
    }
    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        self.calls
            .descriptions
            .borrow_mut()
            .push(description.to_owned());
        if self.calls.cancel_at.get() == Some(self.calls.descriptions.borrow().len()) {
            cancelled.store(true, Ordering::Release);
        }
        let values = if description == "a photo" || description == "a photographed reaction toy" {
            vec![1.0, 0.0]
        } else {
            vec![0.0, 1.0]
        };
        Ok(TextFeature {
            identity: model().text_identity(),
            description: if self.calls.wrong_description.get() {
                revision(9)
            } else {
                ClassificationRevision::description(description)
            },
            embedding: Embedding::normalized(values).unwrap(),
        })
    }
    fn retire(&mut self) -> Result<(), ScanError> {
        if !self.retired {
            self.calls.retired.set(self.calls.retired.get() + 1);
        }
        self.retired = true;
        Ok(())
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

fn taxonomy(photo: &str, meme: &str) -> Taxonomy {
    Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("photo".into()).unwrap(),
            "Photo".into(),
            vec![photo.into()],
        )
        .unwrap(),
        CategoryDefinition::new(
            CategoryId::new("meme".into()).unwrap(),
            "Meme".into(),
            vec![meme.into()],
        )
        .unwrap(),
    ])
    .unwrap()
}

fn image() -> ImageFeature {
    ImageFeature {
        identity: model().image_identity(),
        embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
    }
}

#[test]
fn rule_changes_rematch_a_persisted_image_without_loading_the_image_encoder() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let content = ClassificationContent {
        scheme: ClassificationRecordId::new("content-v1".into()).unwrap(),
        digest: [8; 32],
        byte_length: 20,
    };
    cache
        .store_image_feature(model(), &content, &image())
        .unwrap();
    let original = taxonomy("a photo", "a reaction image");
    let matcher = prepare_classification_matcher(
        original.clone(),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        matcher
            .rank(
                &cache
                    .load_image_feature(model(), &content)
                    .unwrap()
                    .unwrap()
            )
            .unwrap()
            .leading()
            .category_id
            .as_str(),
        "photo"
    );
    assert_eq!(calls.descriptions.borrow().len(), 2);
    assert_eq!(calls.retired.get(), 1);
    assert_eq!(residency.resident_kind(), None);
    drop(cache);
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let renamed = Taxonomy::new(
        original
            .categories()
            .iter()
            .map(|category| {
                category
                    .renamed(format!("Renamed {}", category.label()))
                    .unwrap()
            })
            .collect(),
    )
    .unwrap();
    prepare_classification_matcher(renamed, &mut residency, &mut cache, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(calls.descriptions.borrow().len(), 2);
    let changed = taxonomy("an ordinary photo", "a photographed reaction toy");
    let matcher = prepare_classification_matcher(
        changed,
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    let retained = cache
        .load_image_feature(model(), &content)
        .unwrap()
        .unwrap();
    assert_eq!(
        matcher
            .rank(&retained)
            .unwrap()
            .leading()
            .category_id
            .as_str(),
        "meme"
    );
    assert_eq!(calls.descriptions.borrow().len(), 4);
    assert_eq!(calls.started.get(), 2);
    assert_eq!(calls.retired.get(), 2);
    assert_eq!(calls.images.get(), 0);
}

#[test]
fn interrupted_description_preparation_reuses_only_completed_features() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    calls.cancel_at.set(Some(2));
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(
        prepare_classification_matcher(
            taxonomy("a photo", "a reaction image"),
            &mut residency,
            &mut cache,
            &cancelled
        )
        .is_err()
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 1);
    assert_eq!(residency.resident_kind(), None);
    calls.cancel_at.set(None);
    cancelled.store(false, Ordering::Release);
    prepare_classification_matcher(
        taxonomy("a photo", "a reaction image"),
        &mut residency,
        &mut cache,
        &cancelled,
    )
    .unwrap();
    assert_eq!(calls.descriptions.borrow().len(), 3);
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 2);
    assert_eq!(calls.started.get(), 2);
    assert_eq!(calls.retired.get(), 2);
}

#[test]
fn rejected_description_retires_the_text_worker_without_caching_it() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    calls.wrong_description.set(true);
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    assert!(
        prepare_classification_matcher(
            taxonomy("a photo", "a reaction image"),
            &mut residency,
            &mut cache,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 0);
    assert_eq!(residency.resident_kind(), None);
    assert_eq!(calls.retired.get(), 1);
}

#[test]
fn cached_descriptions_leave_a_warm_image_worker_available() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    prepare_classification_matcher(
        taxonomy("a photo", "a reaction image"),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    residency
        .encode_prepared_image(&[], &AtomicBool::new(false))
        .unwrap();
    prepare_classification_matcher(
        taxonomy("a photo", "a reaction image"),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        residency.resident_kind(),
        Some(ClassificationEncoderKind::Image)
    );
    assert_eq!(calls.started.get(), 2);
    assert_eq!(calls.retired.get(), 1);
    residency.retire().unwrap();
}

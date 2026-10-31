use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use crate::adapters::SqliteClassificationFeatureCache;
use crate::domain::classification::discovery::{ConceptId, DiscoverySample, VocabularyConcept};
use crate::domain::classification::{
    ClassificationContent, ClassificationRecordId, Embedding, ImageFeature, ModelCompatibility,
    TextFeature,
};
use crate::ports::classification_encoder::{ClassificationEncoder, ClassificationEncoderKind};

use super::*;

mod consolidation;

#[derive(Default)]
struct Calls {
    encoded: RefCell<Vec<String>>,
    starts: Cell<usize>,
    retires: Cell<usize>,
    cancel_at: Cell<Option<usize>>,
    wrong_feature: Cell<bool>,
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

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}
fn id(value: u8) -> ConceptId {
    ConceptId::from_bytes([value; 32])
}

impl ClassificationEncoderFactory for Factory {
    type Encoder = Encoder;
    fn compatibility(&self) -> ModelCompatibility {
        model()
    }
    fn create(&self, kind: ClassificationEncoderKind) -> Result<Encoder, ScanError> {
        assert_eq!(kind, ClassificationEncoderKind::Text);
        Ok(Encoder {
            calls: self.0.clone(),
            retired: false,
        })
    }
}

impl ClassificationEncoder for Encoder {
    fn initialize(&mut self, _cancelled: &AtomicBool) -> Result<(), ScanError> {
        self.calls.starts.set(self.calls.starts.get() + 1);
        Ok(())
    }
    fn encode_prepared_image(
        &mut self,
        _pixels: &[f32],
        _cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        panic!("concept retrieval must not read or encode an image")
    }
    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        self.calls.encoded.borrow_mut().push(description.to_owned());
        if self.calls.cancel_at.get() == Some(self.calls.encoded.borrow().len()) {
            cancelled.store(true, Ordering::Release);
        }
        let values = match description {
            "a classroom" => vec![1.0, 0.0],
            "a workshop" => vec![0.6, 0.8],
            _ => vec![0.0, 1.0],
        };
        Ok(TextFeature {
            identity: model().text_identity(),
            description: if self.calls.wrong_feature.get() {
                revision(9)
            } else {
                ClassificationRevision::description(description)
            },
            embedding: Embedding::normalized(values).unwrap(),
        })
    }
    fn retire(&mut self) -> Result<(), ScanError> {
        if !self.retired {
            self.calls.retires.set(self.calls.retires.get() + 1);
        }
        self.retired = true;
        Ok(())
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

struct Vocabulary {
    concepts: BTreeMap<ConceptId, VocabularyConcept>,
    entries: Vec<ConceptId>,
    links: BTreeMap<ConceptId, Vec<ConceptId>>,
    revision: ClassificationRevision,
    changed_on_read: bool,
    invalid_page: bool,
    requests: Vec<(ConceptNeighborhood, usize)>,
}

impl ClassificationVocabulary for Vocabulary {
    fn revision(&self) -> ClassificationRevision {
        self.revision
    }
    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> Result<ConceptPage, ScanError> {
        self.requests.push((neighborhood, limit));
        if self.changed_on_read {
            self.revision = revision(9);
        }
        if self.invalid_page {
            return Ok(ConceptPage {
                concepts: vec![],
                has_more: true,
            });
        }
        let ids = match neighborhood {
            ConceptNeighborhood::EntryPoints => &self.entries,
            ConceptNeighborhood::Related(id) | ConceptNeighborhood::Broader(id) => {
                self.links.get(&id).map(Vec::as_slice).unwrap_or(&[])
            }
        };
        let remaining: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| after.is_none_or(|after| *id > after))
            .collect();
        Ok(ConceptPage {
            concepts: remaining
                .iter()
                .take(limit)
                .map(|id| self.concepts[id].clone())
                .collect(),
            has_more: remaining.len() > limit,
        })
    }
}

fn vocabulary() -> Vocabulary {
    let concepts = [(1, "a workshop"), (2, "a room"), (3, "a classroom")]
        .into_iter()
        .map(|(value, text)| {
            let concept = VocabularyConcept::new(id(value), text.into(), text.into()).unwrap();
            (concept.id(), concept)
        })
        .collect();
    Vocabulary {
        concepts,
        entries: vec![id(1), id(2)],
        links: BTreeMap::from([
            (id(1), vec![id(2)]),
            (id(2), vec![id(1), id(3)]),
            (id(3), vec![id(2)]),
        ]),
        revision: revision(4),
        changed_on_read: false,
        invalid_page: false,
        requests: vec![],
    }
}

fn cohort() -> DiscoveryCohort {
    let sample = |value, coordinates| DiscoverySample {
        content: ClassificationContent {
            scheme: ClassificationRecordId::new("fixture-v1".into()).unwrap(),
            digest: [value; 32],
            byte_length: 100,
        },
        feature: ImageFeature {
            identity: model().image_identity(),
            embedding: Embedding::normalized(coordinates).unwrap(),
        },
    };
    DiscoveryCohort::new(
        model(),
        &[sample(1, vec![1.0, 0.0])],
        &[sample(2, vec![0.0, 1.0])],
    )
    .unwrap()
}

fn limits() -> ConceptSearchLimits {
    ConceptSearchLimits {
        entry_points: 16,
        expanded_concepts: 16,
        edges: 32,
        hops: 2,
        seeds: 1,
        results: 3,
    }
}

#[test]
fn expansion_reaches_a_leaf_through_an_already_ranked_concept_and_reuses_its_features() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let mut vocabulary = vocabulary();
    let result = retrieve_classification_concepts(
        &cohort(),
        &mut vocabulary,
        limits(),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.nearest[0].concept.id(), id(3));
    assert_eq!(result.distinctive[0].concept.id(), id(3));
    assert_eq!(result.scored_concepts, 3);
    assert_eq!(result.visited_edges, 3);
    assert!(result.bounds_reached.is_empty());
    assert_eq!(
        &*calls.encoded.borrow(),
        &["a workshop", "a room", "a classroom"]
    );
    assert_eq!(calls.retires.get(), 1);
    assert_eq!(residency.resident_kind(), None);
    retrieve_classification_concepts(
        &cohort(),
        &mut vocabulary,
        limits(),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(calls.encoded.borrow().len(), 3);
    assert_eq!(calls.starts.get(), 1);
}

#[test]
fn cycles_and_duplicate_edges_stop_at_the_explicit_work_budget() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let mut vocabulary = vocabulary();
    let result = retrieve_classification_concepts(
        &cohort(),
        &mut vocabulary,
        ConceptSearchLimits {
            edges: 1,
            ..limits()
        },
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.visited_edges, 1);
    assert_eq!(result.scored_concepts, 2);
    assert_eq!(result.bounds_reached, vec![ConceptSearchBound::Edges]);
    assert!(vocabulary.requests.iter().all(|(_, size)| *size <= 64));
}

#[test]
fn large_vocabulary_is_paged_and_equal_scores_keep_stable_bounded_results() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let mut vocabulary = vocabulary();
    vocabulary.entries = (1..=130).map(id).collect();
    vocabulary.concepts = (1..=130)
        .map(|value| {
            (
                id(value),
                VocabularyConcept::new(id(value), format!("Concept {value}"), "a classroom".into())
                    .unwrap(),
            )
        })
        .collect();
    let result = retrieve_classification_concepts(
        &cohort(),
        &mut vocabulary,
        ConceptSearchLimits {
            entry_points: 65,
            hops: 0,
            results: 2,
            ..limits()
        },
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.scored_concepts, 65);
    assert_eq!(
        result
            .nearest
            .iter()
            .map(|item| item.concept.id())
            .collect::<Vec<_>>(),
        vec![id(1), id(2)]
    );
    assert_eq!(result.distinctive.len(), 2);
    assert_eq!(result.bounds_reached, vec![ConceptSearchBound::EntryPoints]);
    assert_eq!(
        vocabulary.requests,
        vec![
            (ConceptNeighborhood::EntryPoints, 64),
            (ConceptNeighborhood::EntryPoints, 1)
        ]
    );
    assert_eq!(calls.encoded.borrow().len(), 1);
}

#[test]
fn cancellation_keeps_only_completed_cache_entries_and_retires_the_worker() {
    let directory = tempfile::tempdir().unwrap();
    let calls = Rc::new(Calls::default());
    calls.cancel_at.set(Some(3));
    let mut residency =
        ClassificationEncoderResidency::new(Factory(calls.clone()), Clock, Duration::from_secs(60));
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let mut vocabulary = vocabulary();
    let cancelled = AtomicBool::new(false);
    assert!(
        retrieve_classification_concepts(
            &cohort(),
            &mut vocabulary,
            limits(),
            &mut residency,
            &mut cache,
            &cancelled
        )
        .is_err()
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 2);
    assert_eq!(residency.resident_kind(), None);
    calls.cancel_at.set(None);
    cancelled.store(false, Ordering::Release);
    retrieve_classification_concepts(
        &cohort(),
        &mut vocabulary,
        limits(),
        &mut residency,
        &mut cache,
        &cancelled,
    )
    .unwrap();
    assert_eq!(calls.encoded.borrow().len(), 4);
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 3);
}

#[test]
fn changed_vocabulary_invalid_windows_and_wrong_features_cannot_publish_candidates() {
    for failure in 0..3 {
        let directory = tempfile::tempdir().unwrap();
        let calls = Rc::new(Calls::default());
        let mut residency = ClassificationEncoderResidency::new(
            Factory(calls.clone()),
            Clock,
            Duration::from_secs(60),
        );
        let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
        let mut vocabulary = vocabulary();
        match failure {
            0 => vocabulary.changed_on_read = true,
            1 => vocabulary.invalid_page = true,
            _ => calls.wrong_feature.set(true),
        }
        assert!(
            retrieve_classification_concepts(
                &cohort(),
                &mut vocabulary,
                limits(),
                &mut residency,
                &mut cache,
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert_eq!(cache.feature_cache_stats().unwrap().entries, 0);
        assert_eq!(residency.resident_kind(), None);
    }
}

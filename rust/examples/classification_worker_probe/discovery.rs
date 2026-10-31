use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use rust_lib_cedarflake_ame::classification::{
    ClassificationContent, ClassificationEncoderFactory, ClassificationEncoderResidency,
    ClassificationFeatureRepository, ClassificationRecordId, ClassificationRevision,
    ClassificationVocabulary, ConceptConsolidationLimits, ConceptConsolidationRequest, ConceptId,
    ConceptMatch, ConceptNeighborhood, ConceptPage, ConceptSearchLimits, DiscoveryCohort,
    DiscoverySample, Embedding, ImageFeature, MonotonicResidencyClock,
    SqliteClassificationFeatureCache, VocabularyConcept, consolidate_classification_concepts,
    retrieve_classification_concepts,
};
use rust_lib_cedarflake_ame::classification_worker::LocalClassificationEncoderFactory;
use serde::Deserialize;
use serde_json::json;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[path = "discovery/cost.rs"]
mod cost;

#[derive(Deserialize)]
struct Fixture {
    revision: [u8; 32],
    concepts: Vec<Concept>,
    entries: Vec<String>,
    samples: Vec<Sample>,
    cohorts: Vec<Cohort>,
    #[serde(default)]
    frontier: Vec<String>,
    #[serde(default)]
    results: Option<usize>,
}

#[derive(Deserialize)]
struct Concept {
    id: String,
    label: String,
    description: String,
    related: Vec<String>,
    #[serde(default)]
    broader: Vec<String>,
}

#[derive(Deserialize)]
struct Sample {
    id: String,
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct Cohort {
    label: String,
    members: Vec<String>,
    comparison: Vec<String>,
}

struct ProbeVocabulary {
    revision: ClassificationRevision,
    concepts: BTreeMap<ConceptId, VocabularyConcept>,
    related: BTreeMap<ConceptId, Vec<ConceptId>>,
    broader: BTreeMap<ConceptId, Vec<ConceptId>>,
    entries: Vec<ConceptId>,
}

fn concept_id(value: &str) -> ConceptId {
    ConceptId::from_bytes(blake3::derive_key(
        "ame.classification.discovery-probe-id.v1",
        value.as_bytes(),
    ))
}

impl ProbeVocabulary {
    fn new(revision: [u8; 32], concepts: Vec<Concept>, entries: Vec<String>) -> Result<Self> {
        if concepts.len() > 100_000 || entries.len() > 4096 {
            return Err("Vocabulary fixture exceeds the diagnostic limit".into());
        }
        let mut definitions = BTreeMap::new();
        let mut related = BTreeMap::new();
        let mut broader = BTreeMap::new();
        for concept in concepts {
            if concept.related.len() > 16_384
                || concept.broader.len() > 16_384
                || concept.id.len() > 128
            {
                return Err("Concept fixture exceeds the diagnostic limit".into());
            }
            let id = concept_id(&concept.id);
            let neighbors: BTreeSet<_> = concept
                .related
                .iter()
                .map(|value| concept_id(value))
                .collect();
            related.insert(id, neighbors.into_iter().collect());
            let parents: BTreeSet<_> = concept
                .broader
                .iter()
                .map(|value| concept_id(value))
                .collect();
            broader.insert(id, parents.into_iter().collect());
            let definition = VocabularyConcept::new(id, concept.label, concept.description)?;
            if definitions.insert(id, definition).is_some() {
                return Err("Repeated concept fixture identity".into());
            }
        }
        let entries: BTreeSet<_> = entries.iter().map(|value| concept_id(value)).collect();
        if entries
            .iter()
            .chain(related.values().flatten())
            .chain(broader.values().flatten())
            .any(|id| !definitions.contains_key(id))
        {
            return Err("Vocabulary fixture references an unknown concept".into());
        }
        Ok(Self {
            revision: ClassificationRevision::from_bytes(revision),
            concepts: definitions,
            related,
            broader,
            entries: entries.into_iter().collect(),
        })
    }
}

impl ClassificationVocabulary for ProbeVocabulary {
    fn revision(&self) -> ClassificationRevision {
        self.revision
    }

    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> std::result::Result<ConceptPage, rust_lib_cedarflake_ame::classification::ScanError> {
        let ids = match neighborhood {
            ConceptNeighborhood::EntryPoints => self.entries.as_slice(),
            ConceptNeighborhood::Related(id) => {
                self.related.get(&id).map(Vec::as_slice).unwrap_or(&[])
            }
            ConceptNeighborhood::Broader(id) => {
                self.broader.get(&id).map(Vec::as_slice).unwrap_or(&[])
            }
        };
        let start = after
            .map(|after| ids.partition_point(|id| *id <= after))
            .unwrap_or(0);
        let end = start.saturating_add(limit).min(ids.len());
        Ok(ConceptPage {
            concepts: ids[start..end]
                .iter()
                .map(|id| self.concepts[id].clone())
                .collect(),
            has_more: end < ids.len(),
        })
    }
}

pub fn run(arguments: &[OsString]) -> Result<()> {
    let fixture_path = Path::new(&arguments[3]);
    let mut bytes = Vec::new();
    fs::File::open(fixture_path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Discovery fixture exceeds the input bound".into());
    }
    let fixture: Fixture = serde_json::from_slice(&bytes)?;
    drop(bytes);
    if fixture.samples.len() > 512 || fixture.cohorts.is_empty() || fixture.cohorts.len() > 16 {
        return Err("Discovery fixture exceeds the sample bound".into());
    }
    let factory = LocalClassificationEncoderFactory {
        executable: arguments[0].clone().into(),
        model_directory: arguments[1].clone().into(),
        runtime_directory: arguments[2].clone().into(),
    };
    let model = factory.compatibility();
    let mut samples = BTreeMap::new();
    for sample in fixture.samples {
        if sample.id.len() > 128
            || sample.values.len() != model.dimensions()
            || samples.insert(sample.id, sample.values).is_some()
        {
            return Err("Invalid discovery sample identity or dimensions".into());
        }
    }
    let mut vocabulary = ProbeVocabulary::new(fixture.revision, fixture.concepts, fixture.entries)?;
    let frontier: BTreeSet<_> = fixture
        .frontier
        .iter()
        .map(|value| concept_id(value))
        .collect();
    let mut cache = SqliteClassificationFeatureCache::open(Path::new(&arguments[4]))?;
    let mut residency = ClassificationEncoderResidency::new(
        factory,
        MonotonicResidencyClock::default(),
        Duration::from_secs(60),
    );
    if arguments[5] == "--discovery-cost" {
        return cost::measure(&vocabulary, &mut residency, &mut cache);
    }
    let limits = ConceptSearchLimits {
        entry_points: 4096,
        expanded_concepts: 4096,
        edges: 65_536,
        hops: 2,
        seeds: 16,
        results: fixture
            .results
            .unwrap_or(if frontier.is_empty() { 12 } else { 64 }),
    };
    for cohort in fixture.cohorts {
        let load = |identities: &[String]| -> Result<Vec<DiscoverySample>> {
            identities
                .iter()
                .map(|identity| {
                    let values = samples.get(identity).ok_or("Missing discovery sample")?;
                    Ok(DiscoverySample {
                        content: ClassificationContent {
                            scheme: ClassificationRecordId::new("discovery-fixture-v1".into())?,
                            digest: blake3::derive_key(
                                "ame.classification.discovery-probe-sample.v1",
                                identity.as_bytes(),
                            ),
                            byte_length: (values.len() * 4) as u64,
                        },
                        feature: ImageFeature {
                            identity: model.image_identity(),
                            embedding: Embedding::normalized(values.clone())?,
                        },
                    })
                })
                .collect()
        };
        let member_features = load(&cohort.members)?;
        let member_ids: BTreeMap<_, _> = member_features
            .iter()
            .zip(&cohort.members)
            .map(|(sample, id)| (sample.content.digest, id))
            .collect();
        let cohort_features =
            DiscoveryCohort::new(model, &member_features, &load(&cohort.comparison)?)?;
        drop(member_features);
        for round in 0..2 {
            let started = Instant::now();
            let before = cache.feature_cache_stats()?;
            let result = retrieve_classification_concepts(
                &cohort_features,
                &mut vocabulary,
                limits,
                &mut residency,
                &mut cache,
                &AtomicBool::new(false),
            )?;
            let retrieval_seconds = started.elapsed().as_secs_f64();
            let naming = if frontier.is_empty() {
                None
            } else {
                let result = consolidate_classification_concepts(
                    ConceptConsolidationRequest {
                        cohort: &cohort_features,
                        retrieved: &result,
                        frontier: &frontier,
                        limits: ConceptConsolidationLimits {
                            hops: 4,
                            concepts: 4096,
                            edges: 65_536,
                        },
                    },
                    &mut vocabulary,
                    &mut residency,
                    &mut cache,
                    &AtomicBool::new(false),
                )?;
                Some(json!({
                    "candidates": result.candidates.iter().map(|candidate| json!({
                        "label": candidate.candidate.concept.label(),
                        "description": candidate.candidate.concept.description(),
                        "similarity": candidate.candidate.scores.similarity,
                        "contrast": candidate.candidate.scores.contrast,
                        "narrower_concepts": candidate.narrower_concepts.len(),
                        "members": candidate.member_similarities.iter().map(|member| json!({
                            "id": member_ids[&member.content.digest],
                            "similarity": member.similarity,
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "unresolved_concepts": result.unresolved_concepts.len(),
                    "visited_edges": result.visited_edges,
                }))
            };
            let after = cache.feature_cache_stats()?;
            if residency.resident_kind().is_some() {
                return Err("Discovery left a model resident".into());
            }
            println!(
                "{}",
                json!({
                    "cohort": cohort.label, "round": round, "seconds": started.elapsed().as_secs_f64(),
                    "retrieval_seconds": retrieval_seconds,
                    "scored_concepts": result.scored_concepts, "visited_edges": result.visited_edges,
                    "bounds_reached": format!("{:?}", result.bounds_reached),
                    "cache_entries_before": before.entries, "cache_entries_after": after.entries,
                    "nearest": report(&result.nearest), "distinctive": report(&result.distinctive),
                    "naming": naming,
                    "worker_retired": true, "source_reads": 0,
                })
            );
        }
    }
    Ok(())
}

fn report(candidates: &[ConceptMatch]) -> Vec<serde_json::Value> {
    candidates
        .iter()
        .map(|candidate| {
            json!({
                "label": candidate.concept.label(), "description": candidate.concept.description(),
                "similarity": candidate.scores.similarity, "contrast": candidate.scores.contrast,
            })
        })
        .collect()
}

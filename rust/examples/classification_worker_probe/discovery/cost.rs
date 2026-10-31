use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rust_lib_cedarflake_ame::classification::{
    ClassificationEncoderFactory, ClassificationEncoderResidency, ClassificationFeatureRepository,
    ClassificationRevision, ResidencyClock,
};
use serde_json::json;

use super::{ProbeVocabulary, Result};

pub(super) fn measure<F: ClassificationEncoderFactory, C: ResidencyClock>(
    vocabulary: &ProbeVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
) -> Result<()> {
    let result = measure_features(vocabulary, residency, cache);
    residency.finish_descriptions()?;
    result?;
    if residency.resident_kind().is_some() {
        return Err("Cost measurement left a model resident".into());
    }
    println!("{}", json!({"stage": "finished", "worker_retired": true}));
    Ok(())
}

fn measure_features<F: ClassificationEncoderFactory, C: ResidencyClock>(
    vocabulary: &ProbeVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
) -> Result<()> {
    let model = residency.compatibility();
    let cancelled = AtomicBool::new(false);
    let mut measured = BTreeSet::new();
    let mut missing = Vec::new();
    let mut hits = 0;
    for id in vocabulary
        .entries
        .iter()
        .take(96)
        .chain(vocabulary.concepts.keys().take(128))
    {
        let description = vocabulary.concepts[id].description();
        let revision = ClassificationRevision::description(description);
        if !measured.insert(revision) {
            continue;
        }
        let started = Instant::now();
        let feature = cache.load_text_feature(model, revision)?;
        let load_ms = started.elapsed().as_secs_f64() * 1000.0;
        if feature.is_some() {
            hits += 1;
        } else if missing.len() < 32 {
            missing.push(description);
        }
        println!(
            "{}",
            json!({"stage": "cache_lookup", "hit": feature.is_some(), "milliseconds": load_ms}),
        );
    }
    if hits == 0 || missing.is_empty() {
        return Err("Cost measurement requires retained and missing descriptions".into());
    }
    let encoded_count = missing.len();
    for (index, description) in missing.into_iter().enumerate() {
        let started = Instant::now();
        let feature = residency.encode_description(description, &cancelled)?;
        let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        cache.store_text_feature(model, &feature)?;
        let store_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let stored = cache
            .load_text_feature(model, feature.description)?
            .ok_or("Stored feature was not found")?;
        let reload_ms = started.elapsed().as_secs_f64() * 1000.0;
        if stored != feature {
            return Err("Cache round trip changed the feature".into());
        }
        println!(
            "{}",
            json!({
                "stage": "missing_description", "index": index,
                "encode_ms": encode_ms, "store_ms": store_ms, "reload_ms": reload_ms,
                "includes_initialization": index == 0,
            }),
        );
    }
    println!(
        "{}",
        json!({"stage": "counts", "cache_hits": hits, "encoded": encoded_count, "source_reads": 0}),
    );
    Ok(())
}

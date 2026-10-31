use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationRevision, Taxonomy, TextFeature};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::{
    ClassificationFeatureRepository, TEXT_FEATURE_WINDOW, validate_text_feature_window,
};

use super::classification_matching::CategoryMatcher;
use super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};

pub fn prepare_classification_matcher<F: ClassificationEncoderFactory, C: ResidencyClock>(
    taxonomy: Taxonomy,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<CategoryMatcher, ScanError> {
    let result = prepare(taxonomy, residency, cache, cancelled);
    residency.finish_descriptions()?;
    result
}

fn prepare<F: ClassificationEncoderFactory, C: ResidencyClock>(
    taxonomy: Taxonomy,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<CategoryMatcher, ScanError> {
    let model = residency.compatibility();
    let descriptions: BTreeMap<_, _> = taxonomy
        .categories()
        .iter()
        .flat_map(|category| category.descriptions())
        .map(|description| {
            (
                ClassificationRevision::description(description),
                description,
            )
        })
        .collect();
    let descriptions: Vec<_> = descriptions.into_values().map(String::as_str).collect();
    let mut features = Vec::with_capacity(descriptions.len());
    for window in descriptions.chunks(TEXT_FEATURE_WINDOW) {
        features.extend(
            load_or_encode_descriptions(window, residency, cache, cancelled)?.into_values(),
        );
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    CategoryMatcher::new(taxonomy, model, features)
        .map_err(|error| ScanError::new("classification_feature_mismatch", error.to_string()))
}

pub(crate) fn load_or_encode_descriptions<F: ClassificationEncoderFactory, C: ResidencyClock>(
    descriptions: &[&str],
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<BTreeMap<ClassificationRevision, TextFeature>, ScanError> {
    validate_text_feature_window(descriptions.len())?;
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    let model = residency.compatibility();
    let mut seen = BTreeSet::new();
    let unique: Vec<_> = descriptions
        .iter()
        .map(|description| {
            (
                ClassificationRevision::description(description),
                *description,
            )
        })
        .filter(|(revision, _)| seen.insert(*revision))
        .collect();
    let revisions: Vec<_> = unique.iter().map(|(revision, _)| *revision).collect();
    let cached = cache.load_text_features(model, &revisions)?;
    if cached.len() != revisions.len() {
        return Err(ScanError::new(
            "classification_feature_mismatch",
            "The cache returned an incomplete description window",
        ));
    }
    let mut features = BTreeMap::new();
    for ((revision, description), cached) in unique.into_iter().zip(cached) {
        if cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "classification_cancelled",
                "Classification was cancelled",
            ));
        }
        let needs_storage = cached.is_none();
        let feature = match cached {
            Some(feature) => feature,
            None => residency.encode_description(description, cancelled)?,
        };
        if cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "classification_cancelled",
                "Classification was cancelled",
            ));
        }
        if feature.description != revision
            || feature.identity != model.text_identity()
            || feature.embedding.values().len() != model.dimensions()
        {
            return Err(ScanError::new(
                "classification_feature_mismatch",
                "The description feature does not match the requested model and text",
            ));
        }
        if needs_storage {
            cache.store_text_feature(model, &feature)?;
        }
        features.insert(revision, feature);
    }
    Ok(features)
}

#[cfg(test)]
mod tests;

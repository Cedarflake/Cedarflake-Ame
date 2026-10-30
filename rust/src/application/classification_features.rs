use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationRevision, Taxonomy};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;

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
    let mut features = Vec::with_capacity(descriptions.len());
    for (revision, description) in descriptions {
        if cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "classification_cancelled",
                "Classification was cancelled",
            ));
        }
        let feature = match cache.load_text_feature(model, revision)? {
            Some(feature) => feature,
            None => {
                let feature = residency.encode_description(description, cancelled)?;
                if feature.description != revision {
                    return Err(ScanError::new(
                        "classification_feature_mismatch",
                        "The encoded description changed",
                    ));
                }
                cache.store_text_feature(model, &feature)?;
                feature
            }
        };
        features.push(feature);
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

#[cfg(test)]
mod tests;

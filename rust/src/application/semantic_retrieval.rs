use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::image_features::{AnalysisRevision, Embedding, ModelCompatibility};
use crate::domain::organization::OrganizationSelection;
use crate::domain::semantic_results::{SEMANTIC_CANDIDATE_WINDOW, SemanticResultManifest};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::semantic_catalog::SemanticCatalog;
use crate::ports::semantic_features::SemanticFeatureRepository;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) enum SemanticRelevance {
    Description,
    Reference,
}

impl SemanticRelevance {
    fn accepts(self, score: f32) -> bool {
        let minimum = match self {
            Self::Description => 0.09_f32,
            Self::Reference => 0.65_f32,
        };
        // Scope selects candidates, not their relevance threshold. A stronger match
        // in another folder must not disqualify already matching images.
        score >= minimum
    }
}

pub(crate) struct SemanticRetrievalQuery<'a> {
    pub scope: &'a GalleryQuery,
    pub model: ModelCompatibility,
    pub embedding: &'a Embedding,
    pub identity: AnalysisRevision,
    pub exclude: Option<&'a OrganizationSelection>,
    pub relevance: SemanticRelevance,
}

pub(crate) fn retrieve(
    catalog: &impl SemanticCatalog,
    features: &impl SemanticFeatureRepository,
    query: SemanticRetrievalQuery<'_>,
    cancelled: &AtomicBool,
) -> Result<SemanticResultManifest, ScanError> {
    if query.scope.semantic_results.is_some()
        || !query.scope.search_text.is_empty()
        || query.embedding.values().len() != query.model.dimensions()
    {
        return Err(ScanError::new(
            "semantic_query_invalid",
            "Search requires an unranked library scope",
        ));
    }
    let storage = features.storage()?;
    let snapshot = catalog.semantic_snapshot()?;
    let mut eligible_items = 0;
    let mut indexed_items = 0;
    let mut ranked = Vec::with_capacity(SEMANTIC_CANDIDATE_WINDOW);
    check_cancelled(cancelled)?;
    let revision = snapshot.scan(query.scope, &mut |window| {
        check_cancelled(cancelled)?;
        let candidates: Vec<_> = window
            .into_iter()
            .filter(|candidate| query.exclude != Some(&candidate.source))
            .collect();
        eligible_items += candidates.len() as u64;
        let (sources, contents): (Vec<_>, Vec<_>) = candidates
            .into_iter()
            .filter_map(|candidate| candidate.content.map(|content| (candidate.source, content)))
            .unzip();
        let vectors = features.load_images(query.model, &contents, storage.generation)?;
        if vectors.len() != sources.len() {
            return Err(ScanError::new(
                "semantic_features_invalid",
                "The feature window is incomplete",
            ));
        }
        for (source, feature) in sources.into_iter().zip(vectors) {
            check_cancelled(cancelled)?;
            if let Some(feature) = feature {
                if feature.identity != query.model.image_identity() {
                    return Err(ScanError::new(
                        "semantic_features_invalid",
                        "The feature model changed",
                    ));
                }
                let score = query
                    .embedding
                    .cosine(&feature.embedding)
                    .map_err(|error| {
                        ScanError::new("semantic_features_invalid", error.to_string())
                    })?;
                indexed_items += 1;
                if query.relevance.accepts(score) {
                    ranked.push((score, source));
                }
            }
        }
        Ok(())
    })?;
    check_cancelled(cancelled)?;
    ranked.sort_unstable_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let items: Vec<_> = ranked.into_iter().map(|(_, source)| source).collect();
    let mut identity = blake3::Hasher::new_derive_key("ame.semantic.manifest.v3");
    identity.update(query.identity.bytes());
    identity.update(super::load_catalog::gallery_query_identity(query.scope).as_bytes());
    identity.update(query.model.image_identity().embedding_space.bytes());
    identity.update(query.model.image_identity().pipeline.bytes());
    identity.update(&(query.model.dimensions() as u64).to_le_bytes());
    identity.update(&revision.to_le_bytes());
    identity.update(&storage.generation.to_le_bytes());
    for item in &items {
        identity.update(&(item.location_id.len() as u64).to_le_bytes());
        identity.update(item.location_id.as_bytes());
        identity.update(&item.source_generation.to_le_bytes());
    }
    let manifest = SemanticResultManifest {
        id: identity.finalize().to_hex().to_string(),
        catalog_revision: revision,
        feature_generation: storage.generation,
        eligible_items,
        indexed_items,
        storage_full: storage.budget_reached,
        items,
    };
    manifest.validate()?;
    Ok(manifest)
}

pub(crate) fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ScanError::new("semantic_cancelled", "Search was stopped"))
    } else {
        Ok(())
    }
}

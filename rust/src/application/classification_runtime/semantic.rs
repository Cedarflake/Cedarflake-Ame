use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::image_features::AnalysisRevision;
use crate::domain::organization::OrganizationSelection;
use crate::domain::semantic_results::{SemanticResultManifest, SemanticSearchInput};
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::image_feature_encoding::ImageFeatureEncoding;
use crate::ports::semantic_catalog::SemanticCatalog;

use super::super::semantic_image::{PreparedSemanticImage, SemanticVerification};
use super::super::semantic_requests::SearchRequest;
use super::super::semantic_retrieval::{SemanticRelevance, SemanticRetrievalQuery, retrieve};
use super::executor::NativeExecutor;

#[derive(Default)]
pub(super) struct SemanticExecution {
    verification: SemanticVerification,
}

impl SemanticExecution {
    pub(super) fn finish(
        &mut self,
        catalog: &mut impl ExactDuplicateRepository,
    ) -> Result<(), ScanError> {
        self.verification.finish(catalog)
    }

    pub(super) fn search(
        &mut self,
        catalog: &mut (impl SemanticCatalog + ExactDuplicateRepository),
        executor: &mut NativeExecutor,
        request: &SearchRequest,
    ) -> Result<SemanticResultManifest, ScanError> {
        let mut reference = None;
        let (embedding, identity, exclude) = match &request.input {
            SemanticSearchInput::Text { description } => {
                let resources = executor.feature_resources()?;
                let model = resources.residency.compatibility();
                let description = description.trim();
                let identity = AnalysisRevision::description(description);
                let text = match resources.cache.load_text_feature(model, identity)? {
                    Some(feature) => feature,
                    None => {
                        let feature = resources
                            .residency
                            .encode_description(description, &request.cancelled)?;
                        resources.cache.store_text_feature(model, &feature)?;
                        feature
                    }
                };
                (text.embedding, identity, None)
            }
            SemanticSearchInput::Image { source } => {
                let prepared = self.image(catalog, executor, source, &request.cancelled)?;
                let embedding = prepared.feature.embedding.clone();
                reference = Some(prepared);
                let identity = AnalysisRevision::derive(
                    "ame.semantic.reference.v1",
                    [
                        source.asset_id.as_bytes(),
                        source.location_id.as_bytes(),
                        &source.source_generation.to_le_bytes(),
                    ],
                );
                (embedding, identity, Some(source))
            }
        };
        let resources = executor.feature_resources()?;
        let result = retrieve(
            catalog,
            resources.cache.semantic(),
            SemanticRetrievalQuery {
                scope: &request.scope,
                model: resources.residency.compatibility(),
                embedding: &embedding,
                identity,
                exclude,
                relevance: match request.input {
                    SemanticSearchInput::Text { .. } => SemanticRelevance::Description,
                    SemanticSearchInput::Image { .. } => SemanticRelevance::Reference,
                },
            },
            &request.cancelled,
        )?;
        if let Some(reference) = reference {
            reference.revalidate(catalog)?;
        }
        Ok(result)
    }

    pub(super) fn image(
        &mut self,
        catalog: &mut (impl SemanticCatalog + ExactDuplicateRepository),
        executor: &mut NativeExecutor,
        source: &OrganizationSelection,
        cancelled: &AtomicBool,
    ) -> Result<PreparedSemanticImage, ScanError> {
        let Some(_admission) = super::super::background_analysis::shared().acquire(cancelled)?
        else {
            return Err(ScanError::new(
                "semantic_cancelled",
                "Image preparation was stopped",
            ));
        };
        let resources = executor.feature_resources()?;
        let (fingerprint_reader, image_reader) = super::production::semantic_readers();
        super::super::semantic_image::SemanticImagePreparation {
            catalog,
            fingerprint_reader: &fingerprint_reader,
            image_reader: &image_reader,
            residency: resources.residency,
            features: resources.cache,
            verification: &mut self.verification,
        }
        .prepare(source, cancelled)
    }
}

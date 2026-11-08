use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationContent, ClassificationRecordId};
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::domain::image_features::ImageFeature;
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_analysis::{ClassificationImageLease, ClassificationImageReader};
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::exact_duplicates::{ExactContentReader, ExactDuplicateRepository};
use crate::ports::image_feature_encoding::ImageFeatureEncoding;
use crate::ports::semantic_catalog::SemanticCatalog;

use super::semantic_retrieval::check_cancelled;

mod verification;
pub(crate) use verification::SemanticVerification;

pub(crate) struct PreparedSemanticImage {
    pub feature: ImageFeature,
    source: OrganizationSelection,
    candidate: FingerprintCandidate,
    lease: Box<dyn ClassificationImageLease>,
}

impl PreparedSemanticImage {
    pub(crate) fn revalidate(&self, catalog: &impl SemanticCatalog) -> Result<(), ScanError> {
        self.lease.revalidate()?;
        let current = catalog.semantic_source(&self.source)?.ok_or_else(changed)?;
        let previous = &self.candidate;
        if current.root.root_id != previous.root.root_id
            || current.root.root_path != previous.root.root_path
            || current.root.root_generation != previous.root.root_generation
            || current.root.active_scan_id != previous.root.active_scan_id
            || current.root.publication_root_identity != previous.root.publication_root_identity
            || current.location.asset_id != previous.location.asset_id
            || current.location.location_id != previous.location.location_id
            || current.location.root_id != previous.location.root_id
            || current.location.scan_id != previous.location.scan_id
            || current.location.absolute_path != previous.location.absolute_path
            || current.location.relative_path != previous.location.relative_path
            || current.location.file_size != previous.location.file_size
            || current.location.modified_unix_ms != previous.location.modified_unix_ms
            || current.location.file_identity != previous.location.file_identity
            || current.location.source_revision != previous.location.source_revision
            || current.location.source_generation != previous.location.source_generation
        {
            return Err(changed());
        }
        Ok(())
    }
}

pub(crate) struct SemanticImagePreparation<'a, R, H, I, E: ImageFeatureEncoding, S> {
    pub catalog: &'a mut R,
    pub fingerprint_reader: &'a H,
    pub image_reader: &'a I,
    pub residency: &'a mut E,
    pub features: &'a mut S,
    pub verification: &'a mut SemanticVerification,
}

impl<
    R: SemanticCatalog + ExactDuplicateRepository,
    H: ExactContentReader,
    I: ClassificationImageReader,
    E: ImageFeatureEncoding,
    S: ClassificationFeatureRepository,
> SemanticImagePreparation<'_, R, H, I, E, S>
where
    I::Lease: 'static,
{
    pub(crate) fn prepare(
        &mut self,
        source: &OrganizationSelection,
        cancelled: &AtomicBool,
    ) -> Result<PreparedSemanticImage, ScanError> {
        check_cancelled(cancelled)?;
        let candidate = self.catalog.semantic_source(source)?.ok_or_else(changed)?;
        let content = match self.catalog.semantic_content(source)? {
            Some(content) => content,
            None => self.verification.read(
                self.catalog,
                self.fingerprint_reader,
                &candidate,
                cancelled,
            )?,
        };
        let content = ClassificationContent {
            scheme: ClassificationRecordId::new(content.scheme().to_owned())
                .map_err(|_| changed())?,
            digest: *content.digest(),
            byte_length: content.byte_length(),
        };
        let mut lease = self.image_reader.open(&candidate, &content, cancelled)?;
        let model = self.residency.compatibility();
        let feature = match self.features.load_image_feature(model, &content)? {
            Some(feature) => feature,
            None => {
                let pixels = lease.prepare_pixels(cancelled)?;
                self.residency.encode_prepared_image(&pixels, cancelled)?
            }
        };
        check_cancelled(cancelled)?;
        let prepared = PreparedSemanticImage {
            feature,
            source: source.clone(),
            candidate,
            lease: Box::new(lease),
        };
        prepared.revalidate(self.catalog)?;
        self.features
            .store_image_feature(model, &content, &prepared.feature)?;
        if !self.catalog.publish_semantic_content(source)? {
            return Err(changed());
        }
        Ok(prepared)
    }
}

fn changed() -> ScanError {
    ScanError::new(
        "semantic_source_changed",
        "The reference image changed or is unavailable",
    )
}

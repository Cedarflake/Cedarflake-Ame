use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::{ImageFeature, ModelCompatibility, TextFeature};
use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};

use super::super::background_analysis::{self, BackgroundAnalysisLease};

pub(super) struct AdmittedTextFactory<F>(pub F);

impl<F: ClassificationEncoderFactory> ClassificationEncoderFactory for AdmittedTextFactory<F> {
    type Encoder = AdmittedTextEncoder<F::Encoder>;

    fn compatibility(&self) -> ModelCompatibility {
        self.0.compatibility()
    }

    fn create(&self, kind: ClassificationEncoderKind) -> Result<Self::Encoder, ScanError> {
        Ok(AdmittedTextEncoder {
            inner: self.0.create(kind)?,
            kind,
        })
    }
}

pub(super) struct AdmittedTextEncoder<E> {
    inner: E,
    kind: ClassificationEncoderKind,
}

impl<E: ClassificationEncoder> ClassificationEncoder for AdmittedTextEncoder<E> {
    fn initialize(&mut self, cancelled: &AtomicBool) -> Result<(), ScanError> {
        let _admission = if self.kind == ClassificationEncoderKind::Text {
            Some(admit(cancelled)?)
        } else {
            None
        };
        self.inner.initialize(cancelled)
    }

    fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        self.inner.encode_prepared_image(pixels, cancelled)
    }

    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        let _admission = admit(cancelled)?;
        self.inner.encode_description(description, cancelled)
    }

    fn retire(&mut self) -> Result<(), ScanError> {
        self.inner.retire()
    }
    fn is_retired(&self) -> bool {
        self.inner.is_retired()
    }
}

fn admit(cancelled: &AtomicBool) -> Result<BackgroundAnalysisLease<'static>, ScanError> {
    background_analysis::shared()
        .acquire(cancelled)?
        .ok_or_else(|| ScanError::new("classification_cancelled", "Classification was stopped"))
}

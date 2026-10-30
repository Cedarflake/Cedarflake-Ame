use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::{ImageFeature, ModelCompatibility, TextFeature};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassificationEncoderKind {
    Image,
    Text,
}

pub trait ClassificationEncoder {
    /// Initializes resources after the caller has taken ownership of this encoder.
    fn initialize(&mut self, cancelled: &AtomicBool) -> Result<(), ScanError>;

    fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError>;

    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError>;

    fn retire(&mut self) -> Result<(), ScanError>;
    fn is_retired(&self) -> bool;
}

pub trait ClassificationEncoderFactory {
    type Encoder: ClassificationEncoder;

    fn compatibility(&self) -> ModelCompatibility;
    /// Creates an uninitialized owner without starting a process or loading a model.
    fn create(&self, kind: ClassificationEncoderKind) -> Result<Self::Encoder, ScanError>;
}

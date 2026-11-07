use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::image_features::{ImageFeature, ModelCompatibility, TextFeature};

pub trait ImageFeatureEncoding {
    fn compatibility(&self) -> ModelCompatibility;
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
    fn finish_descriptions(&mut self) -> Result<(), ScanError>;
}

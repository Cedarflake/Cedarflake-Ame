use super::ScanError;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraMetadata {
    pub make: Option<String>,
    pub model: Option<String>,
}

pub(crate) const MAX_CAMERA_FIELD_BYTES: usize = 128;

impl CameraMetadata {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        for value in [&self.make, &self.model].into_iter().flatten() {
            if value.is_empty()
                || value.len() > MAX_CAMERA_FIELD_BYTES
                || value.trim() != value
                || value.chars().any(char::is_control)
            {
                return Err(ScanError::new(
                    "camera_metadata_invalid",
                    "Camera metadata must contain bounded, normalized text",
                ));
            }
        }
        Ok(())
    }
}

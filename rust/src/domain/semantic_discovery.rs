use super::ScanError;
use super::image_features::{AnalysisRevision, ModelCompatibility, image_feature_key};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageContentIdentity {
    scheme: String,
    digest: [u8; 32],
    byte_length: u64,
}

impl ImageContentIdentity {
    pub fn new(scheme: String, digest: [u8; 32], byte_length: u64) -> Result<Self, ScanError> {
        if scheme.is_empty()
            || scheme.len() > 128
            || !scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || byte_length == 0
            || byte_length > i64::MAX as u64
        {
            return Err(ScanError::new(
                "semantic_content_invalid",
                "Image features require a versioned content identity",
            ));
        }
        Ok(Self {
            scheme,
            digest,
            byte_length,
        })
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub(crate) fn feature_key(&self, model: ModelCompatibility) -> AnalysisRevision {
        image_feature_key(model, &self.scheme, &self.digest, self.byte_length)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeaturePublication {
    Stored,
    AlreadyPresent,
    BudgetReached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticFeatureStorage {
    pub generation: u64,
    pub features: u64,
    pub accounted_bytes: u64,
    pub budget_bytes: u64,
    pub budget_reached: bool,
}

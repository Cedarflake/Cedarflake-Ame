use super::{ScanError, stable_id};

pub(crate) mod analysis;
pub(crate) use analysis::*;
pub(crate) mod matching;
pub(crate) mod review;

pub(crate) const SIMILARITY_SCHEME: &str = "ame-perceptual-gradient64-256-v1";
pub(crate) const SIMILARITY_MAX_DISTANCE: u8 = 10;
pub(crate) const SIMILARITY_PARAMETERS: &str = "normalization=ame-visual-sample-v1;gradient=8x8+16x16;confirmation=44;resample=lanczos3;transforms=d4;decoder=image-0.25.10";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimilarContentKey {
    pub scheme: String,
    pub digest: String,
    pub byte_length: u64,
}

impl SimilarContentKey {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        if !valid_scheme(&self.scheme)
            || self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.byte_length == 0
            || self.byte_length > super::exact_duplicates::MAX_FINGERPRINT_SOURCE_BYTES
        {
            return Err(invalid_identity());
        }
        Ok(())
    }
}

pub(crate) fn group_id(scheme: &str, reference: &SimilarContentKey) -> Result<String, ScanError> {
    reference.validate()?;
    if !valid_scheme(scheme) {
        return Err(invalid_identity());
    }
    Ok(stable_id(
        "similarity-group-v1",
        &format!(
            "{scheme}\0{}\0{}\0{}",
            reference.scheme, reference.digest, reference.byte_length
        ),
    ))
}

fn valid_scheme(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn invalid_identity() -> ScanError {
    ScanError::new(
        "similarity_identity_invalid",
        "Similarity requires canonical versioned content identities",
    )
}

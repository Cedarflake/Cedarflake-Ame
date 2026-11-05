use std::sync::atomic::AtomicBool;

use image::GrayImage;
use image_hasher::{HashAlg, Hasher, HasherConfig};

use crate::domain::similar_images::{SimilarEvidence, SimilarVariant};

use super::input::{InputFailure, check_cancelled};
use super::pixels;

pub(super) fn analyze(
    sample: &GrayImage,
    cancelled: &AtomicBool,
) -> Result<SimilarEvidence, InputFailure> {
    let candidates = pixels::hashes(sample, &gradient_hasher(8), cancelled)?;
    if !pixels::has_structure(&candidates) {
        return Ok(SimilarEvidence::InsufficientStructure);
    }
    let confirmations = pixels::hashes(sample, &gradient_hasher(16), cancelled)?;
    let mut variants = Vec::with_capacity(8);
    for (candidate, confirmation) in candidates.into_iter().zip(confirmations) {
        let bytes: [u8; 8] = candidate.try_into().map_err(|_| InputFailure::Decode)?;
        let variant = SimilarVariant {
            candidate: std::array::from_fn(|index| {
                u16::from_le_bytes([bytes[index * 2], bytes[index * 2 + 1]])
            }),
            confirmation: confirmation.try_into().map_err(|_| InputFailure::Decode)?,
        };
        if !variants.contains(&variant) {
            variants.push(variant);
        }
    }
    check_cancelled(cancelled)?;
    Ok(SimilarEvidence::Ready { variants })
}

fn gradient_hasher(edge: u32) -> Hasher {
    HasherConfig::new()
        .hash_alg(HashAlg::Gradient)
        .hash_size(edge, edge)
        .resize_filter(image::imageops::FilterType::Lanczos3)
        .to_hasher()
}

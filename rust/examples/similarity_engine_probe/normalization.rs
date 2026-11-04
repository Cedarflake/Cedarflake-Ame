use std::sync::atomic::AtomicBool;

use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, Pixel, Rgb};
use image_hasher::Hasher;

use super::input::{InputFailure, check_cancelled};

const SAMPLE_EDGE: u32 = 256;

pub(super) fn prepare(
    image: DynamicImage,
    cancelled: &AtomicBool,
) -> Result<GrayImage, InputFailure> {
    check_cancelled(cancelled)?;
    let pixels = image.into_rgba8();
    let mut luminance = GrayImage::new(pixels.width(), pixels.height());
    for (index, (pixel, sample)) in pixels.pixels().zip(luminance.pixels_mut()).enumerate() {
        if index.is_multiple_of(4096) {
            check_cancelled(cancelled)?;
        }
        let alpha = u16::from(pixel[3]);
        let color = Rgb(std::array::from_fn(|channel| {
            ((u16::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
        }));
        *sample = color.to_luma();
    }
    drop(pixels);
    check_cancelled(cancelled)?;
    let sample = DynamicImage::ImageLuma8(luminance)
        .resize(SAMPLE_EDGE, SAMPLE_EDGE, FilterType::Triangle)
        .into_luma8();
    check_cancelled(cancelled)?;
    Ok(sample)
}

pub(super) fn hashes(
    sample: &GrayImage,
    hasher: &Hasher,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<u8>>, InputFailure> {
    let mut result = Vec::with_capacity(8);
    for mirrored in [false, true] {
        let mut rotated = if mirrored {
            image::imageops::flip_horizontal(sample)
        } else {
            sample.clone()
        };
        for quarter_turn in 0..4 {
            check_cancelled(cancelled)?;
            result.push(hasher.hash_image(&rotated).as_bytes().to_vec());
            if quarter_turn < 3 {
                rotated = image::imageops::rotate90(&rotated);
            }
        }
    }
    check_cancelled(cancelled)?;
    Ok(result)
}

pub(super) fn has_structure(hashes: &[Vec<u8>]) -> bool {
    hashes.iter().any(|hash| {
        let ones: u32 = hash.iter().map(|byte| byte.count_ones()).sum();
        ones > 0 && ones < hash.len() as u32 * 8
    })
}

#[test]
fn transparent_colors_and_transforms_use_one_bounded_visual_sample() {
    use image::{Luma, Rgba, RgbaImage};
    use image_hasher::HasherConfig;

    let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(512, 256, |x, _| {
        if x < 256 {
            Rgba([255, 0, 0, 0])
        } else {
            Rgba([0, 0, 0, 255])
        }
    }));
    let sample = prepare(image, &AtomicBool::new(false)).unwrap();
    assert_eq!(sample.dimensions(), (256, 128));
    assert_eq!(*sample.get_pixel(8, 8), Luma([255]));
    assert_eq!(*sample.get_pixel(240, 8), Luma([0]));
    let hasher = HasherConfig::new().hash_size(8, 8).to_hasher();
    let normal = hashes(&sample, &hasher, &AtomicBool::new(false)).unwrap();
    let rotated = hashes(
        &image::imageops::rotate90(&sample),
        &hasher,
        &AtomicBool::new(false),
    )
    .unwrap();
    let normal: std::collections::BTreeSet<_> = normal.into_iter().collect();
    let rotated: std::collections::BTreeSet<_> = rotated.into_iter().collect();
    assert_eq!(normal, rotated);
    let flat = GrayImage::from_pixel(256, 128, Luma([255]));
    assert!(!has_structure(
        &hashes(&flat, &hasher, &AtomicBool::new(false)).unwrap()
    ));
    assert!(has_structure(
        &hashes(&sample, &hasher, &AtomicBool::new(false)).unwrap()
    ));
    assert_eq!(
        hashes(&sample, &hasher, &AtomicBool::new(true)),
        Err(InputFailure::Cancelled)
    );
}

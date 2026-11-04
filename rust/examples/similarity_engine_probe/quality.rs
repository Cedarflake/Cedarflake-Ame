use std::collections::BTreeMap;
use std::time::Instant;

use image::DynamicImage;
use image::imageops::FilterType;
use image_hasher::{HashAlg, Hasher, HasherConfig};

use super::fixtures::{scene, transformed};

pub(super) fn distance(left: &[u8], right: &[u8]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum()
}

fn hashes(image: &DynamicImage, hasher: &Hasher) -> Vec<Vec<u8>> {
    let mirrored = image.fliph();
    [
        image.clone(),
        image.rotate90(),
        image.rotate180(),
        image.rotate270(),
        mirrored.clone(),
        mirrored.rotate90(),
        mirrored.rotate180(),
        mirrored.rotate270(),
    ]
    .iter()
    .map(|image| hasher.hash_image(image).as_bytes().to_vec())
    .collect()
}

pub(crate) fn compare_hashes() {
    let originals: Vec<_> = (1..=24).map(scene).collect();
    for (label, algorithm, edge, dct) in [
        ("gradient8", HashAlg::Gradient, 8, false),
        ("gradient16", HashAlg::Gradient, 16, false),
        ("mean_dct16", HashAlg::Mean, 16, true),
    ] {
        let config = HasherConfig::new()
            .hash_size(edge, edge)
            .hash_alg(algorithm)
            .resize_filter(FilterType::Lanczos3);
        let hasher = if dct { config.preproc_dct() } else { config }.to_hasher();
        let start = Instant::now();
        let base: Vec<_> = originals
            .iter()
            .map(|image| hasher.hash_image(image).as_bytes().to_vec())
            .collect();
        let mut distances = BTreeMap::<&str, Vec<u32>>::new();
        for (index, original) in originals.iter().enumerate() {
            for (kind, changed) in transformed(original) {
                let value = hashes(&changed, &hasher)
                    .iter()
                    .map(|hash| distance(hash, &base[index]))
                    .min()
                    .expect("eight transforms");
                distances.entry(kind).or_default().push(value);
            }
            for other in originals.iter().skip(index + 1) {
                let value = hashes(other, &hasher)
                    .iter()
                    .map(|hash| distance(hash, &base[index]))
                    .min()
                    .expect("eight transforms");
                distances.entry("unrelated").or_default().push(value);
            }
        }
        println!("quality {label} elapsed_ms={}", start.elapsed().as_millis());
        for (kind, values) in &mut distances {
            values.sort();
            let admitted: Vec<_> = [8, 16, 24]
                .map(|threshold| values.iter().filter(|&&value| value <= threshold).count())
                .into();
            println!(
                "  {kind} n={} min={} median={} max={} accepted_at_8_16_24={admitted:?}",
                values.len(),
                values[0],
                values[values.len() / 2],
                values[values.len() - 1]
            );
        }
    }
}

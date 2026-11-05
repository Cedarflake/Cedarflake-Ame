use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use image::imageops::FilterType;
use image_hasher::{HashAlg, HasherConfig};

use super::fixtures::{scene, transformed};

pub(super) fn distance(left: &[u8], right: &[u8]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum()
}

pub(crate) fn compare_hashes() {
    let originals: Vec<_> = (1..=24)
        .map(|seed| {
            let mut encoded = Cursor::new(Vec::new());
            scene(seed)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .expect("generated scene encodes");
            super::input::decode(&mut encoded, &AtomicBool::new(false))
                .expect("bounded borrowed input decodes")
        })
        .collect();
    let cancelled = AtomicBool::new(false);
    let samples: Vec<_> = originals
        .iter()
        .map(|image| super::normalization::prepare(image.clone(), &cancelled).unwrap())
        .collect();
    for (label, algorithm, edge, dct, thresholds) in [
        ("gradient8", HashAlg::Gradient, 8, false, [2, 6, 10]),
        ("gradient16", HashAlg::Gradient, 16, false, [8, 16, 24]),
        ("mean_dct16", HashAlg::Mean, 16, true, [8, 16, 24]),
    ] {
        let config = HasherConfig::new()
            .hash_size(edge, edge)
            .hash_alg(algorithm)
            .resize_filter(FilterType::Lanczos3);
        let hasher = if dct { config.preproc_dct() } else { config }.to_hasher();
        let start = Instant::now();
        let base: Vec<_> = samples
            .iter()
            .map(|image| super::normalization::hashes(image, &hasher, &cancelled).unwrap())
            .collect();
        let mut distances = BTreeMap::<&str, Vec<u32>>::new();
        for (index, original) in originals.iter().enumerate() {
            for (kind, changed) in transformed(original) {
                let changed = super::normalization::prepare(changed, &cancelled).unwrap();
                let value = super::normalization::hashes(&changed, &hasher, &cancelled)
                    .unwrap()
                    .iter()
                    .flat_map(|hash| base[index].iter().map(move |other| distance(hash, other)))
                    .min()
                    .expect("eight transforms");
                distances.entry(kind).or_default().push(value);
            }
            for other in base.iter().skip(index + 1) {
                let value = other
                    .iter()
                    .flat_map(|hash| base[index].iter().map(move |other| distance(hash, other)))
                    .min()
                    .expect("eight transforms");
                distances.entry("unrelated").or_default().push(value);
            }
        }
        println!(
            "quality {label} thresholds={thresholds:?} elapsed_ms={}",
            start.elapsed().as_millis()
        );
        for (kind, values) in &mut distances {
            values.sort();
            let admitted: Vec<_> = thresholds
                .map(|threshold| values.iter().filter(|&&value| value <= threshold).count())
                .into();
            println!(
                "  {kind} n={} min={} median={} max={} accepted={admitted:?}",
                values.len(),
                values[0],
                values[values.len() / 2],
                values[values.len() - 1]
            );
        }
    }
}

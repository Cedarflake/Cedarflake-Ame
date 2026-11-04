use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use image::{DynamicImage, Rgba, RgbaImage};
use image_hasher::{HashAlg, HasherConfig};

use super::input::{InputFailure, decode};
use super::normalization::{has_structure, hashes, prepare};

fn large_image() -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_fn(4096, 4096, |x, y| {
        Rgba([x as u8, y as u8, (x ^ y) as u8, 200])
    }))
}

pub(crate) fn measure() {
    let mut encoded = Cursor::new(Vec::new());
    large_image()
        .write_to(&mut encoded, image::ImageFormat::Png)
        .expect("bounded fixture encodes");
    let cancelled = AtomicBool::new(false);
    let start = Instant::now();
    let image = decode(&mut encoded, &cancelled).expect("16 Mi-pixel fixture decodes");
    println!(
        "resources pixels=16777216 encoded_bytes={} decode_us={}",
        encoded.get_ref().len(),
        start.elapsed().as_micros()
    );
    let start = Instant::now();
    let sample = prepare(image, &cancelled).expect("bounded sample");
    let normalized = start.elapsed();
    let hasher = HasherConfig::new()
        .hash_size(8, 8)
        .hash_alg(HashAlg::Gradient)
        .resize_filter(image::imageops::FilterType::Lanczos3)
        .to_hasher();
    let start = Instant::now();
    let result = hashes(&sample, &hasher, &cancelled).expect("eight transforms");
    println!(
        "resources normalize_us={} sample_bytes={} transforms={} structured={} hash_us={}",
        normalized.as_micros(),
        sample.as_raw().len(),
        result.len(),
        has_structure(&result),
        start.elapsed().as_micros()
    );
    for delay_ms in [1, 50] {
        let image = large_image();
        let cancelled = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let signal = scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(delay_ms));
                let start = Instant::now();
                cancelled.store(true, Ordering::Release);
                start
            });
            let result = prepare(image, &cancelled);
            let finished = Instant::now();
            let signalled = signal.join().expect("cancellation signal");
            println!(
                "resources cancel_after_ms={delay_ms} cancelled={} retirement_us={}",
                result == Err(InputFailure::Cancelled),
                finished.saturating_duration_since(signalled).as_micros()
            );
        });
    }
}

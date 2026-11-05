use std::io::{self, Cursor, Read, Seek, SeekFrom};

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

use super::*;

fn scene() -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(64, 64, |x, y| {
        if x > 12 && x < 30 && y > 9 && y < 51 {
            Rgb([12, 38, 50])
        } else if x + y > 65 {
            Rgb([230, 124, 45])
        } else {
            Rgb([184, 210, 232])
        }
    }))
}

fn encoded(image: &DynamicImage) -> Cursor<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes
}

#[test]
fn similarity_contract_keeps_transform_variants_bounded_and_flat_images_terminal() {
    let mut original = encoded(&scene());
    let bytes = original.get_ref().clone();
    original.set_position(17);
    let SimilarEvidence::Ready { variants: normal } =
        analyze_reader(&mut original, &AtomicBool::new(false))
            .unwrap()
            .unwrap()
    else {
        panic!("structured scene requires evidence")
    };
    let SimilarEvidence::Ready { variants: turned } = analyze_reader(
        &mut encoded(&scene().rotate90().fliph()),
        &AtomicBool::new(false),
    )
    .unwrap()
    .unwrap() else {
        panic!("rotation retains structure")
    };
    assert!((1..=8).contains(&normal.len()));
    assert_eq!(normal.len(), turned.len());
    assert!(normal.iter().all(|variant| turned.contains(variant)));
    assert_eq!(original.into_inner(), bytes);
    assert_eq!(
        analyze_reader(
            &mut encoded(&DynamicImage::new_rgba8(64, 32)),
            &AtomicBool::new(false)
        )
        .unwrap(),
        Some(SimilarEvidence::InsufficientStructure)
    );
}

struct CancellingSource<'a> {
    bytes: Cursor<Vec<u8>>,
    cancelled: &'a AtomicBool,
}
impl Read for CancellingSource<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.bytes.read(buffer)?;
        self.cancelled.store(true, Ordering::Release);
        Ok(count)
    }
}
impl Seek for CancellingSource<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.bytes.seek(position)
    }
}

#[test]
fn cancelled_reads_publish_nothing_and_invalid_or_oversized_images_have_stable_dispositions() {
    let cancelled = AtomicBool::new(false);
    let mut input = CancellingSource {
        bytes: encoded(&scene()),
        cancelled: &cancelled,
    };
    assert_eq!(analyze_reader(&mut input, &cancelled).unwrap(), None);
    assert_eq!(
        analyze_reader(&mut Cursor::new(b"not an image"), &AtomicBool::new(false)).unwrap(),
        Some(SimilarEvidence::DecodeFailed {
            issue_code: "similarity_decode_failed".into()
        })
    );
    assert_eq!(
        analyze_reader(
            &mut encoded(&DynamicImage::new_rgb8(16_385, 1)),
            &AtomicBool::new(false)
        )
        .unwrap(),
        Some(SimilarEvidence::DecodeFailed {
            issue_code: "similarity_image_limit".into()
        })
    );
}

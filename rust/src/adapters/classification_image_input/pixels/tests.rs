use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

use super::*;

fn encoded(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, format).unwrap();
    output.into_inner()
}

fn channel(pixels: &[f32], x: usize, y: usize, channel: usize) -> f32 {
    pixels[channel * 224 * 224 + y * 224 + x]
}

#[test]
fn transparent_rgb_and_channel_order_survive_whole_image_stretch() {
    let image = RgbaImage::from_fn(40, 10, |x, _| {
        if x < 20 {
            Rgba([255, 0, 0, 0])
        } else {
            Rgba([0, 128, 255, 255])
        }
    });
    let bytes = encoded(DynamicImage::ImageRgba8(image), ImageFormat::Png);
    let pixels = prepare(Cursor::new(bytes), &AtomicBool::new(false)).unwrap();
    assert_eq!(pixels.len(), 3 * 224 * 224);
    assert_eq!(channel(&pixels, 8, 8, 0), 1.0);
    assert_eq!(channel(&pixels, 8, 8, 1), -1.0);
    assert_eq!(channel(&pixels, 210, 210, 0), -1.0);
    assert!((channel(&pixels, 210, 210, 1) - 1.0 / 255.0).abs() < 0.000001);
    assert_eq!(channel(&pixels, 210, 210, 2), 1.0);
}

#[test]
fn first_gif_frame_is_used_and_formats_are_identified_from_bytes() {
    let red = RgbaImage::from_pixel(12, 8, Rgba([255, 0, 0, 255]));
    let blue = RgbaImage::from_pixel(12, 8, Rgba([0, 0, 255, 255]));
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder
            .encode_frames([image::Frame::new(red), image::Frame::new(blue)])
            .unwrap();
    }
    let pixels = prepare(Cursor::new(bytes), &AtomicBool::new(false)).unwrap();
    assert_eq!(channel(&pixels, 112, 112, 0), 1.0);
    assert_eq!(channel(&pixels, 112, 112, 2), -1.0);

    for format in [
        ImageFormat::Bmp,
        ImageFormat::Png,
        ImageFormat::Tiff,
        ImageFormat::WebP,
        ImageFormat::Ico,
    ] {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(12, 8, Rgba([0, 255, 0, 255])));
        let pixels = prepare(Cursor::new(encoded(image, format)), &AtomicBool::new(false)).unwrap();
        assert_eq!(channel(&pixels, 112, 112, 1), 1.0, "{format:?}");
    }
}

#[test]
fn partial_gif_frame_uses_transparent_canvas_outside_its_frame() {
    // The two-pixel frame starts one pixel into a four-pixel logical screen.
    let mut bytes = b"GIF89a".to_vec();
    bytes.extend_from_slice(&[
        4, 0, 1, 0, 128, 0, 0, 255, 0, 0, 0, 0, 255, 33, 249, 4, 1, 0, 0, 0, 0, 44, 1, 0, 0, 0, 2,
        0, 1, 0, 0, 2, 2, 68, 10, 0, 59,
    ]);
    let pixels = prepare(Cursor::new(bytes), &AtomicBool::new(false)).unwrap();
    for color in 0..3 {
        assert_eq!(channel(&pixels, 0, 0, color), -1.0);
        assert_eq!(channel(&pixels, 223, 223, color), -1.0);
    }
    assert!(channel(&pixels, 84, 112, 0) > 0.95);
    assert!(channel(&pixels, 140, 112, 2) > 0.95);
}

#[test]
fn exif_rotation_precedes_resize() {
    let image = RgbImage::from_fn(80, 40, |x, _| {
        if x < 40 {
            Rgb([255, 0, 0])
        } else {
            Rgb([0, 0, 255])
        }
    });
    let field = exif::Field {
        tag: exif::Tag::Orientation,
        ifd_num: exif::In::PRIMARY,
        value: exif::Value::Short(vec![6]),
    };
    let mut writer = exif::experimental::Writer::new();
    writer.push_field(&field);
    let mut metadata = Cursor::new(Vec::new());
    writer.write(&mut metadata, false).unwrap();
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 100);
    encoder.set_exif_metadata(metadata.into_inner()).unwrap();
    encoder
        .encode(image.as_raw(), 80, 40, ExtendedColorType::Rgb8)
        .unwrap();
    let pixels = prepare(Cursor::new(bytes), &AtomicBool::new(false)).unwrap();
    assert!(channel(&pixels, 112, 8, 0) > 0.95);
    assert!(channel(&pixels, 112, 210, 2) > 0.95);
}

#[test]
fn cancellation_corrupt_data_and_excessive_dimensions_are_rejected() {
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        prepare(Cursor::new([]), &cancelled).unwrap_err().code,
        "classification_cancelled"
    );
    assert_eq!(
        prepare(Cursor::new(b"not an image"), &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_image_decode_failed"
    );
    let oversized = DynamicImage::ImageRgb8(RgbImage::new(MAX_DIMENSION + 1, 1));
    assert!(
        prepare(
            Cursor::new(encoded(oversized, ImageFormat::Png)),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

struct CancelsDuringRead<'a> {
    source: Cursor<Vec<u8>>,
    cancelled: &'a AtomicBool,
}

impl Read for CancelsDuringRead<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let result = self.source.read(buffer);
        self.cancelled.store(true, Ordering::Release);
        result
    }
}

impl Seek for CancelsDuringRead<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.source.seek(position)
    }
}

#[test]
fn cancellation_during_io_never_returns_partial_pixels() {
    let cancelled = AtomicBool::new(false);
    let bytes = encoded(
        DynamicImage::ImageRgb8(RgbImage::new(64, 32)),
        ImageFormat::Png,
    );
    let reader = CancelsDuringRead {
        source: Cursor::new(bytes),
        cancelled: &cancelled,
    };
    assert!(prepare(reader, &cancelled).is_err());
}

struct UnavailableSource;

impl Read for UnavailableSource {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Source disconnected",
        ))
    }
}

impl Seek for UnavailableSource {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

#[test]
fn unavailable_source_is_distinct_from_corrupt_image_data() {
    assert_eq!(
        prepare(UnavailableSource, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "classification_source_read_failed"
    );
    assert_eq!(
        decode_error(image::ImageError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            "Source disconnected"
        ),))
        .code,
        "classification_source_read_failed"
    );
}

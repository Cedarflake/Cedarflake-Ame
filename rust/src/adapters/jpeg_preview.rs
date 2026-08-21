use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag};
use image::metadata::Orientation;
use image::{DynamicImage, GrayImage, RgbImage};
use jpeg_decoder::{Decoder, PixelFormat};

use crate::domain::ImageOrientation;

use super::image_orientation::{apply_image_orientation, from_image_orientation};
use super::local_files::open_source_file;

pub(crate) struct DecodedJpegPreview {
    pub(crate) image: DynamicImage,
    pub(crate) source_width: u32,
    pub(crate) source_height: u32,
}

pub(crate) fn decode_scaled_jpeg(
    path: &Path,
    requested_edge: u32,
    max_decoding_buffer_size: u64,
) -> Option<DecodedJpegPreview> {
    let file = open_source_file(path).ok()?;
    let mut decoder = Decoder::new(BufReader::new(file));
    decoder.set_max_decoding_buffer_size(max_decoding_buffer_size.try_into().ok()?);
    decoder.read_info().ok()?;
    let original = decoder.info()?;
    if !matches!(original.pixel_format, PixelFormat::L8 | PixelFormat::RGB24) {
        return None;
    }
    let requested_edge = u16::try_from(requested_edge).ok()?;
    decoder.scale(requested_edge, requested_edge).ok()?;
    let pixels = decoder.decode().ok()?;
    let decoded = decoder.info()?;
    let mut image = match decoded.pixel_format {
        PixelFormat::L8 => DynamicImage::ImageLuma8(GrayImage::from_raw(
            u32::from(decoded.width),
            u32::from(decoded.height),
            pixels,
        )?),
        PixelFormat::RGB24 => DynamicImage::ImageRgb8(RgbImage::from_raw(
            u32::from(decoded.width),
            u32::from(decoded.height),
            pixels,
        )?),
        PixelFormat::L16 | PixelFormat::CMYK32 => return None,
    };
    let orientation = exif_orientation(decoder.exif_data());
    apply_image_orientation(&mut image, orientation);
    let (source_width, source_height) =
        orientation.display_dimensions(u32::from(original.width), u32::from(original.height));
    Some(DecodedJpegPreview {
        image,
        source_width,
        source_height,
    })
}

fn exif_orientation(raw_exif: Option<&[u8]>) -> ImageOrientation {
    let Some(raw_exif) = raw_exif else {
        return ImageOrientation::default();
    };
    let Ok(exif) = Reader::new().read_raw(raw_exif.to_vec()) else {
        return ImageOrientation::default();
    };
    let Some(value) = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .and_then(|value| u8::try_from(value).ok())
        .and_then(Orientation::from_exif)
    else {
        return ImageOrientation::default();
    };
    from_image_orientation(value)
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn high_resolution_jpeg_is_reduced_during_decode() {
        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("large.jpg");
        let source = RgbImage::from_fn(4096, 3072, |x, y| {
            Rgb([
                u8::try_from((x / 32) % 256).expect("red channel"),
                u8::try_from((y / 24) % 256).expect("green channel"),
                u8::try_from(((x + y) / 48) % 256).expect("blue channel"),
            ])
        });
        source.save(&source_path).expect("large jpeg fixture");
        drop(source);

        let decoded =
            decode_scaled_jpeg(&source_path, 256, 256 * 1024 * 1024).expect("scaled jpeg decode");
        let scaled_thumbnail = decoded.image.thumbnail(256, 256).to_rgb8();
        let full_thumbnail = image::open(&source_path)
            .expect("full jpeg decode")
            .thumbnail(256, 256)
            .to_rgb8();
        let absolute_error = scaled_thumbnail
            .as_raw()
            .iter()
            .zip(full_thumbnail.as_raw())
            .map(|(scaled, full)| u64::from(scaled.abs_diff(*full)))
            .sum::<u64>();
        let mean_absolute_error = absolute_error as f64 / scaled_thumbnail.as_raw().len() as f64;

        assert_eq!((decoded.source_width, decoded.source_height), (4096, 3072));
        assert_eq!((decoded.image.width(), decoded.image.height()), (512, 384));
        assert_eq!(scaled_thumbnail.dimensions(), full_thumbnail.dimensions());
        assert!(
            mean_absolute_error <= 8.0,
            "scaled JPEG mean absolute error {mean_absolute_error:.2} exceeded the fixture budget"
        );
    }

    #[test]
    #[ignore = "manual performance evidence"]
    fn benchmark_high_resolution_jpeg_preview_conversion() {
        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("benchmark-large.jpg");
        let source = RgbImage::from_fn(6000, 4000, |x, y| {
            Rgb([
                u8::try_from((x / 24) % 256).expect("red channel"),
                u8::try_from((y / 16) % 256).expect("green channel"),
                u8::try_from(((x + y) / 40) % 256).expect("blue channel"),
            ])
        });
        let source_file = File::create(&source_path).expect("benchmark jpeg file");
        JpegEncoder::new_with_quality(source_file, 90)
            .write_image(
                source.as_raw(),
                source.width(),
                source.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("benchmark jpeg encoding");
        drop(source);

        let full_started = std::time::Instant::now();
        let full = image::open(&source_path)
            .expect("full jpeg decode")
            .thumbnail(512, 512);
        let full_elapsed = full_started.elapsed();

        let scaled_started = std::time::Instant::now();
        let scaled = decode_scaled_jpeg(&source_path, 512, 256 * 1024 * 1024)
            .expect("scaled jpeg decode")
            .image
            .thumbnail(512, 512);
        let scaled_elapsed = scaled_started.elapsed();

        assert_eq!(
            (scaled.width(), scaled.height()),
            (full.width(), full.height())
        );
        println!(
            "full_decode_resize_ms={} scaled_decode_resize_ms={} speedup={:.2}",
            full_elapsed.as_secs_f64() * 1000.0,
            scaled_elapsed.as_secs_f64() * 1000.0,
            full_elapsed.as_secs_f64() / scaled_elapsed.as_secs_f64(),
        );
    }
}

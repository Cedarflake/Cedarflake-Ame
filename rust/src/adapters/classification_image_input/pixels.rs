use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};

use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};

use crate::domain::ScanError;

const EDGE: u32 = 224;
const MAX_DIMENSION: u32 = 16_384;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_DECODER_BYTES: u64 = 128 * 1024 * 1024;

pub(super) fn prepare(
    mut source: impl Read + Seek,
    cancelled: &AtomicBool,
) -> Result<Vec<f32>, ScanError> {
    check_cancelled(cancelled)?;
    source.seek(SeekFrom::Start(0)).map_err(read_error)?;
    let guarded = CancellableReader { source, cancelled };
    let mut reader = ImageReader::new(BufReader::new(guarded))
        .with_guessed_format()
        .map_err(read_error)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_BYTES);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(ScanError::new(
            "classification_image_limit",
            "The image exceeds the bounded classification decode size",
        ));
    }
    let orientation = decoder.orientation().map_err(decode_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    check_cancelled(cancelled)?;
    image.apply_orientation(orientation);
    // Convert before resizing to match the admitted RGB input, including transparent source colors.
    let rgb = image.into_rgb8();
    check_cancelled(cancelled)?;
    let resized = image::imageops::resize(&rgb, EDGE, EDGE, FilterType::Triangle);
    check_cancelled(cancelled)?;
    let plane = (EDGE * EDGE) as usize;
    let mut tensor = vec![0.0; 3 * plane];
    for (index, pixel) in resized.pixels().enumerate() {
        for channel in 0..3 {
            tensor[channel * plane + index] = (f32::from(pixel[channel]) / 255.0 - 0.5) / 0.5;
        }
    }
    Ok(tensor)
}

struct CancellableReader<'a, R> {
    source: R,
    cancelled: &'a AtomicBool,
}

impl<R: Read> Read for CancellableReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Classification was cancelled"));
        }
        self.source.read(buffer)
    }
}

impl<R: Seek> Seek for CancellableReader<'_, R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Classification was cancelled"));
        }
        self.source.seek(position)
    }
}

pub(super) fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        Err(cancelled_error())
    } else {
        Ok(())
    }
}

pub(super) fn cancelled_error() -> ScanError {
    ScanError::new("classification_cancelled", "Classification was cancelled")
}

fn decode_error(error: image::ImageError) -> ScanError {
    match error {
        image::ImageError::IoError(error) => read_error(error),
        error => ScanError::new("classification_image_decode_failed", error.to_string()),
    }
}

fn read_error(error: io::Error) -> ScanError {
    ScanError::new("classification_source_read_failed", error.to_string())
}

#[cfg(test)]
mod tests;

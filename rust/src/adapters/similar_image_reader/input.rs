use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};

use image::{DynamicImage, ImageDecoder, ImageReader, Limits};

const MAX_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIMENSION: u32 = 16_384;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_DECODER_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum InputFailure {
    Cancelled,
    Limit,
    Read,
    Decode,
}

pub(super) fn decode(
    source: &mut (impl Read + Seek),
    cancelled: &AtomicBool,
) -> Result<DynamicImage, InputFailure> {
    check_cancelled(cancelled)?;
    let length = source
        .seek(SeekFrom::End(0))
        .map_err(|_| InputFailure::Read)?;
    if length == 0 || length > MAX_SOURCE_BYTES {
        return Err(InputFailure::Limit);
    }
    source
        .seek(SeekFrom::Start(0))
        .map_err(|_| InputFailure::Read)?;
    let guarded = CancellableReader { source, cancelled };
    let mut reader = ImageReader::new(BufReader::new(guarded))
        .with_guessed_format()
        .map_err(|_| read_failure(cancelled))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_BYTES);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| decode_failure(error, cancelled))?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(InputFailure::Limit);
    }
    check_cancelled(cancelled)?;
    let orientation = decoder
        .orientation()
        .map_err(|error| decode_failure(error, cancelled))?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|error| decode_failure(error, cancelled))?;
    check_cancelled(cancelled)?;
    image.apply_orientation(orientation);
    check_cancelled(cancelled)?;
    Ok(image)
}

pub(super) fn check_cancelled(cancelled: &AtomicBool) -> Result<(), InputFailure> {
    if cancelled.load(Ordering::Acquire) {
        Err(InputFailure::Cancelled)
    } else {
        Ok(())
    }
}

struct CancellableReader<'a, R> {
    source: &'a mut R,
    cancelled: &'a AtomicBool,
}

impl<R: Read> Read for CancellableReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Similarity input was cancelled"));
        }
        self.source.read(buffer)
    }
}

impl<R: Seek> Seek for CancellableReader<'_, R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Similarity input was cancelled"));
        }
        self.source.seek(position)
    }
}

fn read_failure(cancelled: &AtomicBool) -> InputFailure {
    if cancelled.load(Ordering::Acquire) {
        InputFailure::Cancelled
    } else {
        InputFailure::Read
    }
}

fn decode_failure(error: image::ImageError, cancelled: &AtomicBool) -> InputFailure {
    if cancelled.load(Ordering::Acquire) {
        return InputFailure::Cancelled;
    }
    match error {
        image::ImageError::IoError(_) => InputFailure::Read,
        image::ImageError::Limits(_) => InputFailure::Limit,
        _ => InputFailure::Decode,
    }
}

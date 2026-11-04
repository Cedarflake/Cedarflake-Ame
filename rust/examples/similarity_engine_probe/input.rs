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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn encoded_scene() -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        super::super::fixtures::scene(7)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .expect("generated scene encodes");
        bytes.into_inner()
    }

    struct CancellingSource<'a> {
        cursor: Cursor<Vec<u8>>,
        cancelled: &'a AtomicBool,
    }

    impl Read for CancellingSource<'_> {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let count = self.cursor.read(bytes)?;
            self.cancelled.store(true, Ordering::Release);
            Ok(count)
        }
    }

    impl Seek for CancellingSource<'_> {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.cursor.seek(position)
        }
    }

    #[test]
    fn borrowed_input_rewinds_and_cancellation_never_publishes_pixels() {
        let bytes = encoded_scene();
        let mut source = Cursor::new(bytes.clone());
        source.set_position(17);
        let image = decode(&mut source, &AtomicBool::new(false)).unwrap();
        assert_eq!((image.width(), image.height()), (320, 240));
        assert_eq!(source.get_ref(), &bytes);

        let cancelled = AtomicBool::new(false);
        let mut source = CancellingSource {
            cursor: Cursor::new(bytes),
            cancelled: &cancelled,
        };
        assert_eq!(
            decode(&mut source, &cancelled),
            Err(InputFailure::Cancelled)
        );
        source.cursor.set_position(0);
        assert_eq!(
            decode(&mut source, &cancelled),
            Err(InputFailure::Cancelled)
        );
        assert_eq!(source.cursor.position(), 0);
    }

    #[test]
    fn invalid_and_oversized_inputs_stop_before_pixel_allocation() {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(MAX_DIMENSION + 1, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert_eq!(
            decode(&mut bytes, &AtomicBool::new(false)),
            Err(InputFailure::Limit)
        );
        assert_eq!(
            decode(&mut Cursor::new(b"not an image"), &AtomicBool::new(false)),
            Err(InputFailure::Decode)
        );
        assert_eq!(
            decode(&mut Cursor::new([]), &AtomicBool::new(false)),
            Err(InputFailure::Limit)
        );
    }
}

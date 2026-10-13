use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

use image::error::{
    DecodingError, LimitError, LimitErrorKind, UnsupportedError, UnsupportedErrorKind,
};
use image::metadata::Orientation;
use image::{ImageError, ImageFormat};
use zune_core::bytestream::ZByteIoError;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;
use zune_jpeg::errors::DecodeErrors;

use super::{MAX_DECODER_ALLOCATION, MAX_SOURCE_DIMENSION, classify_decoder_error, media_failure};
use crate::adapters::exif_metadata::KamadakExifExtractor;
use crate::adapters::image_orientation::from_image_orientation;
use crate::domain::{DiscoveredFile, MediaInspection};
use crate::ports::{MediaInspectionFailure, MetadataExtractor};

pub(super) fn inspect<R: Read + Seek>(
    file: &DiscoveredFile,
    reader: R,
    metadata: &KamadakExifExtractor,
    format_probe_bytes: usize,
) -> Result<MediaInspection, MediaInspectionFailure> {
    let read_limit = MAX_DECODER_ALLOCATION.saturating_sub(format_probe_bytes as u64);
    inspect_with_read_limit(file, reader, metadata, read_limit).map_err(|error| {
        let (kind, code) = classify_decoder_error(&error);
        media_failure(file, kind, code, error)
    })
}

pub(super) fn inspect_with_read_limit<R: Read + Seek>(
    file: &DiscoveredFile,
    reader: R,
    metadata: &KamadakExifExtractor,
    read_limit: u64,
) -> Result<MediaInspection, ImageError> {
    let mut source = HeaderReader {
        reader,
        remaining: read_limit,
        failure: None,
    };
    let options = DecoderOptions::default()
        .set_strict_mode(false)
        .set_max_width(MAX_SOURCE_DIMENSION as usize)
        .set_max_height(MAX_SOURCE_DIMENSION as usize);
    let result = {
        let stream = HeaderStream {
            reader: BufReader::new(&mut source),
        };
        let mut decoder = JpegDecoder::new_with_options(stream, options);
        decoder
            .decode_headers()
            .map_err(decoder_error)
            .and_then(|()| {
                let Some((width, height)) = decoder.dimensions() else {
                    return Err(ImageError::Decoding(DecodingError::new(
                        ImageFormat::Jpeg.into(),
                        "JPEG dimensions are absent after header parsing",
                    )));
                };
                let width = u32::try_from(width).map_err(|_| dimension_limit())?;
                let height = u32::try_from(height).map_err(|_| dimension_limit())?;
                let exif = decoder.exif().map(Vec::as_slice);
                let orientation = from_image_orientation(
                    exif.and_then(Orientation::from_exif_chunk)
                        .unwrap_or(Orientation::NoTransforms),
                );
                let (width, height) = orientation.display_dimensions(width, height);
                Ok(MediaInspection {
                    width,
                    height,
                    metadata: metadata.extract(exif, &file.absolute_path),
                })
            })
    };
    // Some decoder byte helpers suppress I/O errors; source failure still wins over parsed output.
    match source.failure {
        Some(HeaderFailure::ReadLimit) => Err(ImageError::Limits(LimitError::from_kind(
            LimitErrorKind::InsufficientMemory,
        ))),
        Some(HeaderFailure::Io(error)) => Err(ImageError::IoError(error)),
        None => result,
    }
}

fn dimension_limit() -> ImageError {
    ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError))
}

fn decoder_error(error: DecodeErrors) -> ImageError {
    match error {
        DecodeErrors::Unsupported(scheme) => {
            ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                ImageFormat::Jpeg.into(),
                UnsupportedErrorKind::GenericFeature(format!("{scheme:?}")),
            ))
        }
        DecodeErrors::LargeDimensions(_) => dimension_limit(),
        DecodeErrors::IoErrors(ZByteIoError::StdIoError(error)) => ImageError::IoError(error),
        error => ImageError::Decoding(DecodingError::new(ImageFormat::Jpeg.into(), error)),
    }
}

enum HeaderFailure {
    ReadLimit,
    Io(io::Error),
}

struct HeaderReader<R> {
    reader: R,
    remaining: u64,
    failure: Option<HeaderFailure>,
}

impl<R> HeaderReader<R> {
    fn ensure_remaining(&mut self) -> io::Result<()> {
        if self.remaining == 0 || self.failure.is_some() {
            if self.failure.is_none() {
                self.failure = Some(HeaderFailure::ReadLimit);
            }
            return Err(io::Error::other("JPEG header reader is retired"));
        }
        Ok(())
    }

    fn record_io_failure(&mut self, error: io::Error) -> io::Error {
        let result = io::Error::new(error.kind(), "JPEG header source access failed");
        if self.failure.is_none() {
            self.failure = Some(HeaderFailure::Io(error));
        }
        result
    }
}

impl<R: Read> Read for HeaderReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        self.ensure_remaining()?;
        let limit = usize::try_from(self.remaining).unwrap_or(usize::MAX);
        let length = output.len().min(limit);
        loop {
            match self.reader.read(&mut output[..length]) {
                Ok(read) => {
                    self.remaining -= read as u64;
                    return Ok(read);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(self.record_io_failure(error)),
            }
        }
    }
}

impl<R: Seek> Seek for HeaderReader<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.reader
            .seek(position)
            .map_err(|error| self.record_io_failure(error))
    }
}

struct HeaderStream<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> Read for HeaderStream<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.reader.read(output)
    }
}

impl<R: Read> BufRead for HeaderStream<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.reader.consume(amount);
    }
}

impl<R: Read + Seek> Seek for HeaderStream<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if let SeekFrom::Current(offset) = position {
            // Header peeks and empty marker skips must not discard the prefetched bytes.
            self.reader.seek_relative(offset)?;
            self.reader.stream_position()
        } else {
            self.reader.seek(position)
        }
    }
}

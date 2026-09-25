use std::cell::Cell;
use std::io::{Cursor, Read};
use std::rc::Rc;

use exif::experimental::Writer;
use exif::{Field, In, Tag, Value};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageError, ImageFormat};

use super::*;
use crate::media_fixtures::{
    ALL_MEDIA_FORMATS, MediaFixtureFormat, encode_cmyk_gray_jpeg, encode_rgb_quadrants,
};

#[test]
fn jpeg_inspection_reads_headers_without_reading_compressed_pixels() {
    let mut noise = 123_456_789_u32;
    let pixels: Vec<_> = (0..1024 * 1024 * 3)
        .map(|_| {
            noise ^= noise << 13;
            noise ^= noise >> 17;
            noise ^= noise << 5;
            noise.to_le_bytes()[0]
        })
        .collect();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode(&pixels, 1024, 1024, ExtendedColorType::Rgb8)
        .expect("encode generated JPEG");
    assert!(
        jpeg.len() > 1024 * 1024,
        "fixture must have a large payload"
    );
    let total_bytes = jpeg.len();
    let bytes_read = Rc::new(Cell::new(0));
    let source = CountingReader {
        source: Cursor::new(jpeg),
        bytes_read: Rc::clone(&bytes_read),
    };
    let reader = ImageReader::new(BufReader::new(source))
        .with_guessed_format()
        .expect("detect generated format");
    let inspected = LocalMediaInspector::new()
        .inspect_reader(&fixture_file(total_bytes), reader)
        .expect("inspect JPEG header");
    assert_eq!((inspected.width, inspected.height), (1024, 1024));
    assert!(inspected.metadata.issues.is_empty());
    eprintln!(
        "jpeg_total_bytes={total_bytes} inspection_read_bytes={}",
        bytes_read.get()
    );
    assert!(
        bytes_read.get() <= 64 * 1024,
        "small JPEG headers read {} of {total_bytes} bytes",
        bytes_read.get()
    );
}

#[test]
fn jpeg_skipped_header_segments_do_not_amplify_source_reads() {
    let jpeg = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("JPEG");
    let mut bytes = vec![0xff, 0xd8];
    for _ in 0..2048 {
        bytes.extend_from_slice(&[0xff, 0xe3, 0, 2]);
    }
    bytes.extend_from_slice(&jpeg[2..]);
    let length = bytes.len();
    let bytes_read = Rc::new(Cell::new(0));
    let source = CountingReader {
        source: Cursor::new(bytes),
        bytes_read: Rc::clone(&bytes_read),
    };
    let reader = ImageReader::new(BufReader::new(source))
        .with_guessed_format()
        .expect("JPEG format");
    let inspected = LocalMediaInspector::new()
        .inspect_reader(&fixture_file(length), reader)
        .expect("valid skipped header segments");
    assert_eq!((inspected.width, inspected.height), (64, 48));
    assert!(
        bytes_read.get() <= length + 16 * 1024,
        "header seeks amplified {length} source bytes to {} reads",
        bytes_read.get(),
    );
}

#[test]
fn jpeg_headers_retain_late_exif_orientation_and_historical_capture_time() {
    let fields = [
        Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![6]),
        },
        Field {
            tag: Tag::DateTimeOriginal,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![b"1925:07:08 09:10:11".to_vec()]),
        },
    ];
    let mut writer = Writer::new();
    for field in &fields {
        writer.push_field(field);
    }
    let mut exif = Cursor::new(Vec::new());
    writer
        .write(&mut exif, false)
        .expect("encode historical EXIF");
    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new(&mut jpeg);
    encoder
        .set_exif_metadata(exif.into_inner())
        .expect("set EXIF");
    encoder
        .encode(&vec![0; 80 * 60 * 3], 80, 60, ExtendedColorType::Rgb8)
        .expect("JPEG");
    let marker = jpeg
        .windows(2)
        .position(|bytes| bytes == [0xff, 0xe1])
        .expect("EXIF marker");
    let length = usize::from(u16::from_be_bytes([jpeg[marker + 2], jpeg[marker + 3]])) + 2;
    let segment: Vec<_> = jpeg.drain(marker..marker + length).collect();
    let scan = jpeg
        .windows(2)
        .position(|bytes| bytes == [0xff, 0xda])
        .expect("scan marker");
    let frame = jpeg
        .windows(2)
        .position(|bytes| bytes == [0xff, 0xc0])
        .expect("frame marker");
    assert!(frame < scan);
    jpeg.splice(scan..scan, segment);

    let inspected = inspect_bytes(&jpeg).expect("late EXIF inspection");
    assert_eq!((inspected.width, inspected.height), (60, 80));
    assert_eq!(
        inspected
            .metadata
            .capture_time
            .as_ref()
            .expect("historical capture")
            .local_time,
        "1925-07-08T09:10:11.000000000"
    );
    let mut baseline = ImageReader::with_format(Cursor::new(&jpeg), ImageFormat::Jpeg)
        .into_decoder()
        .expect("baseline headers");
    let raw_exif = baseline.exif_metadata().expect("baseline EXIF");
    let old_metadata = KamadakExifExtractor.extract(raw_exif.as_deref(), "fixture");
    assert_eq!(inspected.metadata.capture_time, old_metadata.capture_time);
    assert_eq!(inspected.metadata.engine_id, old_metadata.engine_id);
    assert_eq!(
        inspected.metadata.engine_version,
        old_metadata.engine_version
    );
    assert!(inspected.metadata.issues.is_empty());
}

#[test]
fn jpeg_headers_keep_complete_cmyk_and_non_jpeg_dimensions() {
    for format in ALL_MEDIA_FORMATS {
        let bytes = encode_rgb_quadrants(format, 64, 48).expect("format fixture");
        let inspected = inspect_bytes(&bytes).expect("common format inspection");
        assert_eq!((inspected.width, inspected.height), (64, 48), "{format:?}");
        assert!(inspected.metadata.issues.is_empty(), "{format:?}");
    }
    let bytes = encode_cmyk_gray_jpeg().expect("complete CMYK JPEG");
    let inspected = inspect_bytes(&bytes).expect("CMYK inspection");
    assert_eq!((inspected.width, inspected.height), (64, 48));
}

#[test]
fn truncated_jpeg_headers_remain_terminal_content_failures() {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("JPEG");
    let scan = bytes
        .windows(2)
        .position(|bytes| bytes == [0xff, 0xda])
        .expect("scan marker");
    for end in [3, 16, scan, scan + 4] {
        let reader = ImageReader::with_format(
            BufReader::new(Cursor::new(&bytes[..end])),
            ImageFormat::Jpeg,
        );
        let error = LocalMediaInspector::new()
            .inspect_reader(&fixture_file(end), reader)
            .expect_err("incomplete headers");
        assert!(
            matches!(error.kind, MediaInspectionFailureKind::Terminal),
            "cut={end}: {error:?}"
        );
        assert_eq!(error.issue.code, "image_decode_invalid");
    }
}

#[test]
fn jpeg_header_source_access_failures_remain_retryable() {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("JPEG");
    for kind in [
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::WouldBlock,
    ] {
        let source = FailingReader {
            source: Cursor::new(&bytes),
            kind,
        };
        let reader = ImageReader::with_format(BufReader::new(source), ImageFormat::Jpeg);
        let error = LocalMediaInspector::new()
            .inspect_reader(&fixture_file(bytes.len()), reader)
            .expect_err("source read fails");
        assert!(matches!(error.kind, MediaInspectionFailureKind::Retryable));
        assert_eq!(error.issue.code, "image_header_read_failed");
    }
}

#[test]
fn jpeg_header_resource_limit_cannot_publish_partial_metadata() {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("JPEG");
    let bytes_read = Rc::new(Cell::new(0));
    let source = CountingReader {
        source: Cursor::new(&bytes),
        bytes_read: Rc::clone(&bytes_read),
    };
    let error = jpeg_headers::inspect_with_read_limit(
        &fixture_file(bytes.len()),
        source,
        &KamadakExifExtractor,
        64,
    )
    .expect_err("header exceeds read budget");
    assert!(matches!(error, ImageError::Limits(_)));
    let (kind, code) = classify_decoder_error(&error);
    assert!(matches!(kind, MediaInspectionFailureKind::Terminal));
    assert_eq!(code, "image_limits_exceeded");
    assert_eq!(
        bytes_read.get(),
        64,
        "the budget counts actual source reads below buffering"
    );
}

fn inspect_bytes(bytes: &[u8]) -> Result<MediaInspection, MediaInspectionFailure> {
    let reader = ImageReader::new(BufReader::new(Cursor::new(bytes)))
        .with_guessed_format()
        .expect("format");
    LocalMediaInspector::new().inspect_reader(&fixture_file(bytes.len()), reader)
}

struct FailingReader<'a> {
    source: Cursor<&'a Vec<u8>>,
    kind: std::io::ErrorKind,
}

impl Read for FailingReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.source.position() >= 64 {
            return Err(std::io::Error::from(self.kind));
        }
        let length = output.len().min(64);
        self.source.read(&mut output[..length])
    }
}

impl Seek for FailingReader<'_> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.source.seek(position)
    }
}

fn fixture_file(file_size: usize) -> DiscoveredFile {
    DiscoveredFile {
        source_root_path: "generated".to_owned(),
        absolute_path: "generated/header.jpg".to_owned(),
        relative_path: "header.jpg".to_owned(),
        file_size: u64::try_from(file_size).expect("fixture size"),
        created_unix_ms: None,
        modified_unix_ms: 0,
        file_identity: None,
        source_revision: None,
        source_generation: 0,
        issues: Vec::new(),
    }
}

struct CountingReader<R> {
    source: R,
    bytes_read: Rc<Cell<usize>>,
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let read = self.source.read(output)?;
        self.bytes_read.set(self.bytes_read.get() + read);
        Ok(read)
    }
}

impl<R: Seek> Seek for CountingReader<R> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.source.seek(position)
    }
}

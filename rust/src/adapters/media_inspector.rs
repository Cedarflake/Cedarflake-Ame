use std::path::Path;

use image::{ImageDecoder, ImageReader, Limits};

use crate::domain::{DiscoveredFile, ImageOrientation, MediaInspection, ScanIssue};
use crate::ports::{MediaInspector, MetadataExtractor};

use super::exif_metadata::KamadakExifExtractor;
use super::image_orientation::from_image_orientation;

const MAX_SOURCE_DIMENSION: u32 = 100_000;
const MAX_DECODER_ALLOCATION: u64 = 256 * 1024 * 1024;

#[flutter_rust_bridge::frb(opaque)]
pub struct LocalMediaInspector {
    metadata: KamadakExifExtractor,
}

impl LocalMediaInspector {
    pub(crate) fn new() -> Self {
        Self {
            metadata: KamadakExifExtractor,
        }
    }
}

impl MediaInspector for LocalMediaInspector {
    #[flutter_rust_bridge::frb(ignore)]
    fn metadata_engine_id(&self) -> &'static str {
        self.metadata.engine_id()
    }

    #[flutter_rust_bridge::frb(ignore)]
    fn metadata_engine_version(&self) -> &'static str {
        self.metadata.engine_version()
    }

    #[flutter_rust_bridge::frb(ignore)]
    fn inspect(&self, file: &DiscoveredFile) -> Result<MediaInspection, ScanIssue> {
        let source_path = Path::new(&file.absolute_path);
        let mut reader = ImageReader::open(source_path)
            .and_then(|reader| reader.with_guessed_format())
            .map_err(|error| media_issue(file, "image_open_failed", error))?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
        limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
        reader.limits(limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|error| media_issue(file, "image_dimensions_failed", error))?;
        let (pixel_width, pixel_height) = decoder.dimensions();
        if pixel_width > MAX_SOURCE_DIMENSION || pixel_height > MAX_SOURCE_DIMENSION {
            return Err(ScanIssue {
                path: Some(file.absolute_path.clone()),
                code: "image_dimensions_exceeded".to_owned(),
                message: format!(
                    "Image dimensions {pixel_width}x{pixel_height} exceed the supported limit"
                ),
            });
        }

        let (orientation, orientation_read_issue) = match decoder.orientation() {
            Ok(orientation) => (from_image_orientation(orientation), None),
            Err(error) => (
                ImageOrientation::default(),
                Some(media_issue(file, "orientation_read_failed", error)),
            ),
        };
        let (raw_exif, metadata_read_issue) = match decoder.exif_metadata() {
            Ok(raw_exif) => (raw_exif, None),
            Err(error) => (None, Some(media_issue(file, "metadata_read_failed", error))),
        };
        let mut metadata = self
            .metadata
            .extract(raw_exif.as_deref(), &file.absolute_path);
        if let Some(issue) = metadata_read_issue {
            metadata.issues.insert(0, issue);
        }
        if let Some(issue) = orientation_read_issue {
            metadata.issues.insert(0, issue);
        }
        let (width, height) = orientation.display_dimensions(pixel_width, pixel_height);

        Ok(MediaInspection {
            width,
            height,
            metadata,
        })
    }
}

fn media_issue(file: &DiscoveredFile, code: &str, error: impl std::fmt::Display) -> ScanIssue {
    ScanIssue {
        path: Some(file.absolute_path.clone()),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use exif::experimental::Writer;
    use exif::{Field, In, Tag, Value};
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn inspects_dimensions_and_capture_time_without_decoding_pixels() {
        let exif_field = Field {
            tag: Tag::DateTimeOriginal,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![b"2025:07:08 09:10:11".to_vec()]),
        };
        let mut exif_writer = Writer::new();
        exif_writer.push_field(&exif_field);
        let mut exif = Cursor::new(Vec::new());
        exif_writer.write(&mut exif, false).expect("encode EXIF");

        let mut jpeg = Vec::new();
        let mut encoder = JpegEncoder::new(&mut jpeg);
        encoder
            .set_exif_metadata(exif.into_inner())
            .expect("set JPEG EXIF");
        encoder
            .encode(&[0, 0, 0], 1, 1, ExtendedColorType::Rgb8)
            .expect("encode JPEG");

        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("capture.jpg");
        std::fs::write(&path, &jpeg).expect("write JPEG");
        let file = DiscoveredFile {
            absolute_path: path.to_string_lossy().into_owned(),
            relative_path: "capture.jpg".to_owned(),
            file_size: u64::try_from(jpeg.len()).expect("JPEG size"),
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            issues: Vec::new(),
        };

        let inspected = LocalMediaInspector::new()
            .inspect(&file)
            .expect("inspect JPEG");

        assert_eq!((inspected.width, inspected.height), (1, 1));
        assert_eq!(
            inspected
                .metadata
                .capture_time
                .expect("capture evidence")
                .local_time,
            "2025-07-08T09:10:11.000000000"
        );
        assert!(inspected.metadata.issues.is_empty());
        assert_eq!(std::fs::read(path).expect("read JPEG"), jpeg);
    }

    #[test]
    fn reports_orientation_corrected_display_dimensions_without_modifying_source() {
        for (orientation, expected_dimensions) in [
            (1, (80, 60)),
            (3, (80, 60)),
            (5, (60, 80)),
            (6, (60, 80)),
            (8, (60, 80)),
            (9, (80, 60)),
        ] {
            let orientation_field = Field {
                tag: Tag::Orientation,
                ifd_num: In::PRIMARY,
                value: Value::Short(vec![orientation]),
            };
            let mut exif_writer = Writer::new();
            exif_writer.push_field(&orientation_field);
            let mut exif = Cursor::new(Vec::new());
            exif_writer.write(&mut exif, false).expect("encode EXIF");

            let pixels = vec![0; 80 * 60 * 3];
            let mut jpeg = Vec::new();
            let mut encoder = JpegEncoder::new(&mut jpeg);
            encoder
                .set_exif_metadata(exif.into_inner())
                .expect("set JPEG EXIF");
            encoder
                .encode(&pixels, 80, 60, ExtendedColorType::Rgb8)
                .expect("encode JPEG");

            let directory = tempdir().expect("temporary directory");
            let path = directory
                .path()
                .join(format!("orientation-{orientation}.jpg"));
            std::fs::write(&path, &jpeg).expect("write JPEG");
            let file = DiscoveredFile {
                absolute_path: path.to_string_lossy().into_owned(),
                relative_path: format!("orientation-{orientation}.jpg"),
                file_size: u64::try_from(jpeg.len()).expect("JPEG size"),
                created_unix_ms: None,
                modified_unix_ms: 0,
                file_identity: None,
                issues: Vec::new(),
            };

            let inspected = LocalMediaInspector::new()
                .inspect(&file)
                .expect("inspect oriented JPEG");

            assert_eq!(
                (inspected.width, inspected.height),
                expected_dimensions,
                "orientation {orientation}"
            );
            assert!(inspected.metadata.issues.is_empty());
            assert_eq!(std::fs::read(path).expect("read JPEG"), jpeg);
        }
    }
}

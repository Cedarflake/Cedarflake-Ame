use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use blake3::Hasher;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

use crate::domain::{DiscoveredFile, ImageOrientation, PreviewArtifact, ScanIssue};
use crate::ports::PreviewStore;

use super::image_orientation::{apply_image_orientation, from_image_orientation};

pub(crate) const PREVIEW_CACHE_VERSION: &str = "ame-jpeg-thumbnail-v2-orientation";
const PREVIEW_ALGORITHM: &str = PREVIEW_CACHE_VERSION;
const MAX_PREVIEW_EDGE: u32 = 1024;
const MAX_SOURCE_DIMENSION: u32 = 100_000;
const MAX_DECODER_ALLOCATION: u64 = 256 * 1024 * 1024;
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct LocalPreviewStore {
    root: PathBuf,
    budget_bytes: u64,
    used_bytes: AtomicU64,
}

impl LocalPreviewStore {
    pub fn new(root: PathBuf, budget_bytes: u64) -> Result<Self, ScanIssue> {
        fs::create_dir_all(&root).map_err(|error| ScanIssue {
            path: Some(root.to_string_lossy().into_owned()),
            code: "preview_cache_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let used_bytes = cache_usage(&root)?;
        Ok(Self {
            root,
            budget_bytes,
            used_bytes: AtomicU64::new(used_bytes),
        })
    }

    fn artifact_path(&self, file: &DiscoveredFile, preview_edge: u32) -> PathBuf {
        let hash = preview_cache_key(PREVIEW_ALGORITHM, file, preview_edge);
        self.root
            .join(format!("{PREVIEW_ALGORITHM}-{}.jpg", hash.to_hex()))
    }
}

fn preview_cache_key(algorithm: &str, file: &DiscoveredFile, preview_edge: u32) -> blake3::Hash {
    let mut hasher = Hasher::new();
    hasher.update(algorithm.as_bytes());
    hasher.update(&preview_edge.to_le_bytes());
    if let Some(identity) = &file.file_identity {
        hasher.update(identity.scheme.as_bytes());
        hasher.update(&[0]);
        hasher.update(identity.value.as_bytes());
    } else {
        hasher.update(file.absolute_path.as_bytes());
    }
    hasher.update(&file.file_size.to_le_bytes());
    hasher.update(&file.modified_unix_ms.to_le_bytes());
    hasher.finalize()
}

impl PreviewStore for LocalPreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
    ) -> Result<PreviewArtifact, ScanIssue> {
        let edge = preview_edge.clamp(96, MAX_PREVIEW_EDGE);
        let source_path = Path::new(&file.absolute_path);
        let artifact_path = self.artifact_path(file, edge);

        let has_valid_artifact =
            artifact_path.is_file() && is_valid_cached_artifact(&artifact_path, edge);
        if source_width > 0 && source_height > 0 && has_valid_artifact {
            return Ok(PreviewArtifact {
                path: artifact_path.to_string_lossy().into_owned(),
                width: source_width,
                height: source_height,
            });
        }
        if artifact_path.exists() && !has_valid_artifact {
            let invalid_size = artifact_path
                .metadata()
                .map_or(0, |metadata| metadata.len());
            fs::remove_file(&artifact_path).map_err(|error| {
                preview_issue(file, "preview_cache_corrupt_remove_failed", error)
            })?;
            self.release(invalid_size);
        }

        let mut reader = ImageReader::open(source_path)
            .and_then(|reader| reader.with_guessed_format())
            .map_err(|error| preview_issue(file, "image_open_failed", error))?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
        limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
        reader.limits(limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|error| preview_issue(file, "image_decode_failed", error))?;
        let orientation = decoder
            .orientation()
            .map(from_image_orientation)
            .unwrap_or_else(|_| ImageOrientation::default());
        let mut image = DynamicImage::from_decoder(decoder)
            .map_err(|error| preview_issue(file, "image_decode_failed", error))?;
        apply_image_orientation(&mut image, orientation);
        let width = image.width();
        let height = image.height();

        if !artifact_path.exists() {
            let thumbnail = image.thumbnail(edge, edge);
            let temporary_path = artifact_path.with_extension(format!(
                "{}-{}.tmp",
                std::process::id(),
                TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            if let Err(error) = thumbnail.save_with_format(&temporary_path, ImageFormat::Jpeg) {
                let _ = fs::remove_file(&temporary_path);
                return Err(preview_issue(file, "preview_write_failed", error));
            }
            let preview_size = match temporary_path.metadata() {
                Ok(metadata) => metadata.len(),
                Err(error) => {
                    let _ = fs::remove_file(&temporary_path);
                    return Err(preview_issue(file, "preview_size_unavailable", error));
                }
            };
            if !self.reserve(preview_size) {
                let _ = fs::remove_file(&temporary_path);
                return Err(ScanIssue {
                    path: Some(file.absolute_path.clone()),
                    code: "preview_cache_budget_exceeded".to_owned(),
                    message: format!(
                        "The preview cache budget of {} bytes is exhausted",
                        self.budget_bytes
                    ),
                });
            }
            if let Err(error) = fs::rename(&temporary_path, &artifact_path) {
                self.release(preview_size);
                if is_valid_cached_artifact(&artifact_path, edge) {
                    let _ = fs::remove_file(&temporary_path);
                } else {
                    let _ = fs::remove_file(&temporary_path);
                    return Err(preview_issue(file, "preview_publish_failed", error));
                }
            }
        }

        Ok(PreviewArtifact {
            path: artifact_path.to_string_lossy().into_owned(),
            width,
            height,
        })
    }
}

impl LocalPreviewStore {
    fn reserve(&self, preview_size: u64) -> bool {
        let mut used_bytes = self.used_bytes.load(Ordering::Acquire);
        loop {
            let next = used_bytes.saturating_add(preview_size);
            if next > self.budget_bytes {
                return false;
            }
            match self.used_bytes.compare_exchange_weak(
                used_bytes,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(observed) => used_bytes = observed,
            }
        }
    }

    fn release(&self, preview_size: u64) {
        let mut used_bytes = self.used_bytes.load(Ordering::Acquire);
        loop {
            let next = used_bytes.saturating_sub(preview_size);
            match self.used_bytes.compare_exchange_weak(
                used_bytes,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(observed) => used_bytes = observed,
            }
        }
    }
}

pub(crate) fn is_current_preview_artifact(path: &str) -> bool {
    let Some(file_name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(hash) = file_name
        .strip_prefix(PREVIEW_ALGORITHM)
        .and_then(|suffix| suffix.strip_prefix('-'))
        .and_then(|suffix| suffix.strip_suffix(".jpg"))
    else {
        return false;
    };
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_valid_cached_artifact(path: &Path, edge: u32) -> bool {
    let Ok(mut reader) = ImageReader::open(path).and_then(|reader| reader.with_guessed_format())
    else {
        return false;
    };
    if reader.format() != Some(ImageFormat::Jpeg) {
        return false;
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(edge);
    limits.max_image_height = Some(edge);
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    reader.limits(limits);
    let Ok(image) = reader.decode() else {
        return false;
    };
    let (width, height) = (image.width(), image.height());
    width > 0 && height > 0 && width <= edge && height <= edge
}

fn cache_usage(root: &Path) -> Result<u64, ScanIssue> {
    let mut used_bytes = 0_u64;
    let entries = fs::read_dir(root).map_err(|error| ScanIssue {
        path: Some(root.to_string_lossy().into_owned()),
        code: "preview_cache_usage_unavailable".to_owned(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ScanIssue {
            path: Some(root.to_string_lossy().into_owned()),
            code: "preview_cache_usage_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let metadata = entry.metadata().map_err(|error| ScanIssue {
            path: Some(entry.path().to_string_lossy().into_owned()),
            code: "preview_cache_usage_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if metadata.is_file() {
            used_bytes = used_bytes.saturating_add(metadata.len());
        }
    }
    Ok(used_bytes)
}

fn preview_issue(file: &DiscoveredFile, code: &str, error: impl std::fmt::Display) -> ScanIssue {
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
    use image::{ExtendedColorType, GenericImageView, ImageEncoder, Rgb, RgbImage, Rgba};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn exhausted_budget_does_not_publish_or_modify_media() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 32, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let source_before = fs::read(&source_path).expect("source before");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1).expect("preview store");

        let issue = store
            .materialize(&file, 256, 32, 32)
            .expect_err("budget issue");

        assert_eq!(issue.code, "preview_cache_budget_exceeded");
        assert_eq!(fs::read(&source_path).expect("source after"), source_before);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            0
        );
    }

    #[test]
    fn existing_artifact_uses_catalog_dimensions_without_decoding_source() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.jpg");
        let source_bytes = b"not a decodable image";
        fs::write(&source_path, source_bytes).expect("write source");
        let file = DiscoveredFile {
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.jpg".to_owned(),
            file_size: u64::try_from(source_bytes.len()).expect("source size"),
            created_unix_ms: None,
            modified_unix_ms: 7,
            file_identity: None,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1024).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        RgbImage::from_pixel(16, 16, Rgb([12, 34, 56]))
            .save(&artifact_path)
            .expect("write preview artifact");
        drop(store);
        let store = LocalPreviewStore::new(preview_root, 1024).expect("reopen preview store");

        let preview = store
            .materialize(&file, 256, 4032, 3024)
            .expect("reuse cached preview");

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert_eq!((preview.width, preview.height), (4032, 3024));
        assert_eq!(fs::read(source_path).expect("source after"), source_bytes);
    }

    #[test]
    fn corrupt_cached_artifact_is_rebuilt_from_source() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([48, 96, 144]))
            .save(&source_path)
            .expect("source image");
        let source_before = fs::read(&source_path).expect("source before");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 11,
            file_identity: None,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 4096).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        write_truncated_jpeg(&artifact_path);
        drop(store);
        let store = LocalPreviewStore::new(preview_root, 4096).expect("reopen preview store");

        let preview = store
            .materialize(&file, 256, 32, 24)
            .expect("rebuild preview");

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert!(is_valid_cached_artifact(&artifact_path, 256));
        assert_eq!(fs::read(&source_path).expect("source after"), source_before);
    }

    #[test]
    fn exif_orientation_transforms_preview_pixels_dimensions_and_cache_identity() {
        let cases = [
            (1, (80, 60), (256, 192), [RED, GREEN, BLUE, YELLOW]),
            (3, (80, 60), (256, 192), [YELLOW, BLUE, GREEN, RED]),
            (5, (60, 80), (192, 256), [RED, BLUE, GREEN, YELLOW]),
            (6, (60, 80), (192, 256), [BLUE, RED, YELLOW, GREEN]),
            (8, (60, 80), (192, 256), [GREEN, YELLOW, RED, BLUE]),
            (9, (80, 60), (256, 192), [RED, GREEN, BLUE, YELLOW]),
        ];

        for (orientation, expected_dimensions, expected_preview_dimensions, expected_corners) in
            cases
        {
            let storage = tempdir().expect("storage");
            let source_path = storage
                .path()
                .join(format!("orientation-{orientation}.jpg"));
            let source_bytes = orientation_jpeg(orientation);
            fs::write(&source_path, &source_bytes).expect("write orientation fixture");
            let file = DiscoveredFile {
                absolute_path: source_path.to_string_lossy().into_owned(),
                relative_path: format!("orientation-{orientation}.jpg"),
                file_size: u64::try_from(source_bytes.len()).expect("source size"),
                created_unix_ms: None,
                modified_unix_ms: i64::from(orientation),
                file_identity: None,
                issues: Vec::new(),
            };
            let preview_root = storage.path().join("previews");
            let store = LocalPreviewStore::new(preview_root, 1024 * 1024)
                .expect("orientation preview store");

            let preview = store
                .materialize(&file, 256, 80, 60)
                .expect("oriented preview");
            let preview_path = PathBuf::from(&preview.path);
            let rendered = image::open(&preview_path).expect("decode oriented preview");

            assert_eq!((preview.width, preview.height), expected_dimensions);
            assert_eq!(rendered.dimensions(), expected_preview_dimensions);
            assert_corners_near(&rendered, expected_corners);
            assert!(is_current_preview_artifact(&preview.path));
            assert_eq!(
                fs::read(&source_path).expect("source after preview"),
                source_bytes,
            );

            let legacy_key = preview_cache_key("ame-jpeg-thumbnail-v1", &file, 256);
            let current_key = preview_cache_key(PREVIEW_ALGORITHM, &file, 256);
            assert_ne!(legacy_key, current_key);
            let legacy_path = storage.path().join(format!("{}.jpg", legacy_key.to_hex()));
            assert!(!is_current_preview_artifact(&legacy_path.to_string_lossy()));
        }
    }

    const RED: [u8; 3] = [240, 24, 24];
    const GREEN: [u8; 3] = [24, 220, 24];
    const BLUE: [u8; 3] = [24, 24, 240];
    const YELLOW: [u8; 3] = [240, 220, 24];

    fn orientation_jpeg(orientation: u16) -> Vec<u8> {
        let image = RgbImage::from_fn(80, 60, |x, y| match (x < 40, y < 30) {
            (true, true) => Rgb(RED),
            (false, true) => Rgb(GREEN),
            (true, false) => Rgb(BLUE),
            (false, false) => Rgb(YELLOW),
        });
        let mut exif_writer = Writer::new();
        let orientation_field = Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![orientation]),
        };
        exif_writer.push_field(&orientation_field);
        let mut exif = Cursor::new(Vec::new());
        exif_writer.write(&mut exif, false).expect("encode EXIF");

        let mut jpeg = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 100);
        encoder
            .set_exif_metadata(exif.into_inner())
            .expect("set orientation EXIF");
        encoder
            .encode(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("encode orientation JPEG");
        jpeg
    }

    fn assert_corners_near(image: &DynamicImage, expected: [[u8; 3]; 4]) {
        let (width, height) = image.dimensions();
        let points = [
            (width / 4, height / 4),
            (width * 3 / 4, height / 4),
            (width / 4, height * 3 / 4),
            (width * 3 / 4, height * 3 / 4),
        ];
        for ((x, y), expected) in points.into_iter().zip(expected) {
            let actual = image.get_pixel(x, y);
            assert_color_near(actual, expected);
        }
    }

    fn assert_color_near(actual: Rgba<u8>, expected: [u8; 3]) {
        for (actual, expected) in actual.0[..3].iter().zip(expected) {
            assert!(
                actual.abs_diff(expected) <= 32,
                "actual color {:?} differs from expected {expected:?}",
                actual
            );
        }
    }

    fn write_truncated_jpeg(path: &Path) {
        let image = RgbImage::from_fn(32, 24, |x, y| {
            Rgb([
                (x * 7 + y * 3) as u8,
                (x * 5 + y * 11) as u8,
                (x * 13 + y * 17) as u8,
            ])
        });
        image.save(path).expect("complete jpeg");
        let jpeg = fs::read(path).expect("jpeg bytes");
        fs::write(path, &jpeg[..jpeg.len() / 2]).expect("truncated jpeg");
        assert!(!is_valid_cached_artifact(path, 256));
    }
}

use std::io::Cursor;

use image::{AnimationDecoder, GenericImageView};
use tempfile::TempDir;

use crate::media_fixtures::{
    ALL_MEDIA_FORMATS, MediaFixtureFormat, QUADRANT_COLORS, encode_animated_gif,
    encode_cmyk_gray_jpeg, encode_rgb_quadrants, encode_rgba_quadrants, invalid_jpeg_scan,
    truncated_header, truncated_pixels,
};

use super::*;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const EDGE: u32 = 128;

pub(crate) fn seed_legacy_jpeg_preview(
    store: &LocalPreviewStore,
    file: &DiscoveredFile,
    source: &File,
    edge: u32,
) -> PreviewArtifact {
    // Reproduce the previous permissive JPEG path with the real old decoder and current key.
    let mut source = source.try_clone().expect("legacy source clone");
    source
        .seek(SeekFrom::Start(0))
        .expect("legacy source rewind");
    let image = ImageReader::with_format(BufReader::new(source), ImageFormat::Jpeg)
        .decode()
        .expect("legacy permissive decode");
    let source_dimensions = (image.width(), image.height());
    let staged = publish_preview(
        store,
        file,
        PreviewPublication {
            artifact_key: store.artifact_key(file, edge),
            artifact_path: store.artifact_path(file, edge),
            edge,
            image,
            source_dimensions,
            force_regenerate: false,
        },
    )
    .expect("legacy staging");
    store.commit(staged).expect("legacy commit")
}

struct PreviewFixture {
    _directory: TempDir,
    source_path: PathBuf,
    source_bytes: Vec<u8>,
    file: DiscoveredFile,
    store: LocalPreviewStore,
}

impl PreviewFixture {
    fn new(extension: &str, source_bytes: Vec<u8>) -> Self {
        let directory = tempfile::tempdir().expect("media contract directory");
        let source_root = directory.path().join("source");
        fs::create_dir(&source_root).expect("source directory");
        let relative_path = format!("色块.{extension}");
        let source_path = source_root.join(&relative_path);
        fs::write(&source_path, &source_bytes).expect("encoded media fixture");
        let file = DiscoveredFile {
            source_root_path: crate::adapters::canonical_source_root_path(&source_root)
                .expect("canonical source root")
                .to_string_lossy()
                .into_owned(),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path,
            file_size: u64::try_from(source_bytes.len()).expect("fixture size"),
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let store = LocalPreviewStore::new(directory.path().join("previews"), 1024 * 1024)
            .expect("preview store");
        Self {
            _directory: directory,
            source_path,
            source_bytes,
            file,
            store,
        }
    }

    fn materialize(&self) -> Result<PreviewMaterialization, ScanIssue> {
        let source = File::open(&self.source_path).expect("open fixture source");
        self.store
            .materialize(&self.file, &source, EDGE, WIDTH, HEIGHT, false)
    }

    fn assert_source_unchanged(&self) {
        assert_eq!(
            fs::read(&self.source_path).expect("source bytes"),
            self.source_bytes
        );
    }

    fn assert_failed_without_artifacts(&self) {
        self.assert_failure_code_without_artifacts("image_decode_failed");
    }

    fn assert_failure_code_without_artifacts(&self, expected_code: &str) {
        let issue = self
            .materialize()
            .expect_err("incomplete media must not produce a preview");
        assert_eq!(
            issue.code, expected_code,
            "incomplete encoded input must fail decoding, not storage or publication: {issue:?}"
        );
        assert!(!self.store.artifact_path(&self.file, EDGE).exists());
        assert_eq!(self.store.used_bytes(), 0);
        assert_eq!(self.store.take_rejected_reservation_bytes(), 0);
        assert!(
            self.store
                .staging_targets
                .lock()
                .expect("staging ownership")
                .is_empty()
        );
        assert_eq!(
            fs::read_dir(&self.store.root)
                .expect("preview files")
                .count(),
            0
        );
        self.assert_source_unchanged();
    }

    fn commit_preview(&self) -> PreviewArtifact {
        let staged = self.materialize().expect("decode and stage valid media");
        assert!(
            staged
                .staged_path
                .as_ref()
                .is_some_and(|path| Path::new(path).is_file())
        );
        assert!(!Path::new(&staged.artifact.path).exists());
        assert!(staged.reserved_bytes > 0);
        assert_eq!(self.store.used_bytes(), staged.reserved_bytes);
        let artifact = self.store.commit(staged).expect("commit staged preview");
        assert_eq!(self.store.used_bytes(), artifact.byte_size);
        assert_eq!((artifact.width, artifact.height), (WIDTH, HEIGHT));
        assert_eq!((artifact.encoded_width, artifact.encoded_height), (128, 96));
        assert!(self.store.has_usable_artifact(&artifact.path));
        assert_eq!(
            fs::read_dir(&self.store.root)
                .expect("committed files")
                .count(),
            1
        );
        self.assert_source_unchanged();
        artifact
    }
}

fn assert_corner_colors(image: &DynamicImage, colors: [[u8; 3]; 4]) {
    let (width, height) = image.dimensions();
    let points = [
        (width / 4, height / 4),
        (3 * width / 4, height / 4),
        (width / 4, 3 * height / 4),
        (3 * width / 4, 3 * height / 4),
    ];
    for ((x, y), expected) in points.into_iter().zip(colors) {
        let actual = image.get_pixel(x, y).0;
        for channel in 0..3 {
            assert!(
                actual[channel].abs_diff(expected[channel]) <= 24,
                "corner ({x}, {y}) channel {channel}: actual={actual:?} expected={expected:?}"
            );
        }
    }
}

fn assert_format_contract(format: MediaFixtureFormat) {
    let valid = encode_rgb_quadrants(format, WIDTH, HEIGHT).expect("encode valid format");
    assert_eq!(
        valid,
        encode_rgb_quadrants(format, WIDTH, HEIGHT).expect("deterministic encoding")
    );
    assert_eq!(
        image::guess_format(&valid).expect("real encoded magic"),
        format.image_format()
    );
    let decoded_source = image::load_from_memory(&valid).expect("fixture source decodes");
    assert_eq!(decoded_source.dimensions(), (WIDTH, HEIGHT));
    if format == MediaFixtureFormat::Ico {
        assert_eq!(decoded_source.color(), image::ColorType::Rgba8);
    }
    assert_corner_colors(&decoded_source, QUADRANT_COLORS);
    let wrong_extension = if format == MediaFixtureFormat::Jpeg {
        "png"
    } else {
        "jpg"
    };
    for extension in [format.extension(), wrong_extension, "data"] {
        let fixture = PreviewFixture::new(extension, valid.clone());
        let artifact = fixture.commit_preview();
        let bytes = fs::read(&artifact.path).expect("committed JPEG bytes");
        assert_eq!(
            image::guess_format(&bytes).expect("preview encoding"),
            ImageFormat::Jpeg
        );
        let rendered = image::load_from_memory(&bytes).expect("committed preview decodes");
        assert_eq!(rendered.dimensions(), (128, 96));
        assert_corner_colors(&rendered, QUADRANT_COLORS);

        let reused = fixture.materialize().expect("reuse preview");
        assert!(reused.staged_path.is_none());
        assert_eq!(reused.reserved_bytes, 0);
        assert_eq!(reused.artifact.path, artifact.path);
        fixture.store.commit(reused).expect("commit cache hit");
        assert_eq!(fs::read(&artifact.path).expect("unchanged cache"), bytes);
        assert_eq!(fixture.store.used_bytes(), artifact.byte_size);
        fixture.assert_source_unchanged();

        PreviewFixture::new(extension, truncated_header(format)).assert_failed_without_artifacts();
        PreviewFixture::new(extension, truncated_pixels(format, &valid))
            .assert_failed_without_artifacts();
    }
}

macro_rules! format_contract {
    ($name:ident, $format:ident) => {
        #[test]
        fn $name() {
            assert_format_contract(MediaFixtureFormat::$format);
        }
    };
}

format_contract!(
    jpeg_decode_stage_commit_reuse_and_incomplete_input_contract,
    Jpeg
);
format_contract!(
    png_decode_stage_commit_reuse_and_incomplete_input_contract,
    Png
);
format_contract!(
    webp_decode_stage_commit_reuse_and_incomplete_input_contract,
    WebP
);
format_contract!(
    gif_decode_stage_commit_reuse_and_incomplete_input_contract,
    Gif
);
format_contract!(
    bmp_decode_stage_commit_reuse_and_incomplete_input_contract,
    Bmp
);
format_contract!(
    tiff_decode_stage_commit_reuse_and_incomplete_input_contract,
    Tiff
);
format_contract!(
    ico_decode_stage_commit_reuse_and_incomplete_input_contract,
    Ico
);

#[test]
fn empty_non_image_and_forged_magic_leave_no_artifact_or_budget_debt() {
    for bytes in [Vec::new(), b"this is text, not an encoded image".to_vec()] {
        PreviewFixture::new("jpg", bytes)
            .assert_failure_code_without_artifacts("image_format_unsupported");
    }
    for format in ALL_MEDIA_FORMATS {
        let mut forged = truncated_header(format);
        forged.extend_from_slice(&[0; 64]);
        let expected_code = if format == MediaFixtureFormat::Ico {
            "image_format_unsupported"
        } else {
            "image_decode_failed"
        };
        PreviewFixture::new(format.extension(), forged)
            .assert_failure_code_without_artifacts(expected_code);
    }
}

#[test]
fn invalid_jpeg_scan_keeps_frame_dimensions_but_both_decoders_reject_its_tables() {
    let valid =
        encode_rgb_quadrants(MediaFixtureFormat::Jpeg, WIDTH, HEIGHT).expect("valid JPEG fixture");
    let damaged = invalid_jpeg_scan(&valid);
    let mut scaled = jpeg_decoder::Decoder::new(Cursor::new(&damaged));
    scaled.read_info().expect("frame header is intact");
    let info = scaled.info().expect("JPEG dimensions");
    assert_eq!(
        (u32::from(info.width), u32::from(info.height)),
        (WIDTH, HEIGHT)
    );
    assert!(
        scaled.decode().is_err(),
        "scaled decoder must reject missing tables"
    );
    assert!(
        image::load_from_memory(&damaged).is_err(),
        "fallback must reject missing tables"
    );
}

#[test]
fn rgba_inputs_characterize_current_jpeg_rgb_projection_without_changing_sources() {
    // This characterizes alpha removal in the current JPEG cache, not a product background policy.
    for format in [MediaFixtureFormat::Png, MediaFixtureFormat::WebP] {
        let bytes = encode_rgba_quadrants(format, WIDTH, HEIGHT, [0, 128, 255, 64])
            .expect("transparent fixture");
        let source = image::load_from_memory(&bytes).expect("RGBA source");
        let points = [(16, 12), (48, 12), (16, 36), (48, 36)];
        let mut projected_colors = [[0; 3]; 4];
        for (index, ((x, y), alpha)) in points.into_iter().zip([0, 128, 255, 64]).enumerate() {
            let pixel = source.get_pixel(x, y).0;
            assert_eq!(pixel[3], alpha);
            projected_colors[index].copy_from_slice(&pixel[..3]);
        }
        let fixture = PreviewFixture::new(format.extension(), bytes);
        let artifact = fixture.commit_preview();
        let rendered = image::open(&artifact.path).expect("RGB preview");
        assert!(!rendered.color().has_alpha());
        assert_corner_colors(&rendered, projected_colors);
        fixture.assert_source_unchanged();
    }
}

#[test]
fn validated_cmyk_fallback_publishes_pixels_but_truncated_cmyk_leaves_no_artifact() {
    let valid = encode_cmyk_gray_jpeg().expect("valid constant CMYK fixture");
    let source = image::load_from_memory(&valid).expect("CMYK source decodes");
    assert_eq!(source.dimensions(), (WIDTH, HEIGHT));
    assert_corner_colors(&source, [[64; 3]; 4]);
    let fixture = PreviewFixture::new("data", valid.clone());
    let artifact = fixture.commit_preview();
    assert_corner_colors(
        &image::open(&artifact.path).expect("CMYK preview"),
        [[64; 3]; 4],
    );
    PreviewFixture::new("jpg", truncated_pixels(MediaFixtureFormat::Jpeg, &valid))
        .assert_failed_without_artifacts();
}

#[test]
fn animated_gif_materializes_a_static_first_frame_without_mutating_animation() {
    let bytes = encode_animated_gif(WIDTH, HEIGHT).expect("two-frame GIF");
    let frames = image::codecs::gif::GifDecoder::new(Cursor::new(&bytes))
        .expect("animated source")
        .into_frames()
        .collect_frames()
        .expect("all source frames");
    assert_eq!(frames.len(), 2);
    assert_ne!(frames[0].buffer(), frames[1].buffer());
    let fixture = PreviewFixture::new("data", bytes);
    let artifact = fixture.commit_preview();
    assert_corner_colors(
        &image::open(&artifact.path).expect("static preview"),
        QUADRANT_COLORS,
    );
    fixture.assert_source_unchanged();
}

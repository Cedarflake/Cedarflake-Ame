use std::io::Cursor;

use image::GenericImageView;

use crate::media_fixtures::{
    MediaFixtureFormat, encode_cmyk_gray_jpeg, encode_rgb_quadrants, truncated_pixels,
};

use super::*;

fn decode_fixture(bytes: &[u8], budget: u64) -> Result<JpegPreviewDecode, JpegPreviewError> {
    let directory = tempfile::tempdir().expect("JPEG contract directory");
    let path = directory.path().join("source.jpg");
    std::fs::write(&path, bytes).expect("JPEG source");
    let result = decode_scaled_jpeg(File::open(&path).expect("open JPEG"), 128, budget);
    assert_eq!(
        std::fs::read(path).expect("source remains unchanged"),
        bytes
    );
    result
}

#[test]
fn complete_rgb_keeps_the_scaled_decoder_path() {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("RGB JPEG");
    let JpegPreviewDecode::Decoded(decoded) =
        decode_fixture(&bytes, 256 * 1024 * 1024).expect("valid RGB decode")
    else {
        panic!("RGB does not need fallback")
    };
    assert_eq!((decoded.source_width, decoded.source_height), (64, 48));
}

#[test]
fn complete_cmyk_is_validated_before_requesting_the_general_decoder() {
    let bytes = encode_cmyk_gray_jpeg().expect("CMYK JPEG");
    let mut native = Decoder::new(Cursor::new(&bytes));
    native.read_info().expect("CMYK header");
    assert_eq!(
        native.info().expect("source mode").pixel_format,
        PixelFormat::CMYK32
    );
    assert_eq!(
        native.decode().expect("complete CMYK pixels").len(),
        64 * 48 * 4
    );
    assert_eq!(
        image::load_from_memory(&bytes)
            .expect("general decoder supports CMYK")
            .dimensions(),
        (64, 48)
    );
    assert!(matches!(
        decode_fixture(&bytes, 256 * 1024 * 1024),
        Ok(JpegPreviewDecode::UnsupportedPixelFormat)
    ));
}

#[test]
fn missing_rgb_and_cmyk_entropy_never_request_permissive_fallback() {
    for bytes in [
        encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("RGB JPEG"),
        encode_cmyk_gray_jpeg().expect("CMYK JPEG"),
    ] {
        let truncated = truncated_pixels(MediaFixtureFormat::Jpeg, &bytes);
        assert!(
            decode_fixture(&truncated, 256 * 1024 * 1024).is_err(),
            "missing entropy must remain a decoding failure"
        );
    }
}

#[test]
fn decoding_buffer_limit_does_not_request_fallback() {
    for bytes in [
        encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 64, 48).expect("RGB JPEG"),
        encode_cmyk_gray_jpeg().expect("CMYK JPEG"),
    ] {
        assert!(
            matches!(
                decode_fixture(&bytes, 1),
                Err(JpegPreviewError::ResourceLimit)
            ),
            "fallback must not bypass the decoding budget"
        );
    }
}

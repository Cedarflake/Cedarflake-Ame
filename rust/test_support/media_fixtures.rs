use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageResult, Rgb, RgbImage, Rgba, RgbaImage};

#[path = "media_fixtures/cmyk.rs"]
mod cmyk;
pub(crate) use cmyk::encode_cmyk_gray_jpeg;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MediaFixtureFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

pub(crate) const ALL_MEDIA_FORMATS: [MediaFixtureFormat; 7] = [
    MediaFixtureFormat::Jpeg,
    MediaFixtureFormat::Png,
    MediaFixtureFormat::WebP,
    MediaFixtureFormat::Gif,
    MediaFixtureFormat::Bmp,
    MediaFixtureFormat::Tiff,
    MediaFixtureFormat::Ico,
];

pub(crate) const QUADRANT_COLORS: [[u8; 3]; 4] =
    [[220, 32, 48], [32, 200, 64], [32, 64, 224], [224, 192, 32]];

impl MediaFixtureFormat {
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
        }
    }

    pub(crate) fn image_format(self) -> ImageFormat {
        match self {
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Png => ImageFormat::Png,
            Self::WebP => ImageFormat::WebP,
            Self::Gif => ImageFormat::Gif,
            Self::Bmp => ImageFormat::Bmp,
            Self::Tiff => ImageFormat::Tiff,
            Self::Ico => ImageFormat::Ico,
        }
    }
}

pub(crate) fn encode_rgb_quadrants(
    format: MediaFixtureFormat,
    width: u32,
    height: u32,
) -> ImageResult<Vec<u8>> {
    encode_rgb_quadrants_with_colors(format, width, height, QUADRANT_COLORS)
}

pub(crate) fn encode_rgb_quadrants_with_colors(
    format: MediaFixtureFormat,
    width: u32,
    height: u32,
    colors: [[u8; 3]; 4],
) -> ImageResult<Vec<u8>> {
    let pixels = RgbImage::from_fn(width, height, |x, y| {
        Rgb(colors[quadrant(x, y, width, height)])
    });
    encode(DynamicImage::ImageRgb8(pixels), format)
}

pub(crate) fn encode_rgba_quadrants(
    format: MediaFixtureFormat,
    width: u32,
    height: u32,
    alpha: [u8; 4],
) -> ImageResult<Vec<u8>> {
    let pixels = RgbaImage::from_fn(width, height, |x, y| {
        let index = quadrant(x, y, width, height);
        let [r, g, b] = QUADRANT_COLORS[index];
        Rgba([r, g, b, alpha[index]])
    });
    encode(DynamicImage::ImageRgba8(pixels), format)
}

pub(crate) fn encode_animated_gif(width: u32, height: u32) -> ImageResult<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;
        for inverted in [false, true] {
            let pixels = RgbaImage::from_fn(width, height, |x, y| {
                let [r, g, b] = QUADRANT_COLORS[quadrant(x, y, width, height)];
                if inverted {
                    Rgba([255 - r, 255 - g, 255 - b, 255])
                } else {
                    Rgba([r, g, b, 255])
                }
            });
            encoder.encode_frame(image::Frame::from_parts(
                pixels,
                0,
                0,
                image::Delay::from_numer_denom_ms(80, 1),
            ))?;
        }
    }
    Ok(bytes)
}

pub(crate) fn truncated_header(format: MediaFixtureFormat) -> Vec<u8> {
    match format {
        MediaFixtureFormat::Jpeg => b"\xff\xd8\xff".to_vec(),
        MediaFixtureFormat::Png => b"\x89PNG\r\n\x1a\n".to_vec(),
        MediaFixtureFormat::WebP => b"RIFF\x20\0\0\0WEBP".to_vec(),
        MediaFixtureFormat::Gif => b"GIF89a".to_vec(),
        MediaFixtureFormat::Bmp => b"BM".to_vec(),
        MediaFixtureFormat::Tiff => b"II*\0".to_vec(),
        MediaFixtureFormat::Ico => b"\0\0\x01\0\x01\0".to_vec(),
    }
}

pub(crate) fn truncated_pixels(format: MediaFixtureFormat, valid: &[u8]) -> Vec<u8> {
    let end = match format {
        MediaFixtureFormat::Jpeg => {
            let sos = valid
                .windows(2)
                .position(|bytes| bytes == b"\xff\xda")
                .expect("JPEG SOS");
            sos + 2 + usize::from(u16::from_be_bytes([valid[sos + 2], valid[sos + 3]]))
        }
        MediaFixtureFormat::Png => png_payload_start(valid),
        MediaFixtureFormat::WebP => {
            let chunk = valid
                .windows(4)
                .position(|bytes| bytes == b"VP8L")
                .expect("lossless WebP chunk");
            chunk + 8 + 5
        }
        MediaFixtureFormat::Gif => gif_payload_start(valid),
        MediaFixtureFormat::Bmp => {
            u32::from_le_bytes(valid[10..14].try_into().expect("BMP offset")) as usize
        }
        MediaFixtureFormat::Tiff => return missing_tiff_strip(valid),
        MediaFixtureFormat::Ico => {
            let png =
                u32::from_le_bytes(valid[18..22].try_into().expect("ICO image offset")) as usize;
            png + png_payload_start(&valid[png..])
        }
    };
    assert!(end < valid.len(), "fixture must lose pixel payload");
    valid[..end].to_vec()
}

fn quadrant(x: u32, y: u32, width: u32, height: u32) -> usize {
    usize::from(x >= width / 2) + 2 * usize::from(y >= height / 2)
}

fn encode(image: DynamicImage, format: MediaFixtureFormat) -> ImageResult<Vec<u8>> {
    // PNG-backed ICO entries require RGBA even when every source pixel is opaque.
    let image = if format == MediaFixtureFormat::Ico {
        DynamicImage::ImageRgba8(image.into_rgba8())
    } else {
        image
    };
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format.image_format())?;
    Ok(bytes.into_inner())
}

pub(crate) fn invalid_jpeg_scan(valid: &[u8]) -> Vec<u8> {
    let sos = valid
        .windows(2)
        .position(|bytes| bytes == b"\xff\xda")
        .expect("JPEG SOS");
    let mut damaged = truncated_pixels(MediaFixtureFormat::Jpeg, valid);
    // The fallback tolerates early entropy EOF. An absent DC/AC table 3 is a
    // structural scan error they must reject, independent of their EOF recovery policy.
    damaged[sos + 6] = 0x33;
    damaged
}

fn png_payload_start(bytes: &[u8]) -> usize {
    let mut offset = 8;
    loop {
        let length =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("PNG chunk")) as usize;
        if &bytes[offset + 4..offset + 8] == b"IDAT" {
            return offset + 8;
        }
        offset += 12 + length;
    }
}

fn gif_payload_start(bytes: &[u8]) -> usize {
    let packed = bytes[10];
    let mut offset = 13
        + if packed & 0x80 != 0 {
            3 * (1 << ((packed & 7) + 1))
        } else {
            0
        };
    loop {
        match bytes[offset] {
            0x21 => {
                offset += 2;
                while bytes[offset] != 0 {
                    offset += 1 + usize::from(bytes[offset]);
                }
                offset += 1;
            }
            0x2c => {
                let packed = bytes[offset + 9];
                offset += 10
                    + if packed & 0x80 != 0 {
                        3 * (1 << ((packed & 7) + 1))
                    } else {
                        0
                    };
                return offset + 2;
            }
            _ => panic!("expected generated GIF image descriptor"),
        }
    }
}

fn missing_tiff_strip(valid: &[u8]) -> Vec<u8> {
    // TIFF's IFD follows its pixels. Preserve that directory but point its first strip
    // beyond EOF, so this fixture tests missing pixel data rather than a missing IFD.
    let little = &valid[..2] == b"II";
    let read_u16 = |offset| {
        let bytes = [valid[offset], valid[offset + 1]];
        if little {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        }
    };
    let read_u32 = |offset| {
        let bytes = valid[offset..offset + 4].try_into().expect("TIFF offset");
        if little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    };
    let ifd = read_u32(4) as usize;
    let entry = (0..usize::from(read_u16(ifd)))
        .map(|index| ifd + 2 + index * 12)
        .find(|offset| read_u16(*offset) == 273)
        .expect("TIFF strip offsets");
    assert_eq!(read_u16(entry + 2), 4, "generated TIFF LONG strip offsets");
    let offset = if read_u32(entry + 4) == 1 {
        entry + 8
    } else {
        read_u32(entry + 8) as usize
    };
    let missing = u32::try_from(valid.len())
        .expect("small TIFF fixture")
        .checked_add(64)
        .expect("missing strip offset");
    let mut damaged = valid.to_vec();
    damaged[offset..offset + 4].copy_from_slice(&if little {
        missing.to_le_bytes()
    } else {
        missing.to_be_bytes()
    });
    damaged
}

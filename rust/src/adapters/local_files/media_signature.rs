use std::io::{self, Read};
use std::path::Path;

use image::ImageFormat;

const IMAGE_EXTENSIONS: &[&str] = &[
    "bmp", "gif", "ico", "jpeg", "jpg", "png", "tif", "tiff", "webp",
];
const SIGNATURE_BYTES: u8 = 16;

pub(super) fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

pub(super) fn has_supported_magic_from_reader(reader: &mut impl Read) -> io::Result<bool> {
    let mut header = Vec::with_capacity(usize::from(SIGNATURE_BYTES));
    reader
        .take(u64::from(SIGNATURE_BYTES))
        .read_to_end(&mut header)?;
    // Recognition admits a candidate, not valid pixels. Keep the admitted decoder set explicit.
    Ok(matches!(
        image::guess_format(&header),
        Ok(ImageFormat::Bmp
            | ImageFormat::Gif
            | ImageFormat::Ico
            | ImageFormat::Jpeg
            | ImageFormat::Png
            | ImageFormat::Tiff
            | ImageFormat::WebP)
    ))
}

#[cfg(test)]
#[path = "media_signature_tests.rs"]
mod tests;

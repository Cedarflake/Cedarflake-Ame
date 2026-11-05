use image::imageops::FilterType;
use image::{DynamicImage, Rgb, RgbImage};

pub(super) fn next_value(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

pub(super) fn scene(seed: u64) -> DynamicImage {
    let mut state = seed;
    let colors: [[u8; 3]; 4] =
        std::array::from_fn(|_| std::array::from_fn(|_| next_value(&mut state) as u8));
    let mut pixels = RgbImage::from_fn(320, 240, |x, y| {
        Rgb(std::array::from_fn(|channel| {
            let top = u32::from(colors[0][channel]) * (319 - x) + u32::from(colors[1][channel]) * x;
            let bottom =
                u32::from(colors[2][channel]) * (319 - x) + u32::from(colors[3][channel]) * x;
            ((top * (239 - y) + bottom * y) / (319 * 239)) as u8
        }))
    });
    for _ in 0..32 {
        let cx = (next_value(&mut state) % 320) as i32;
        let cy = (next_value(&mut state) % 240) as i32;
        let rx = (next_value(&mut state) % 80 + 8) as i32;
        let ry = (next_value(&mut state) % 60 + 8) as i32;
        let rectangle = next_value(&mut state).is_multiple_of(2);
        let color = Rgb(std::array::from_fn(|_| next_value(&mut state) as u8));
        for (x, y, pixel) in pixels.enumerate_pixels_mut() {
            let dx = (x as i32 - cx).abs();
            let dy = (y as i32 - cy).abs();
            if dx < rx
                && dy < ry
                && (rectangle || dx * dx * ry * ry + dy * dy * rx * rx < rx * rx * ry * ry)
            {
                *pixel = color;
            }
        }
    }
    DynamicImage::ImageRgb8(pixels)
}

pub(super) fn transformed(image: &DynamicImage) -> Vec<(&'static str, DynamicImage)> {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 65)
        .encode_image(image)
        .expect("encode generated JPEG");
    let mut watermark = image.to_rgb8();
    for y in 205..230 {
        for x in 230..310 {
            watermark.put_pixel(x, y, Rgb([230, 230, 230]));
        }
    }
    vec![
        (
            "jpeg65",
            image::load_from_memory(&jpeg).expect("decode generated JPEG"),
        ),
        ("resize", image.resize_exact(160, 120, FilterType::Triangle)),
        ("rotate90", image.rotate90()),
        ("mirror", image.fliph()),
        ("crop5pct", image.crop_imm(8, 6, 304, 228)),
        ("crop20pct", image.crop_imm(32, 24, 256, 192)),
        ("watermark", DynamicImage::ImageRgb8(watermark)),
    ]
}

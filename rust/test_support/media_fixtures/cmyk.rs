use super::*;

pub(crate) fn encode_cmyk_gray_jpeg() -> ImageResult<Vec<u8>> {
    // This fixed 64x48 fixture has four 1x1-sampled, constant-128 components.
    // Reuse the installed encoder's quantization/Huffman tables; each component's
    // blocks have zero DC difference (00) followed by EOB (1010), not a general codec.
    let gray = image::GrayImage::from_pixel(64, 48, image::Luma([128]));
    let valid = encode(DynamicImage::ImageLuma8(gray), MediaFixtureFormat::Jpeg)?;
    let frame = valid
        .windows(2)
        .position(|bytes| bytes == b"\xff\xc0")
        .expect("baseline frame");
    let scan = valid
        .windows(2)
        .position(|bytes| bytes == b"\xff\xda")
        .expect("baseline scan");
    let frame_end =
        frame + 2 + usize::from(u16::from_be_bytes([valid[frame + 2], valid[frame + 3]]));
    assert_eq!(valid[frame + 9], 1, "encoder supplied a grayscale frame");

    let mut bytes = b"\xff\xd8\xff\xee\0\x0eAdobe\0\x64\0\0\0\0\0".to_vec();
    bytes.extend_from_slice(b"\xff\xc0\0\x14");
    bytes.extend_from_slice(&valid[frame + 4..frame + 9]);
    bytes.push(4);
    for component in 1..=4 {
        bytes.extend_from_slice(&[component, 0x11, 0]);
    }
    bytes.extend_from_slice(&valid[frame_end..scan]);
    bytes.extend_from_slice(b"\xff\xda\0\x0e\x04\x01\0\x02\0\x03\0\x04\0\0\x3f\0");
    // Four zero-coefficient blocks occupy 24 bits: 001010 repeated four times.
    for _ in 0..(8 * 6) {
        bytes.extend_from_slice(&[0x28, 0xa2, 0x8a]);
    }
    bytes.extend_from_slice(b"\xff\xd9");
    Ok(bytes)
}

use image::DynamicImage;
use image::metadata::Orientation;

use crate::domain::ImageOrientation;

pub(crate) fn from_image_orientation(orientation: Orientation) -> ImageOrientation {
    match orientation {
        Orientation::NoTransforms => ImageOrientation::Normal,
        Orientation::FlipHorizontal => ImageOrientation::FlipHorizontal,
        Orientation::Rotate180 => ImageOrientation::Rotate180,
        Orientation::FlipVertical => ImageOrientation::FlipVertical,
        Orientation::Rotate90FlipH => ImageOrientation::Rotate90FlipHorizontal,
        Orientation::Rotate90 => ImageOrientation::Rotate90,
        Orientation::Rotate270FlipH => ImageOrientation::Rotate270FlipHorizontal,
        Orientation::Rotate270 => ImageOrientation::Rotate270,
    }
}

pub(crate) fn apply_image_orientation(image: &mut DynamicImage, orientation: ImageOrientation) {
    image.apply_orientation(match orientation {
        ImageOrientation::Normal => Orientation::NoTransforms,
        ImageOrientation::FlipHorizontal => Orientation::FlipHorizontal,
        ImageOrientation::Rotate180 => Orientation::Rotate180,
        ImageOrientation::FlipVertical => Orientation::FlipVertical,
        ImageOrientation::Rotate90FlipHorizontal => Orientation::Rotate90FlipH,
        ImageOrientation::Rotate90 => Orientation::Rotate90,
        ImageOrientation::Rotate270FlipHorizontal => Orientation::Rotate270FlipH,
        ImageOrientation::Rotate270 => Orientation::Rotate270,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_exif_orientation_values_to_ame_contract() {
        let expected = [
            ImageOrientation::Normal,
            ImageOrientation::FlipHorizontal,
            ImageOrientation::Rotate180,
            ImageOrientation::FlipVertical,
            ImageOrientation::Rotate90FlipHorizontal,
            ImageOrientation::Rotate90,
            ImageOrientation::Rotate270FlipHorizontal,
            ImageOrientation::Rotate270,
        ];

        for (value, expected) in (1..=8).zip(expected) {
            let image_orientation = Orientation::from_exif(value).expect("valid orientation");
            let orientation = from_image_orientation(image_orientation);

            assert_eq!(orientation, expected);
            let expected_dimensions = if value >= 5 { (60, 80) } else { (80, 60) };
            assert_eq!(orientation.display_dimensions(80, 60), expected_dimensions);
        }
    }
}

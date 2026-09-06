use std::io::ErrorKind;

use image::ImageError;

use crate::domain::{DiscoveredFile, ScanIssue};

use super::super::jpeg_preview::JpegPreviewError;
use super::preview_issue;

pub(super) fn image_failure(file: &DiscoveredFile, error: ImageError) -> ScanIssue {
    let code = match &error {
        ImageError::IoError(error) if error.kind() == ErrorKind::UnexpectedEof => {
            "image_decode_failed"
        }
        ImageError::Decoding(_) => "image_decode_failed",
        ImageError::IoError(_) => "image_read_failed",
        ImageError::Limits(_) => "image_decode_budget_exceeded",
        ImageError::Unsupported(_) => "image_format_unsupported",
        ImageError::Parameter(_) | ImageError::Encoding(_) => "image_decoder_failed",
    };
    preview_issue(file, code, error)
}

pub(super) fn jpeg_failure(file: &DiscoveredFile, error: JpegPreviewError) -> ScanIssue {
    let code = match &error {
        JpegPreviewError::Decoder(jpeg_decoder::Error::Format(_)) => "image_decode_failed",
        JpegPreviewError::Decoder(jpeg_decoder::Error::Io(error))
            if error.kind() == ErrorKind::UnexpectedEof =>
        {
            "image_decode_failed"
        }
        JpegPreviewError::Decoder(jpeg_decoder::Error::Io(_)) => "image_read_failed",
        JpegPreviewError::ResourceLimit => "image_decode_budget_exceeded",
        JpegPreviewError::Decoder(jpeg_decoder::Error::Unsupported(_)) => {
            "image_format_unsupported"
        }
        JpegPreviewError::Decoder(jpeg_decoder::Error::Internal(_))
        | JpegPreviewError::InvalidOutput => "image_decoder_failed",
    };
    preview_issue(file, code, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> DiscoveredFile {
        DiscoveredFile {
            source_root_path: String::new(),
            absolute_path: "fixture".to_owned(),
            relative_path: "fixture".to_owned(),
            file_size: 1,
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        }
    }

    #[test]
    fn only_exhausted_input_not_transient_reads_is_corruption_evidence() {
        for kind in [
            ErrorKind::PermissionDenied,
            ErrorKind::WouldBlock,
            ErrorKind::Interrupted,
            ErrorKind::Other,
        ] {
            assert_eq!(
                image_failure(&file(), ImageError::IoError(kind.into())).code,
                "image_read_failed"
            );
            assert_eq!(
                jpeg_failure(&file(), jpeg_decoder::Error::Io(kind.into()).into()).code,
                "image_read_failed"
            );
        }
        assert_eq!(
            image_failure(
                &file(),
                ImageError::IoError(ErrorKind::UnexpectedEof.into())
            )
            .code,
            "image_decode_failed"
        );
        assert_eq!(
            jpeg_failure(
                &file(),
                jpeg_decoder::Error::Io(ErrorKind::UnexpectedEof.into()).into()
            )
            .code,
            "image_decode_failed"
        );
    }

    #[test]
    fn decoder_budget_unsupported_features_and_internal_faults_do_not_prove_corruption() {
        assert_eq!(
            jpeg_failure(&file(), JpegPreviewError::ResourceLimit).code,
            "image_decode_budget_exceeded"
        );
        assert_eq!(
            jpeg_failure(
                &file(),
                jpeg_decoder::Error::Unsupported(
                    jpeg_decoder::UnsupportedFeature::ArithmeticEntropyCoding
                )
                .into()
            )
            .code,
            "image_format_unsupported"
        );
        assert_eq!(
            jpeg_failure(&file(), JpegPreviewError::InvalidOutput).code,
            "image_decoder_failed"
        );
        assert_eq!(
            image_failure(
                &file(),
                ImageError::Limits(image::error::LimitError::from_kind(
                    image::error::LimitErrorKind::InsufficientMemory
                ))
            )
            .code,
            "image_decode_budget_exceeded"
        );
    }
}

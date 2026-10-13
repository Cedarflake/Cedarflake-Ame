use std::io::ErrorKind;

use image::ImageError;

use crate::ports::MediaInspectionFailureKind;

pub(super) fn classify_decoder_error(
    error: &ImageError,
) -> (MediaInspectionFailureKind, &'static str) {
    use MediaInspectionFailureKind::{Retryable, Terminal};

    match error {
        // A decoder exhausting the opened bytes is content evidence, not a new open attempt.
        ImageError::IoError(error) if error.kind() == ErrorKind::UnexpectedEof => {
            (Terminal, "image_decode_invalid")
        }
        ImageError::IoError(_) => (Retryable, "image_header_read_failed"),
        ImageError::Unsupported(_) => (Terminal, "image_format_unsupported"),
        ImageError::Decoding(_) => (Terminal, "image_decode_invalid"),
        ImageError::Limits(_) => (Terminal, "image_limits_exceeded"),
        ImageError::Parameter(_) | ImageError::Encoding(_) => (Terminal, "image_decoder_rejected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_decoder_input_is_terminal_content_evidence() {
        let (kind, code) =
            classify_decoder_error(&ImageError::IoError(ErrorKind::UnexpectedEof.into()));
        assert!(matches!(kind, MediaInspectionFailureKind::Terminal));
        assert_eq!(code, "image_decode_invalid");
    }

    #[test]
    fn access_failures_do_not_become_terminal_media_evidence() {
        for error in [
            ErrorKind::PermissionDenied,
            ErrorKind::WouldBlock,
            ErrorKind::Interrupted,
            ErrorKind::NotFound,
            ErrorKind::Other,
        ] {
            let (kind, code) = classify_decoder_error(&ImageError::IoError(error.into()));
            assert!(matches!(kind, MediaInspectionFailureKind::Retryable));
            assert_eq!(code, "image_header_read_failed");
        }
    }
}

use crate::domain::{AssetLocationView, PreviewStatus, ScanIssue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FailureDisposition {
    RetainReady,
    PublishFailed,
}

pub(super) fn disposition(
    issue: &ScanIssue,
    forced_retry: bool,
    had_ready: bool,
) -> FailureDisposition {
    // Only the decoder's positive corruption evidence revokes an existing ready result.
    // Unsupported media, resource limits, and storage/read failures do not establish this.
    if had_ready && !(forced_retry && issue.code == "image_decode_failed") {
        FailureDisposition::RetainReady
    } else {
        FailureDisposition::PublishFailed
    }
}

pub(super) fn apply_failure(location: &mut AssetLocationView, issue: &ScanIssue) {
    location.preview_path.clear();
    location.preview_status = PreviewStatus::Failed;
    location.preview_issue_code = Some(issue.code.clone());
    location.preview_issue_message = Some(issue.message.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_retry_with_corruption_evidence_revokes_ready() {
        for code in [
            "image_decode_failed",
            "image_read_failed",
            "image_open_failed",
            "image_decode_budget_exceeded",
            "image_format_unsupported",
            "image_decoder_failed",
            "preview_cache_budget_exceeded",
            "preview_write_failed",
            "preview_publish_failed",
            "future_failure",
        ] {
            let issue = ScanIssue {
                path: None,
                code: code.to_owned(),
                message: String::new(),
            };
            assert_eq!(
                disposition(&issue, false, true),
                FailureDisposition::RetainReady
            );
            assert_eq!(
                disposition(&issue, true, false),
                FailureDisposition::PublishFailed
            );
            assert_eq!(
                disposition(&issue, true, true),
                if code == "image_decode_failed" {
                    FailureDisposition::PublishFailed
                } else {
                    FailureDisposition::RetainReady
                }
            );
        }
    }
}

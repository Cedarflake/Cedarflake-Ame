use crate::domain::ScanIssue;

use super::*;

#[test]
fn placeholder_revalidation_remains_distinct_from_missing_or_changed_files() {
    for (code, expected) in [
        (
            "source_became_unavailable",
            DuplicateLocationAvailability::CloudOnly,
        ),
        (
            "cloud_placeholder_skipped",
            DuplicateLocationAvailability::CloudOnly,
        ),
        (
            "source_revalidation_failed",
            DuplicateLocationAvailability::Unavailable,
        ),
        (
            "source_revision_changed",
            DuplicateLocationAvailability::Unavailable,
        ),
    ] {
        let issue = ScanIssue {
            path: None,
            code: code.to_owned(),
            message: "fixture".to_owned(),
        };
        assert_eq!(file_availability(Err(issue)), expected);
    }
    assert_eq!(
        file_availability(Ok(())),
        DuplicateLocationAvailability::Local
    );
}

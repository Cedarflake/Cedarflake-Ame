use crate::domain::{ExpectedFileState, RootAvailabilityEvidence, ScanIssue};

/// Inspect source state without reading content or recalling cloud files.
pub(crate) trait SourceStateProbe {
    fn inspect_root(&self, root_path: &str) -> RootAvailabilityEvidence;
    fn revalidate_file(&self, expected: &ExpectedFileState) -> Result<(), ScanIssue>;
}

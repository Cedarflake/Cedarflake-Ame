use crate::domain::{ExpectedFileState, RootAvailabilityEvidence, ScanIssue};
use crate::ports::source_state::SourceStateProbe;

pub(crate) struct LocalSourceState;

impl SourceStateProbe for LocalSourceState {
    fn inspect_root(&self, root_path: &str) -> RootAvailabilityEvidence {
        super::inspect_root_availability(root_path)
    }

    fn revalidate_file(&self, expected: &ExpectedFileState) -> Result<(), ScanIssue> {
        super::revalidate_file_state(expected)
    }
}

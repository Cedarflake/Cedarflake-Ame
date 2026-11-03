use std::path::Path;

use crate::domain::{ExpectedFileState, FileIdentityEvidence, ScanIssue, SourceRevisionEvidence};

use super::PreviewSource;

pub(crate) trait PreviewSourceFactory {
    type Source: PreviewSourceLease;
    type Publication: PreviewPublicationLease;

    fn open(
        &self,
        expected: &ExpectedFileState,
        source_root: &Path,
        expected_root_identity: Option<&FileIdentityEvidence>,
    ) -> Result<Self::Source, ScanIssue>;

    fn protect_publication(
        &self,
        expected: &ExpectedFileState,
        source_root: &Path,
        expected_root_identity: Option<&FileIdentityEvidence>,
    ) -> Result<Self::Publication, ScanIssue>;
}

/// Reads and revalidates the admitted handle without reopening its path or recalling cloud files.
pub(crate) trait PreviewSourceLease: PreviewSource {
    fn source_root(&self) -> &Path;
    fn source_revision(&self) -> Option<&SourceRevisionEvidence>;
    fn revalidate(&self, expected: &ExpectedFileState) -> Result<(), ScanIssue>;
}

/// Pins the source and ancestor namespace through catalog publication or rollback.
pub(crate) trait PreviewPublicationLease {
    fn revalidate(&self, expected: &ExpectedFileState) -> Result<(), ScanIssue>;
}

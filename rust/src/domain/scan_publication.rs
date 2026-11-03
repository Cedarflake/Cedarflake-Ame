use super::ConsolidatedLibraryRoot;

#[derive(Clone, Copy)]
pub(crate) enum StagedValidationOutcome {
    FilesystemVerified = 1,
    AwaitingLive = 2,
    SupersededByLive = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScanPublicationReceipt {
    pub(crate) asset_count: u64,
    pub(crate) consolidated_roots: Vec<ConsolidatedLibraryRoot>,
}

pub(crate) struct RetainedScanIssue {
    pub(crate) id: i64,
    pub(crate) path: Option<String>,
}

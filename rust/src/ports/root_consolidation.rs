use crate::domain::{FileIdentityEvidence, LibraryRootGeneration};

pub(crate) struct RootConsolidationRegistration {
    pub(crate) root_id: String,
    pub(crate) root_path: String,
    pub(crate) active_scan_id: Option<String>,
    pub(crate) root_generation: LibraryRootGeneration,
    pub(crate) is_active: bool,
    pub(crate) has_running_scan: bool,
    pub(crate) publication_root_identity: Option<FileIdentityEvidence>,
}

pub(crate) struct RootConsolidationCandidate {
    pub(crate) expected: RootConsolidationRegistration,
    pub(crate) relative_path: String,
}

pub(crate) struct RootConsolidationPlan {
    pub(crate) scan_id: String,
    pub(crate) destination_root_id: String,
    pub(crate) destination_path: String,
    pub(crate) candidates: Vec<RootConsolidationCandidate>,
}

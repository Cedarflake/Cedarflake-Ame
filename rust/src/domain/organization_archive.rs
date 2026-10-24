#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationArchivePhase {
    Snapshot,
    Writing,
    Verifying,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationArchiveProgress {
    pub phase: OrganizationArchivePhase,
    pub completed_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationArchiveReceipt {
    pub archive_id: String,
    pub record_count: u64,
    pub byte_length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationArchiveEvent {
    Progress {
        progress: OrganizationArchiveProgress,
    },
    Completed {
        receipt: OrganizationArchiveReceipt,
    },
    Cancelled,
    Failed {
        code: String,
        message: String,
    },
}

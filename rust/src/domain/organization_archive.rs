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
pub struct OrganizationArchiveInspection {
    pub archive_id: String,
    pub checksum: String,
    pub created_unix_ms: u64,
    pub record_count: u64,
    pub active_albums: u64,
    pub groups: u64,
    pub favorite_groups: u64,
    pub conflicting_groups: u64,
    pub duplicate_choices: u64,
    pub has_continuation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationArchiveEvent {
    Progress {
        progress: OrganizationArchiveProgress,
    },
    Completed {
        receipt: OrganizationArchiveReceipt,
    },
    Inspected {
        inspection: OrganizationArchiveInspection,
    },
    Cancelled,
    Failed {
        code: String,
        message: String,
    },
}

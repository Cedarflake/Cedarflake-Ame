#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationArchivePhase {
    Snapshot,
    Writing,
    Verifying,
    Matching,
    Restoring,
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
    pub associations: OrganizationArchiveAssociations,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrganizationArchiveAssociations {
    pub catalog_revision: u64,
    pub organization_revision: u64,
    pub matched_groups: u64,
    pub unmatched_groups: u64,
    pub resolved_preferred_copies: u64,
    pub unresolved_preferred_copies: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationArchiveRestoreRequest {
    pub operation_id: String,
    pub archive_id: String,
    pub checksum: String,
    pub expected_catalog_revision: u64,
    pub expected_organization_revision: u64,
}

impl OrganizationArchiveRestoreRequest {
    pub(crate) fn validate(&self) -> Result<(), super::ScanError> {
        super::organization::validate_identifier(&self.operation_id)?;
        let is_hex = |value: &str, length| {
            value.len() == length
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        if !is_hex(&self.archive_id, 32)
            || !is_hex(&self.checksum, 64)
            || self.expected_catalog_revision > i64::MAX as u64
            || self.expected_organization_revision > i64::MAX as u64
        {
            return Err(super::ScanError::new(
                "organization_archive_request_invalid",
                "Inspect the backup before requesting restoration",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationArchiveRestoreReceipt {
    pub operation_id: String,
    pub archive_id: String,
    pub organization_revision: u64,
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
    Restored {
        receipt: OrganizationArchiveRestoreReceipt,
    },
    Cancelled,
    Failed {
        code: String,
        message: String,
    },
}

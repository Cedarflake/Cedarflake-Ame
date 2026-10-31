use super::ScanError;
use super::organization::validate_identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationCleanupInspection {
    pub cutoff_unix_ms: i64,
    pub inspected_unix_ms: i64,
    pub catalog_revision: u64,
    pub organization_revision: u64,
    pub selection_digest: String,
    pub groups: u64,
    pub album_groups: u64,
    pub favorite_groups: u64,
    pub progress_groups: u64,
    pub classification_groups: u64,
    pub duplicate_choices: u64,
    pub oldest_unassociated_unix_ms: Option<i64>,
    pub newest_unassociated_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationCleanupRequest {
    pub operation_id: String,
    pub inspection: OrganizationCleanupInspection,
}

impl OrganizationCleanupRequest {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.operation_id)?;
        let inspection = &self.inspection;
        if inspection.cutoff_unix_ms < 0
            || inspection.selection_digest.len() != 64
            || !inspection
                .selection_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ScanError::new(
                "organization_cleanup_request_invalid",
                "Inspect the selected records before cleanup",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationCleanupReceipt {
    pub operation_id: String,
    pub organization_revision: u64,
    pub removed_records: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationCleanupPhase {
    Inspecting,
    Applying,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationCleanupProgress {
    pub phase: OrganizationCleanupPhase,
    pub examined_records: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationCleanupEvent {
    Progress {
        progress: OrganizationCleanupProgress,
    },
    Inspected {
        inspection: OrganizationCleanupInspection,
    },
    Completed {
        receipt: OrganizationCleanupReceipt,
    },
    Cancelled,
    Failed {
        code: String,
        message: String,
    },
}

use super::organization::{OrganizationSelection, OrganizationWriteReceipt, validate_identifier};
use super::{GalleryQuery, ScanError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationReviewSession {
    pub session_id: String,
    pub position_sequence: u64,
}

impl OrganizationReviewSession {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.session_id)?;
        if self.position_sequence > i64::MAX as u64 {
            return Err(stale_session());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationContinuationStatus {
    NoActivity,
    Ready,
    AnchorUnavailable,
    ScopeUnavailable,
    EmptyScope,
    EndOfQueue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationContinuation {
    pub status: OrganizationContinuationStatus,
    pub query: Option<GalleryQuery>,
    pub target: Option<OrganizationSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationReviewWriteReceipt {
    pub operation: OrganizationWriteReceipt,
    pub session: OrganizationReviewSession,
}

pub(crate) fn stale_session() -> ScanError {
    ScanError::new(
        "organization_review_changed",
        "The organization session or position changed; reload the current work",
    )
}

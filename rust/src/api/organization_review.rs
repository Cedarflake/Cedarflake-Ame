use crate::domain::organization::{
    OrganizationAction, OrganizationPreparedSelection, OrganizationSelection,
};
use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationReviewSession, OrganizationReviewWriteReceipt,
};
use crate::domain::{GalleryQuery, ScanError};

pub fn load_organization_admission_boundary() -> Result<u64, ScanError> {
    crate::application::organization_review::admission_boundary()
}

pub fn begin_organization_review(
    query: GalleryQuery,
) -> Result<OrganizationReviewSession, ScanError> {
    crate::application::organization_review::begin(query)
}

pub fn end_organization_review(session_id: String) -> Result<(), ScanError> {
    crate::application::organization_review::end(session_id)
}

pub fn checkpoint_organization_review(
    position: OrganizationReviewSession,
    selection: OrganizationSelection,
) -> Result<OrganizationReviewSession, ScanError> {
    crate::application::organization_review::checkpoint(position, selection)
}

pub fn load_organization_continuation(
    query: Option<GalleryQuery>,
) -> Result<OrganizationContinuation, ScanError> {
    crate::application::organization_review::load(query)
}

pub fn change_current_organization_review(
    operation_id: String,
    session: OrganizationReviewSession,
    prepared: OrganizationPreparedSelection,
    action: OrganizationAction,
) -> Result<OrganizationReviewWriteReceipt, ScanError> {
    crate::application::organization_review::change(operation_id, session, prepared, action)
}

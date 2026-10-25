use crate::domain::organization::{
    OrganizationAction, OrganizationPreparedSelection, OrganizationSelection, validate_identifier,
    validate_selections,
};
use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationReviewSession, OrganizationReviewWriteReceipt,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::organization::open_catalog;

pub(crate) fn admission_boundary() -> Result<u64, ScanError> {
    open_catalog()?.load_organization_admission_boundary()
}

pub(crate) fn begin(query: GalleryQuery) -> Result<OrganizationReviewSession, ScanError> {
    open_catalog()?.begin_organization_review(&query)
}

pub(crate) fn end(session_id: String) -> Result<(), ScanError> {
    validate_identifier(&session_id)?;
    open_catalog()?.end_organization_review(&session_id)
}

pub(crate) fn checkpoint(
    position: OrganizationReviewSession,
    selection: OrganizationSelection,
) -> Result<OrganizationReviewSession, ScanError> {
    position.validate()?;
    validate_selections(std::slice::from_ref(&selection))?;
    open_catalog()?.checkpoint_organization_review(&position, &selection)
}

pub(crate) fn load(query: Option<GalleryQuery>) -> Result<OrganizationContinuation, ScanError> {
    open_catalog()?.load_organization_continuation(query.as_ref())
}

pub(crate) fn change(
    operation_id: String,
    session: OrganizationReviewSession,
    prepared: OrganizationPreparedSelection,
    action: OrganizationAction,
) -> Result<OrganizationReviewWriteReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    session.validate()?;
    validate_selections(&prepared.selections)?;
    action.validate()?;
    open_catalog()?.change_current_organization_review(&operation_id, &session, &prepared, &action)
}

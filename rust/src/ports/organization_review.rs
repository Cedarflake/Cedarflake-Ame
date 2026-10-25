use crate::domain::organization::{
    OrganizationAction, OrganizationPreparedSelection, OrganizationSelection,
};
use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationReviewSession, OrganizationReviewWriteReceipt,
};
use crate::domain::{GalleryQuery, ScanError};

pub(crate) trait OrganizationReviewRepository {
    fn begin_organization_review(
        &mut self,
        query: &GalleryQuery,
    ) -> Result<OrganizationReviewSession, ScanError>;

    fn end_organization_review(&mut self, session_id: &str) -> Result<(), ScanError>;

    fn checkpoint_organization_review(
        &mut self,
        position: &OrganizationReviewSession,
        selection: &OrganizationSelection,
    ) -> Result<OrganizationReviewSession, ScanError>;

    fn load_organization_continuation(
        &self,
        query: Option<&GalleryQuery>,
    ) -> Result<OrganizationContinuation, ScanError>;

    fn change_current_organization_review(
        &mut self,
        operation_id: &str,
        session: &OrganizationReviewSession,
        prepared: &OrganizationPreparedSelection,
        action: &OrganizationAction,
    ) -> Result<OrganizationReviewWriteReceipt, ScanError>;
}

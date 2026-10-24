use crate::application::organization_batch::OrganizationBatchSession;
use crate::domain::organization::OrganizationWriteReceipt;
use crate::domain::organization_batch::{
    OrganizationBatchAction, OrganizationBatchProgress, OrganizationBatchSummary,
};
use crate::domain::{GalleryQuery, ScanError};

#[flutter_rust_bridge::frb(opaque)]
pub struct OrganizationSelectionDraft {
    inner: OrganizationBatchSession,
}

pub fn open_gallery_organization_selection(
    query: GalleryQuery,
    catalog_revision: u64,
    all_matching: bool,
) -> Result<OrganizationSelectionDraft, ScanError> {
    OrganizationBatchSession::open(query, catalog_revision, all_matching)
        .map(|inner| OrganizationSelectionDraft { inner })
}

impl OrganizationSelectionDraft {
    pub fn append_ids(&self, asset_ids: Vec<String>) -> Result<(), ScanError> {
        self.inner.append_ids(asset_ids)
    }

    pub fn prepare(&self) -> Result<OrganizationBatchSummary, ScanError> {
        self.inner.prepare()
    }

    pub fn apply(
        &self,
        operation_id: String,
        action: OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        self.inner.apply(operation_id, action)
    }

    pub fn progress(&self) -> OrganizationBatchProgress {
        self.inner.progress()
    }

    pub fn close(&self) -> Result<(), ScanError> {
        self.inner.close()
    }
}

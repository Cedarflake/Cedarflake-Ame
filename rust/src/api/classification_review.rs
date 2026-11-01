use crate::application::classification_selection::ClassificationSelectionSession;
use crate::domain::classification::{CategoryId, ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::classification_selection::{
    ClassificationAnnotation, ClassificationItemState, ClassificationSelectionProgress,
    ClassificationSelectionSummary,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{GalleryQuery, ScanError};

#[flutter_rust_bridge::frb(opaque)]
pub struct ClassificationSelectionDraft {
    inner: ClassificationSelectionSession,
}

pub fn open_classification_selection(
    query: GalleryQuery,
    catalog_revision: u64,
    all_matching: bool,
) -> Result<ClassificationSelectionDraft, ScanError> {
    Ok(ClassificationSelectionDraft {
        inner: ClassificationSelectionSession::open(query, catalog_revision, all_matching)?,
    })
}

pub fn load_classification_items(
    selections: Vec<OrganizationSelection>,
) -> Result<Vec<ClassificationItemState>, ScanError> {
    crate::application::classification_selection::load_items(&selections)
}

pub fn load_classification_annotations(
    catalog_revision: u64,
    selections: Vec<OrganizationSelection>,
) -> Result<Vec<ClassificationAnnotation>, ScanError> {
    crate::application::classification_selection::load_annotations(catalog_revision, &selections)
}

pub fn undo_classification_change(
    operation_id: String,
    original_operation_id: String,
) -> Result<ClassificationWriteReceipt, ScanError> {
    crate::application::classification_selection::undo(
        &identifier(operation_id)?,
        &identifier(original_operation_id)?,
    )
}

impl ClassificationSelectionDraft {
    pub fn append_targets(&self, targets: Vec<OrganizationSelection>) -> Result<(), ScanError> {
        self.inner.append_targets(targets)
    }
    pub fn append_ids(&self, asset_ids: Vec<String>) -> Result<(), ScanError> {
        self.inner.append_ids(asset_ids)
    }
    pub fn prepare(&self) -> Result<ClassificationSelectionSummary, ScanError> {
        self.inner.prepare()
    }
    pub fn confirm(&self, operation_id: String) -> Result<ClassificationWriteReceipt, ScanError> {
        self.inner
            .apply(identifier(operation_id)?, ClassificationAction::Confirm)
    }
    pub fn correct(
        &self,
        operation_id: String,
        category_id: String,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        let category = CategoryId::new(category_id).map_err(|error| {
            ScanError::new("classification_category_invalid", error.to_string())
        })?;
        self.inner.apply(
            identifier(operation_id)?,
            ClassificationAction::Correct(category),
        )
    }
    pub fn defer(&self, operation_id: String) -> Result<ClassificationWriteReceipt, ScanError> {
        self.inner
            .apply(identifier(operation_id)?, ClassificationAction::Defer)
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn progress(&self) -> ClassificationSelectionProgress {
        self.inner.progress()
    }
    pub fn close(&self) -> Result<(), ScanError> {
        self.inner.close()
    }
}

fn identifier(value: String) -> Result<ClassificationRecordId, ScanError> {
    ClassificationRecordId::new(value)
        .map_err(|error| ScanError::new("classification_operation_invalid", error.to_string()))
}

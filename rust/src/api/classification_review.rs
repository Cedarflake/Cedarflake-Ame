use crate::domain::ScanError;
use crate::domain::classification_selection::{ClassificationAnnotation, ClassificationItemState};
use crate::domain::organization::OrganizationSelection;

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

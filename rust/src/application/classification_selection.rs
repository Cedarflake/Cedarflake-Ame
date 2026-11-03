use crate::domain::ScanError;
use crate::domain::classification_selection::{ClassificationAnnotation, ClassificationItemState};
use crate::domain::organization::OrganizationSelection;

pub(crate) fn load_annotations(
    revision: u64,
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationAnnotation>, ScanError> {
    super::organization::open_catalog()?.classification_annotations(revision, selections)
}

pub(crate) fn load_items(
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationItemState>, ScanError> {
    super::organization::open_catalog()?.classification_items(selections)
}

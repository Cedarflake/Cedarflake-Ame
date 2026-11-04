use crate::domain::ScanError;
use crate::domain::classification_selection::{ClassificationAnnotation, ClassificationItemState};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_selection::ClassificationSelectionReader;

pub(crate) fn load_annotations(
    revision: u64,
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationAnnotation>, ScanError> {
    let reader: &dyn ClassificationSelectionReader = &super::organization::open_catalog()?;
    reader.classification_annotations(revision, selections)
}

pub(crate) fn load_items(
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationItemState>, ScanError> {
    let reader: &dyn ClassificationSelectionReader = &super::organization::open_catalog()?;
    reader.classification_items(selections)
}

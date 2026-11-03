use crate::domain::ScanError;
use crate::domain::classification_selection::{ClassificationAnnotation, ClassificationItemState};
use crate::domain::organization::OrganizationSelection;

pub(crate) trait ClassificationSelectionReader {
    fn classification_annotations(
        &self,
        revision: u64,
        selections: &[OrganizationSelection],
    ) -> Result<Vec<ClassificationAnnotation>, ScanError>;

    fn classification_items(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<Vec<ClassificationItemState>, ScanError>;
}

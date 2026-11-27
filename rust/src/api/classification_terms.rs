use crate::domain::ScanError;
use crate::domain::classification_configuration::{
    ClassificationCategoryInput, ClassificationSettings, ClassificationTerm,
};

#[cfg(all(windows, feature = "classification-worker"))]
use crate::application::classification_configuration::terms;

pub fn load_classification_terms() -> Result<Vec<ClassificationTerm>, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        terms::load()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn save_classification_term(
    operation_id: String,
    expected: Option<ClassificationCategoryInput>,
    replacement: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        terms::save(operation_id, expected, replacement)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (operation_id, expected, replacement);
        Err(super::classification::unsupported())
    }
}

pub fn remove_classification_term(
    operation_id: String,
    expected: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        terms::remove(operation_id, expected)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (operation_id, expected);
        Err(super::classification::unsupported())
    }
}

pub fn reset_classification_term(
    operation_id: String,
    expected: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        terms::reset(operation_id, expected)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (operation_id, expected);
        Err(super::classification::unsupported())
    }
}

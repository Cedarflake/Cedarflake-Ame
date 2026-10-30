use crate::domain::ScanError;
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

pub trait ClassificationConfigurationRepository {
    fn load_classification_configuration(
        &self,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;

    fn save_classification_configuration(
        &mut self,
        change: &ClassificationConfigurationChange,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;
}

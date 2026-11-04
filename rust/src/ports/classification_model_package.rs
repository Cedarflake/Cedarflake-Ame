use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ModelCompatibility;
use crate::domain::classification::installation::{
    ClassificationInstallation, ModelInstallationProgress,
};

pub(crate) trait PreparedClassificationModel: Send + Sync {
    /// File and namespace guards remain held until this owner is dropped after publication or disposal.
    fn installation(&self) -> ClassificationInstallation;
}

pub(crate) trait ClassificationModelPackage {
    type Prepared: PreparedClassificationModel + 'static;

    fn compatibility(&self) -> ModelCompatibility;
    fn package_directory(&self) -> &'static str;
    fn verify(&self, installation: &ClassificationInstallation) -> Result<(), ScanError>;

    fn prepare_download(
        &self,
        model_storage: &Path,
        runtime: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(ModelInstallationProgress),
    ) -> Result<Self::Prepared, ScanError>;

    fn prepare_bundled(
        &self,
        models: &Path,
        runtime: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(ModelInstallationProgress),
    ) -> Result<Self::Prepared, ScanError>;
}

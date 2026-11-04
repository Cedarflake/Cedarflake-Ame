use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ModelCompatibility;
use crate::domain::classification::installation::{
    ClassificationInstallation, ModelInstallationProgress,
};
use crate::ports::classification_model_package::{
    ClassificationModelPackage, PreparedClassificationModel,
};

use super::installation::{self, VerifiedModelInstallation};
use super::{EncoderKind, artifacts, factory, manifest};

pub(crate) struct LocalClassificationModelPackage;

impl ClassificationModelPackage for LocalClassificationModelPackage {
    type Prepared = VerifiedModelInstallation;

    fn compatibility(&self) -> ModelCompatibility {
        manifest::compatibility()
    }

    fn package_directory(&self) -> &'static str {
        installation::PACKAGE_DIRECTORY
    }

    fn verify(&self, installation: &ClassificationInstallation) -> Result<(), ScanError> {
        let _image = artifacts::ArtifactSet::open(
            &installation.models,
            &installation.runtime,
            EncoderKind::Image,
        )
        .map_err(factory::runtime_error)?;
        let _text = artifacts::ArtifactSet::open(
            &installation.models,
            &installation.runtime,
            EncoderKind::Text,
        )
        .map_err(factory::runtime_error)?;
        Ok(())
    }

    fn prepare_download(
        &self,
        model_storage: &Path,
        runtime: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(ModelInstallationProgress),
    ) -> Result<Self::Prepared, ScanError> {
        installation::prepare(model_storage, runtime, cancelled, progress)
    }

    fn prepare_bundled(
        &self,
        models: &Path,
        runtime: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(ModelInstallationProgress),
    ) -> Result<Self::Prepared, ScanError> {
        installation::prepare_bundled(models, runtime, cancelled, progress)
    }
}

impl PreparedClassificationModel for VerifiedModelInstallation {
    fn installation(&self) -> ClassificationInstallation {
        ClassificationInstallation {
            models: self.models.clone(),
            runtime: self.runtime.clone(),
        }
    }
}

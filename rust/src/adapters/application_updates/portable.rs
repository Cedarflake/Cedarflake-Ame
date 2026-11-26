use std::path::PathBuf;

use velopack::locator::{
    LocationContext, VelopackLocatorConfig, auto_locate_app_manifest, create_config_from_root_dir,
};

use super::package::reject_reparse;
use crate::domain::application_update::{ApplicationUpdateError, ApplicationUpdateFailureCode};

#[derive(Clone)]
pub(super) struct PortableInstallation {
    config: VelopackLocatorConfig,
}

impl PortableInstallation {
    pub fn locate() -> Result<Option<Self>, ApplicationUpdateError> {
        let locator = match auto_locate_app_manifest(LocationContext::FromCurrentExe) {
            Ok(locator) => locator,
            Err(velopack::Error::NotInstalled(_)) => return Ok(None),
            Err(error) => return Err(installation(error)),
        };
        if !locator.get_is_portable() || locator.get_manifest_id() != "CedarflakeAme" {
            return Err(installation("Unsupported portable package identity"));
        }
        let root = locator.get_root_dir();
        reject_reparse(&root)?;
        Ok(Some(Self {
            config: create_config_from_root_dir(root),
        }))
    }

    pub fn package_directory(&self) -> PathBuf {
        self.config.PackagesDir.clone()
    }

    pub fn application_path(&self) -> PathBuf {
        self.config.CurrentBinaryDir.join("cedarflake_ame.exe")
    }
}

fn installation(error: impl std::fmt::Display) -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::Installation,
        format!("Portable update: {error}"),
    )
}

pub(crate) fn startup() {
    velopack::VelopackApp::build()
        .set_auto_apply_on_startup(false)
        .run();
}

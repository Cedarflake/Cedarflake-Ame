mod distribution;
mod handoff;
mod manifest;
mod package;
mod portable;
mod portable_engine;
mod result;
mod retention;
mod store;
mod transport;
pub(crate) use distribution::{open, startup};

use std::path::PathBuf;
use std::sync::Mutex;

use crate::domain::application_update::{
    ApplicationRelease, ApplicationUpdateChannel, ApplicationUpdateDelivery,
    ApplicationUpdateError, ApplicationUpdateFailureCode, is_newer_release,
};
use crate::ports::application_updates::{
    ApplicationDownloadObserver, ApplicationUpdatePackage, ApplicationUpdateSource,
};
use manifest::{GithubRelease, REPOSITORY, UpdateManifest, asset_url, invalid};
use package::{RetainedPackage, cache_directory, reject_reparse, storage};
use portable::PortableInstallation;
use transport::UpdateTransport;

pub(crate) struct WindowsApplicationUpdates {
    version: String,
    build: u64,
    installation: Option<Installation>,
}

#[derive(Clone)]
enum Installation {
    Installed(PathBuf),
    Portable(PortableInstallation),
}

impl WindowsApplicationUpdates {
    pub fn new(version: String, build: u64) -> Result<Self, ApplicationUpdateError> {
        if version != env!("CARGO_PKG_VERSION") || build == 0 || build > 65535 {
            return Err(invalid());
        }
        let application = std::env::current_exe().map_err(storage)?;
        let installation = if let Some(portable) = PortableInstallation::locate()? {
            Some(Installation::Portable(portable))
        } else if handoff::is_installed(&application)? {
            Some(Installation::Installed(application))
        } else {
            None
        };
        Ok(Self {
            version,
            build,
            installation,
        })
    }
}

impl ApplicationUpdateSource for WindowsApplicationUpdates {
    fn take_result(
        &self,
    ) -> Result<
        Option<crate::domain::application_update::ApplicationUpdateResult>,
        ApplicationUpdateError,
    > {
        result::take_result(&self.version)
    }
    fn delivery(&self) -> ApplicationUpdateDelivery {
        if self.installation.is_some() {
            ApplicationUpdateDelivery::Direct
        } else {
            ApplicationUpdateDelivery::Unavailable
        }
    }

    fn check(
        &self,
        channel: ApplicationUpdateChannel,
    ) -> Result<Option<Box<dyn ApplicationUpdatePackage>>, ApplicationUpdateError> {
        let installation = self.installation.as_ref().ok_or_else(|| {
            ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::Unavailable,
                "Application updates require a packaged installation",
            )
        })?;
        let transport = UpdateTransport::new()?;
        let mut releases: Vec<GithubRelease> = if channel == ApplicationUpdateChannel::Stable {
            transport
                .optional_json::<GithubRelease>(
                    &format!("https://api.github.com/repos/{REPOSITORY}/releases/latest"),
                    1024 * 1024,
                )?
                .into_iter()
                .collect()
        } else {
            transport.json(
                &format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=100"),
                4 * 1024 * 1024,
            )?
        };
        releases.retain(|release| {
            !release.draft && (channel == ApplicationUpdateChannel::Preview || !release.prerelease)
        });
        releases.sort_by_cached_key(|release| {
            std::cmp::Reverse(
                release
                    .tag_name
                    .strip_prefix('v')
                    .and_then(|v| semver::Version::parse(v).ok()),
            )
        });
        let architecture = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };
        let metadata_name = format!("ame-update-windows-{architecture}.json");
        for github in releases {
            let Some(version) = github
                .tag_name
                .strip_prefix('v')
                .and_then(|v| semver::Version::parse(v).ok())
            else {
                continue;
            };
            let current = semver::Version::parse(&self.version).map_err(|_| invalid())?;
            if version.cmp_precedence(&current) != std::cmp::Ordering::Greater {
                continue;
            }
            if !github
                .assets
                .iter()
                .any(|asset| asset.name == metadata_name)
            {
                continue;
            }
            let metadata = github.asset(&metadata_name)?;
            if metadata.size == 0 || metadata.size > 16 * 1024 {
                return Err(invalid());
            }
            let manifest: UpdateManifest =
                transport.json(&metadata.browser_download_url, 16 * 1024)?;
            let portable = matches!(installation, Installation::Portable(_));
            let (release, asset) = manifest.admit(&github, architecture, portable)?;
            if !is_newer_release(&self.version, self.build, &release, channel)? {
                continue;
            }
            let directory = match installation {
                Installation::Portable(portable) => portable.package_directory(),
                Installation::Installed(_) => cache_directory()?.join(&asset.sha256),
            };
            reject_reparse(&directory)?;
            let package = RetainedPackage {
                url: asset_url(&github.tag_name, &asset.file_name)?,
                path: directory.join(&asset.file_name),
                asset,
                transport,
            };
            return Ok(Some(Box::new(WindowsUpdatePackage {
                installation: installation.clone(),
                release,
                package,
                handoff: Mutex::new(None),
            })));
        }
        Ok(None)
    }
}

struct WindowsUpdatePackage {
    installation: Installation,
    release: ApplicationRelease,
    package: RetainedPackage,
    handoff: Mutex<Option<handoff::PreparedHandoff>>,
}

impl ApplicationUpdatePackage for WindowsUpdatePackage {
    fn release(&self) -> ApplicationRelease {
        self.release.clone()
    }
    fn download(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError> {
        reject_reparse(&self.package.path)?;
        let (root, portable) = match &self.installation {
            Installation::Installed(_) => (cache_directory()?, false),
            Installation::Portable(installation) => (installation.package_directory(), true),
        };
        retention::retire_superseded(&root, &self.package.path, portable)?;
        self.package.download(observer)
    }
    fn verify(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError> {
        reject_reparse(&self.package.path)?;
        self.package.verified_file(observer).map(|_| ())
    }
    fn prepare_handoff(&self) -> Result<(), ApplicationUpdateError> {
        let mut prepared = self.handoff.lock().map_err(storage)?;
        if prepared.is_none() {
            let (application, portable) = match &self.installation {
                Installation::Installed(application) => (application.clone(), false),
                Installation::Portable(portable) => (portable.application_path(), true),
            };
            *prepared = Some(handoff::prepare(
                &application,
                &self.package,
                &self.release.version,
                portable,
            )?);
        }
        Ok(())
    }
    fn apply_after_exit(&self) -> Result<(), ApplicationUpdateError> {
        self.handoff
            .lock()
            .map_err(storage)?
            .as_ref()
            .ok_or_else(|| storage("Update handoff was not prepared"))?
            .commit()
    }
}

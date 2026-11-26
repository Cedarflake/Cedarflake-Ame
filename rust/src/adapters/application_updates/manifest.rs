use serde::Deserialize;

use crate::domain::application_update::{
    ApplicationRelease, ApplicationUpdateError, ApplicationUpdateFailureCode,
};

pub(super) const REPOSITORY: &str = "Cedarflake/Cedarflake-Ame";
pub(super) const MAX_PACKAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackageAsset {
    pub file_name: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateManifest {
    pub schema: u32,
    pub version: String,
    pub build: u64,
    pub architecture: String,
    pub installed: PackageAsset,
    pub portable: PackageAsset,
}

#[derive(Deserialize)]
pub(super) struct GithubRelease {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub body: Option<String>,
    pub assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
pub(super) struct GithubAsset {
    pub name: String,
    pub size: u64,
    pub state: String,
    pub digest: Option<String>,
    pub browser_download_url: String,
}

impl GithubRelease {
    pub fn asset(&self, name: &str) -> Result<&GithubAsset, ApplicationUpdateError> {
        let mut matches = self.assets.iter().filter(|asset| asset.name == name);
        let asset = matches.next().ok_or_else(invalid)?;
        if matches.next().is_some()
            || asset.state != "uploaded"
            || asset.browser_download_url != asset_url(&self.tag_name, name)?
        {
            return Err(invalid());
        }
        Ok(asset)
    }
}

impl UpdateManifest {
    pub fn admit(
        &self,
        github: &GithubRelease,
        architecture: &str,
        portable: bool,
    ) -> Result<(ApplicationRelease, PackageAsset), ApplicationUpdateError> {
        let version = semver::Version::parse(&self.version).map_err(|_| invalid())?;
        if self.schema != 1
            || self.architecture != architecture
            || !version.build.is_empty()
            || github.tag_name != format!("v{}", self.version)
            || github.prerelease == version.pre.is_empty()
            || version.major > 255
            || version.minor > 255
            || self.build == 0
            || self.build > 65535
        {
            return Err(invalid());
        }
        let package = if portable {
            &self.portable
        } else {
            &self.installed
        };
        package.admit()?;
        let suffix = if portable { "-full.nupkg" } else { ".msi" };
        if !package.file_name.ends_with(suffix) {
            return Err(invalid());
        }
        let published = github.asset(&package.file_name)?;
        if published.size != package.size
            || published.digest.as_deref() != Some(format!("sha256:{}", package.sha256).as_str())
        {
            return Err(invalid());
        }
        let notes = github.body.clone().unwrap_or_default();
        if notes.len() > 64 * 1024 {
            return Err(invalid());
        }
        Ok((
            ApplicationRelease {
                version: self.version.clone(),
                build: self.build,
                download_bytes: package.size,
                notes,
            },
            package.clone(),
        ))
    }
}

impl PackageAsset {
    pub fn admit(&self) -> Result<(), ApplicationUpdateError> {
        if !is_leaf(&self.file_name)
            || self.size == 0
            || self.size > MAX_PACKAGE_BYTES
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        Ok(())
    }
}

pub(super) fn asset_url(tag: &str, name: &str) -> Result<String, ApplicationUpdateError> {
    let version = tag
        .strip_prefix('v')
        .and_then(|v| semver::Version::parse(v).ok())
        .ok_or_else(invalid)?;
    if !version.build.is_empty() || !is_leaf(name) {
        return Err(invalid());
    }
    Ok(format!(
        "https://github.com/{REPOSITORY}/releases/download/{tag}/{name}"
    ))
}

fn is_leaf(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 180
        && !value.contains("..")
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

pub(super) fn invalid() -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::InvalidRelease,
        "Application release metadata is invalid",
    )
}

#[cfg(test)]
mod tests;

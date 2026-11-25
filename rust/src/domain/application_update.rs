use std::cmp::Ordering;

use semver::Version;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationUpdateChannel {
    Stable,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationUpdateOutcome {
    Updated,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ApplicationUpdateResult {
    pub outcome: ApplicationUpdateOutcome,
    pub code: u32,
}

#[derive(Clone, Debug)]
pub struct ApplicationRelease {
    pub version: String,
    pub build: u64,
    pub download_bytes: u64,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationUpdateFailureCode {
    Unavailable,
    Network,
    InvalidRelease,
    DamagedDownload,
    Storage,
    Installation,
    Shutdown,
    Cancelled,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("{diagnostic}")]
pub struct ApplicationUpdateError {
    pub code: ApplicationUpdateFailureCode,
    pub diagnostic: String,
}

impl ApplicationUpdateError {
    pub fn new(code: ApplicationUpdateFailureCode, diagnostic: impl Into<String>) -> Self {
        Self {
            code,
            diagnostic: diagnostic.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationDownloadPhase {
    Downloading,
    Verifying,
    Ready,
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct ApplicationDownloadProgress {
    pub phase: ApplicationDownloadPhase,
    pub received_bytes: u64,
    pub total_bytes: u64,
}

pub(crate) fn is_newer_release(
    current_version: &str,
    current_build: u64,
    release: &ApplicationRelease,
    channel: ApplicationUpdateChannel,
) -> Result<bool, ApplicationUpdateError> {
    let parse = |value: &str| {
        Version::parse(value).map_err(|error| {
            ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::InvalidRelease,
                format!("Invalid application version: {error}"),
            )
        })
    };
    let current = parse(current_version)?;
    let target = parse(&release.version)?;
    if !target.build.is_empty() || release.build == 0 || release.download_bytes == 0 {
        return Err(ApplicationUpdateError::new(
            ApplicationUpdateFailureCode::InvalidRelease,
            "The release has invalid build or package metadata",
        ));
    }
    if channel == ApplicationUpdateChannel::Stable && !target.pre.is_empty() {
        return Ok(false);
    }
    match target.cmp_precedence(&current) {
        Ordering::Less => Ok(false),
        Ordering::Equal => Ok(false),
        Ordering::Greater => {
            let same_line = (target.major, target.minor) == (current.major, current.minor);
            Ok(!same_line || release.build > current_build)
        }
    }
}

#[cfg(test)]
mod tests;

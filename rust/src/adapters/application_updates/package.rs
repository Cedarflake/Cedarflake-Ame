use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::manifest::PackageAsset;
use super::transport::{UpdateTransport, network};
use crate::domain::application_update::{ApplicationUpdateError, ApplicationUpdateFailureCode};
use crate::ports::application_updates::ApplicationDownloadObserver;

mod storage_lease;

pub(super) struct RetainedPackage {
    pub asset: PackageAsset,
    pub path: PathBuf,
    pub url: String,
    pub transport: UpdateTransport,
}

impl RetainedPackage {
    pub fn download(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError> {
        if self.path.exists() {
            match self.verified_file(observer) {
                Ok(_) => {
                    observer.received(self.asset.size);
                    return Ok(());
                }
                Err(error) if error.code == ApplicationUpdateFailureCode::DamagedDownload => {
                    fs::remove_file(&self.path).map_err(storage)?;
                }
                Err(error) => return Err(error),
            }
        }
        let directory = self
            .path
            .parent()
            .ok_or_else(|| storage("Missing update directory"))?;
        fs::create_dir_all(directory).map_err(storage)?;
        let partial = self.path.with_extension("partial");
        let (mut file, _namespace) = storage_lease::open_partial(&partial)?;
        let length = file.metadata().map_err(storage)?.len();
        if length > self.asset.size {
            file.set_len(0).map_err(storage)?;
        }
        let offset = file.metadata().map_err(storage)?.len();
        if offset < self.asset.size {
            cancelled(observer)?;
            let (mut response, start) = self.transport.open(&self.url, offset, self.asset.size)?;
            file.set_len(start).map_err(storage)?;
            file.seek(SeekFrom::Start(start)).map_err(storage)?;
            let mut received = start;
            let mut buffer = [0_u8; 128 * 1024];
            loop {
                cancelled(observer)?;
                let count = response.read(&mut buffer).map_err(|_| network())?;
                if count == 0 {
                    break;
                }
                received += count as u64;
                if received > self.asset.size {
                    return Err(damaged());
                }
                file.write_all(&buffer[..count]).map_err(storage)?;
                observer.received(received);
            }
        }
        file.sync_all().map_err(storage)?;
        observer.verifying();
        if let Err(error) = verify(&mut file, &self.asset, observer) {
            drop(file);
            if error.code == ApplicationUpdateFailureCode::DamagedDownload {
                fs::remove_file(&partial).map_err(storage)?;
            }
            return Err(error);
        }
        drop(file);
        fs::rename(&partial, &self.path).map_err(storage)?;
        Ok(())
    }

    pub fn verified_file(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<File, ApplicationUpdateError> {
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&self.path)
            .map_err(storage)?;
        observer.verifying();
        verify(&mut file, &self.asset, observer)?;
        Ok(file)
    }
}

pub(super) fn verify(
    file: &mut File,
    asset: &PackageAsset,
    observer: &dyn ApplicationDownloadObserver,
) -> Result<(), ApplicationUpdateError> {
    if file.metadata().map_err(storage)?.len() != asset.size {
        return Err(damaged());
    }
    file.seek(SeekFrom::Start(0)).map_err(storage)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        cancelled(observer)?;
        let count = file.read(&mut buffer).map_err(storage)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != asset.sha256 {
        return Err(damaged());
    }
    Ok(())
}

pub(super) fn storage(error: impl std::fmt::Display) -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::Storage,
        format!("Application update storage: {error}"),
    )
}

fn damaged() -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::DamagedDownload,
        "The downloaded application package does not match its release",
    )
}

pub(super) fn cancelled(
    observer: &dyn ApplicationDownloadObserver,
) -> Result<(), ApplicationUpdateError> {
    if observer.cancelled() {
        Err(ApplicationUpdateError::new(
            ApplicationUpdateFailureCode::Cancelled,
            "Application download was cancelled",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn cache_directory() -> Result<PathBuf, ApplicationUpdateError> {
    directories::ProjectDirs::from("com", "Cedarflake", "Ame")
        .map(|dirs| dirs.cache_dir().join("application-updates"))
        .ok_or_else(|| storage("User cache directory is unavailable"))
}

pub(super) fn reject_reparse(path: &Path) -> Result<(), ApplicationUpdateError> {
    use std::os::windows::fs::MetadataExt;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(storage("Redirected update storage is not supported"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(storage(error)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
